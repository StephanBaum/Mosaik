"""Independent reference for the M6b share, identity and roster constructions
(CRYPTO_SPEC 8.1, 8.2, 8.4, 8.5, 10).

It shares no code with mosaic-core and uses only the Python standard library.
X25519 (RFC 7748), ChaCha20-Poly1305 (RFC 8439), XChaCha20 (draft-irtf-cfrg-
xchacha) and HPKE mode Auth (RFC 9180) are written out here, and the script
checks them against published vectors before it emits anything: RFC 7748
section 5.2, the XChaCha20-Poly1305 draft vector, and RFC 9180 A.2.3
(DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305, mode Auth), which
covers key derivation, the auth encapsulation, the key schedule and the AEAD.

    python -I crates/core/tests/vectors/share_kat.py > crates/core/tests/vectors/share_kat.txt
    python -I crates/core/tests/vectors/share_kat.py --check

`--check` regenerates the vectors and fails if `share_kat.txt` differs. The unit
tests in `src/share.rs`, `src/identity.rs`, `src/envelope.rs` and
`src/sync_auth.rs` read `share_kat.txt` and compare it with the Rust code
(`share::kat::mosaic_vectors_match_python`, `sync_auth::kat::view2_digest`).

Fixed inputs. Accounts: sender (and grantor) 0xA1 x 16, recipient (and grantee)
0xB2 x 16; vault 0x0B x 16; key generation 3; vault key VK = 0xC3 x 32; account
key AK = 0x11 x 32. HPKE key derivation inputs (`ikm`): sender 0x51 x 32,
recipient 0x52 x 32, ephemeral key of the share seal 0x53 x 32, of the emergency
seal 0x54 x 32. XChaCha nonces: identity wrap 0x61 x 24, roster 0x62 x 24,
contacts 0x63 x 24. Roster plaintext b'{"roster":"kat"}', contacts plaintext
b'{"contacts":"kat"}'. View digest: vault 0x0B x 16, high_seq 8, key commit
0x44 x 32, metadata at seq 5 with SHA-256("meta"), roster at seq 6 with
SHA-256("roster"), and two served rows (0x0D x 16 at seq 3 with SHA-256("first"),
0x0F x 16 at seq 8 with SHA-256("second")).

`isk` is the clamped X25519 scalar, which is what the `hpke` crate serializes
and mosaic-core wraps (RFC 9180 DeriveKeyPair returns the unclamped bytes; both
describe the same key).
"""

import hashlib
import hmac
import pathlib
import struct
import sys

CTX = b"mosaic/v1/"


def ctx(label: str) -> bytes:
    return CTX + label.encode()


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


# ---------------------------------------------------------------- HKDF (RFC 5869)


def hkdf_extract(salt: bytes, ikm: bytes) -> bytes:
    return hmac.new(salt if salt else b"\x00" * 32, ikm, hashlib.sha256).digest()


def hkdf_expand(prk: bytes, info: bytes, length: int) -> bytes:
    okm, block = b"", b""
    counter = 1
    while len(okm) < length:
        block = hmac.new(prk, block + info + bytes([counter]), hashlib.sha256).digest()
        okm += block
        counter += 1
    return okm[:length]


def hkdf(ikm: bytes, salt: bytes, info: bytes, length: int) -> bytes:
    return hkdf_expand(hkdf_extract(salt, ikm), info, length)


# ---------------------------------------------------------------- X25519 (RFC 7748)

P = 2**255 - 19
A24 = 121665


def x25519(k: bytes, u: bytes) -> bytes:
    kb = bytearray(k)
    kb[0] &= 248
    kb[31] &= 127
    kb[31] |= 64
    scalar = int.from_bytes(kb, "little")
    x1 = int.from_bytes(u, "little") & ((1 << 255) - 1)
    x2, z2, x3, z3, swap = 1, 0, x1, 1, 0
    for t in range(254, -1, -1):
        bit = (scalar >> t) & 1
        if swap ^ bit:
            x2, x3, z2, z3 = x3, x2, z3, z2
        swap = bit
        a = (x2 + z2) % P
        aa = a * a % P
        b = (x2 - z2) % P
        bb = b * b % P
        e = (aa - bb) % P
        c = (x3 + z3) % P
        d = (x3 - z3) % P
        da = d * a % P
        cb = c * b % P
        x3 = (da + cb) ** 2 % P
        z3 = x1 * (da - cb) ** 2 % P
        x2 = aa * bb % P
        z2 = e * (aa + A24 * e) % P
    if swap:
        x2, x3, z2, z3 = x3, x2, z3, z2
    return (x2 * pow(z2, P - 2, P) % P).to_bytes(32, "little")


BASE = (9).to_bytes(32, "little")


def clamp(k: bytes) -> bytes:
    kb = bytearray(k)
    kb[0] &= 248
    kb[31] &= 127
    kb[31] |= 64
    return bytes(kb)


# ---------------------------------------------------------------- ChaCha20-Poly1305


def rotl(v: int, n: int) -> int:
    return ((v << n) & 0xFFFFFFFF) | (v >> (32 - n))


def quarter(s, a, b, c, d):
    s[a] = (s[a] + s[b]) & 0xFFFFFFFF
    s[d] = rotl(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = rotl(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b]) & 0xFFFFFFFF
    s[d] = rotl(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = rotl(s[b] ^ s[c], 7)


CONSTS = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574]


def rounds(s):
    for _ in range(10):
        quarter(s, 0, 4, 8, 12)
        quarter(s, 1, 5, 9, 13)
        quarter(s, 2, 6, 10, 14)
        quarter(s, 3, 7, 11, 15)
        quarter(s, 0, 5, 10, 15)
        quarter(s, 1, 6, 11, 12)
        quarter(s, 2, 7, 8, 13)
        quarter(s, 3, 4, 9, 14)


def chacha_block(key: bytes, counter: int, nonce12: bytes) -> bytes:
    init = CONSTS + list(struct.unpack("<8I", key)) + [counter] + list(struct.unpack("<3I", nonce12))
    s = list(init)
    rounds(s)
    return struct.pack("<16I", *[(a + b) & 0xFFFFFFFF for a, b in zip(s, init)])


def hchacha20(key: bytes, nonce16: bytes) -> bytes:
    s = CONSTS + list(struct.unpack("<8I", key)) + list(struct.unpack("<4I", nonce16))
    rounds(s)
    return struct.pack("<8I", *(s[0:4] + s[12:16]))


def poly1305(key: bytes, msg: bytes) -> bytes:
    r = int.from_bytes(key[:16], "little") & 0x0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF
    s = int.from_bytes(key[16:], "little")
    acc = 0
    p = (1 << 130) - 5
    for i in range(0, len(msg), 16):
        block = msg[i : i + 16]
        n = int.from_bytes(block + b"\x01", "little")
        acc = (acc + n) * r % p
    return ((acc + s) & ((1 << 128) - 1)).to_bytes(16, "little")


def pad16(b: bytes) -> bytes:
    return b"\x00" * (-len(b) % 16)


def aead_seal(key: bytes, nonce12: bytes, aad: bytes, pt: bytes) -> bytes:
    otk = chacha_block(key, 0, nonce12)[:32]
    ct = bytearray()
    for i in range(0, len(pt), 64):
        ks = chacha_block(key, 1 + i // 64, nonce12)
        ct += bytes(a ^ b for a, b in zip(pt[i : i + 64], ks))
    ct = bytes(ct)
    mac = poly1305(
        otk, aad + pad16(aad) + ct + pad16(ct) + struct.pack("<QQ", len(aad), len(ct))
    )
    return ct + mac


def xaead_seal(key: bytes, nonce24: bytes, aad: bytes, pt: bytes) -> bytes:
    return aead_seal(hchacha20(key, nonce24[:16]), b"\x00" * 4 + nonce24[16:], aad, pt)


# ---------------------------------------------------------------- HPKE (RFC 9180), mode Auth

KEM_ID, KDF_ID, AEAD_ID = 0x0020, 0x0001, 0x0003
KEM_SUITE = b"KEM" + struct.pack(">H", KEM_ID)
HPKE_SUITE = b"HPKE" + struct.pack(">HHH", KEM_ID, KDF_ID, AEAD_ID)


def labeled_extract(suite: bytes, salt: bytes, label: bytes, ikm: bytes) -> bytes:
    return hkdf_extract(salt, b"HPKE-v1" + suite + label + ikm)


def labeled_expand(suite: bytes, prk: bytes, label: bytes, info: bytes, length: int) -> bytes:
    return hkdf_expand(
        prk, struct.pack(">H", length) + b"HPKE-v1" + suite + label + info, length
    )


def derive_keypair(ikm: bytes):
    prk = labeled_extract(KEM_SUITE, b"", b"dkp_prk", ikm)
    sk = labeled_expand(KEM_SUITE, prk, b"sk", b"", 32)
    return sk, x25519(sk, BASE)


def extract_and_expand(dh: bytes, kem_context: bytes) -> bytes:
    prk = labeled_extract(KEM_SUITE, b"", b"eae_prk", dh)
    return labeled_expand(KEM_SUITE, prk, b"shared_secret", kem_context, 32)


def key_schedule_auth(shared_secret: bytes, info: bytes):
    psk_id_hash = labeled_extract(HPKE_SUITE, b"", b"psk_id_hash", b"")
    info_hash = labeled_extract(HPKE_SUITE, b"", b"info_hash", info)
    context = b"\x02" + psk_id_hash + info_hash
    secret = labeled_extract(HPKE_SUITE, shared_secret, b"secret", b"")
    key = labeled_expand(HPKE_SUITE, secret, b"key", context, 32)
    nonce = labeled_expand(HPKE_SUITE, secret, b"base_nonce", context, 12)
    return key, nonce


def hpke_seal_auth(pk_r: bytes, sk_s: bytes, ikm_e: bytes, info: bytes, aad: bytes, pt: bytes):
    """Returns (enc, ct) for sequence number 0."""
    sk_e, pk_e = derive_keypair(ikm_e)
    dh = x25519(sk_e, pk_r) + x25519(sk_s, pk_r)
    pk_s = x25519(sk_s, BASE)
    shared = extract_and_expand(dh, pk_e + pk_r + pk_s)
    key, nonce = key_schedule_auth(shared, info)
    return pk_e, aead_seal(key, nonce, aad, pt)


def self_test() -> None:
    # RFC 7748 section 5.2, first vector.
    out = x25519(
        bytes.fromhex("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4"),
        bytes.fromhex("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c"),
    )
    assert out.hex() == "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"
    # draft-irtf-cfrg-xchacha-03 A.3.1.
    key = bytes(range(0x80, 0xA0))
    nonce = bytes(range(0x40, 0x58))
    aad = bytes.fromhex("50515253c0c1c2c3c4c5c6c7")
    pt = (
        b"Ladies and Gentlemen of the class of '99: If I could offer you only one "
        b"tip for the future, sunscreen would be it."
    )
    sealed = xaead_seal(key, nonce, aad, pt)
    assert sealed[:32].hex() == (
        "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb"
    )
    assert sealed[-16:].hex() == "c0875924c1c7987947deafd8780acf49"
    # RFC 9180 A.2.3 (mode Auth).
    h = bytes.fromhex
    sk_s, pk_s = derive_keypair(h("9d8f94537d5a3ddef71234c0baedfad4ca6861634d0b94c3007fed557ad17df6"))
    assert pk_s.hex() == "f0f4f9e96c54aeed3f323de8534fffd7e0577e4ce269896716bcb95643c8712b"
    assert sk_s.hex() == "2def0cb58ffcf83d1062dd085c8aceca7f4c0c3fd05912d847b61f3e54121f05"
    sk_r, pk_r = derive_keypair(h("64835d5ee64aa7aad57c6f2e4f758f7696617f8829e70bc9ac7a5ef95d1c756c"))
    assert pk_r.hex() == "1a478716d63cb2e16786ee93004486dc151e988b34b475043d3e0175bdb01c44"
    enc, ct = hpke_seal_auth(
        pk_r,
        sk_s,
        h("938d3daa5a8904540bc24f48ae90eed3f4f7f11839560597b55e7c9598c996c0"),
        h("4f6465206f6e2061204772656369616e2055726e"),
        h("436f756e742d30"),
        h("4265617574792069732074727574682c20747275746820626561757479"),
    )
    assert enc.hex() == "f7674cc8cd7baa5872d1f33dbaffe3314239f6197ddf5ded1746760bfc847e0e"
    assert ct.hex() == (
        "ab1a13c9d4f01a87ec3440dbd756e2677bd2ecf9df0ce7ed73869b98e00c09be"
        "111cb9fdf077347aeb88e61bdf"
    )


# ---------------------------------------------------------------- Mosaic objects


def pad(msg: bytes) -> bytes:
    n = max(512, (len(msg) // 256 + 1) * 256)
    return msg + b"\x80" + b"\x00" * (n - len(msg) - 1)


CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"


def safety_code(fpr: bytes) -> str:
    bits = "".join(f"{b:08b}" for b in fpr)[:130]
    chars = "".join(CROCKFORD[int(bits[i : i + 5], 2)] for i in range(0, 130, 5))
    return " ".join([chars[0:5], chars[5:10], chars[10:15], chars[15:20], chars[20:26]])


def generate() -> str:
    sender, recipient, vault = b"\xa1" * 16, b"\xb2" * 16, b"\x0b" * 16
    gen = 3
    vk = b"\xc3" * 32
    ak = b"\x11" * 32
    ikm_s, ikm_r = b"\x51" * 32, b"\x52" * 32
    ikm_e_share, ikm_e_emerg = b"\x53" * 32, b"\x54" * 32

    sk_s, ipk_s = derive_keypair(ikm_s)
    sk_r, ipk_r = derive_keypair(ikm_r)
    isk_s = clamp(sk_s)  # what the hpke crate serializes and mosaic-core wraps
    assert x25519(isk_s, BASE) == ipk_s

    fpr_s = sha256(ctx("fingerprint") + sender + ipk_s)
    fpr_r = sha256(ctx("fingerprint") + recipient + ipk_r)

    header = bytes([1, 3])
    g = struct.pack(">I", gen)

    share_info = ctx("share/vault") + vault + sender + recipient
    share_aad = header + ctx("share/vault") + b"\x00" + vault + g
    enc, ct = hpke_seal_auth(ipk_r, sk_s, ikm_e_share, share_info, share_aad, vk)
    share_seal = header + enc + ct

    emerg_info = ctx("emergency") + sender + recipient
    emerg_aad = header + ctx("emergency") + b"\x00" + b"\x02" + vault + g
    enc, ct = hpke_seal_auth(ipk_r, sk_s, ikm_e_emerg, emerg_info, emerg_aad, vk)
    emergency_seal = header + enc + ct

    short = bytes([1, 1])
    n_id = b"\x61" * 24
    id_aad = short + ctx("wrap/identity") + b"\x00" + sender
    identity_wrap = short + n_id + xaead_seal(ak, n_id, id_aad, isk_s)

    n_ro = b"\x62" * 24
    roster_header = short + g
    roster_aad = roster_header + ctx("vault-roster") + b"\x00" + vault + g
    roster_ct = roster_header + n_ro + xaead_seal(
        vk, n_ro, roster_aad, pad(b'{"roster":"kat"}')
    )

    k_contacts = hkdf(ak, b"", ctx("contacts-key"), 32)
    n_co = b"\x63" * 24
    contacts_aad = short + ctx("contacts") + b"\x00" + sender
    contacts_ct = short + n_co + xaead_seal(
        k_contacts, n_co, contacts_aad, pad(b'{"contacts":"kat"}')
    )

    meta_hash, roster_hash = sha256(b"meta"), sha256(b"roster")
    meta_commit = sha256(
        ctx("sync/view-meta")
        + b"\x00"
        + vault
        + struct.pack(">Q", 5)
        + meta_hash
        + struct.pack(">Q", 6)
        + roster_hash
    )
    key_commit = b"\x44" * 32
    served = [
        (b"\x0d" * 16, 3, sha256(b"first")),
        (b"\x0f" * 16, 8, sha256(b"second")),
    ]
    view2 = sha256(
        ctx("sync/view2")
        + b"\x00"
        + vault
        + struct.pack(">Q", 8)
        + key_commit
        + meta_commit
        + struct.pack(">I", len(served))
        + b"".join(r + struct.pack(">Q", q) + h for r, q, h in served)
    )

    lines = [
        "# Generated by share_kat.py (independent of mosaic-core); see the script header.",
        f"sender_ikm={ikm_s.hex()}",
        f"recipient_ikm={ikm_r.hex()}",
        f"eph_share_ikm={ikm_e_share.hex()}",
        f"eph_emergency_ikm={ikm_e_emerg.hex()}",
        f"vk={vk.hex()}",
        f"ak={ak.hex()}",
        f"sender_isk={isk_s.hex()}",
        f"sender_ipk={ipk_s.hex()}",
        f"recipient_ipk={ipk_r.hex()}",
        f"sender_fpr={fpr_s.hex()}",
        f"recipient_fpr={fpr_r.hex()}",
        f"sender_safety_code={safety_code(fpr_s)}",
        f"recipient_safety_code={safety_code(fpr_r)}",
        f"share_info={share_info.hex()}",
        f"share_aad={share_aad.hex()}",
        f"share_seal={share_seal.hex()}",
        f"emergency_info={emerg_info.hex()}",
        f"emergency_aad={emerg_aad.hex()}",
        f"emergency_seal={emergency_seal.hex()}",
        f"identity_wrap_nonce={n_id.hex()}",
        f"identity_wrap={identity_wrap.hex()}",
        f"roster_nonce={n_ro.hex()}",
        f"roster_ct={roster_ct.hex()}",
        f"contacts_key={k_contacts.hex()}",
        f"contacts_nonce={n_co.hex()}",
        f"contacts_ct={contacts_ct.hex()}",
        f"meta_commit={meta_commit.hex()}",
        f"view2_digest={view2.hex()}",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    self_test()
    out = generate()
    if "--check" in sys.argv[1:]:
        path = pathlib.Path(__file__).resolve().with_name("share_kat.txt")
        expected = path.read_text(encoding="utf-8").replace("\r\n", "\n")
        if expected != out:
            sys.stderr.write("share_kat.txt is stale: regenerate it with share_kat.py\n")
            sys.exit(1)
        sys.stdout.write("share_kat.txt matches\n")
        return
    sys.stdout.write(out)


if __name__ == "__main__":
    main()

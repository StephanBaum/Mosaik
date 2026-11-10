"""Independent reference for the M8 encrypted export and CLI session file
(CRYPTO_SPEC 6.8, 11.2).

It shares no code with mosaic-core and uses only the Python standard library.
HKDF-SHA-256 (RFC 5869) and XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha) are
written out here, and the script checks them against published vectors (RFC 5869
A.1, the XChaCha20-Poly1305 draft A.3.1) before it emits anything. Argon2id is
not re-implemented: the export vectors start from a fixed stretched password
`P_x` (Argon2id itself is covered by the M1 vectors, RFC 9106).

    python -I crates/core/tests/vectors/export_kat.py > crates/core/tests/vectors/export_kat.txt
    python -I crates/core/tests/vectors/export_kat.py --check

`--check` regenerates the vectors and fails if `export_kat.txt` differs. The
unit tests in `src/export.rs` (`export::tests::kat::matches_python`) and
`src/cli_session.rs` (`cli_session::tests::kat::matches_python`) read the file
and compare it with the Rust code.

Fixed inputs. Export: `P_x` = 0x33 x 32, salt 0x5A x 16, export_id 0xE1 x 16,
nonce prefix 0x77 x 19, profile 0x02, chunk_log2 16. Small file: plaintext
`{"format":"mosaic-export","v":1,"vaults":[]}` (one chunk). Big file: the
plaintext 65 536 + 100 bytes of `i % 251` (two chunks; its hash and lengths are
listed instead of the whole file). Session: AK 0x11 x 32, W_cli 0x22 x 32,
session_id 0x55 x 16, account 0xA1 x 16, install 0xB2 x 16, ak_epoch 3, account
salt 0xC3 x 16, created 1_700_000_000_000 ms, TTL 300_000 ms, nonce 0x66 x 24.
"""

import base64
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


def self_test() -> None:
    # RFC 5869 Appendix A.1.
    okm = hkdf(
        b"\x0b" * 22,
        bytes.fromhex("000102030405060708090a0b0c"),
        bytes.fromhex("f0f1f2f3f4f5f6f7f8f9"),
        42,
    )
    assert okm.hex() == (
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        "34007208d5b887185865"
    )
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


# ---------------------------------------------------------------- Mosaic objects

CHUNK = 1 << 16
SALT = b"\x5a" * 16
EXPORT_ID = b"\xe1" * 16
PREFIX = b"\x77" * 19
PROFILE = 0x02
NOW = 1_700_000_000_000
TTL = 300_000


def export_header() -> bytes:
    return b"MOSX" + bytes([1, 2, 16, PROFILE]) + SALT + EXPORT_ID + PREFIX


def export_file(k_x: bytes, plaintext: bytes) -> bytes:
    """header || STREAM-BE32 chunks: nonce = prefix || BE32(i) || last flag."""
    header = export_header()
    assert len(header) == 59
    aad = header + ctx("export") + b"\x00" + EXPORT_ID
    chunks = [plaintext[i : i + CHUNK] for i in range(0, len(plaintext), CHUNK)] or [b""]
    out = header
    for i, chunk in enumerate(chunks):
        last = i == len(chunks) - 1
        nonce = PREFIX + struct.pack(">I", i) + bytes([1 if last else 0])
        out += xaead_seal(k_x, nonce, aad, chunk)
    return out


def session_file(ak: bytes, w: bytes, session_id: bytes) -> bytes:
    account, install, salt = b"\xa1" * 16, b"\xb2" * 16, b"\xc3" * 16
    header = (
        b"MCLS"
        + bytes([1, 1])
        + account
        + install
        + session_id
        + struct.pack(">I", 3)
        + salt
        + struct.pack(">QQ", NOW, NOW + TTL)
    )
    assert len(header) == 90
    aad = header + ctx("cli-session") + b"\x00" + account + install + session_id
    nonce = b"\x66" * 24
    out = header + nonce + xaead_seal(w, nonce, aad, ak)
    assert len(out) == 162
    return out


def generate() -> str:
    p_x = b"\x33" * 32
    k_x = hkdf(p_x, EXPORT_ID, ctx("export-key"), 32)
    small_pt = b'{"format":"mosaic-export","v":1,"vaults":[]}'
    small = export_file(k_x, small_pt)
    big_pt = bytes(i % 251 for i in range(CHUNK + 100))
    big = export_file(k_x, big_pt)
    ak, w, session_id = b"\x11" * 32, b"\x22" * 32, b"\x55" * 16
    keystore_text = base64.urlsafe_b64encode(session_id + w).decode().rstrip("=")
    exact_pt = bytes(i % 251 for i in range(CHUNK))
    exact = export_file(k_x, exact_pt)
    empty = export_file(k_x, b"")
    lines = [
        "# Generated by export_kat.py (independent of mosaic-core); see the script header.",
        f"k_x={k_x.hex()}",
        f"header={export_header().hex()}",
        f"small_plaintext={small_pt.hex()}",
        f"small_file={small.hex()}",
        f"big_file_len_text={len(big)}",
        f"big_file_sha256={sha256(big).hex()}",
        f"big_chunk0_sha256={sha256(big[59 : 59 + CHUNK + 16]).hex()}",
        f"big_last_tag={big[-16:].hex()}",
        f"empty_file={empty.hex()}",
        f"exact_file_sha256={sha256(exact).hex()}",
        f"session_file={session_file(ak, w, session_id).hex()}",
        f"session_keystore_value_text={keystore_text}",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    self_test()
    out = generate()
    if "--check" in sys.argv[1:]:
        path = pathlib.Path(__file__).resolve().with_name("export_kat.txt")
        expected = path.read_text(encoding="utf-8").replace("\r\n", "\n")
        if expected != out:
            sys.stderr.write("export_kat.txt is stale: regenerate it with export_kat.py\n")
            sys.exit(1)
        sys.stdout.write("export_kat.txt matches\n")
        return
    sys.stdout.write(out)


if __name__ == "__main__":
    main()

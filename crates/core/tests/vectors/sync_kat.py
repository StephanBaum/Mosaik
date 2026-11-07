"""Independent reference for the M6 sync keys and MACs (CRYPTO_SPEC 9.2, 10, 10.1).

It shares no code with mosaic-core: HMAC and SHA-256 come from the Python
standard library and HKDF-SHA-256 is written out from RFC 5869. The unit tests
in `src/sync_auth.rs` read the checked-in output `sync_kat.txt` and compare it
with the Rust derivations.

    python crates/core/tests/vectors/sync_kat.py > crates/core/tests/vectors/sync_kat.txt

Inputs (fixed): AK = 0x11 x 32; device (install) ID = 0x0E x 16; checkpoint
counter 7 with one entry (vault 0x0B x 16, generation 1, high_seq 8, the view
digest of two served records: 0x0D x 16 at seq 3 with ciphertext hash
SHA-256("first") and 0x0F x 16 at seq 8 with SHA-256("second"), under the key
commitment of generation 1 and the 74-byte wrapping 0x77 x 74) issued at
1_700_000_000_000 ms; folder message "folder message"; session key = 0x22 x 64.

It also writes complete byte vectors of the sync wire objects (CRYPTO_SPEC 10,
10.1), built field by field from the spec, which `mosaic-sync` reads and
compares with its encoders and decoders (`src/wire/kat_tests.rs`):
`checkpoint_bytes`, `blob_delete_bytes` (the 93-byte `.mdel`),
`account_file_bytes` (the 168-byte `.macct`), `segment_bytes` (no snapshot)
and `snapshot_segment_bytes` (part 2 of a 3-part snapshot). Further fixed
inputs: account ID = 0x0A x 16, vault ID = 0x0B x 16, attachment ID =
0x0C x 16, record ID = 0x0D x 16; ciphertext bodies are fixed fill bytes (the
decoders check their shape only, the segment MAC covers them as bytes).
"""

import hashlib
import hmac
import struct
import sys

CTX = b"mosaic/v1/"


def hkdf_sha256(ikm: bytes, salt: bytes, info: bytes, length: int) -> bytes:
    """RFC 5869 Extract-then-Expand (an empty salt means HashLen zero bytes)."""
    prk = hmac.new(salt if salt else b"\x00" * 32, ikm, hashlib.sha256).digest()
    okm, block = b"", b""
    counter = 1
    while len(okm) < length:
        block = hmac.new(prk, block + info + bytes([counter]), hashlib.sha256).digest()
        okm += block
        counter += 1
    return okm[:length]


def self_test() -> None:
    # RFC 5869 Appendix A.1.
    okm = hkdf_sha256(
        b"\x0b" * 22,
        bytes.fromhex("000102030405060708090a0b0c"),
        bytes.fromhex("f0f1f2f3f4f5f6f7f8f9"),
        42,
    )
    assert okm.hex() == (
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        "34007208d5b887185865"
    )
    # RFC 5869 Appendix A.3 (empty salt and info).
    okm = hkdf_sha256(b"\x0b" * 22, b"", b"", 42)
    assert okm.hex() == (
        "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d"
        "9d201395faa4b61a96c8"
    )


def ctx(label: str) -> bytes:
    return CTX + label.encode()


def hmac_sha256(key: bytes, msg: bytes) -> bytes:
    return hmac.new(key, msg, hashlib.sha256).digest()


def main() -> None:
    sys.stdout.reconfigure(newline="\n")  # LF on every platform
    self_test()
    ak = b"\x11" * 32
    device = b"\x0e" * 16
    vault = b"\x0b" * 16
    counter = 7
    issued_at_ms = 1_700_000_000_000
    # Key commitment and view digest (CRYPTO_SPEC 10):
    #   key_commit = SHA-256(ctx("sync/view-key") || 0x00 || vault || generation
    #                        || wrapped_key_bytes)
    #   view_digest = SHA-256(ctx("sync/view") || 0x00 || vault || high_seq
    #                         || key_commit || n || n x (record_id || seq || ct_hash))
    key_commit = hashlib.sha256(
        ctx("sync/view-key") + b"\x00" + vault + struct.pack(">I", 1) + b"\x77" * 74
    ).digest()
    high_seq = 8
    served = [
        (b"\x0d" * 16, 3, hashlib.sha256(b"first").digest()),
        (b"\x0f" * 16, 8, hashlib.sha256(b"second").digest()),
    ]
    view_digest = hashlib.sha256(
        ctx("sync/view")
        + b"\x00"
        + vault
        + struct.pack(">Q", high_seq)
        + key_commit
        + struct.pack(">I", len(served))
        + b"".join(r + struct.pack(">Q", q) + h for r, q, h in served)
    ).digest()
    entries = (
        vault
        + struct.pack(">I", 1)
        + struct.pack(">Q", high_seq)
        + view_digest
    )
    seen_digest = hashlib.sha256(
        struct.pack(">Q", issued_at_ms) + struct.pack(">I", 1) + entries
    ).digest()

    k_cp = hkdf_sha256(ak, b"", ctx("checkpoint-key"), 32)
    k_fold = hkdf_sha256(ak, b"", ctx("sync/folder-mac-key"), 32)
    k_dev = hkdf_sha256(ak, b"", ctx("sync/device-info-key"), 32)
    checkpoint_mac = hmac_sha256(
        k_cp, ctx("checkpoint") + device + struct.pack(">Q", counter) + seen_digest
    )
    msg = b"folder message"

    def folder(label: str) -> bytes:
        return hmac_sha256(k_fold, ctx(label) + b"\x00" + msg)

    session_token = hkdf_sha256(b"\x22" * 64, b"", ctx("session"), 32)

    # ---- full byte vectors -------------------------------------------------
    account = b"\x0a" * 16
    attachment = b"\x0c" * 16
    record = b"\x0d" * 16
    u64 = lambda n: struct.pack(">Q", n)  # noqa: E731
    u32 = lambda n: struct.pack(">I", n)  # noqa: E731

    def folder_mac(label: str, body: bytes) -> bytes:
        return hmac_sha256(k_fold, ctx(label) + b"\x00" + body)

    # Checkpoint: "MCKP" 1 device counter issued_at n entries mac.
    checkpoint_body = (
        b"MCKP\x01" + device + u64(counter) + u64(issued_at_ms) + u32(1) + entries
    )
    checkpoint_bytes = checkpoint_body + checkpoint_mac

    # Blob delete marker (93 bytes).
    blob_delete_body = (
        b"MDEL\x01" + attachment + vault + device + u64(1_700_000_100_000)
    )
    blob_delete_bytes = blob_delete_body + folder_mac(
        "sync/folder-blob-delete", blob_delete_body
    )
    assert len(blob_delete_bytes) == 93

    # Account file (168 bytes): version 2, profile 1 (baseline).
    wrapped = b"\x01\x01" + b"\x77" * 72
    account_body = (
        b"MACC\x01" + account + u64(2) + device + b"\x66" * 16 + b"\x01" + wrapped
    )
    account_file_bytes = account_body + folder_mac(
        "sync/folder-account", account_body
    )
    assert len(account_file_bytes) == 168

    # Segments. Object bodies: a vault key (kind 3: generation, wrapped key)
    # and a tombstone (kind 2: record ID, then 01 01 || generation || nonce ||
    # 4 plaintext bytes || 16 tag bytes).
    key_object = u32(1) + b"\x01\x01" + b"\x55" * 72
    tombstone_ct = b"\x01\x01" + u32(1) + b"\x33" * 24 + b"\x44" * 20
    tombstone_object = record + tombstone_ct

    def obj(kind: int, body: bytes) -> bytes:
        return bytes([kind]) + u32(len(body)) + body

    created_at_ms = 1_700_000_000_000
    header = lambda flags, seq: (  # noqa: E731
        b"MSEG\x01" + bytes([flags]) + account + vault + device + u64(seq)
        + u64(created_at_ms)
    )
    segment_body = (
        header(0, 3) + u32(2) + obj(3, key_object) + obj(2, tombstone_object)
    )
    segment_bytes = segment_body + folder_mac("sync/folder-segment", segment_body)
    # Part 2 of 3 of a snapshot whose first part is seq 4: seq 5.
    snapshot_body = (
        header(1, 5) + u64(4) + u32(3) + u32(1) + obj(2, tombstone_object)
    )
    snapshot_segment_bytes = snapshot_body + folder_mac(
        "sync/folder-segment", snapshot_body
    )

    print("# Generated by sync_kat.py (independent of mosaic-core); see the script header.")
    print(f"k_cp={k_cp.hex()}")
    print(f"k_fold={k_fold.hex()}")
    print(f"k_dev={k_dev.hex()}")
    print(f"key_commit={key_commit.hex()}")
    print(f"view_digest={view_digest.hex()}")
    print(f"seen_digest={seen_digest.hex()}")
    print(f"checkpoint_mac={checkpoint_mac.hex()}")
    print(f"folder_mac_segment={folder('sync/folder-segment').hex()}")
    print(f"folder_mac_account={folder('sync/folder-account').hex()}")
    print(f"folder_mac_blob_delete={folder('sync/folder-blob-delete').hex()}")
    print(f"session_token={session_token.hex()}")
    print(f"checkpoint_bytes={checkpoint_bytes.hex()}")
    print(f"blob_delete_bytes={blob_delete_bytes.hex()}")
    print(f"account_file_bytes={account_file_bytes.hex()}")
    print(f"segment_bytes={segment_bytes.hex()}")
    print(f"snapshot_segment_bytes={snapshot_segment_bytes.hex()}")


if __name__ == "__main__":
    main()

"""Independent reference for the CRYPTO_SPEC 5.3/5.4 derivation under the
real `baseline` profile (m = 128 MiB, t = 3, p = 4).

It shares no code with mosaic-core: Argon2id comes from argon2-cffi (the
reference C implementation), HKDF-SHA-256 is written out from RFC 5869 with
the Python standard library. tests/baseline_profile.rs compares the
`baseline` line with the Rust derivation; the `test-cost` line reproduces
golden/derived_keys.hex.

    python -m pip install argon2-cffi==23.1.0
    python crates/core/tests/vectors/derive_keys_baseline.py

Prints KEK || OPW as hex, one line per cost.
"""

import hashlib
import hmac
import unicodedata

from argon2.low_level import Type, hash_secret_raw

CTX = b"mosaic/v1/"


def hkdf_sha256(ikm: bytes, salt: bytes, info: bytes, length: int) -> bytes:
    """RFC 5869 Extract-then-Expand."""
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
        bytes(range(0x00, 0x0D)),
        bytes(range(0xF0, 0xFA)),
        42,
    )
    assert okm.hex() == (
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        "34007208d5b887185865"
    ), "HKDF self-test failed"


def derive(m_kib: int, t: int, p_lanes: int) -> bytes:
    """KEK || OPW for the fixed inputs shared with the Rust tests."""
    mp = unicodedata.normalize("NFC", "correct horse battery staple").encode("utf-8")
    salt = b"\x5a" * 16
    sk = b"\x0f" * 16
    account_id = b"\x0a" * 16

    p = hash_secret_raw(
        secret=mp,
        salt=salt,
        time_cost=t,
        memory_cost=m_kib,
        parallelism=p_lanes,
        hash_len=32,
        type=Type.ID,
        version=0x13,
    )
    s = hkdf_sha256(sk, account_id, CTX + b"secret-key", 32)
    r = hkdf_sha256(p + s, account_id, CTX + b"root", 32)
    kek = hkdf_sha256(r, b"", CTX + b"kek", 32)
    opw = hkdf_sha256(r, b"", CTX + b"opaque-password", 32)
    return kek + opw


def main() -> None:
    self_test()
    # Test-only cost of golden/derived_keys.hex: m = 64 KiB, t = 1, p = 1.
    print("test-cost", derive(64, 1, 1).hex())
    # Baseline profile 0x01: m = 128 MiB (in KiB), t = 3, p = 4.
    print("baseline ", derive(128 * 1024, 3, 4).hex())


if __name__ == "__main__":
    main()

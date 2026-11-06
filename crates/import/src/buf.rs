//! Zeroizing buffers for intermediate plaintext: decompressed payloads,
//! decrypted KDBX data, archive members and base64-decoded values.
//!
//! A growing `Vec` leaves its old allocation behind unwiped, so every buffer
//! here grows by moving its bytes into a larger allocation and wiping the old
//! one first.

use std::io::{ErrorKind, Read};

use base64ct::{Base64, Base64Unpadded, Base64Url, Base64UrlUnpadded, Encoding};
use zeroize::{Zeroize, Zeroizing};

use crate::error::{Error, Result};

/// A byte buffer that is wiped (over its whole capacity) on drop.
pub(crate) type Buf = Zeroizing<Vec<u8>>;

/// A string that is wiped on drop.
pub(crate) type Str = Zeroizing<String>;

/// An empty buffer with room for `capacity` bytes.
pub(crate) fn with_capacity(capacity: usize) -> Result<Buf> {
    let mut v = Vec::new();
    v.try_reserve_exact(capacity)
        .map_err(|_| Error::Allocation)?;
    Ok(Zeroizing::new(v))
}

/// Appends `data` to `buf`, growing it without leaving an unwiped copy, and
/// fails with [`Error::TooLarge`] if `buf` would exceed `limit` bytes.
pub(crate) fn append(buf: &mut Buf, data: &[u8], limit: usize) -> Result<()> {
    let needed = buf.len().checked_add(data.len()).ok_or(Error::TooLarge)?;
    if needed > limit {
        return Err(Error::TooLarge);
    }
    if needed > buf.capacity() {
        let cap = needed.max(buf.capacity().saturating_mul(2)).min(limit);
        let mut grown = with_capacity(cap)?;
        grown.extend_from_slice(buf);
        buf.zeroize();
        *buf = grown;
    }
    buf.extend_from_slice(data);
    Ok(())
}

/// Reads `reader` to its end into a zeroizing buffer of at most `limit`
/// bytes. `hint` is the expected size (never trusted beyond `limit`).
pub(crate) fn read_limited<R: Read>(mut reader: R, limit: usize, hint: usize) -> Result<Buf> {
    let mut buf = with_capacity(hint.min(limit))?;
    let mut chunk = Zeroizing::new([0u8; 16 * 1024]);
    loop {
        let n = match reader.read(chunk.as_mut_slice()) {
            Ok(0) => return Ok(buf),
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return Err(Error::Malformed),
        };
        append(&mut buf, chunk.get(..n).ok_or(Error::Malformed)?, limit)?;
    }
}

/// Decodes standard base64 (with or without padding; ASCII whitespace is
/// ignored) into a zeroizing buffer. Constant time in the data
/// (`base64ct`).
pub(crate) fn base64_decode(text: &str) -> Result<Buf> {
    let bytes = text.as_bytes();
    let compact: Buf;
    let src = if bytes.iter().any(u8::is_ascii_whitespace) {
        let mut c = with_capacity(bytes.len())?;
        c.extend(bytes.iter().copied().filter(|b| !b.is_ascii_whitespace()));
        compact = c;
        compact.as_slice()
    } else {
        bytes
    };
    let max = src.len().div_ceil(4).saturating_mul(3);
    let mut out = with_capacity(max)?;
    out.resize(max, 0);
    let len = match Base64::decode(src, out.as_mut_slice()) {
        Ok(d) => d.len(),
        Err(_) => Base64Unpadded::decode(src, out.as_mut_slice())
            .map_err(|_| Error::Malformed)?
            .len(),
    };
    out.truncate(len);
    Ok(out)
}

/// Decodes base64url (RFC 4648 section 5) into a zeroizing buffer, strictly:
/// no whitespace, no characters outside the alphabet, no non-zero trailing
/// bits; padding is accepted but not required. The decoded size is checked
/// against `max_len` **before** anything is allocated.
///
/// # Errors
/// [`Error::TooLarge`] if the value decodes to more than `max_len` bytes,
/// [`Error::Malformed`] for anything that is not canonical base64url.
pub(crate) fn base64url_decode(text: &str, max_len: usize) -> Result<Buf> {
    let max_chars = max_len.div_ceil(3).saturating_mul(4);
    if text.len() > max_chars {
        return Err(Error::TooLarge);
    }
    let mut out = with_capacity(max_chars / 4 * 3 + 3)?;
    out.resize(max_chars / 4 * 3 + 3, 0);
    let len = match Base64UrlUnpadded::decode(text, out.as_mut_slice()) {
        Ok(d) => d.len(),
        Err(_) => Base64Url::decode(text, out.as_mut_slice())
            .map_err(|_| Error::Malformed)?
            .len(),
    };
    if len > max_len {
        return Err(Error::TooLarge);
    }
    out.truncate(len);
    Ok(out)
}

/// The DER bytes of a PEM `PRIVATE KEY` (PKCS#8) block: the BEGIN and END
/// lines are checked and removed, the base64 body is gathered **in a
/// zeroizing buffer** and decoded strictly (standard alphabet, padded). Text
/// before the BEGIN line or after the END line, any other label, and bodies
/// over `max_len` bytes are refused.
///
/// # Errors
/// [`Error::Malformed`], [`Error::TooLarge`].
pub(crate) fn pem_private_key_der(pem: &str, max_len: usize) -> Result<Buf> {
    const BEGIN: &str = "-----BEGIN PRIVATE KEY-----";
    const END: &str = "-----END PRIVATE KEY-----";
    let max_chars = max_len.div_ceil(3).saturating_mul(4);
    // Whitespace between lines is allowed; bound the whole block first.
    if pem.len() > max_chars.saturating_add(max_chars / 32 + 128) {
        return Err(Error::TooLarge);
    }
    let rest = pem.trim().strip_prefix(BEGIN).ok_or(Error::Malformed)?;
    let body = rest.strip_suffix(END).ok_or(Error::Malformed)?;
    let mut compact = with_capacity(body.len())?;
    compact.extend(body.bytes().filter(|b| !b.is_ascii_whitespace()));
    if compact.len() > max_chars {
        return Err(Error::TooLarge);
    }
    let max = compact.len() / 4 * 3 + 3;
    let mut out = with_capacity(max)?;
    out.resize(max, 0);
    let len = Base64::decode(compact.as_slice(), out.as_mut_slice())
        .map_err(|_| Error::Malformed)?
        .len();
    out.truncate(len);
    Ok(out)
}

/// Moves the bytes of `buf` into a zeroizing string if they are UTF-8.
pub(crate) fn into_string(mut buf: Buf) -> Result<Str> {
    let v = core::mem::take(&mut *buf);
    String::from_utf8(v).map(Zeroizing::new).map_err(|e| {
        let mut bytes = e.into_bytes();
        bytes.zeroize();
        Error::Malformed
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn append_grows_and_enforces_the_limit() {
        let mut b = with_capacity(1).unwrap();
        append(&mut b, b"abc", 8).unwrap();
        append(&mut b, b"defgh", 8).unwrap();
        assert_eq!(b.as_slice(), b"abcdefgh");
        assert_eq!(append(&mut b, b"i", 8), Err(Error::TooLarge));
        assert_eq!(b.as_slice(), b"abcdefgh");
    }

    struct Failing;

    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("x"))
        }
    }

    struct Interrupted(bool);

    impl Read for Interrupted {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.0 {
                return Ok(0);
            }
            self.0 = true;
            out[0] = 1;
            Err(std::io::Error::from(ErrorKind::Interrupted))
        }
    }

    #[test]
    fn read_limited_reads_and_limits() {
        let data = vec![7u8; 40_000];
        let b = read_limited(data.as_slice(), 40_000, 10).unwrap();
        assert_eq!(b.len(), 40_000);
        assert_eq!(
            read_limited(data.as_slice(), 39_999, 0).unwrap_err(),
            Error::TooLarge
        );
        assert_eq!(read_limited(Failing, 10, 0).unwrap_err(), Error::Malformed);
        assert!(read_limited(Interrupted(false), 10, 0).unwrap().is_empty());
    }

    #[test]
    fn base64_variants() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap().as_slice(), b"hello");
        assert_eq!(base64_decode("aGVsbG8").unwrap().as_slice(), b"hello");
        assert_eq!(
            base64_decode("aGVs\nbG8=\r\n").unwrap().as_slice(),
            b"hello"
        );
        assert!(base64_decode("").unwrap().is_empty());
        assert_eq!(base64_decode("a*==").unwrap_err(), Error::Malformed);
    }

    #[test]
    fn base64url_is_strict_and_bounded() {
        assert_eq!(
            base64url_decode("aGVsbG8", 16).unwrap().as_slice(),
            b"hello"
        );
        assert_eq!(
            base64url_decode("aGVsbG8=", 16).unwrap().as_slice(),
            b"hello"
        );
        assert_eq!(
            base64url_decode("-_-_", 16).unwrap().as_slice(),
            [0xfb, 0xff, 0xbf]
        );
        assert!(base64url_decode("", 16).unwrap().is_empty());
        // Standard alphabet, whitespace, bad padding, non-canonical tail.
        for bad in [
            "+/+/",
            "aGVs bG8",
            "aGVsbG8==",
            "aGVsbG9",
            "a",
            "aGVsbG8
",
        ] {
            assert_eq!(
                base64url_decode(bad, 16).unwrap_err(),
                Error::Malformed,
                "{bad:?}"
            );
        }
        assert_eq!(base64url_decode("aGVsbG8", 4).unwrap_err(), Error::TooLarge);
        assert_eq!(
            base64url_decode(&"A".repeat(100_000), 8).unwrap_err(),
            Error::TooLarge
        );
    }

    #[test]
    fn pem_private_key_is_stripped_and_checked() {
        let pem = "-----BEGIN PRIVATE KEY-----
AAEC
AwQF
-----END PRIVATE KEY-----
";
        assert_eq!(
            pem_private_key_der(pem, 16).unwrap().as_slice(),
            [0, 1, 2, 3, 4, 5]
        );
        let crlf = "-----BEGIN PRIVATE KEY-----
AAECAwQF
-----END PRIVATE KEY-----";
        assert_eq!(pem_private_key_der(crlf, 16).unwrap().len(), 6);
        for bad in [
            "AAECAwQF",
            "-----BEGIN EC PRIVATE KEY-----
AAECAwQF
-----END EC PRIVATE KEY-----",
            "junk -----BEGIN PRIVATE KEY-----
AAECAwQF
-----END PRIVATE KEY-----",
            "-----BEGIN PRIVATE KEY-----
AAECAwQF
-----END PRIVATE KEY----- junk",
            "-----BEGIN PRIVATE KEY-----
AA*CAwQF
-----END PRIVATE KEY-----",
            "-----BEGIN PRIVATE KEY-----
AAECAwQ
-----END PRIVATE KEY-----",
        ] {
            assert_eq!(
                pem_private_key_der(bad, 16).unwrap_err(),
                Error::Malformed,
                "{bad:?}"
            );
        }
        assert_eq!(pem_private_key_der(pem, 3).unwrap_err(), Error::TooLarge);
    }

    #[test]
    fn into_string_checks_utf8() {
        let ok = Zeroizing::new(b"caf\xc3\xa9".to_vec());
        assert_eq!(into_string(ok).unwrap().as_str(), "caf\u{e9}");
        let bad = Zeroizing::new(vec![0xff, 0xfe]);
        assert_eq!(into_string(bad).unwrap_err(), Error::Malformed);
    }
}

//! Turning the passkey members of an export into a [`PasskeyDraft`].
//!
//! Every format hands over the same pieces (credential ID, relying party,
//! user handle and names, a PKCS#8 private key, counter, creation time);
//! this module checks them and builds the credential. The key is parsed and
//! re-encoded by `mosaic_passkey::Es256Key`, so only a valid P-256 key gets
//! in, in the one encoding the vault stores. Names go through the same
//! `mosaic_passkey::sanitize_name` the desktop applies to names from sites.
//!
//! Nothing here puts a value from the file in an error: a passkey that
//! cannot be used is a [`Problem`], a bare marker.

use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_core::model::{PasskeyCredential, Text};
use mosaic_core::{SecretVec, Timestamp};
use mosaic_passkey::{
    COSE_ALG_ES256, Es256Key, MAX_CREDENTIAL_ID_LEN, MAX_RP_ID_LEN, MAX_USER_HANDLE_LEN,
    is_pkcs8_private_key, sanitize_name,
};

use crate::buf::Buf;
use crate::draft::PasskeyDraft;

/// The longest PKCS#8 private key read: an ES256 key is 138 bytes; the room
/// leaves space for keys of other algorithms, which are recognized and then
/// refused as unsupported.
pub(crate) const MAX_PKCS8_LEN: usize = 4096;

/// Why a passkey of the export is not imported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Problem {
    /// Bad encoding, a missing member, or a size out of range.
    Unreadable,
    /// A valid passkey of an algorithm Mosaic does not implement (ES256 only).
    Unsupported,
}

/// A passkey draft or the reason there is none.
pub(crate) type Built = Result<PasskeyDraft, Problem>;

/// The members of one passkey as the format gave them, decoded.
pub(crate) struct Parts<'a> {
    pub(crate) credential_id: Buf,
    pub(crate) rp_id: &'a str,
    pub(crate) user_handle: Buf,
    pub(crate) user_name: &'a str,
    pub(crate) user_display_name: &'a str,
    /// PKCS#8 DER of the private key.
    pub(crate) private_key: Buf,
    pub(crate) sign_count: u64,
    pub(crate) discoverable: bool,
    /// When the credential was created; the import time if the file has none.
    pub(crate) created_at: Option<Timestamp>,
}

/// The import time, for credentials whose export has no creation date.
fn now() -> Timestamp {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    Timestamp::from_unix_millis(ms)
}

/// Checks the members and builds the draft (its title is filled in later,
/// from the entry).
pub(crate) fn build(parts: &Parts<'_>) -> Built {
    if !(1..=MAX_CREDENTIAL_ID_LEN).contains(&parts.credential_id.len())
        || !(1..=MAX_USER_HANDLE_LEN).contains(&parts.user_handle.len())
    {
        return Err(Problem::Unreadable);
    }
    let rp_id = parts.rp_id.trim();
    if rp_id.is_empty()
        || rp_id.len() > MAX_RP_ID_LEN
        || rp_id.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(Problem::Unreadable);
    }
    let key = match Es256Key::from_pkcs8_der(&parts.private_key) {
        Ok(key) => key,
        // A well-formed key of another algorithm (Ed25519, RSA) is
        // unsupported, anything else is damaged.
        Err(_) if is_pkcs8_private_key(&parts.private_key) => return Err(Problem::Unsupported),
        Err(_) => return Err(Problem::Unreadable),
    };
    let der: SecretVec = key.to_pkcs8_der().map_err(|_| Problem::Unreadable)?;
    let mut credential = PasskeyCredential::new(
        parts.credential_id.to_vec(),
        Text::new(rp_id),
        parts.user_handle.to_vec(),
        der,
        COSE_ALG_ES256,
        parts.created_at.unwrap_or_else(now),
    );
    credential.user_name = Text::new(&sanitize_name(parts.user_name));
    credential.user_display_name = Text::new(&sanitize_name(parts.user_display_name));
    credential.discoverable = parts.discoverable;
    credential.validate().map_err(|_| Problem::Unreadable)?;
    Ok(PasskeyDraft {
        credential,
        sign_count: parts.sign_count,
        title: Text::new(""),
    })
}

/// A Unix time in milliseconds from an RFC 3339 timestamp
/// (`2024-05-01T12:30:00Z`, fractions and `+hh:mm` offsets allowed).
/// `None` for anything else, years outside 1970..=9999 included.
pub(crate) fn parse_rfc3339(text: &str) -> Option<Timestamp> {
    let b = text.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let digits = b.get(from..from.checked_add(len)?)?;
        digits.iter().all(u8::is_ascii_digit).then(|| {
            digits
                .iter()
                .fold(0i64, |n, d| n * 10 + i64::from(d - b'0'))
        })
    };
    let at = |i: usize, c: u8| b.get(i) == Some(&c);
    if !(at(4, b'-')
        && at(7, b'-')
        && matches!(b.get(10), Some(b'T' | b't' | b' '))
        && at(13, b':')
        && at(16, b':'))
    {
        return None;
    }
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hour, minute, second) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let mut i = 19;
    let mut millis = 0i64;
    if at(i, b'.') {
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            if i - start < 3 {
                millis = millis * 10 + i64::from(b.get(i)? - b'0');
            }
            i += 1;
        }
        if i == start {
            return None;
        }
        for _ in 0..(3usize.saturating_sub(i - start)) {
            millis *= 10;
        }
    }
    let offset = match b.get(i)? {
        b'Z' | b'z' if i + 1 == b.len() => 0,
        sign @ (b'+' | b'-') if i + 6 == b.len() && at(i + 3, b':') => {
            let off = num(i + 1, 2)? * 60 + num(i + 4, 2)?;
            if num(i + 1, 2)? > 23 || num(i + 4, 2)? > 59 {
                return None;
            }
            if *sign == b'+' { off } else { -off }
        }
        _ => return None,
    };
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second.min(59) - offset * 60;
    (secs >= 0).then(|| Timestamp::from_unix_millis(secs * 1_000 + millis))
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Hinnant's
/// `days_from_civil`).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A Unix time in seconds (CXF) as a timestamp; `None` if it is out of range.
pub(crate) fn from_unix_seconds(secs: i64) -> Option<Timestamp> {
    (0..=253_402_300_799)
        .contains(&secs)
        .then(|| Timestamp::from_unix_millis(secs * 1_000))
}

/// The 16 bytes of a UUID in its hyphenated form (`8-4-4-4-12` hex digits),
/// as Bitwarden writes a credential ID. `None` for any other text.
pub(crate) fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
    let b = text.as_bytes();
    if b.len() != 36 {
        return None;
    }
    let mut out = [0u8; 16];
    let mut n = 0usize;
    let mut i = 0usize;
    while i < b.len() {
        if matches!(i, 8 | 13 | 18 | 23) {
            if b.get(i) != Some(&b'-') {
                return None;
            }
            i += 1;
            continue;
        }
        let hi = hex(*b.get(i)?)?;
        let lo = hex(*b.get(i + 1)?)?;
        *out.get_mut(n)? = hi << 4 | lo;
        n += 1;
        i += 2;
    }
    (n == 16).then_some(out)
}

const fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339() {
        let ms = |s: &str| parse_rfc3339(s).map(Timestamp::as_unix_millis);
        assert_eq!(ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(ms("2024-05-01T12:30:00Z"), Some(1_714_566_600_000));
        assert_eq!(ms("2024-05-01T12:30:00.5Z"), Some(1_714_566_600_500));
        assert_eq!(
            ms("2024-05-01T12:30:00.123456+02:00"),
            Some(1_714_559_400_123)
        );
        assert_eq!(ms("2024-05-01T12:30:00-01:30"), Some(1_714_572_000_000));
        assert_eq!(ms("2024-02-29T00:00:00Z"), Some(1_709_164_800_000));
        for bad in [
            "",
            "2024-05-01",
            "2024-05-01T12:30:00",
            "2024-13-01T00:00:00Z",
            "2024-05-32T00:00:00Z",
            "2024-05-01T24:00:00Z",
            "1969-12-31T23:59:59Z",
            "2024-05-01T12:30:00.Z",
            "2024-05-01T12:30:00+0200",
            "2024-05-01T12:30:00Zjunk",
            "2024-05-01T12:30:00+24:00",
            "2024-05-0\u{e9}T12:30:00Z",
        ] {
            assert_eq!(ms(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn uuids() {
        assert_eq!(
            uuid_bytes("00010203-0405-0607-0809-0a0b0c0d0e0F"),
            Some([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
        );
        for bad in [
            "",
            "00010203040506070809 0a0b0c0d0e0f",
            "00010203-0405-0607-0809-0a0b0c0d0e0",
            "00010203-0405-0607-0809-0a0b0c0d0e0g",
            "000102030405060708090a0b0c0d0e0f0000",
        ] {
            assert_eq!(uuid_bytes(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn unix_seconds() {
        assert_eq!(
            from_unix_seconds(1).map(Timestamp::as_unix_millis),
            Some(1_000)
        );
        assert!(from_unix_seconds(-1).is_none());
        assert!(from_unix_seconds(i64::MAX).is_none());
    }
}

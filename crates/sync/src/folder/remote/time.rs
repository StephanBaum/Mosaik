//! The two date formats of the providers: RFC 3339 (`2026-11-15T10:20:30Z`,
//! Dropbox and Graph) and the HTTP IMF-fixdate (`Sun, 15 Nov 2026 10:20:30
//! GMT`, `WebDAV`). Strict; anything else is `None`.

use std::time::{Duration, SystemTime};

/// Days from 1970-01-01 to the civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

fn num(s: &str, len: usize) -> Option<i64> {
    if s.len() != len || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// The time of a civil date and time of day, as seconds since the epoch; the
/// dates before 1970 and after 9999 are refused.
fn to_time(y: i64, m: i64, d: i64, hh: i64, mm: i64, ss: i64, offset_s: i64) -> Option<SystemTime> {
    if !(1970..=9999).contains(&y)
        || !(1..=12).contains(&m)
        || !(1..=days_in_month(y, m)).contains(&d)
        || hh > 23
        || mm > 59
        || ss > 60
    {
        return None;
    }
    let secs = days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss.min(59) - offset_s;
    let secs = u64::try_from(secs).ok()?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

/// An RFC 3339 timestamp: `YYYY-MM-DDTHH:MM:SS[.fraction](Z|+-HH:MM)`.
#[must_use]
pub fn parse_rfc3339(s: &str) -> Option<SystemTime> {
    if !s.is_ascii() {
        return None;
    }
    let field = |a: usize, b: usize| num(s.get(a..b)?, b - a);
    let sep = |i: usize, c: &[u8]| s.as_bytes().get(i).is_some_and(|x| c.contains(x));
    if !(sep(4, b"-") && sep(7, b"-") && sep(10, b"Tt") && sep(13, b":") && sep(16, b":")) {
        return None;
    }
    let (y, mo, d) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hh, mm, ss) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let mut rest = s.get(19..)?;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        rest = frac.get(digits..)?;
    }
    let offset = if matches!(rest, "Z" | "z") {
        0
    } else {
        let sign = match rest.as_bytes().first()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let (oh, colon, om) = (num(rest.get(1..3)?, 2)?, rest.get(3..4)?, num(rest.get(4..)?, 2)?);
        if colon != ":" || oh > 23 || om > 59 {
            return None;
        }
        sign * (oh * 3600 + om * 60)
    };
    to_time(y, mo, d, hh, mm, ss, offset)
}

/// An HTTP IMF-fixdate: `Sun, 15 Nov 2026 10:20:30 GMT` (the weekday is
/// checked for its shape, not its value).
#[must_use]
pub fn parse_http_date(s: &str) -> Option<SystemTime> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut parts = s.split(' ');
    let wd = parts.next()?.strip_suffix(',')?;
    let (day, mon, year, time, zone) = (
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
    );
    if parts.next().is_some() || zone != "GMT" || !DAYS.contains(&wd) {
        return None;
    }
    let m = i64::try_from(MONTHS.iter().position(|x| *x == mon)?).ok()? + 1;
    let mut hms = time.split(':');
    let (hh, mm, ss) = (hms.next()?, hms.next()?, hms.next()?);
    if hms.next().is_some() {
        return None;
    }
    to_time(
        num(year, 4)?,
        m,
        num(day, 2)?,
        num(hh, 2)?,
        num(mm, 2)?,
        num(ss, 2)?,
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(t: SystemTime) -> u64 {
        t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
    }

    #[test]
    fn parse_dates() {
        // 2026-11-15T10:20:30Z
        let expected = 1_794_738_030;
        assert_eq!(secs(parse_rfc3339("2026-11-15T10:20:30Z").unwrap()), expected);
        assert_eq!(
            secs(parse_http_date("Sun, 15 Nov 2026 10:20:30 GMT").unwrap()),
            expected
        );
        assert_eq!(
            secs(parse_rfc3339("2026-11-15T11:20:30.123+01:00").unwrap()),
            expected
        );
        assert_eq!(secs(parse_rfc3339("1970-01-01T00:00:00Z").unwrap()), 0);
        for bad in [
            "",
            "garbage",
            "2026-11-15 10:20:30Z",
            "2026-13-15T10:20:30Z",
            "2026-02-30T10:20:30Z",
            "2026-11-15T25:20:30Z",
            "2026-11-15T10:20:30",
            "2026-11-15T10:20:30.Z",
            "1969-12-31T23:59:59Z",
        ] {
            assert!(parse_rfc3339(bad).is_none(), "{bad}");
        }
        for bad in [
            "",
            "garbage",
            "Sun, 15 Nov 2026 10:20:30 UTC",
            "Xyz, 15 Nov 2026 10:20:30 GMT",
            "Sun, 15 Foo 2026 10:20:30 GMT",
            "Sun, 5 Nov 2026 10:20:30 GMT",
            "Sun, 15 Nov 2026 10:20 GMT",
        ] {
            assert!(parse_http_date(bad).is_none(), "{bad}");
        }
    }
}

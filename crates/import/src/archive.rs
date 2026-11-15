//! Bounded ZIP reading shared by the archive importers (1PUX, Dashlane,
//! Proton Pass).
//!
//! The member count is bounded before the central directory is parsed, every
//! member is read through a hard size limit (a declared size is never
//! trusted), only stored and deflate members are read, and a member that
//! inflates by more than [`MAX_RATIO`] above [`RATIO_FLOOR`] bytes is refused.

use std::io::{Cursor, Read};

use zip::ZipArchive;
use zip::result::ZipError;

use crate::buf::{self, Buf};
use crate::error::{Error, Result};

/// Largest inflated-to-compressed ratio of a member.
pub(crate) const MAX_RATIO: u64 = 1_000;
/// The ratio applies only to members that inflate past 1 MiB.
pub(crate) const RATIO_FLOOR: usize = 1024 * 1024;

/// Opens `input` as a zip archive of at most `max_members` members.
///
/// # Errors
/// [`Error::LimitExceeded`] for more members; [`Error::Malformed`] if the
/// input is not a zip archive.
pub(crate) fn open(input: &[u8], max_members: usize) -> Result<ZipArchive<Cursor<&[u8]>>> {
    // Bound the central directory before the zip crate reads it: every
    // member needs one central directory header (`PK\x01\x02`).
    let headers = input.windows(4).filter(|w| *w == b"PK\x01\x02").count();
    if headers > max_members {
        return Err(Error::LimitExceeded);
    }
    ZipArchive::new(Cursor::new(input)).map_err(|_| Error::Malformed)
}

/// Reads member `index` into a zeroizing buffer of at most `limit` bytes.
///
/// # Errors
/// [`Error::Encrypted`] for an encrypted member; [`Error::Unsupported`] for
/// a compression other than stored or deflate; [`Error::TooLarge`] above
/// `limit`; [`Error::LimitExceeded`] for a ratio above [`MAX_RATIO`];
/// [`Error::Malformed`] otherwise.
pub(crate) fn read_member(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    index: usize,
    limit: usize,
) -> Result<Buf> {
    let file = zip.by_index(index).map_err(|e| match e {
        ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED) => Error::Encrypted,
        ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
            Error::Unsupported
        }
        _ => Error::Malformed,
    })?;
    // zip 9.5 instantiates the decompressor lazily, so an unsupported method
    // would otherwise only surface as a read error: check it up front.
    if !matches!(
        file.compression(),
        zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
    ) {
        return Err(Error::Unsupported);
    }
    let hint = usize::try_from(file.size()).unwrap_or(limit);
    let compressed = file.compressed_size();
    // Read one byte past the limit so an oversized member is detected
    // whatever size its header declares.
    let reader = file.take(u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1));
    let out = buf::read_limited(reader, limit, hint.min(limit))?;
    if out.len() > RATIO_FLOOR && out.len() as u64 > compressed.saturating_mul(MAX_RATIO) {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

/// The lowest member index whose name is UTF-8 and satisfies `pred`.
pub(crate) fn find(zip: &ZipArchive<Cursor<&[u8]>>, pred: impl Fn(&str) -> bool) -> Option<usize> {
    (0..zip.len()).find(|&i| matches!(zip.name_for_index(i), Some(Ok(name)) if pred(&name)))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::CompressionMethod::{Deflated, Stored};
    use zip::write::SimpleFileOptions;

    use super::*;

    fn zip_of(members: &[(&[u8], &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(method);
        for (name, data) in members {
            zip.start_file(String::from_utf8_lossy(name), options)
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn open_refuses_more_members_than_the_cap() {
        let z = zip_of(&[(b"a", b"1"), (b"b", b"2"), (b"c", b"3")], Stored);
        assert_eq!(open(&z, 2).err(), Some(Error::LimitExceeded));
        assert!(open(&z, 3).is_ok());
        assert_eq!(open(b"nope", 3).err(), Some(Error::Malformed));
    }

    #[test]
    fn read_refuses_a_member_over_its_limit() {
        let z = zip_of(&[(b"a", b"0123456789")], Stored);
        let mut zip = open(&z, 4).unwrap();
        assert_eq!(read_member(&mut zip, 0, 9).err(), Some(Error::TooLarge));
        assert_eq!(read_member(&mut zip, 0, 10).unwrap().len(), 10);
    }

    #[test]
    fn read_refuses_deflate_ratio_over_1000_above_1_mib() {
        let data = vec![0u8; 2 * 1024 * 1024];
        let deflated = zip_of(&[(b"a", &data)], Deflated);
        let mut zip = open(&deflated, 4).unwrap();
        assert_eq!(
            read_member(&mut zip, 0, 4 * 1024 * 1024).err(),
            Some(Error::LimitExceeded)
        );
        let stored = zip_of(&[(b"a", &data)], Stored);
        let mut zip = open(&stored, 4).unwrap();
        assert_eq!(
            read_member(&mut zip, 0, 4 * 1024 * 1024).unwrap().len(),
            2 * 1024 * 1024
        );
    }

    #[test]
    fn find_skips_non_utf8_names_and_takes_the_lowest_index() {
        let mut z = zip_of(
            &[(b"a/x.csv", b"1"), (b"QQ.csv", b"2"), (b"b/x.csv", b"3")],
            Stored,
        );
        // Make the second member's name non-UTF-8 in both the local header
        // and the central directory.
        for w in 0..z.len().saturating_sub(2) {
            if z[w..].starts_with(b"QQ.csv") {
                z[w] = 0xff;
            }
        }
        let zip = open(&z, 4).unwrap();
        assert_eq!(find(&zip, |n| n.ends_with("x.csv")), Some(0));
        assert_eq!(find(&zip, |n| n.ends_with("b/x.csv")), Some(2));
        assert_eq!(find(&zip, |n| n.ends_with("QQ.csv")), None);
    }
}

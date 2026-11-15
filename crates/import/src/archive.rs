//! Bounded ZIP reading shared by the archive importers (1PUX, Dashlane,
//! Proton Pass).
//!
//! The member count is bounded before the central directory is parsed, every
//! member is read through a hard size limit (a declared size is never
//! trusted), only stored and deflate members are read, and a member that
//! inflates by more than [`MAX_RATIO`] above [`RATIO_FLOOR`] bytes is refused.
//! The ratio is computed from the bytes actually consumed from the archive
//! while the member is read, never from the `compressed_size` its header
//! declares, and an archive whose entries overlap is malformed.

use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use zip::ZipArchive;
use zip::result::ZipError;

use crate::buf::{self, Buf};
use crate::error::{Error, Result};

/// Largest inflated-to-compressed ratio of a member.
pub(crate) const MAX_RATIO: u64 = 1_000;
/// The ratio applies only to members that inflate past 1 MiB.
pub(crate) const RATIO_FLOOR: usize = 1024 * 1024;

/// The archive bytes, counting what is read from them.
pub(crate) struct Counted<'a> {
    inner: Cursor<&'a [u8]>,
    read: Arc<AtomicU64>,
}

impl Read for Counted<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

impl Seek for Counted<'_> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// An opened archive; derefs to the zip crate's reader.
pub(crate) struct Archive<'a> {
    zip: ZipArchive<Counted<'a>>,
    read: Arc<AtomicU64>,
}

impl<'a> Deref for Archive<'a> {
    type Target = ZipArchive<Counted<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.zip
    }
}

impl DerefMut for Archive<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.zip
    }
}

/// Opens `input` as a zip archive of at most `max_members` members whose
/// entries do not overlap.
///
/// # Errors
/// [`Error::LimitExceeded`] for more members; [`Error::Malformed`] if the
/// input is not a zip archive or two entries share bytes.
pub(crate) fn open(input: &[u8], max_members: usize) -> Result<Archive<'_>> {
    let mut archive = open_unchecked(input, max_members)?;
    // Entries that share bytes make the archive larger on extraction than
    // on disk (a zip quine or overlap bomb): never legitimate in an export.
    if archive.zip.has_overlapping_entries().unwrap_or(true) {
        return Err(Error::Malformed);
    }
    Ok(archive)
}

fn open_unchecked(input: &[u8], max_members: usize) -> Result<Archive<'_>> {
    // Bound the central directory before the zip crate reads it: every
    // member needs one central directory header (`PK\x01\x02`).
    let headers = input.windows(4).filter(|w| *w == b"PK\x01\x02").count();
    if headers > max_members {
        return Err(Error::LimitExceeded);
    }
    let read = Arc::new(AtomicU64::new(0));
    let reader = Counted {
        inner: Cursor::new(input),
        read: Arc::clone(&read),
    };
    let zip = ZipArchive::new(reader).map_err(|_| Error::Malformed)?;
    Ok(Archive { zip, read })
}

/// Reads member `index` into a zeroizing buffer of at most `limit` bytes.
///
/// # Errors
/// [`Error::Encrypted`] for an encrypted member; [`Error::Unsupported`] for
/// a compression other than stored or deflate; [`Error::TooLarge`] above
/// `limit`; [`Error::LimitExceeded`] for a ratio above [`MAX_RATIO`];
/// [`Error::Malformed`] otherwise.
pub(crate) fn read_member(zip: &mut Archive<'_>, index: usize, limit: usize) -> Result<Buf> {
    let read = Arc::clone(&zip.read);
    let file = zip.zip.by_index(index).map_err(|e| match e {
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
    // The header is read; count the member's data from here.
    read.store(0, Ordering::Relaxed);
    // Read one byte past the limit so an oversized member is detected
    // whatever size its header declares.
    let reader = file.take(u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1));
    let out = buf::read_limited(reader, limit, hint.min(limit))?;
    let consumed = read.load(Ordering::Relaxed);
    if out.len() > RATIO_FLOOR && out.len() as u64 > consumed.saturating_mul(MAX_RATIO) {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

/// The lowest member index whose name is UTF-8 and satisfies `pred`.
pub(crate) fn find(zip: &Archive<'_>, pred: impl Fn(&str) -> bool) -> Option<usize> {
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

    /// The offsets of the central directory headers (`PK`).
    fn central_headers(z: &[u8]) -> Vec<usize> {
        (0..z.len().saturating_sub(4))
            .filter(|&i| z[i..].starts_with(b"PK"))
            .collect()
    }

    #[test]
    fn open_refuses_overlapping_entries() {
        let mut z = zip_of(&[(b"a", b"0123456789"), (b"b", b"abcdefghij")], Stored);
        assert!(open(&z, 4).is_ok());
        // Point the second entry at the first one's local header (offset 0).
        let second = central_headers(&z)[1];
        z[second + 42..second + 46].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(open(&z, 4).err(), Some(Error::Malformed));
    }

    #[test]
    fn open_refuses_a_declared_size_that_runs_past_its_data() {
        let data = vec![0u8; 8 * 1024 * 1024];
        let mut z = zip_of(&[(b"a", &data)], Deflated);
        let header = central_headers(&z)[0];
        let real = u32::from_le_bytes(z[header + 20..header + 24].try_into().unwrap());
        z[header + 20..header + 24].copy_from_slice(&(real + 1_000_000).to_le_bytes());
        assert_eq!(open(&z, 4).err(), Some(Error::Malformed));
    }

    #[test]
    fn the_ratio_uses_the_bytes_consumed_not_the_declared_size() {
        let data = vec![0u8; 8 * 1024 * 1024];
        let mut z = zip_of(&[(b"a", &data)], Deflated);
        let header = central_headers(&z)[0];
        let real = u32::from_le_bytes(z[header + 20..header + 24].try_into().unwrap());
        // Declared 1 MB larger than the stream: the declared ratio is far
        // below the limit, the consumed one is not.
        z[header + 20..header + 24].copy_from_slice(&(real + 1_000_000).to_le_bytes());
        let mut zip = open_unchecked(&z, 4).unwrap();
        assert!(zip.by_index_raw(0).unwrap().compressed_size() > 1_000_000);
        assert_eq!(
            read_member(&mut zip, 0, 16 * 1024 * 1024).err(),
            Some(Error::LimitExceeded)
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

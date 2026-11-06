//! The strict file-name parser of the folder backend (CRYPTO_SPEC §10.1,
//! ADR-0043).
//!
//! A sync client renames and adds files on its own (conflict copies, temp
//! files, OS metadata). Readers therefore look only at names that match
//! exactly what Mosaic writes; every other name is ignored:
//!
//! | Name | Meaning |
//! |---|---|
//! | `^[0-9a-f]{16}\.mseg$` | segment, the digits are the sequence number (>= 1) |
//! | `^[0-9a-f]{16}-[0-9a-f]{32}\.macct$` | account file: version (>= 1), device |
//! | `^[0-9a-f]{32}\.matt$` | attachment blob |
//! | `^[0-9a-f]{32}\.mdel$` | blob delete marker |
//! | `head.mck` | checkpoint |
//! | `info.mdev` | sealed device info |
//! | `^[0-9a-f]{32}$` | an account, vault or device directory |
//!
//! A 32-digit ID is never the nil UUID. The one exception to "exact names
//! only" is a device directory's `*.mck` files, which readers parse as
//! candidate checkpoints ([`is_checkpoint_candidate`]).

use std::ffi::OsStr;

/// A recognized file or directory name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderName {
    /// `<seq:016x>.mseg`.
    Segment {
        /// The sequence number (at least 1).
        seq: u64,
    },
    /// `<version:016x>-<device:32hex>.macct`.
    Account {
        /// The account version (at least 1).
        version: u64,
        /// The device that wrote it.
        device: [u8; 16],
    },
    /// `<attachment:32hex>.matt`.
    Blob([u8; 16]),
    /// `<attachment:32hex>.mdel`.
    BlobDelete([u8; 16]),
    /// `head.mck`.
    Head,
    /// `info.mdev`.
    DeviceInfo,
    /// A directory named by an ID (`<32 hex>`).
    Dir([u8; 16]),
}

/// A lowercase-hex string of exactly `N * 2` digits as `N` bytes.
fn hex_exact<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    let (pairs, _) = s.as_bytes().as_chunks::<2>();
    for (byte, pair) in out.iter_mut().zip(pairs) {
        let [hi, lo] = *pair;
        *byte = (digit(hi)? << 4) | digit(lo)?;
    }
    Some(out)
}

const fn digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// A non-nil ID of 32 lowercase hex digits.
fn id32(s: &str) -> Option<[u8; 16]> {
    hex_exact::<16>(s).filter(|id| *id != [0u8; 16])
}

/// A sequence number or version of 16 lowercase hex digits, at least 1.
fn number16(s: &str) -> Option<u64> {
    hex_exact::<8>(s)
        .map(u64::from_be_bytes)
        .filter(|n| *n != 0)
}

/// Parses a file or directory name; `None` for every name Mosaic did not
/// write.
#[must_use]
pub fn parse(name: &OsStr) -> Option<FolderName> {
    let name = name.to_str()?;
    match name {
        "head.mck" => return Some(FolderName::Head),
        "info.mdev" => return Some(FolderName::DeviceInfo),
        _ => {}
    }
    if let Some(stem) = name.strip_suffix(".mseg") {
        return number16(stem).map(|seq| FolderName::Segment { seq });
    }
    if let Some(stem) = name.strip_suffix(".macct") {
        let (version, device) = stem.split_once('-')?;
        return Some(FolderName::Account {
            version: number16(version)?,
            device: id32(device)?,
        });
    }
    if let Some(stem) = name.strip_suffix(".matt") {
        return id32(stem).map(FolderName::Blob);
    }
    if let Some(stem) = name.strip_suffix(".mdel") {
        return id32(stem).map(FolderName::BlobDelete);
    }
    id32(name).map(FolderName::Dir)
}

/// Whether a file in a device directory is a candidate checkpoint: any file
/// ending in `.mck`, canonical or a conflict copy. The reader verifies each
/// candidate and uses the one with the highest counter.
#[must_use]
pub fn is_checkpoint_candidate(name: &OsStr) -> bool {
    name.to_str()
        .and_then(|n| n.rsplit_once('.'))
        .is_some_and(|(_, extension)| extension == "mck")
}

/// The file name of segment `seq`.
#[must_use]
pub fn segment_name(seq: u64) -> String {
    format!("{seq:016x}.mseg")
}

/// The file name of an account file.
#[must_use]
pub fn account_name(version: u64, device: &[u8; 16]) -> String {
    format!("{version:016x}-{}.macct", super::hex(device))
}

/// The file name of an attachment blob.
#[must_use]
pub fn blob_name(attachment: &[u8; 16]) -> String {
    format!("{}.matt", super::hex(attachment))
}

/// The file name of a blob delete marker.
#[must_use]
pub fn blob_delete_name(attachment: &[u8; 16]) -> String {
    format!("{}.mdel", super::hex(attachment))
}

/// The directory name of an account, vault or device.
#[must_use]
pub fn dir_name(id: &[u8; 16]) -> String {
    super::hex(id)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    const ID: &str = "0123456789abcdef0123456789abcdef";
    const ID_BYTES: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
        0xef,
    ];

    fn p(s: &str) -> Option<FolderName> {
        parse(OsStr::new(s))
    }

    #[test]
    fn canonical_names_parse_and_round_trip() {
        assert_eq!(p("head.mck"), Some(FolderName::Head));
        assert_eq!(p("info.mdev"), Some(FolderName::DeviceInfo));
        assert_eq!(
            p("0000000000000001.mseg"),
            Some(FolderName::Segment { seq: 1 })
        );
        assert_eq!(
            p("00000000000000ff.mseg"),
            Some(FolderName::Segment { seq: 255 })
        );
        assert_eq!(p(&format!("{ID}.matt")), Some(FolderName::Blob(ID_BYTES)));
        assert_eq!(
            p(&format!("{ID}.mdel")),
            Some(FolderName::BlobDelete(ID_BYTES))
        );
        assert_eq!(p(ID), Some(FolderName::Dir(ID_BYTES)));
        assert_eq!(
            p(&format!("0000000000000003-{ID}.macct")),
            Some(FolderName::Account {
                version: 3,
                device: ID_BYTES
            })
        );
        assert_eq!(segment_name(255), "00000000000000ff.mseg");
        assert_eq!(
            account_name(3, &ID_BYTES),
            format!("0000000000000003-{ID}.macct")
        );
        assert_eq!(blob_name(&ID_BYTES), format!("{ID}.matt"));
        assert_eq!(blob_delete_name(&ID_BYTES), format!("{ID}.mdel"));
        assert_eq!(dir_name(&ID_BYTES), ID);
        for seq in [1u64, 2, 0x1234_5678_9abc_def0, u64::MAX] {
            assert_eq!(p(&segment_name(seq)), Some(FolderName::Segment { seq }));
        }
    }

    #[test]
    fn folder_names_accept_only_canonical() {
        let seg = "0000000000000001";
        let conflicts = [
            // Dropbox
            format!("{seg} (conflicted copy 2026-11-06).mseg"),
            format!("{seg} (Desktop's conflicted copy 2026-11-06).mseg"),
            format!("{seg} (1).mseg"),
            "head (conflicted copy).mck".to_owned(),
            "info (Anna's conflicted copy 2026-11-06).mdev".to_owned(),
            // OneDrive
            format!("{seg}-DESKTOP-ABC123.mseg"),
            format!("{seg}-LAPTOP.mseg"),
            "head-DESKTOP-ABC123.mck".to_owned(),
            // iCloud Drive
            format!("{seg} 2.mseg"),
            "head 2.mck".to_owned(),
            "info 2.mdev".to_owned(),
            // Nextcloud
            format!("{seg} (conflicted copy 2026-11-06 120000).mseg"),
            format!("{seg}_conflict-20261106-120000.mseg"),
            // Syncthing
            format!("{seg}.sync-conflict-20261106-120000-ABCDEFG.mseg"),
            "head.sync-conflict-20261106-120000-ABCDEFG.mck".to_owned(),
            ".stfolder".to_owned(),
            ".syncthing.0000000000000001.mseg.tmp".to_owned(),
            // Google Drive
            format!("{seg} (2).mseg"),
            "head (1).mck".to_owned(),
            // Case conflicts and case changes
            format!("{seg} (Case Conflict).mseg"),
            "HEAD.MCK".to_owned(),
            "Head.mck".to_owned(),
            "0000000000000001.MSEG".to_owned(),
            "000000000000000A.mseg".to_owned(),
            format!("{}.matt", ID.to_uppercase()),
            ID.to_uppercase(),
            // Our own temp files and OS files
            ".0123456789abcdef.mtmp".to_owned(),
            "desktop.ini".to_owned(),
            "Thumbs.db".to_owned(),
            ".DS_Store".to_owned(),
            "~$info.mdev".to_owned(),
            "~$0000000000000001.mseg".to_owned(),
            ".dropbox".to_owned(),
            ".dropbox.attr".to_owned(),
            ".dropbox.cache".to_owned(),
            "._0000000000000001.mseg".to_owned(),
            "mosaic-sync.json".to_owned(),
            "README.txt".to_owned(),
            // Shape errors
            String::new(),
            ".".to_owned(),
            "..".to_owned(),
            ".mseg".to_owned(),
            "0000000000000001mseg".to_owned(),
            "000000000000001.mseg".to_owned(),
            "00000000000000001.mseg".to_owned(),
            "0000000000000000.mseg".to_owned(),
            "0000000000000000-".to_owned() + ID + ".macct",
            format!("0000000000000001_{ID}.macct"),
            format!("0000000000000001-{ID}.mseg"),
            format!("0000000000000001-{}.macct", &ID[1..]),
            format!("{ID}x.matt"),
            format!("{ID}0.matt"),
            format!("{}.matt", &ID[1..]),
            format!("{}.matt", "0".repeat(32)),
            "0".repeat(32),
            format!("../{ID}"),
            format!("{ID}/"),
            format!("a/{ID}"),
            format!("{ID}\\x"),
            format!("{ID}.matt.mdel"),
            format!("{ID} .matt"),
            format!("{ID}\u{0}.matt"),
            "0000000000000001.mseg\u{202e}".to_owned(),
            "\u{ff10}000000000000001.mseg".to_owned(),
        ];
        for name in &conflicts {
            assert_eq!(p(name), None, "{name:?}");
        }
        // Checkpoint candidates are the one place that accepts conflict names.
        for name in [
            "head.mck",
            "head (conflicted copy).mck",
            "x.sync-conflict-1.mck",
            "head-PC.mck",
        ] {
            assert!(is_checkpoint_candidate(OsStr::new(name)), "{name}");
        }
        for name in ["head.mck.tmp", "head.mdev", "mck", "head.MCK", ".mtmp"] {
            assert!(!is_checkpoint_candidate(OsStr::new(name)), "{name}");
        }
    }

    #[test]
    fn non_utf8_names_are_ignored() {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            assert_eq!(parse(&OsString::from_vec(vec![0xFF, 0xFE])), None);
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            assert_eq!(parse(&OsString::from_wide(&[0xD800, 0x0041])), None);
        }
        assert_eq!(parse(&OsString::from("head.mck")), Some(FolderName::Head));
    }
}

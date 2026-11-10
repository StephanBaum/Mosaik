//! Small helpers.

use std::time::Duration;

/// A duration like `30s`, `5m` or a bare number of `bare_unit` seconds.
#[must_use]
pub fn parse_duration(text: &str, bare_unit_secs: u64) -> Option<Duration> {
    let (digits, unit) = match text.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((at, _)) => text.split_at(at),
        None => (text, ""),
    };
    let n: u64 = digits.parse().ok()?;
    let secs = match unit {
        "" => bare_unit_secs,
        "s" => 1,
        "m" => 60,
        _ => return None,
    };
    n.checked_mul(secs).map(Duration::from_secs)
}

/// A private temporary file next to `dest`, to be moved into place with [`finish_private`].
///
/// # Errors
/// `fileExists` when `dest` exists (Mosaic never overwrites), `fileWrite` otherwise.
pub fn create_private(
    dest: &std::path::Path,
) -> crate::error::Result<(std::fs::File, std::path::PathBuf)> {
    use crate::error::{CliError, EXIT_OTHER};
    if dest.exists() {
        return Err(
            CliError::new("fileExists", EXIT_OTHER, crate::i18n::Msg::ErrFileExists)
                .with("path", dest.display().to_string()),
        );
    }
    let mut name = dest
        .file_name()
        .map(std::ffi::OsString::from)
        .ok_or_else(|| CliError::file_write(dest))?;
    name.push(format!(".part-{}", std::process::id()));
    let tmp = dest.with_file_name(name);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let file = options.open(&tmp).map_err(|_| CliError::file_write(dest))?;
    Ok((file, tmp))
}

/// Flushes `file` and moves it from `tmp` to `dest`. A failure removes the temporary file.
///
/// # Errors
/// `fileWrite`.
pub fn finish_private(
    file: std::fs::File,
    tmp: &std::path::Path,
    dest: &std::path::Path,
    mark_of_the_web: bool,
) -> crate::error::Result<()> {
    use crate::error::{CliError, EXIT_OTHER};
    if file.sync_all().is_err() {
        let _ = std::fs::remove_file(tmp);
        return Err(CliError::file_write(dest));
    }
    drop(file);
    #[cfg(windows)]
    if mark_of_the_web {
        // NTFS alternate data stream: the file is marked as coming from the Internet zone
        // (ZoneId=3), as ADR-0034 does for the desktop's exports. Not NTFS: no mark, no error.
        let mut stream = tmp.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        let _ = std::fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n");
    }
    #[cfg(not(windows))]
    let _ = mark_of_the_web;
    // A hard link fails when `dest` appeared meanwhile, so a file made in the meantime is never
    // replaced (rename would overwrite it on Unix).
    let moved = match std::fs::hard_link(tmp, dest) {
        Ok(()) => {
            let _ = std::fs::remove_file(tmp);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(true),
        // No hard links here (some network and FAT file systems): rename behind a check.
        Err(_) if !dest.exists() => std::fs::rename(tmp, dest).map_err(|_| false),
        Err(_) => Err(true),
    };
    match moved {
        Ok(()) => Ok(()),
        Err(exists) => {
            let _ = std::fs::remove_file(tmp);
            Err(if exists {
                CliError::new("fileExists", EXIT_OTHER, crate::i18n::Msg::ErrFileExists)
                    .with("path", dest.display().to_string())
            } else {
                CliError::file_write(dest)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("5m", 60), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("30s", 60), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("7", 60), Some(Duration::from_secs(420)));
        assert_eq!(parse_duration("7", 1), Some(Duration::from_secs(7)));
        assert_eq!(parse_duration("m", 1), None);
        assert_eq!(parse_duration("5h", 1), None);
        assert_eq!(parse_duration("", 1), None);
    }
}

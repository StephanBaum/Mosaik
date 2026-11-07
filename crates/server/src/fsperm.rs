//! File modes on Unix: the data directory is 0700 and the database, WAL, blobs
//! and the secret file are 0600 (a no-op elsewhere, where the user profile's
//! ACL applies).

use std::io;
use std::path::Path;

/// Mode 0600 for a file.
///
/// # Errors
/// I/O errors.
#[cfg(unix)]
pub fn restrict_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

/// Mode 0600 for a file (no-op on this platform).
///
/// # Errors
/// Never.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the signature of the Unix version"
)]
pub fn restrict_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Mode 0700 for a directory.
///
/// # Errors
/// I/O errors.
#[cfg(unix)]
pub fn restrict_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

/// Mode 0700 for a directory (no-op on this platform).
///
/// # Errors
/// Never.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the signature of the Unix version"
)]
pub fn restrict_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Whether neither group nor others can access the file (always true where
/// there are no Unix modes).
///
/// # Errors
/// I/O errors.
#[cfg(unix)]
#[expect(
    clippy::verbose_bit_mask,
    reason = "a permission mask reads clearer than trailing_zeros"
)]
pub fn is_private(path: &Path) -> io::Result<bool> {
    use std::os::unix::fs::PermissionsExt as _;
    Ok(std::fs::metadata(path)?.permissions().mode() & 0o077 == 0)
}

/// Whether neither group nor others can access the file (always true here).
///
/// # Errors
/// Never.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the signature of the Unix version"
)]
pub fn is_private(_path: &Path) -> io::Result<bool> {
    Ok(true)
}

//! The override audit (ADR-0083, `docs/NATIVE_MESSAGING.md` §7.3): which
//! per-user manifests or registry keys point a browser at a host other than
//! the installed one.

use super::{
    Browser, Error, Layout, Os, Path, PathBuf, RegExe, Registry, fs, install, manifest_dir,
    registry_key,
};
use crate::ids::HOST_NAME;

/// Shown as the path of a manifest that exists but cannot be read as a host
/// manifest: an unreadable entry is as suspicious as a foreign one.
pub const UNREADABLE: &str = "(unreadable manifest)";

/// A per-user manifest or registry key that does not point to the installed
/// host.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ManifestFinding {
    /// The browser the entry belongs to.
    pub browser: Browser,
    /// The manifest file, or `HKCU\<key>` on Windows.
    pub location: String,
    /// The host program the entry points to ([`UNREADABLE`] if the manifest
    /// has none).
    pub path: PathBuf,
}

/// The host path a manifest file names, if it is a readable manifest.
fn manifest_host(file: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(file).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("path")?.as_str().map(PathBuf::from)
}

/// Whether two host paths are the same for this OS (Windows paths are not
/// case-sensitive).
fn same_host(os: Os, a: &Path, b: &Path) -> bool {
    if os == Os::Windows {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Audits the per-user locations of `layout` against `expected_host`.
pub fn audit(
    layout: &Layout,
    expected_host: &Path,
    registry: &mut dyn Registry,
) -> Vec<ManifestFinding> {
    let mut findings = Vec::new();
    for browser in Browser::ALL {
        let (location, manifest) = if layout.os == Os::Windows {
            let key = registry_key(browser);
            let Some(manifest) = registry.default_value(&key) else {
                continue;
            };
            (format!(r"HKCU\{key}"), manifest)
        } else {
            let file = manifest_dir(layout, browser).join(format!("{HOST_NAME}.json"));
            // Only a missing entry is skipped; a dangling link or an
            // unreadable file is reported as unreadable below.
            if fs::symlink_metadata(&file).is_err() {
                continue;
            }
            (file.to_string_lossy().into_owned(), file)
        };
        let path = manifest_host(&manifest).unwrap_or_else(|| PathBuf::from(UNREADABLE));
        if !same_host(layout.os, &path, expected_host) {
            findings.push(ManifestFinding {
                browser,
                location,
                path,
            });
        }
    }
    findings
}

/// [`audit`] for the running OS and the real registry. An environment with no
/// usable home directory has no per-user entries to report.
#[must_use]
pub fn audit_manifests(expected_host: &Path) -> Vec<ManifestFinding> {
    match Layout::current() {
        Ok(layout) => audit(&layout, expected_host, &mut RegExe),
        Err(_) => Vec::new(),
    }
}

/// Points the entries of the browsers in `findings` at `host`.
///
/// # Errors
/// [`Error::Install`] if `host` is not absolute or a file or key could not
/// be written.
pub fn repair(
    layout: &Layout,
    findings: &[ManifestFinding],
    host: &Path,
    registry: &mut dyn Registry,
) -> Result<(), Error> {
    let mut browsers = Vec::new();
    for finding in findings {
        if !browsers.contains(&finding.browser) {
            browsers.push(finding.browser);
        }
    }
    install(layout, &browsers, host, registry).map(|_| ())
}

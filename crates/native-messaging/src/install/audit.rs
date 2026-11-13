//! The override audit (ADR-0083, `docs/NATIVE_MESSAGING.md` §7.3): which
//! per-user manifests or registry keys point a browser at a host other than
//! the installed one.

use super::{
    Browser, Error, Layout, Os, Path, PathBuf, RegExe, RegExeMachine, Registry, fs, install,
    manifest_dir, manifest_json, registry_key,
};
use crate::ids::HOST_NAME;

/// Shown as the path of a manifest that exists but cannot be read as a host
/// manifest: an unreadable entry is as suspicious as a foreign one.
pub const UNREADABLE: &str = "(unreadable manifest)";

/// Shown as the path of a manifest that names the installed host but lets
/// other extensions start it (M9-F2).
pub const EXTENSIONS_DIFFER: &str = "(unexpected extension list)";

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

/// What a manifest file says compared with what Mosaic writes for `browser`:
/// the host path it names, and whether the extension list (`allowed_origins`
/// or `allowed_extensions`) is the expected one. `None` if it is not a
/// readable host manifest.
fn manifest_host(file: &Path, browser: Browser, expected_host: &Path) -> Option<(PathBuf, bool)> {
    let text = fs::read_to_string(file).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let path = PathBuf::from(value.get("path")?.as_str()?);
    let expected: serde_json::Value =
        serde_json::from_str(&manifest_json(browser.is_firefox(), expected_host)).ok()?;
    let key = if browser.is_firefox() {
        "allowed_extensions"
    } else {
        "allowed_origins"
    };
    // Any other list, or the other browser's list, is a difference.
    let same_list = value.get(key) == expected.get(key)
        && value.get("allowed_origins").is_some() != browser.is_firefox()
        && value.get("allowed_extensions").is_some() == browser.is_firefox();
    Some((path, same_list))
}

/// Whether two host paths are the same for this OS. Windows paths are not
/// case-sensitive for ASCII letters; any other difference, such as the
/// KELVIN SIGN that lowercases to `k`, is a different path (M9-F71).
fn same_host(os: Os, a: &Path, b: &Path) -> bool {
    if os == Os::Windows {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
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
            (format!(r"{}\{key}", registry.root()), manifest)
        } else {
            let file = manifest_dir(layout, browser).join(format!("{HOST_NAME}.json"));
            // Only a missing entry is skipped; a dangling link or an
            // unreadable file is reported as unreadable below.
            if fs::symlink_metadata(&file).is_err() {
                continue;
            }
            (file.to_string_lossy().into_owned(), file)
        };
        let path = match manifest_host(&manifest, browser, expected_host) {
            None => PathBuf::from(UNREADABLE),
            Some((path, true)) => path,
            Some((path, false)) if !same_host(layout.os, &path, expected_host) => path,
            Some((_, false)) => PathBuf::from(EXTENSIONS_DIFFER),
        };
        let own = path != Path::new(UNREADABLE)
            && path != Path::new(EXTENSIONS_DIFFER)
            && same_host(layout.os, &path, expected_host);
        if !own {
            findings.push(ManifestFinding {
                browser,
                location,
                path,
            });
        }
    }
    findings
}

/// Audits the machine-wide entries the MSI writes (Windows only; the Linux
/// packages' system manifests are root-owned files, and macOS has none).
/// An entry whose host is not `expected_host`, for example an install into
/// another folder than the one the system manifests name, is reported unless
/// the same browser has a per-user entry: browsers read that one first, and
/// [`audit`] reports it if it is wrong.
pub fn audit_machine(
    layout: &Layout,
    expected_host: &Path,
    user: &mut dyn Registry,
    machine: &mut dyn Registry,
) -> Vec<ManifestFinding> {
    if layout.os != Os::Windows {
        return Vec::new();
    }
    audit(layout, expected_host, machine)
        .into_iter()
        .filter(|f| user.default_value(&registry_key(f.browser)).is_none())
        .collect()
}

/// [`audit`] and [`audit_machine`] for the running OS and the real registry.
/// An environment with no usable home directory has no per-user entries to
/// report.
#[must_use]
pub fn audit_manifests(expected_host: &Path) -> Vec<ManifestFinding> {
    match Layout::current() {
        Ok(layout) => {
            let mut findings = audit(&layout, expected_host, &mut RegExe);
            findings.extend(audit_machine(
                &layout,
                expected_host,
                &mut RegExe,
                &mut RegExeMachine,
            ));
            findings
        }
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

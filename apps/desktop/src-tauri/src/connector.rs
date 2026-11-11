//! The browser connector audit (ADR-0083, `docs/NATIVE_MESSAGING.md` §7.3):
//! per-user native-messaging manifests and registry keys that point a browser
//! at a host other than the one installed with this app, and their repair.
//!
//! The installed host is found from this executable's own path, never from
//! the webview: the host beside the app (Windows, macOS: `Contents/MacOS/`),
//! or `/usr/lib/mosaic/mosaic-native-host` of the Linux packages. A build
//! without an installed host (a developer build) has nothing to audit.

use std::path::{Path, PathBuf};

use mosaic_native_messaging::install::{
    self, Browser, LINUX_SYSTEM_HOST, Layout, ManifestFinding, RegExe, RegExeMachine, Registry,
};

use crate::contract::browser::{BrowserConnectorAudit, BrowserConnectorFinding, BrowserFamily};

/// The host program of this install: the Linux package location if it exists
/// there, otherwise the one beside `exe`. `None` if there is none.
#[must_use]
pub(crate) fn host_beside(exe: &Path, linux_system_host: &Path) -> Option<PathBuf> {
    if cfg!(target_os = "linux") && linux_system_host.is_file() {
        return Some(linux_system_host.to_path_buf());
    }
    let name = if cfg!(windows) {
        "mosaic-native-host.exe"
    } else {
        "mosaic-native-host"
    };
    let sibling = exe.parent()?.join(name);
    sibling.is_file().then_some(sibling)
}

/// The host program installed with this app.
#[must_use]
pub fn installed_host() -> Option<PathBuf> {
    host_beside(&std::env::current_exe().ok()?, Path::new(LINUX_SYSTEM_HOST))
}

fn family(browser: Browser) -> BrowserFamily {
    match browser {
        Browser::Firefox => BrowserFamily::Firefox,
        _ => BrowserFamily::Chromium,
    }
}

fn to_contract(findings: &[ManifestFinding]) -> BrowserConnectorAudit {
    BrowserConnectorAudit {
        findings: findings
            .iter()
            .map(|f| BrowserConnectorFinding {
                browser: family(f.browser),
                location: f.location.clone(),
                path: f.path.to_string_lossy().into_owned(),
            })
            .collect(),
    }
}

/// The audit against `host`.
pub(crate) fn audit_with(
    layout: &Layout,
    host: &Path,
    registry: &mut dyn Registry,
) -> BrowserConnectorAudit {
    to_contract(&install::audit(layout, host, registry))
}

/// Rewrites the entries that [`audit_with`] reports to `host`, then audits
/// again. `None` if the rewrite failed.
pub(crate) fn repair_with(
    layout: &Layout,
    host: &Path,
    registry: &mut dyn Registry,
) -> Option<BrowserConnectorAudit> {
    let findings = install::audit(layout, host, registry);
    if !findings.is_empty() {
        install::repair(layout, &findings, host, registry).ok()?;
    }
    Some(audit_with(layout, host, registry))
}

/// Writes the per-user entries of every browser, pointing at `host`
/// (macOS has no system location, so the app owns these).
pub(crate) fn refresh_with(
    layout: &Layout,
    host: &Path,
    registry: &mut dyn Registry,
) -> Option<()> {
    install::install(layout, &Browser::ALL, host, registry).ok()?;
    Some(())
}

/// The audit of this machine: the per-user entries, and on Windows the
/// machine-wide ones of the MSI (M9-F3: a host that is not where the system
/// manifests say is reported too). Empty without an installed host.
#[must_use]
pub fn audit() -> BrowserConnectorAudit {
    match (installed_host(), Layout::current()) {
        (Some(host), Ok(layout)) => {
            let mut audit = audit_with(&layout, &host, &mut RegExe);
            audit.findings.extend(
                to_contract(&install::audit_machine(
                    &layout,
                    &host,
                    &mut RegExe,
                    &mut RegExeMachine,
                ))
                .findings,
            );
            audit
        }
        _ => BrowserConnectorAudit { findings: vec![] },
    }
}

/// Why [`repair`] did nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RepairError {
    /// No host program is installed with this app (a developer build).
    HostMissing,
    /// An entry could not be written.
    WriteFailed,
}

/// The repair of this machine. A machine-wide entry that is wrong cannot be
/// rewritten without an administrator; the per-user entry that repair writes
/// for that browser comes first for the browsers, so the audit is clean after.
///
/// # Errors
/// [`RepairError::HostMissing`] without an installed host,
/// [`RepairError::WriteFailed`] if the rewrite failed.
pub fn repair() -> Result<BrowserConnectorAudit, RepairError> {
    let host = installed_host().ok_or(RepairError::HostMissing)?;
    let layout = Layout::current().map_err(|_| RepairError::WriteFailed)?;
    repair_with(&layout, &host, &mut RegExe).ok_or(RepairError::WriteFailed)?;
    let machine = install::audit_machine(&layout, &host, &mut RegExe, &mut RegExeMachine);
    if !machine.is_empty() {
        install::repair(&layout, &machine, &host, &mut RegExe)
            .map_err(|_| RepairError::WriteFailed)?;
    }
    Ok(audit())
}

/// At start on macOS: points the per-user manifests into the signed bundle.
/// Best effort; on other systems it does nothing (the installers own the
/// manifests there and the audit reports overrides).
pub fn refresh_at_start() {
    if !cfg!(target_os = "macos") {
        return;
    }
    if let (Some(host), Ok(layout)) = (installed_host(), Layout::current()) {
        let _ = refresh_with(&layout, &host, &mut RegExe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct NoRegistry;

    impl Registry for NoRegistry {
        fn set_default(
            &mut self,
            _: &str,
            _: &Path,
        ) -> Result<(), mosaic_native_messaging::error::Error> {
            Ok(())
        }
        fn default_value(&mut self, _: &str) -> Option<PathBuf> {
            None
        }
        fn delete(&mut self, _: &str) -> Result<(), mosaic_native_messaging::error::Error> {
            Ok(())
        }
    }

    fn layout(root: &Path) -> Layout {
        Layout {
            os: if cfg!(target_os = "macos") {
                install::Os::MacOs
            } else {
                install::Os::Linux
            },
            home: root.join("home"),
            local_app_data: PathBuf::new(),
        }
    }

    #[test]
    fn host_is_found_beside_the_executable_only() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("mosaic-desktop");
        let nowhere = dir.path().join("none");
        assert_eq!(host_beside(&exe, &nowhere), None);
        let name = if cfg!(windows) {
            "mosaic-native-host.exe"
        } else {
            "mosaic-native-host"
        };
        std::fs::write(dir.path().join(name), b"").unwrap();
        assert_eq!(host_beside(&exe, &nowhere), Some(dir.path().join(name)));
    }

    #[cfg(not(windows))]
    #[test]
    fn audit_and_repair_work_through_the_installed_host() {
        let dir = tempfile::tempdir().unwrap();
        let layout = layout(dir.path());
        let installed = dir.path().join("bin").join("mosaic-native-host");
        let other = dir.path().join("evil").join("mosaic-native-host");
        let mut registry = NoRegistry;

        // A developer-style install pointing elsewhere is a finding.
        install::install(
            &layout,
            &[Browser::Chrome, Browser::Firefox],
            &other,
            &mut registry,
        )
        .unwrap();
        let audit = audit_with(&layout, &installed, &mut registry);
        let browsers: Vec<_> = audit.findings.iter().map(|f| f.browser).collect();
        assert_eq!(browsers, [BrowserFamily::Chromium, BrowserFamily::Firefox]);
        assert!(
            audit
                .findings
                .iter()
                .all(|f| f.path == other.to_string_lossy())
        );

        // Repair rewrites those entries and audits clean.
        let after = repair_with(&layout, &installed, &mut registry).unwrap();
        assert!(after.findings.is_empty());
        // Repair does not create entries for browsers that had none.
        assert!(
            !install::plan(&layout, &[Browser::Edge], &installed)
                .unwrap()
                .files[0]
                .path
                .exists()
        );

        // The macOS refresh writes all four browsers at once.
        refresh_with(&layout, &installed, &mut registry).unwrap();
        assert!(
            install::plan(&layout, &Browser::ALL, &installed)
                .unwrap()
                .files
                .iter()
                .all(|f| f.path.exists())
        );
        assert!(
            audit_with(&layout, &installed, &mut registry)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn a_relative_host_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let layout = layout(dir.path());
        assert!(refresh_with(&layout, Path::new("relative-host"), &mut NoRegistry).is_none());
    }
}

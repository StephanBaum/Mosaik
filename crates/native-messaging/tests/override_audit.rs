//! M9 T12 red team: the per-user override audit (`install::audit`, ADR-0083,
//! `docs/NATIVE_MESSAGING.md` §7.3) against entries made to slip past it.
//!
//! The attacker runs as the user: it can write the user's manifests and
//! `HKCU`, not the system ones. Each test names the attempt and the defense:
//! a per-user entry that makes a browser start another program is reported.
//! The Windows registry is modelled as `reg.exe` output parsed by the same
//! [`parse_reg_query_default`] the real `RegExe` uses.
#![expect(
    clippy::unwrap_used,
    reason = "test helpers: a failed setup should panic"
)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use mosaic_native_messaging::Error;
use mosaic_native_messaging::ids::HOST_NAME;
use mosaic_native_messaging::install::{
    Browser, EXTENSIONS_DIFFER, Layout, Os, Registry, UNREADABLE, audit, manifest_json,
    parse_reg_query_default,
};

/// `HKCU\<key>` of each browser (`install.rs` `registry_key`).
fn key(browser: Browser) -> String {
    let vendor = match browser {
        Browser::Chrome => r"Google\Chrome",
        Browser::Edge => r"Microsoft\Edge",
        Browser::Chromium => "Chromium",
        Browser::Firefox => "Mozilla",
    };
    format!(r"Software\{vendor}\NativeMessagingHosts\{HOST_NAME}")
}

/// The registry as `reg.exe query <key> /ve` prints it: one value line per key,
/// `    (Default)    <TYPE>    <data>`.
#[derive(Default)]
struct RegQuery {
    lines: HashMap<String, String>,
}

impl RegQuery {
    fn put(&mut self, browser: Browser, kind: &str, data: &str) {
        self.lines.insert(
            key(browser),
            format!(
                "\r\nHKEY_CURRENT_USER\\{}\r\n    (Default)    {kind}    {data}\r\n\r\n",
                key(browser)
            ),
        );
    }
}

impl Registry for RegQuery {
    fn set_default(&mut self, key: &str, value: &Path) -> Result<(), Error> {
        self.lines.insert(
            key.to_owned(),
            format!("    (Default)    REG_SZ    {}\r\n", value.display()),
        );
        Ok(())
    }

    fn default_value(&mut self, key: &str) -> Option<PathBuf> {
        parse_reg_query_default(self.lines.get(key)?)
    }

    fn delete(&mut self, key: &str) -> Result<(), Error> {
        self.lines.remove(key);
        Ok(())
    }
}

fn windows_layout(root: &Path) -> Layout {
    Layout {
        os: Os::Windows,
        home: PathBuf::new(),
        local_app_data: root.join("Local"),
    }
}

fn unix_layout(os: Os, root: &Path) -> Layout {
    Layout {
        os,
        home: root.join("home"),
        local_app_data: PathBuf::new(),
    }
}

/// Chrome's per-user manifest of a Linux or macOS `layout`, its directory created.
/// (Built by hand: `install::plan` wants a host path that is absolute on the
/// machine running the test.)
fn chrome_manifest(layout: &Layout) -> PathBuf {
    let dir = if layout.os == Os::MacOs {
        layout
            .home
            .join("Library/Application Support/Google/Chrome/NativeMessagingHosts")
    } else {
        layout
            .home
            .join(".config/google-chrome/NativeMessagingHosts")
    };
    fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{HOST_NAME}.json"))
}

const WINDOWS_HOST: &str = r"C:\Program Files\Mosaic\mosaic-native-host.exe";
const LINUX_HOST: &str = "/usr/lib/mosaic/mosaic-native-host";

/// A Chromium manifest naming `host`, written to `dir/name`.
fn manifest_at(dir: &Path, name: &str, host: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, manifest_json(false, Path::new(host))).unwrap();
    path
}

fn browsers(findings: &[mosaic_native_messaging::install::ManifestFinding]) -> Vec<Browser> {
    findings.iter().map(|f| f.browser).collect()
}

/// Attempt (M9-F70): point Chrome at a foreign manifest with a `REG_EXPAND_SZ`
/// default value instead of `REG_SZ`. Chrome (`base::win::RegKey::ReadValue`)
/// and Firefox (`nsIWindowsRegKey::readStringValue`) read and expand both
/// types; the audit's parser looks for `REG_SZ` only, finds no value and treats
/// the key as absent.
/// Expected defense: the entry is reported (with its host, or as unreadable).
#[test]
fn audit_reports_a_reg_expand_sz_override() {
    let root = tempfile::tempdir().unwrap();
    let evil = manifest_at(root.path(), "evil.json", r"C:\Users\u\evil\host.exe");
    let mut registry = RegQuery::default();
    registry.put(Browser::Chrome, "REG_EXPAND_SZ", &evil.to_string_lossy());
    // The same as `%LOCALAPPDATA%`-relative text, which only the browser expands.
    registry.put(
        Browser::Edge,
        "REG_EXPAND_SZ",
        r"%LOCALAPPDATA%\evil\evil.json",
    );
    let findings = audit(
        &windows_layout(root.path()),
        Path::new(WINDOWS_HOST),
        &mut registry,
    );
    assert_eq!(browsers(&findings), [Browser::Chrome, Browser::Edge]);
}

/// Attempt: the same foreign manifest behind `REG_SZ` values padded with spaces,
/// tabs, an extended-length (`\\?\`) path, or a UNC path.
/// Expected defense: every one is reported; padding of Mosaic's own value is not.
#[test]
fn audit_reports_padded_and_unusual_registry_values() {
    let root = tempfile::tempdir().unwrap();
    let evil = manifest_at(root.path(), "evil.json", r"C:\Users\u\evil\host.exe");
    let own = manifest_at(root.path(), "own.json", WINDOWS_HOST);
    let evil_text = evil.to_string_lossy().into_owned();
    let mut registry = RegQuery::default();
    registry.put(Browser::Chrome, "REG_SZ", &format!("{evil_text}   "));
    registry.put(Browser::Edge, "REG_SZ", &format!("\t{evil_text}\t"));
    registry.put(Browser::Chromium, "REG_SZ", &format!(r"\\?\{evil_text}"));
    // A share that does not answer: the audit cannot read it, so it reports it.
    registry.put(
        Browser::Firefox,
        "REG_SZ",
        r"\\mosaic-redteam.invalid\share\evil.json",
    );
    let layout = windows_layout(root.path());
    let findings = audit(&layout, Path::new(WINDOWS_HOST), &mut registry);
    assert_eq!(browsers(&findings), Browser::ALL);
    assert_eq!(findings[3].path, PathBuf::from(UNREADABLE));

    // Mosaic's own manifest behind a padded value: the same file, nothing to report.
    let mut registry = RegQuery::default();
    registry.put(
        Browser::Chrome,
        "REG_SZ",
        &format!("{}   ", own.to_string_lossy()),
    );
    assert_eq!(
        audit(&layout, Path::new(WINDOWS_HOST), &mut registry),
        Vec::new()
    );
}

/// Attempt: a manifest whose host path is a different program that reads like the
/// installed one: a UNC share, a `..` detour, a trailing space or dot, forward
/// slashes, a relative name, an extended-length prefix.
/// Expected defense: all are reported (a false alarm on an equivalent spelling is
/// the safe side); a pure case change on Windows is the same file and is not.
#[test]
fn audit_reports_host_paths_that_only_look_like_the_installed_one() {
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let spoofs = [
        r"\\attacker.invalid\share\mosaic-native-host.exe",
        r"C:\Program Files\Mosaic\..\Mosaic Evil\mosaic-native-host.exe",
        r"C:\Program Files\Mosaic\mosaic-native-host.exe ",
        r"C:\Program Files\Mosaic\mosaic-native-host.exe.",
        "C:/Program Files/Mosaic/mosaic-native-host.exe",
        "mosaic-native-host.exe",
        r"\\?\C:\Program Files\Mosaic\mosaic-native-host.exe",
        r"C:\Program Files\Mosaic\mosaic-native-host.exe:evil",
    ];
    for spoof in spoofs {
        let manifest = manifest_at(root.path(), "m.json", spoof);
        let mut registry = RegQuery::default();
        registry.put(Browser::Chrome, "REG_SZ", &manifest.to_string_lossy());
        let findings = audit(&layout, Path::new(WINDOWS_HOST), &mut registry);
        assert_eq!(browsers(&findings), [Browser::Chrome], "{spoof}");
        assert_eq!(findings[0].path, PathBuf::from(spoof), "{spoof}");
    }
    let manifest = manifest_at(
        root.path(),
        "m.json",
        r"c:\PROGRAM FILES\mosaic\MOSAIC-NATIVE-HOST.EXE",
    );
    let mut registry = RegQuery::default();
    registry.put(Browser::Chrome, "REG_SZ", &manifest.to_string_lossy());
    assert_eq!(
        audit(&layout, Path::new(WINDOWS_HOST), &mut registry),
        Vec::new()
    );

    // Linux and macOS paths are case-sensitive and not normalized: each is reported.
    for os in [Os::Linux, Os::MacOs] {
        let layout = unix_layout(os, root.path());
        for spoof in [
            "/usr/lib/mosaic/Mosaic-Native-Host",
            "/usr/lib/mosaic/../mosaic-evil/mosaic-native-host",
            "/tmp/mosaic-native-host",
            "mosaic-native-host",
        ] {
            fs::write(
                chrome_manifest(&layout),
                manifest_json(false, Path::new(spoof)),
            )
            .unwrap();
            let findings = audit(&layout, Path::new(LINUX_HOST), &mut RegQuery::default());
            assert_eq!(browsers(&findings), [Browser::Chrome], "{os:?} {spoof}");
        }
    }
}

/// Attempt (M9-F71): a host path that differs from the installed one only by a
/// character whose Unicode lowercase is ASCII (KELVIN SIGN U+212A lowercases to
/// `k`). NTFS compares names through its upcase table, where U+212A and `K`
/// stay different, so the path names another file in a directory the user can
/// create; the audit's `to_lowercase` comparison calls it the same.
/// Expected defense: the entry is reported.
#[test]
fn audit_reports_a_kelvin_sign_look_alike_host() {
    let root = tempfile::tempdir().unwrap();
    let installed = r"C:\Users\Kai\AppData\Local\Mosaic\mosaic-native-host.exe";
    let look_alike = "C:\\Users\\\u{212A}ai\\AppData\\Local\\Mosaic\\mosaic-native-host.exe";
    let manifest = manifest_at(root.path(), "m.json", look_alike);
    let mut registry = RegQuery::default();
    registry.put(Browser::Chrome, "REG_SZ", &manifest.to_string_lossy());
    let findings = audit(
        &windows_layout(root.path()),
        Path::new(installed),
        &mut registry,
    );
    assert_eq!(browsers(&findings), [Browser::Chrome]);
}

/// A symbolic link to a file, or `None` where this platform or account cannot
/// make one (Windows without the privilege).
fn try_symlink(target: &Path, link: &Path) -> Option<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).ok()
    }
}

/// Attempt: replace the per-user manifest with a symbolic link (to a foreign
/// manifest, to nothing) or with a directory.
/// Expected defense: the link is followed and its target's host reported; a
/// dangling link and a directory are reported as unreadable; a link to Mosaic's
/// own manifest is not a finding.
#[test]
fn audit_follows_links_and_reports_unreadable_entries() {
    let root = tempfile::tempdir().unwrap();
    let layout = unix_layout(Os::Linux, root.path());
    let host = Path::new(LINUX_HOST);
    let file = chrome_manifest(&layout);
    fs::write(&file, manifest_json(false, host)).unwrap();
    assert_eq!(audit(&layout, host, &mut RegQuery::default()), Vec::new());

    // A directory where the manifest should be.
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let findings = audit(&layout, host, &mut RegQuery::default());
    assert_eq!(browsers(&findings), [Browser::Chrome]);
    assert_eq!(findings[0].path, PathBuf::from(UNREADABLE));
    fs::remove_dir(&file).unwrap();

    let evil = manifest_at(root.path(), "evil.json", "/home/u/.local/evil-host");
    let own = manifest_at(root.path(), "own.json", LINUX_HOST);
    if try_symlink(&evil, &file).is_none() {
        // No symlink privilege on this machine: the directory case above ran.
        return;
    }
    let findings = audit(&layout, host, &mut RegQuery::default());
    assert_eq!(browsers(&findings), [Browser::Chrome]);
    assert_eq!(findings[0].path, PathBuf::from("/home/u/.local/evil-host"));

    fs::remove_file(&file).unwrap();
    try_symlink(&root.path().join("gone.json"), &file).unwrap();
    let findings = audit(&layout, host, &mut RegQuery::default());
    assert_eq!(browsers(&findings), [Browser::Chrome]);
    assert_eq!(findings[0].path, PathBuf::from(UNREADABLE));

    fs::remove_file(&file).unwrap();
    try_symlink(&own, &file).unwrap();
    assert_eq!(audit(&layout, host, &mut RegQuery::default()), Vec::new());
}

/// Attempt (M9-F2): a manifest with the installed host's path but another
/// extension allowed to start it, or with the other browser family's list.
/// Expected defense: reported; the exact manifest Mosaic writes is not.
#[test]
fn audit_reports_a_foreign_extension_behind_the_right_host_path() {
    let root = tempfile::tempdir().unwrap();
    let layout = unix_layout(Os::Linux, root.path());
    let host = Path::new(LINUX_HOST);
    let file = chrome_manifest(&layout);
    for edit in [
        (
            "allowed_origins",
            serde_json::json!(["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"]),
        ),
        ("allowed_origins", serde_json::json!([])),
        (
            "allowed_extensions",
            serde_json::json!(["evil@example.org"]),
        ),
    ] {
        let mut manifest: serde_json::Value =
            serde_json::from_str(&manifest_json(false, host)).unwrap();
        manifest[edit.0] = edit.1;
        fs::write(&file, manifest.to_string()).unwrap();
        let findings = audit(&layout, host, &mut RegQuery::default());
        assert_eq!(browsers(&findings), [Browser::Chrome], "{manifest}");
        assert_eq!(findings[0].path, PathBuf::from(EXTENSIONS_DIFFER));
    }
    // An extra allowed origin is a difference too.
    let mut manifest: serde_json::Value =
        serde_json::from_str(&manifest_json(false, host)).unwrap();
    manifest["allowed_origins"]
        .as_array_mut()
        .unwrap()
        .push("chrome-extension://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/".into());
    fs::write(&file, manifest.to_string()).unwrap();
    assert_eq!(audit(&layout, host, &mut RegQuery::default()).len(), 1);
    // The manifest as Mosaic writes it is clean.
    fs::write(&file, manifest_json(false, host)).unwrap();
    assert_eq!(audit(&layout, host, &mut RegQuery::default()), Vec::new());
}

/// A registry value of a type that cannot name a manifest is a finding, not an
/// absent key; `REG_EXPAND_SZ` is expanded like the browsers do (M9-F70).
#[test]
fn audit_reports_other_registry_value_types_as_unreadable() {
    let root = tempfile::tempdir().unwrap();
    let mut registry = RegQuery::default();
    registry.put(Browser::Chrome, "REG_BINARY", "DEADBEEF");
    registry.put(Browser::Edge, "REG_MULTI_SZ", r"C:.json C:.json");
    let findings = audit(
        &windows_layout(root.path()),
        Path::new(WINDOWS_HOST),
        &mut registry,
    );
    assert_eq!(browsers(&findings), [Browser::Chrome, Browser::Edge]);
    assert!(findings.iter().all(|f| f.path == Path::new(UNREADABLE)));
    // The value name may hold spaces (localized); the data keeps its `REG_`.
    let line = r"    (Par défaut)    REG_SZ    C:\REG_x\m.json";
    assert_eq!(
        parse_reg_query_default(line),
        Some(PathBuf::from(r"C:\REG_x\m.json"))
    );
    unknown_variables_stay_as_written();
}

fn unknown_variables_stay_as_written() {
    // `%SystemRoot%` exists on Windows only; elsewhere the text stays as written.
    let line = r"    (Default)    REG_EXPAND_SZ    %MOSAIC_NO_SUCH_VAR_9%\m.json";
    assert_eq!(
        parse_reg_query_default(line),
        Some(PathBuf::from(r"%MOSAIC_NO_SUCH_VAR_9%\m.json"))
    );
}

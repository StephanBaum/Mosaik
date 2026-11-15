//! Tests of the override audit and the system manifests (M9 T8).

use super::tests::{Recorder, host, unix_layout, windows_layout};
use super::*;

fn other_host(root: &Path) -> PathBuf {
    root.join("evil").join("mosaic-native-host")
}

#[test]
fn audit_flags_per_user_manifest_pointing_elsewhere() {
    let root = tempfile::tempdir().unwrap();
    for os in [Os::Linux, Os::MacOs] {
        let layout = unix_layout(os, root.path());
        let installed = host(root.path());
        // Nothing written: nothing to report.
        assert_eq!(
            audit(&layout, &installed, &mut Recorder::default()),
            Vec::new()
        );

        install(&layout, &Browser::ALL, &installed, &mut Recorder::default()).unwrap();
        assert_eq!(
            audit(&layout, &installed, &mut Recorder::default()),
            Vec::new()
        );

        // Another program's host: every browser is reported with its path.
        let findings = audit(&layout, &other_host(root.path()), &mut Recorder::default());
        let browsers: Vec<_> = findings.iter().map(|f| f.browser).collect();
        assert_eq!(browsers, Browser::ALL.to_vec());
        assert!(findings.iter().all(|f| f.path == installed));

        // One manifest rewritten, one damaged: exactly those two.
        let written = plan(&layout, &Browser::ALL, &installed).unwrap();
        let chrome = &written.files[0].path;
        let edge = &written.files[1].path;
        fs::write(chrome, manifest_json(false, &other_host(root.path()))).unwrap();
        fs::write(edge, "not json").unwrap();
        let findings = audit(&layout, &installed, &mut Recorder::default());
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].browser, Browser::Chrome);
        assert_eq!(findings[0].path, other_host(root.path()));
        assert_eq!(findings[0].location, chrome.to_string_lossy());
        assert_eq!(findings[1].browser, Browser::Edge);
        assert_eq!(findings[1].path, PathBuf::from(UNREADABLE));

        // Repair fixes what was found.
        repair(&layout, &findings, &installed, &mut Recorder::default()).unwrap();
        assert_eq!(
            audit(&layout, &installed, &mut Recorder::default()),
            Vec::new()
        );
    }
}

#[test]
fn audit_reads_the_windows_registry_keys() {
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let installed = root
        .path()
        .join("Program Files")
        .join("mosaic-native-host.exe");
    let mut registry = Recorder::default();
    install(&layout, &Browser::ALL, &installed, &mut registry).unwrap();
    // Case differences are not a difference on Windows.
    let upper = PathBuf::from(installed.to_string_lossy().to_uppercase());
    assert_eq!(audit(&layout, &upper, &mut registry), Vec::new());

    // A key that points to somebody else's manifest.
    let evil = root.path().join("evil.json");
    fs::write(&evil, manifest_json(false, &other_host(root.path()))).unwrap();
    let key = registry_key(Browser::Edge);
    registry.values.insert(key.clone(), evil);
    let findings = audit(&layout, &installed, &mut registry);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].browser, Browser::Edge);
    assert_eq!(findings[0].location, format!(r"HKCU\{key}"));
    assert_eq!(findings[0].path, other_host(root.path()));

    // A key whose manifest is gone is reported as unreadable; a missing key
    // is not an entry.
    registry.values.insert(
        registry_key(Browser::Firefox),
        root.path().join("gone.json"),
    );
    registry.values.remove(&registry_key(Browser::Chrome));
    let findings = audit(&layout, &installed, &mut registry);
    let browsers: Vec<_> = findings.iter().map(|f| f.browser).collect();
    assert_eq!(browsers, [Browser::Edge, Browser::Firefox]);
    assert_eq!(findings[1].path, PathBuf::from(UNREADABLE));

    repair(&layout, &findings, &installed, &mut registry).unwrap();
    assert_eq!(audit(&layout, &installed, &mut registry), Vec::new());
}

#[test]
fn system_manifest_generation_matches_install_rules() {
    // Linux: the four system locations, each with the manifest the developer
    // install would write for that browser family.
    let linux = system_plan(Os::Linux, LINUX_SYSTEM_HOST).unwrap();
    let rel: Vec<String> = linux
        .iter()
        .map(|f| f.path.to_string_lossy().replace('\\', "/"))
        .collect();
    let name = format!("{HOST_NAME}.json");
    assert_eq!(
        rel,
        [
            format!("etc/opt/chrome/native-messaging-hosts/{name}"),
            format!("etc/opt/edge/native-messaging-hosts/{name}"),
            format!("etc/chromium/native-messaging-hosts/{name}"),
            format!("usr/lib/mozilla/native-messaging-hosts/{name}"),
        ]
    );
    let host_path = Path::new(LINUX_SYSTEM_HOST);
    for (file, firefox) in linux.iter().zip([false, false, false, true]) {
        assert_eq!(file.contents, manifest_json(firefox, host_path));
        let json: serde_json::Value = serde_json::from_str(&file.contents).unwrap();
        assert_eq!(json["name"], HOST_NAME);
        assert_eq!(json["path"], LINUX_SYSTEM_HOST);
    }
    // The per-user layout writes the same file names, and its contents come
    // from the same function (a host path that is absolute on this OS).
    let user_host = Path::new(if cfg!(windows) { r"C:\h" } else { "/h" });
    let layout = unix_layout(Os::Linux, user_host);
    let user = plan(&layout, &Browser::ALL, user_host).unwrap();
    for ((system, user), firefox) in linux
        .iter()
        .zip(&user.files)
        .zip([false, false, false, true])
    {
        assert_eq!(system.path.file_name(), user.path.file_name());
        assert_eq!(user.contents, manifest_json(firefox, user_host));
    }

    // Windows: the two files the per-user layout also uses, with the Program
    // Files host (generated on any OS).
    let windows = system_plan(Os::Windows, WINDOWS_SYSTEM_HOST).unwrap();
    let names: Vec<_> = windows
        .iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["chromium.json", "firefox.json"]);
    let json: serde_json::Value = serde_json::from_str(&windows[0].contents).unwrap();
    assert_eq!(json["path"], WINDOWS_SYSTEM_HOST);
    assert_eq!(
        windows[1].contents,
        manifest_json(true, Path::new(WINDOWS_SYSTEM_HOST))
    );
    assert_eq!(windows_manifest_name(false), "chromium.json");
    assert_eq!(windows_manifest_name(true), "firefox.json");

    // The WiX fragment writes the per-user keys below HKLM, and names the
    // directory the manifests are installed to.
    let wxs = include_str!("../../../../apps/desktop/src-tauri/wix/native-messaging.wxs");
    for browser in Browser::ALL {
        let key = registry_key(browser);
        let below_software = key.strip_prefix(r"Software\").unwrap();
        assert!(
            wxs.contains(below_software),
            "{below_software} missing from the WiX fragment"
        );
    }
    assert!(wxs.contains(r"[INSTALLDIR]native-messaging\"));
    assert!(wxs.contains("HKLM"));
    // M9-F74: nothing is added to the system PATH (the install folder is not pinned).
    assert!(!wxs.contains("<Environment"));
    assert!(!wxs.contains("MosaicSystemPath"));
}

#[test]
fn system_manifests_are_refused_for_macos_and_relative_hosts() {
    assert!(system_plan(Os::MacOs, "/Applications/Mosaic.app/x").is_err());
    assert!(system_host(Os::MacOs).is_none());
    assert!(system_plan(Os::Linux, "mosaic-native-host").is_err());
    assert!(system_plan(Os::Linux, r"C:\x\host.exe").is_err());
    assert!(system_plan(Os::Windows, "/usr/lib/host").is_err());
    assert!(system_plan(Os::Windows, "host.exe").is_err());
    assert!(system_plan(Os::Windows, r"\\server\share\host.exe").is_ok());
}

#[test]
fn write_system_creates_the_tree_below_out() {
    let root = tempfile::tempdir().unwrap();
    let written = write_system(Os::Linux, LINUX_SYSTEM_HOST, root.path()).unwrap();
    assert_eq!(written.len(), 4);
    for path in &written {
        assert!(path.starts_with(root.path()));
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(json["name"], HOST_NAME);
    }
    assert_eq!(Os::parse("windows").unwrap(), Os::Windows);
    assert!(Os::parse("beos").is_err());
}

#[test]
fn reg_expand_sz_values_are_expanded_from_the_environment() {
    let env = |name: &str| (name == "LOCALAPPDATA").then(|| r"C:\Users\u\AppData\Local".to_owned());
    let line = r"    (Default)    REG_EXPAND_SZ    %LOCALAPPDATA%\Mosaic\chromium.json";
    assert_eq!(
        parse_reg_query_with(line, env),
        Some(PathBuf::from(
            r"C:\Users\u\AppData\Local\Mosaic\chromium.json"
        ))
    );
    // Unknown names, a lone percent sign and an empty pair stay as written.
    assert_eq!(expand_env(r"%NOPE%\a", &env), r"%NOPE%\a");
    assert_eq!(expand_env(r"100% \a", &env), r"100% \a");
    assert_eq!(expand_env("%%x", &env), "%%x");
    // A value of another type is the unreadable marker, not an absent key.
    assert_eq!(
        parse_reg_query_with("    (Default)    REG_DWORD    0x1", env),
        Some(PathBuf::from(UNREADABLE))
    );
    assert_eq!(parse_reg_query_with("no value here", env), None);
}

#[test]
fn machine_audit_reports_a_system_entry_for_another_host_unless_shadowed() {
    use std::collections::HashMap;
    struct Machine(HashMap<String, PathBuf>);
    impl Registry for Machine {
        fn set_default(&mut self, _: &str, _: &Path) -> Result<(), Error> {
            Err(Error::Install)
        }
        fn default_value(&mut self, key: &str) -> Option<PathBuf> {
            self.0.get(key).cloned()
        }
        fn delete(&mut self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn root(&self) -> &'static str {
            "HKLM"
        }
    }
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let installed = Path::new(r"D:\Apps\Mosaic\mosaic-native-host.exe");
    // The MSI's manifest, generated for the default folder.
    let system = root.path().join("chromium.json");
    fs::write(
        &system,
        manifest_json(false, Path::new(WINDOWS_SYSTEM_HOST)),
    )
    .unwrap();
    let key = registry_key(Browser::Chrome);
    let mut machine = Machine(HashMap::from([(key.clone(), system)]));
    let found = audit_machine(&layout, installed, &mut Recorder::default(), &mut machine);
    assert_eq!(found.len(), 1);
    assert!(found[0].location.starts_with(r"HKLM\Software\Google"));
    assert_eq!(found[0].path, PathBuf::from(WINDOWS_SYSTEM_HOST));
    // Where the host is where the system manifest says, there is nothing.
    assert_eq!(
        audit_machine(
            &layout,
            Path::new(WINDOWS_SYSTEM_HOST),
            &mut Recorder::default(),
            &mut machine
        ),
        Vec::new()
    );
    // Other systems have no machine-wide registry.
    let unix = unix_layout(Os::Linux, root.path());
    assert_eq!(
        audit_machine(&unix, installed, &mut Recorder::default(), &mut machine),
        Vec::new()
    );
}

/// CHAIN-1: a release install removes the preview's per-user keys and
/// manifests (they point at another host) once the MSI's entry is the fallback,
/// and nothing else.
#[test]
fn foreign_user_entries_are_removed_once_the_machine_entry_is_the_fallback() {
    use std::collections::HashMap;
    struct Machine(HashMap<String, PathBuf>);
    impl Registry for Machine {
        fn set_default(&mut self, _: &str, _: &Path) -> Result<(), Error> {
            Err(Error::Install)
        }
        fn default_value(&mut self, key: &str) -> Option<PathBuf> {
            self.0.get(key).cloned()
        }
        fn delete(&mut self, _: &str) -> Result<(), Error> {
            Err(Error::Install)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let release = &root
        .path()
        .join("Program Files")
        .join("mosaic-native-host.exe");
    let preview = root
        .path()
        .join("Local")
        .join("Mosaic")
        .join("mosaic-native-host.exe");
    // The MSI's manifests (HKLM) for the release host.
    let mut machine = Machine(HashMap::new());
    for browser in Browser::ALL {
        let file = root.path().join(format!("system-{}.json", browser.name()));
        fs::write(&file, manifest_json(browser.is_firefox(), release)).unwrap();
        machine.0.insert(registry_key(browser), file);
    }
    // What the preview left: per-user manifests and keys for its own host.
    let mut user = Recorder::default();
    install(&layout, &Browser::ALL, &preview, &mut user).unwrap();
    let files = plan(&layout, &Browser::ALL, &preview).unwrap().files;
    assert!(files.iter().all(|f| f.path.is_file()));

    // Without the machine entry nothing is removed (no fallback).
    let mut empty = Machine(HashMap::new());
    assert_eq!(
        remove_foreign_user_entries(&layout, release, &mut user, &mut empty).unwrap(),
        0
    );
    assert_eq!(user.values.len(), 4);

    // An entry of somebody else stays; the rest of the residue goes.
    let foreign = PathBuf::from(r"D:\other\chromium.json");
    user.values
        .insert(registry_key(Browser::Edge), foreign.clone());
    let removed = remove_foreign_user_entries(&layout, release, &mut user, &mut machine).unwrap();
    assert_eq!(removed, 3 + files.len());
    assert_eq!(user.values.len(), 1);
    assert_eq!(
        user.values.get(&registry_key(Browser::Edge)),
        Some(&foreign)
    );
    assert!(files.iter().all(|f| !f.path.exists()));
    assert_eq!(
        remove_foreign_user_entries(&layout, release, &mut user, &mut machine).unwrap(),
        0,
        "nothing is left to remove"
    );

    // Entries that point to the installed host (what a repair writes) stay.
    let mut user = Recorder::default();
    install(&layout, &Browser::ALL, release, &mut user).unwrap();
    assert_eq!(
        remove_foreign_user_entries(&layout, release, &mut user, &mut machine).unwrap(),
        0
    );
    assert_eq!(user.values.len(), 4);
    // Other systems have no registry.
    let unix = unix_layout(Os::Linux, root.path());
    assert_eq!(
        remove_foreign_user_entries(&unix, release, &mut user, &mut machine).unwrap(),
        0
    );
}

/// FIX-3: a key that reg.exe could not delete keeps its manifest and its
/// value (the audit's banner and repair path see it); keys that did go lose
/// their manifests; the failure is reported, not swallowed.
#[test]
fn a_key_that_cannot_be_deleted_keeps_its_manifest() {
    use std::collections::{HashMap, HashSet};
    struct Machine(HashMap<String, PathBuf>);
    impl Registry for Machine {
        fn set_default(&mut self, _: &str, _: &Path) -> Result<(), Error> {
            Err(Error::Install)
        }
        fn default_value(&mut self, key: &str) -> Option<PathBuf> {
            self.0.get(key).cloned()
        }
        fn delete(&mut self, _: &str) -> Result<(), Error> {
            Err(Error::Install)
        }
    }
    /// Deleting the keys in `stuck` fails and leaves them in place.
    struct Failing {
        inner: Recorder,
        stuck: HashSet<String>,
    }
    impl Registry for Failing {
        fn set_default(&mut self, key: &str, value: &Path) -> Result<(), Error> {
            self.inner.set_default(key, value)
        }
        fn default_value(&mut self, key: &str) -> Option<PathBuf> {
            self.inner.default_value(key)
        }
        fn delete(&mut self, key: &str) -> Result<(), Error> {
            if self.stuck.contains(key) {
                Err(Error::Install)
            } else {
                self.inner.delete(key)
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let release = &root
        .path()
        .join("Program Files")
        .join("mosaic-native-host.exe");
    let preview = root
        .path()
        .join("Local")
        .join("Mosaic")
        .join("mosaic-native-host.exe");
    let mut machine = Machine(HashMap::new());
    for browser in Browser::ALL {
        let file = root.path().join(format!("system-{}.json", browser.name()));
        fs::write(&file, manifest_json(browser.is_firefox(), release)).unwrap();
        machine.0.insert(registry_key(browser), file);
    }
    let stuck_key = registry_key(Browser::Chrome);
    let mut user = Failing {
        inner: Recorder::default(),
        stuck: HashSet::from([stuck_key.clone()]),
    };
    install(&layout, &Browser::ALL, &preview, &mut user).unwrap();
    let files = plan(&layout, &Browser::ALL, &preview).unwrap();
    let stuck_manifest = files
        .registry
        .iter()
        .find(|e| e.key == stuck_key)
        .unwrap()
        .manifest
        .clone();
    assert!(stuck_manifest.is_file());

    let result = remove_foreign_user_entries(&layout, release, &mut user, &mut machine);
    assert!(result.is_err(), "the failure is reported");
    assert!(
        user.inner.values.contains_key(&stuck_key),
        "the key that could not go is still there"
    );
    assert!(
        stuck_manifest.is_file(),
        "its manifest stays while the key does"
    );
    for entry in files.registry.iter().filter(|e| e.key != stuck_key) {
        assert!(
            !user.inner.values.contains_key(&entry.key),
            "the other keys went"
        );
        if entry.manifest != stuck_manifest {
            assert!(!entry.manifest.exists(), "and their manifests with them");
        }
    }
}

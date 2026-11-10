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

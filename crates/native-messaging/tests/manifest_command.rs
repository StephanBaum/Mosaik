//! `mosaic-native-host manifest --system` as the release build runs it.

use std::process::Command;

const HOST: &str = env!("CARGO_BIN_EXE_mosaic-native-host");

#[test]
fn manifest_system_writes_the_packages_manifests() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(HOST)
        .args(["manifest", "--system", "--os", "linux", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    let chrome = out
        .path()
        .join("etc/opt/chrome/native-messaging-hosts/io.github.stephanbaum.mosaic.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(chrome).unwrap()).unwrap();
    assert_eq!(json["path"], "/usr/lib/mosaic/mosaic-native-host");
    assert_eq!(json["type"], "stdio");

    let windows = tempfile::tempdir().unwrap();
    let status = Command::new(HOST)
        .args(["manifest", "--system", "--os", "windows", "--out"])
        .arg(windows.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    let firefox = std::fs::read_to_string(windows.path().join("firefox.json")).unwrap();
    assert!(firefox.contains("allowed_extensions"));
    assert!(firefox.contains(r"C:\\Program Files\\Mosaic\\mosaic-native-host.exe"));
}

#[test]
fn manifest_refuses_incomplete_or_unsupported_requests() {
    let out = tempfile::tempdir().unwrap();
    for args in [
        vec!["manifest", "--os", "linux"],
        vec!["manifest", "--system", "--os", "macos"],
        vec![
            "manifest", "--system", "--os", "linux", "--host", "relative",
        ],
        vec!["manifest", "--system", "--os", "plan9"],
    ] {
        let status = Command::new(HOST)
            .args(&args)
            .arg("--out")
            .arg(out.path())
            .output()
            .unwrap();
        assert!(!status.status.success(), "{args:?}");
    }
    assert!(std::fs::read_dir(out.path()).unwrap().next().is_none());
}

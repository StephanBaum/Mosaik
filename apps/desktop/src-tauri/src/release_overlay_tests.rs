//! `tauri.release.conf.json` is an overlay of `bundle.*` keys: it must not
//! touch any security setting (M9 T8, ADR-0082).

use serde_json::Value;

use mosaic_native_messaging::install::{LINUX_SYSTEM_HOST, Os, system_plan};

const OVERLAY: &str = include_str!("../tauri.release.conf.json");
const BASE: &str = include_str!("../tauri.conf.json");
const ENTITLEMENTS: &str = include_str!("../entitlements.plist");

fn overlay() -> Value {
    serde_json::from_str(OVERLAY).unwrap()
}

#[test]
fn release_overlay_touches_bundle_only() {
    let overlay = overlay();
    let top: Vec<&String> = overlay.as_object().unwrap().keys().collect();
    assert_eq!(top, ["bundle"], "the overlay may only set bundle keys");

    // The bundle keys it sets: packaging, signing and installer settings.
    let allowed = [
        "targets",
        "createUpdaterArtifacts",
        "externalBin",
        "windows",
        "macOS",
        "linux",
    ];
    for key in overlay["bundle"].as_object().unwrap().keys() {
        assert!(
            allowed.contains(&key.as_str()),
            "bundle.{key} is not expected"
        );
    }
    // `bundle.windows` is the installer's, not the app's `app.windows`; no
    // security-relevant word appears anywhere in the overlay.
    for word in [
        "csp",
        "devCsp",
        "capabilities",
        "security",
        "freezePrototype",
    ] {
        assert!(
            !OVERLAY.contains(&format!("\"{word}\"")),
            "the overlay mentions {word}"
        );
    }

    // Merged over the base, `app` is unchanged and the baseline is intact.
    let base: Value = serde_json::from_str(BASE).unwrap();
    assert!(base["app"]["security"]["csp"].is_string());
    assert!(base.get("bundle").is_some());
}

#[test]
fn release_overlay_installs_the_protected_layout() {
    let overlay = overlay();
    let bundle = &overlay["bundle"];
    // No NSIS and no AppImage: both install to user-writable places.
    assert_eq!(
        bundle["targets"],
        serde_json::json!(["msi", "dmg", "app", "deb", "rpm"])
    );
    // The names the release workflow builds (`<name>-<target triple>`).
    assert_eq!(
        bundle["externalBin"],
        serde_json::json!(["binaries/mosaic-native-host", "binaries/mosaic"])
    );
    assert_eq!(bundle["macOS"]["hardenedRuntime"], Value::Bool(true));
    assert_eq!(bundle["macOS"]["entitlements"], "entitlements.plist");
    assert_eq!(
        bundle["windows"]["wix"]["fragmentPaths"],
        serde_json::json!(["wix/native-messaging.wxs"])
    );
    let wxs = include_str!("../wix/native-messaging.wxs");
    let group = bundle["windows"]["wix"]["componentRefs"][0]
        .as_str()
        .unwrap();
    assert!(wxs.contains(&format!("ComponentGroup Id=\"{group}\"")));

    // deb and rpm install the host and exactly the manifests that
    // `mosaic-native-host manifest --system --os linux` writes.
    let manifests = system_plan(Os::Linux, LINUX_SYSTEM_HOST).unwrap();
    for format in ["deb", "rpm"] {
        let files = bundle["linux"][format]["files"].as_object().unwrap();
        assert_eq!(files.len(), manifests.len() + 1, "{format}");
        assert_eq!(
            files[LINUX_SYSTEM_HOST],
            "binaries/mosaic-native-host-x86_64-unknown-linux-gnu"
        );
        for manifest in &manifests {
            let relative = manifest.path.to_string_lossy().replace('\\', "/");
            assert_eq!(
                files[&format!("/{relative}")],
                format!("binaries/native-messaging/linux/{relative}").as_str(),
                "{format} {relative}"
            );
        }
    }
}

#[test]
fn entitlements_grant_nothing() {
    // Hardened runtime without exceptions (ADR-0082): an empty dictionary.
    assert!(ENTITLEMENTS.contains("<dict/>"));
    assert!(!ENTITLEMENTS.contains("<key>"));
}

use super::*;

#[derive(Default)]
struct Recorder {
    set: Vec<(String, PathBuf)>,
    deleted: Vec<String>,
}

impl Registry for Recorder {
    fn set_default(&mut self, key: &str, value: &Path) -> Result<(), Error> {
        self.set.push((key.to_owned(), value.to_path_buf()));
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), Error> {
        self.deleted.push(key.to_owned());
        Ok(())
    }
}

fn unix_layout(os: Os, root: &Path) -> Layout {
    Layout {
        os,
        home: root.join("home"),
        local_app_data: PathBuf::new(),
    }
}

fn windows_layout(root: &Path) -> Layout {
    Layout {
        os: Os::Windows,
        home: PathBuf::new(),
        local_app_data: root.join("Local"),
    }
}

fn host(root: &Path) -> PathBuf {
    root.join("bin").join("mosaic-native-host")
}

fn paths(plan: &Plan) -> Vec<PathBuf> {
    plan.files.iter().map(|f| f.path.clone()).collect()
}

#[test]
fn linux_locations() {
    let root = tempfile::tempdir().unwrap();
    let layout = unix_layout(Os::Linux, root.path());
    let home = &layout.home;
    let plan = plan(&layout, &Browser::ALL, &host(root.path())).unwrap();
    let name = format!("{HOST_NAME}.json");
    assert_eq!(
        paths(&plan),
        vec![
            home.join(".config/google-chrome/NativeMessagingHosts")
                .join(&name),
            home.join(".config/microsoft-edge/NativeMessagingHosts")
                .join(&name),
            home.join(".config/chromium/NativeMessagingHosts")
                .join(&name),
            home.join(".mozilla/native-messaging-hosts").join(&name),
        ]
    );
    assert_eq!(plan.registry.len(), 0);
}

#[test]
fn macos_locations() {
    let root = tempfile::tempdir().unwrap();
    let layout = unix_layout(Os::MacOs, root.path());
    let support = layout.home.join("Library/Application Support");
    let plan = plan(&layout, &Browser::ALL, &host(root.path())).unwrap();
    let name = format!("{HOST_NAME}.json");
    assert_eq!(
        paths(&plan),
        vec![
            support
                .join("Google/Chrome/NativeMessagingHosts")
                .join(&name),
            support
                .join("Microsoft Edge/NativeMessagingHosts")
                .join(&name),
            support.join("Chromium/NativeMessagingHosts").join(&name),
            support.join("Mozilla/NativeMessagingHosts").join(&name),
        ]
    );
}

#[test]
fn windows_locations_share_one_chromium_manifest() {
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let plan = plan(&layout, &Browser::ALL, &host(root.path())).unwrap();
    let dir = layout
        .local_app_data
        .join("io.github.stephanbaum.mosaic")
        .join("NativeMessagingHosts");
    assert_eq!(
        paths(&plan),
        vec![dir.join("chromium.json"), dir.join("firefox.json")]
    );
    let keys: Vec<(&str, PathBuf)> = plan
        .registry
        .iter()
        .map(|e| (e.key.as_str(), e.manifest.clone()))
        .collect();
    assert_eq!(
        keys,
        vec![
            (
                r"Software\Google\Chrome\NativeMessagingHosts\io.github.stephanbaum.mosaic",
                dir.join("chromium.json")
            ),
            (
                r"Software\Microsoft\Edge\NativeMessagingHosts\io.github.stephanbaum.mosaic",
                dir.join("chromium.json")
            ),
            (
                r"Software\Chromium\NativeMessagingHosts\io.github.stephanbaum.mosaic",
                dir.join("chromium.json")
            ),
            (
                r"Software\Mozilla\NativeMessagingHosts\io.github.stephanbaum.mosaic",
                dir.join("firefox.json")
            ),
        ]
    );
}

#[test]
fn manifests_name_the_host_and_who_may_start_it() {
    let path = host(Path::new(if cfg!(windows) { "C:\\bin" } else { "/bin" }));
    let chromium: serde_json::Value = serde_json::from_str(&manifest_json(false, &path)).unwrap();
    assert_eq!(chromium["name"], HOST_NAME);
    assert_eq!(chromium["type"], "stdio");
    assert_eq!(chromium["path"], path.to_string_lossy().as_ref());
    let origins: Vec<String> = serde_json::from_value(chromium["allowed_origins"].clone()).unwrap();
    assert_eq!(origins, allowed_chromium_origins());
    assert!(
        origins
            .iter()
            .all(|o| o.starts_with("chrome-extension://") && o.ends_with('/'))
    );
    assert!(chromium.get("allowed_extensions").is_none());

    let firefox: serde_json::Value = serde_json::from_str(&manifest_json(true, &path)).unwrap();
    assert_eq!(
        firefox["allowed_extensions"],
        serde_json::json!([FIREFOX_EXTENSION_ID])
    );
    assert!(firefox.get("allowed_origins").is_none());
    assert_eq!(firefox["name"], HOST_NAME);
}

#[test]
fn install_writes_and_uninstall_removes_everything_it_wrote() {
    for os in [Os::Linux, Os::MacOs] {
        let root = tempfile::tempdir().unwrap();
        let layout = unix_layout(os, root.path());
        let mut registry = Recorder::default();
        let plan = install(
            &layout,
            &[Browser::Chrome, Browser::Firefox],
            &host(root.path()),
            &mut registry,
        )
        .unwrap();
        assert_eq!(plan.files.len(), 2);
        assert_eq!(registry.set.len(), 0);
        for file in &plan.files {
            assert_eq!(fs::read_to_string(&file.path).unwrap(), file.contents);
        }
        uninstall(&layout, &mut registry).unwrap();
        for file in &plan.files {
            assert!(!file.path.exists(), "{}", file.path.display());
        }
        // A second uninstall has nothing to do and succeeds.
        uninstall(&layout, &mut registry).unwrap();
    }
}

#[test]
fn windows_install_sets_the_keys_and_uninstall_deletes_them() {
    let root = tempfile::tempdir().unwrap();
    let layout = windows_layout(root.path());
    let mut registry = Recorder::default();
    let plan = install(
        &layout,
        &[Browser::Chrome, Browser::Edge],
        &host(root.path()),
        &mut registry,
    )
    .unwrap();
    assert_eq!(plan.files.len(), 1, "chrome and edge share a manifest");
    assert_eq!(registry.set.len(), 2);
    assert!(registry.set.iter().all(|(_, v)| *v == plan.files[0].path));
    assert!(plan.files[0].path.exists());

    let mut registry = Recorder::default();
    uninstall(&layout, &mut registry).unwrap();
    assert_eq!(registry.deleted.len(), 4, "every browser's key");
    assert!(!plan.files[0].path.exists());
    assert!(
        !plan.files[0].path.parent().unwrap().exists(),
        "empty directory removed"
    );
}

#[test]
fn a_relative_host_path_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let layout = unix_layout(Os::Linux, root.path());
    assert_eq!(
        plan(&layout, &Browser::ALL, Path::new("mosaic-native-host")),
        Err(Error::Install)
    );
}

#[test]
fn browser_lists_parse() {
    assert_eq!(
        Browser::parse_list("chrome,edge,chromium").unwrap(),
        vec![Browser::Chrome, Browser::Edge, Browser::Chromium]
    );
    assert_eq!(
        Browser::parse_list("firefox, firefox").unwrap(),
        vec![Browser::Firefox]
    );
    for bad in ["", "safari", "chrome,", ",chrome", "Chrome"] {
        assert_eq!(Browser::parse_list(bad), Err(Error::Install), "{bad:?}");
    }
}

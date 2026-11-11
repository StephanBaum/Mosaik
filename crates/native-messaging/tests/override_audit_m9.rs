//! M9 test-engineer pass on the override audit and the `reg query` parser:
//! non-ASCII manifest paths, `REG_SZ` and `REG_EXPAND_SZ`, CRLF output.
#![expect(
    clippy::unwrap_used,
    reason = "test helpers: a failed setup should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use mosaic_native_messaging::Error;
use mosaic_native_messaging::ids::HOST_NAME;
use mosaic_native_messaging::install::{
    Browser, Layout, Os, Registry, audit, manifest_json, parse_reg_query_default,
};

/// A registry with one fixed `reg query` answer for Chrome's key.
struct One(String);

impl Registry for One {
    fn set_default(&mut self, _: &str, _: &Path) -> Result<(), Error> {
        Ok(())
    }
    fn default_value(&mut self, key: &str) -> Option<PathBuf> {
        key.contains(r"Google\Chrome")
            .then(|| parse_reg_query_default(&self.0))
            .flatten()
    }
    fn delete(&mut self, _: &str) -> Result<(), Error> {
        Ok(())
    }
    fn root(&self) -> &'static str {
        "HKCU"
    }
}

fn layout(root: &Path) -> Layout {
    Layout {
        os: Os::Windows,
        home: PathBuf::new(),
        local_app_data: root.join("Local"),
    }
}

fn chrome(kind: &str, manifest: &Path) -> One {
    One(format!(
        "\r\nHKEY_CURRENT_USER\\Software\\Google\\Chrome\\NativeMessagingHosts\\{HOST_NAME}\r\n    (Default)    {kind}    {}\r\n\r\n",
        manifest.display()
    ))
}

fn write(dir: &Path, name: &str, host: &str) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, manifest_json(false, Path::new(host))).unwrap();
    p
}

/// A non-ASCII manifest path (and host path) survives `REG_SZ` and
/// `REG_EXPAND_SZ` parsing. The same host is not a finding, an ASCII-case
/// difference is the same Windows path, and a non-ASCII case difference
/// (`ë` against `Ë`) is another file and is reported.
#[test]
fn non_ascii_paths_are_audited_by_the_exact_host() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("Zo\u{eb} \u{65e5}\u{672c} \u{dc}n\u{ef}");
    fs::create_dir_all(&dir).unwrap();
    let own = "C:\\Users\\Zo\u{eb}\\AppData\\Local\\Mosaic\\mosaic-native-host.exe";
    for kind in ["REG_SZ", "REG_EXPAND_SZ"] {
        let m = write(&dir, "m.json", own);
        assert_eq!(
            parse_reg_query_default(&chrome(kind, &m).0),
            Some(m.clone()),
            "{kind}: the path survives parsing"
        );
        let found = audit(&layout(root.path()), Path::new(own), &mut chrome(kind, &m));
        assert!(found.is_empty(), "{kind}: {found:?}");

        let upper = own.replace("mosaic-native-host", "MOSAIC-NATIVE-HOST");
        let m = write(&dir, "m2.json", &upper);
        let found = audit(&layout(root.path()), Path::new(own), &mut chrome(kind, &m));
        assert!(found.is_empty(), "{kind}: ASCII case is the same path");

        let other = own.replace('\u{eb}', "\u{cb}");
        let m = write(&dir, "m3.json", &other);
        let found = audit(&layout(root.path()), Path::new(own), &mut chrome(kind, &m));
        assert_eq!(found.len(), 1, "{kind}");
        assert_eq!(found[0].browser, Browser::Chrome);
        assert_eq!(found[0].path, PathBuf::from(&other));
    }
}

/// `reg query` prints a blank line, the key and the value with CRLF line ends
/// and may separate with tabs. Adjacent `%VAR%` references both expand, a
/// non-ASCII tail is kept, and output without a value line is absent.
#[test]
fn reg_query_crlf_output_and_adjacent_references() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\X\r\n    (Default)\tREG_SZ\tC:\\a\\b.json\r\n\r\n";
    assert_eq!(
        parse_reg_query_default(out),
        Some(PathBuf::from("C:\\a\\b.json"))
    );
    // Cargo sets this for the test run.
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let line =
        "    (Default)    REG_EXPAND_SZ    %CARGO_MANIFEST_DIR%%CARGO_MANIFEST_DIR%\\\u{e9}.json";
    assert_eq!(
        parse_reg_query_default(line),
        Some(PathBuf::from(format!("{dir}{dir}\\\u{e9}.json")))
    );
    assert_eq!(
        parse_reg_query_default("HKEY_CURRENT_USER\\Software\\X\r\n"),
        None
    );
    assert_eq!(parse_reg_query_default(""), None);
}

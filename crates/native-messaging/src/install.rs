//! Registering the host with the browsers (`mosaic-native-host install` and
//! `uninstall`; `docs/NATIVE_MESSAGING.md` §7).
//!
//! A host manifest tells a browser where the host binary is and which
//! extensions may start it:
//!
//! - Chromium family: `{name, description, path, type: "stdio", allowed_origins}`
//!   with [`allowed_chromium_origins`].
//! - Firefox: the same with `allowed_extensions: [FIREFOX_EXTENSION_ID]`.
//!
//! | OS | Chromium family | Firefox |
//! |---|---|---|
//! | Windows | JSON in `%LOCALAPPDATA%\io.github.stephanbaum.mosaic\NativeMessagingHosts\{chromium,firefox}.json`, and the registry default value under `HKCU\Software\{Google\Chrome,Microsoft\Edge,Chromium}\NativeMessagingHosts\<HOST_NAME>` | `HKCU\Software\Mozilla\NativeMessagingHosts\<HOST_NAME>` |
//! | macOS | `~/Library/Application Support/{Google/Chrome,Microsoft Edge,Chromium}/NativeMessagingHosts/<HOST_NAME>.json` | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |
//! | Linux | `~/.config/{google-chrome,microsoft-edge,chromium}/NativeMessagingHosts/` | `~/.mozilla/native-messaging-hosts/` |
//!
//! The registry is written through `reg.exe` (no unsafe code, no extra
//! crate). This is the developer install; the M9 installer takes over with
//! store IDs only and a signed host.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::endpoint::APP_IDENTIFIER;
use crate::error::Error;
use crate::ids::{FIREFOX_EXTENSION_ID, HOST_NAME, allowed_chromium_origins};

/// A browser the host can be registered with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Browser {
    /// Google Chrome.
    Chrome,
    /// Microsoft Edge.
    Edge,
    /// Chromium.
    Chromium,
    /// Mozilla Firefox.
    Firefox,
}

impl Browser {
    /// Every browser, in install order.
    pub const ALL: [Self; 4] = [Self::Chrome, Self::Edge, Self::Chromium, Self::Firefox];

    /// The name used on the command line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Edge => "edge",
            Self::Chromium => "chromium",
            Self::Firefox => "firefox",
        }
    }

    const fn is_firefox(self) -> bool {
        matches!(self, Self::Firefox)
    }

    /// Parses a comma-separated list such as `chrome,edge`.
    ///
    /// # Errors
    /// [`Error::Install`] for an empty list or an unknown name.
    pub fn parse_list(list: &str) -> Result<Vec<Self>, Error> {
        let mut out = Vec::new();
        for name in list.split(',') {
            let browser = Self::ALL
                .into_iter()
                .find(|b| b.name() == name.trim())
                .ok_or(Error::Install)?;
            if !out.contains(&browser) {
                out.push(browser);
            }
        }
        Ok(out)
    }
}

/// The operating system layout to write for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Os {
    /// Manifests under `%LOCALAPPDATA%`, pointed to by registry keys.
    Windows,
    /// Manifests under `~/Library/Application Support`.
    MacOs,
    /// Manifests under `~/.config` and `~/.mozilla`.
    Linux,
}

/// Where manifests go on a machine.
#[derive(Clone, Debug)]
pub struct Layout {
    /// The OS conventions to follow.
    pub os: Os,
    /// The user's home directory (macOS, Linux).
    pub home: PathBuf,
    /// `%LOCALAPPDATA%` (Windows).
    pub local_app_data: PathBuf,
}

impl Layout {
    /// The layout of the running OS from the environment.
    ///
    /// # Errors
    /// [`Error::Install`] if the home directory (or `%LOCALAPPDATA%` on
    /// Windows) is not set to an absolute path.
    pub fn current() -> Result<Self, Error> {
        let var = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        let os = if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        };
        let (home, local_app_data) = if os == Os::Windows {
            (PathBuf::new(), var("LOCALAPPDATA").ok_or(Error::Install)?)
        } else {
            (var("HOME").ok_or(Error::Install)?, PathBuf::new())
        };
        Ok(Self {
            os,
            home,
            local_app_data,
        })
    }
}

/// A manifest file to write.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ManifestFile {
    /// Where.
    pub path: PathBuf,
    /// Its content.
    pub contents: String,
}

/// A registry key (under `HKCU`) whose default value is a manifest path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RegistryEntry {
    /// The key below `HKCU`.
    pub key: String,
    /// The manifest the key points to.
    pub manifest: PathBuf,
}

/// Everything an install writes.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Plan {
    /// Manifest files.
    pub files: Vec<ManifestFile>,
    /// Registry keys (Windows).
    pub registry: Vec<RegistryEntry>,
}

/// The manifest JSON for a Chromium-family browser or Firefox.
#[must_use]
pub fn manifest_json(firefox: bool, host_path: &Path) -> String {
    let mut manifest = serde_json::json!({
        "name": HOST_NAME,
        "description": "Mosaic password manager: connects the browser extension to the desktop app",
        "path": host_path.to_string_lossy(),
        "type": "stdio",
    });
    if let Some(map) = manifest.as_object_mut() {
        if firefox {
            map.insert(
                "allowed_extensions".to_owned(),
                serde_json::json!([FIREFOX_EXTENSION_ID]),
            );
        } else {
            map.insert(
                "allowed_origins".to_owned(),
                serde_json::json!(allowed_chromium_origins()),
            );
        }
    }
    let mut text = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    text.push('\n');
    text
}

/// The directory of a browser's manifests on macOS and Linux.
fn manifest_dir(layout: &Layout, browser: Browser) -> PathBuf {
    match (layout.os, browser) {
        (Os::MacOs, b) => {
            let support = layout.home.join("Library").join("Application Support");
            match b {
                Browser::Chrome => support.join("Google").join("Chrome"),
                Browser::Edge => support.join("Microsoft Edge"),
                Browser::Chromium => support.join("Chromium"),
                Browser::Firefox => support.join("Mozilla"),
            }
            .join("NativeMessagingHosts")
        }
        (_, Browser::Firefox) => layout.home.join(".mozilla").join("native-messaging-hosts"),
        (_, b) => {
            let config = layout.home.join(".config");
            match b {
                Browser::Chrome => config.join("google-chrome"),
                Browser::Edge => config.join("microsoft-edge"),
                _ => config.join("chromium"),
            }
            .join("NativeMessagingHosts")
        }
    }
}

/// The registry key below `HKCU` of a browser on Windows.
fn registry_key(browser: Browser) -> String {
    let vendor = match browser {
        Browser::Chrome => r"Google\Chrome",
        Browser::Edge => r"Microsoft\Edge",
        Browser::Chromium => "Chromium",
        Browser::Firefox => "Mozilla",
    };
    format!(r"Software\{vendor}\NativeMessagingHosts\{HOST_NAME}")
}

/// The Windows manifest files (shared by the Chromium family).
fn windows_manifest_path(layout: &Layout, firefox: bool) -> PathBuf {
    layout
        .local_app_data
        .join(APP_IDENTIFIER)
        .join("NativeMessagingHosts")
        .join(if firefox {
            "firefox.json"
        } else {
            "chromium.json"
        })
}

/// What installing `browsers` writes, with the host at `host_path`.
///
/// # Errors
/// [`Error::Install`] if `host_path` is not absolute.
pub fn plan(layout: &Layout, browsers: &[Browser], host_path: &Path) -> Result<Plan, Error> {
    if !host_path.is_absolute() {
        return Err(Error::Install);
    }
    let mut plan = Plan::default();
    for &browser in browsers {
        let path = if layout.os == Os::Windows {
            plan.registry.push(RegistryEntry {
                key: registry_key(browser),
                manifest: windows_manifest_path(layout, browser.is_firefox()),
            });
            windows_manifest_path(layout, browser.is_firefox())
        } else {
            manifest_dir(layout, browser).join(format!("{HOST_NAME}.json"))
        };
        if !plan.files.iter().any(|f| f.path == path) {
            plan.files.push(ManifestFile {
                path,
                contents: manifest_json(browser.is_firefox(), host_path),
            });
        }
    }
    Ok(plan)
}

/// Writes registry values; a trait so tests can record instead of touching
/// the registry.
pub trait Registry {
    /// Sets the default value of `HKCU\<key>`.
    ///
    /// # Errors
    /// [`Error::Install`] if it could not be set.
    fn set_default(&mut self, key: &str, value: &Path) -> Result<(), Error>;

    /// Deletes `HKCU\<key>`; a missing key is fine.
    ///
    /// # Errors
    /// Never in practice; the install is best effort.
    fn delete(&mut self, key: &str) -> Result<(), Error>;
}

/// The Windows registry through `reg.exe`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegExe;

impl Registry for RegExe {
    fn set_default(&mut self, key: &str, value: &Path) -> Result<(), Error> {
        let status = Command::new("reg")
            .arg("add")
            .arg(format!(r"HKCU\{key}"))
            .args(["/ve", "/t", "REG_SZ", "/d"])
            .arg(value)
            .arg("/f")
            .output()
            .map_err(|_| Error::Install)?
            .status;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Install)
        }
    }

    fn delete(&mut self, key: &str) -> Result<(), Error> {
        // A missing key is a non-zero exit; either way the key is gone.
        let _ = Command::new("reg")
            .arg("delete")
            .arg(format!(r"HKCU\{key}"))
            .arg("/f")
            .output();
        Ok(())
    }
}

/// Installs `browsers`: writes the manifests (and registry keys).
///
/// # Errors
/// [`Error::Install`] if a file or key could not be written.
pub fn install(
    layout: &Layout,
    browsers: &[Browser],
    host_path: &Path,
    registry: &mut dyn Registry,
) -> Result<Plan, Error> {
    let plan = plan(layout, browsers, host_path)?;
    for file in &plan.files {
        let dir = file.path.parent().ok_or(Error::Install)?;
        fs::create_dir_all(dir).map_err(|_| Error::Install)?;
        fs::write(&file.path, &file.contents).map_err(|_| Error::Install)?;
    }
    for entry in &plan.registry {
        registry.set_default(&entry.key, &entry.manifest)?;
    }
    Ok(plan)
}

/// Removes everything [`install`] can write, for every browser.
///
/// # Errors
/// [`Error::Install`] if a file could not be removed.
pub fn uninstall(layout: &Layout, registry: &mut dyn Registry) -> Result<Plan, Error> {
    // The path is not written, only used to build the plan.
    let any_host = if layout.os == Os::Windows {
        layout.local_app_data.join("mosaic-native-host.exe")
    } else {
        layout.home.join("mosaic-native-host")
    };
    let plan = plan(layout, &Browser::ALL, &any_host)?;
    for entry in &plan.registry {
        registry.delete(&entry.key)?;
    }
    for file in &plan.files {
        match fs::remove_file(&file.path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Install),
        }
    }
    if layout.os == Os::Windows
        && let Some(dir) = plan.files.first().and_then(|f| f.path.parent())
    {
        // Mosaic's own directory: remove it if nothing else is in it.
        let _ = fs::remove_dir(dir);
    }
    Ok(plan)
}

#[cfg(test)]
mod tests;

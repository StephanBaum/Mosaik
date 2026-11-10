//! Where Mosaic keeps its files. The desktop (Tauri's `app_data_dir`) and the
//! CLI use the same directory, so the CLI's `shared` profile opens the
//! desktop's store.
//!
//! The functions read the standard environment variables of each OS (no
//! extra dependency); the `*_in` forms take the lookup so tests can pin
//! every OS.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::{PlatformError, Result};

/// The app identifier: the directory name under the data directory and the
/// keystore service name. A test compares it with `tauri.conf.json`.
pub const APP_ID: &str = crate::keystore::SERVICE;

/// File name of the vault store inside its directory (as the desktop names it).
pub const STORE_FILE: &str = "vault.sqlite3";

/// The operating systems whose conventions are implemented.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Os {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and other Unix.
    Unix,
}

impl Os {
    /// The OS this build runs on.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unix
        }
    }
}

/// An environment lookup.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

fn var(env: Env<'_>, name: &str) -> Option<PathBuf> {
    env(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.has_root())
}

fn home(env: Env<'_>, os: Os) -> Option<PathBuf> {
    var(
        env,
        if os == Os::Windows {
            "USERPROFILE"
        } else {
            "HOME"
        },
    )
}

/// Per-user roaming data (`%APPDATA%`, `~/Library/Application Support`,
/// `$XDG_DATA_HOME` or `~/.local/share`).
///
/// # Errors
/// [`PlatformError::NoDirectory`] when the environment names none.
pub fn data_dir_in(env: Env<'_>, os: Os) -> Result<PathBuf> {
    let found = match os {
        Os::Windows => var(env, "APPDATA"),
        Os::MacOs => home(env, os).map(|h| h.join("Library").join("Application Support")),
        Os::Unix => {
            var(env, "XDG_DATA_HOME").or_else(|| home(env, os).map(|h| h.join(".local/share")))
        }
    };
    found.ok_or(PlatformError::NoDirectory)
}

/// Per-user local (non-roaming) data: `%LOCALAPPDATA%` on Windows, the same
/// as [`data_dir_in`] elsewhere.
///
/// # Errors
/// [`PlatformError::NoDirectory`] when the environment names none.
pub fn local_data_dir_in(env: Env<'_>, os: Os) -> Result<PathBuf> {
    match os {
        Os::Windows => var(env, "LOCALAPPDATA").ok_or(PlatformError::NoDirectory),
        Os::MacOs | Os::Unix => data_dir_in(env, os),
    }
}

/// Where the CLI keeps its session file (CRYPTO_SPEC §6.8). Unix: `$XDG_RUNTIME_DIR`
/// only (tmpfs, cleared at logout), with no persistent fallback. Windows and
/// macOS have no session-scoped directory: `<local_data_dir>/<APP_ID>/cli/run`
/// (the parts persist until swept or locked).
///
/// # Errors
/// [`PlatformError::NoRuntimeDir`] on Unix without `$XDG_RUNTIME_DIR`;
/// [`PlatformError::NoDirectory`] elsewhere when no fallback can be determined.
pub fn runtime_dir_in(env: Env<'_>, os: Os) -> Result<PathBuf> {
    if os == Os::Unix {
        return var(env, "XDG_RUNTIME_DIR").ok_or(PlatformError::NoRuntimeDir);
    }
    Ok(local_data_dir_in(env, os)?
        .join(APP_ID)
        .join("cli")
        .join("run"))
}

/// The desktop's store: `<data_dir>/<APP_ID>/vault.sqlite3`.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn desktop_store_in(env: Env<'_>, os: Os) -> Result<PathBuf> {
    Ok(data_dir_in(env, os)?.join(APP_ID).join(STORE_FILE))
}

/// The CLI's own device store: `<data_dir>/<APP_ID>/cli/vault.sqlite3`.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn cli_store_in(env: Env<'_>, os: Os) -> Result<PathBuf> {
    Ok(data_dir_in(env, os)?
        .join(APP_ID)
        .join("cli")
        .join(STORE_FILE))
}

fn process_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// [`data_dir_in`] for this process.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn data_dir() -> Result<PathBuf> {
    data_dir_in(&process_env, Os::current())
}

/// [`local_data_dir_in`] for this process.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn local_data_dir() -> Result<PathBuf> {
    local_data_dir_in(&process_env, Os::current())
}

/// [`runtime_dir_in`] for this process.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn runtime_dir() -> Result<PathBuf> {
    runtime_dir_in(&process_env, Os::current())
}

/// [`desktop_store_in`] for this process.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn desktop_store() -> Result<PathBuf> {
    desktop_store_in(&process_env, Os::current())
}

/// [`cli_store_in`] for this process.
///
/// # Errors
/// [`PlatformError::NoDirectory`].
pub fn cli_store() -> Result<PathBuf> {
    cli_store_in(&process_env, Os::current())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| OsString::from(*v))
        }
    }

    #[test]
    fn unix_prefers_xdg_and_falls_back_to_home() {
        let with_xdg = env(&[("XDG_DATA_HOME", "/x/data"), ("HOME", "/h")]);
        assert_eq!(
            desktop_store_in(&with_xdg, Os::Unix).unwrap(),
            Path::new("/x/data/io.github.stephanbaum.mosaic/vault.sqlite3")
        );
        let home_only = env(&[("HOME", "/h")]);
        assert_eq!(
            data_dir_in(&home_only, Os::Unix).unwrap(),
            Path::new("/h/.local/share")
        );
        assert_eq!(
            cli_store_in(&home_only, Os::Unix).unwrap(),
            Path::new("/h/.local/share/io.github.stephanbaum.mosaic/cli/vault.sqlite3")
        );
    }

    #[test]
    fn relative_or_empty_variables_are_ignored() {
        let bad = env(&[("XDG_DATA_HOME", "relative"), ("HOME", "")]);
        assert_eq!(data_dir_in(&bad, Os::Unix), Err(PlatformError::NoDirectory));
    }

    #[test]
    fn macos_uses_application_support() {
        let e = env(&[("HOME", "/Users/u")]);
        assert_eq!(
            data_dir_in(&e, Os::MacOs).unwrap(),
            Path::new("/Users/u/Library/Application Support")
        );
        assert_eq!(
            local_data_dir_in(&e, Os::MacOs).unwrap(),
            data_dir_in(&e, Os::MacOs).unwrap()
        );
    }

    #[test]
    fn windows_uses_roaming_and_local_app_data() {
        let e = env(&[("APPDATA", "/r"), ("LOCALAPPDATA", "/l")]);
        assert_eq!(data_dir_in(&e, Os::Windows).unwrap(), Path::new("/r"));
        assert_eq!(local_data_dir_in(&e, Os::Windows).unwrap(), Path::new("/l"));
    }

    #[test]
    fn runtime_dir_is_xdg_runtime_dir_only_on_unix() {
        let with = env(&[("XDG_RUNTIME_DIR", "/run/user/1000"), ("HOME", "/h")]);
        assert_eq!(
            runtime_dir_in(&with, Os::Unix).unwrap(),
            Path::new("/run/user/1000")
        );
        let without = env(&[("HOME", "/h")]);
        assert_eq!(
            runtime_dir_in(&without, Os::Unix),
            Err(PlatformError::NoRuntimeDir),
            "no persistent fallback"
        );
        let relative = env(&[("XDG_RUNTIME_DIR", "run"), ("HOME", "/h")]);
        assert_eq!(
            runtime_dir_in(&relative, Os::Unix),
            Err(PlatformError::NoRuntimeDir)
        );
    }

    #[test]
    fn runtime_dir_falls_back_to_local_data_on_windows_and_macos() {
        let win = env(&[("LOCALAPPDATA", "/l")]);
        assert_eq!(
            runtime_dir_in(&win, Os::Windows).unwrap(),
            Path::new("/l/io.github.stephanbaum.mosaic/cli/run")
        );
        let mac = env(&[("HOME", "/Users/u")]);
        assert_eq!(
            runtime_dir_in(&mac, Os::MacOs).unwrap(),
            Path::new("/Users/u/Library/Application Support/io.github.stephanbaum.mosaic/cli/run")
        );
    }
}

//! Where the desktop serves the browser channel and how the host finds it
//! (`docs/NATIVE_MESSAGING.md` §1).
//!
//! - **Linux:** `$XDG_RUNTIME_DIR/mosaic/browser.sock` if the variable is set
//!   and absolute, else
//!   `${XDG_DATA_HOME:-$HOME/.local/share}/io.github.stephanbaum.mosaic/browser/browser.sock`.
//! - **macOS:** `$HOME/Library/Application Support/io.github.stephanbaum.mosaic/browser/browser.sock`.
//! - **Windows:** a pipe `\\.\pipe\mosaic-browser-<32 lowercase hex>` with a
//!   fresh random name per desktop start, published in
//!   `%LOCALAPPDATA%\io.github.stephanbaum.mosaic\browser\endpoint.json`,
//!   which other accounts cannot read, so they cannot squat the name
//!   beforehand (named pipe names are machine-global).
//!
//! The file is trusted only while the desktop holds a lease on it
//! (`DiscoveryLease`, Windows): after a crash nothing holds it and the host
//! refuses the stale name instead of connecting to whatever program created
//! a pipe with it (THREAT_MODEL R42).
//!
//! The functions that derive paths take their inputs as arguments so they
//! can be tested for every OS on any OS; [`default_endpoint`] feeds them from
//! the environment.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use mosaic_core::browser_channel::new_endpoint_token;

use crate::error::Error;

#[cfg(windows)]
mod lease;
#[cfg(windows)]
pub use lease::{
    DiscoveryLease, discovery_is_live, read_live_discovery_file, read_live_discovery_file_for,
};

/// The app identifier of the desktop bundle (`tauri.conf.json`); the
/// directory name under the user's data directories.
pub const APP_IDENTIFIER: &str = "io.github.stephanbaum.mosaic";

/// The prefix of the Windows pipe name of the browser channel.
pub const PIPE_PREFIX: &str = r"\\.\pipe\mosaic-browser-";

/// The prefix of the Windows pipe name of the agent channel.
pub const AGENT_PIPE_PREFIX: &str = r"\\.\pipe\mosaic-agent-";

/// Which local channel an endpoint serves. The two channels have separate
/// directories, socket files, pipe names and discovery files, so a pairing
/// or a squatter of one never reaches the other (CRYPTO_SPEC section 15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelKind {
    /// The browser extension channel.
    Browser,
    /// The agent (MCP relay) channel.
    Agent,
}

impl ChannelKind {
    /// The directory name under the app directory: `browser` or `agent`.
    #[must_use]
    pub const fn dir_name(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::Agent => "agent",
        }
    }

    /// The Unix socket file name: `browser.sock` or `agent.sock`.
    #[must_use]
    pub const fn socket_file(self) -> &'static str {
        match self {
            Self::Browser => "browser.sock",
            Self::Agent => "agent.sock",
        }
    }

    /// The Windows pipe name prefix of this channel.
    #[must_use]
    pub const fn pipe_prefix(self) -> &'static str {
        match self {
            Self::Browser => PIPE_PREFIX,
            Self::Agent => AGENT_PIPE_PREFIX,
        }
    }

    /// The debug-build override variable of this channel.
    const fn override_var(self) -> &'static str {
        match self {
            Self::Browser => "MOSAIC_BROWSER_ENDPOINT",
            Self::Agent => "MOSAIC_AGENT_ENDPOINT",
        }
    }
}

/// A longest Unix socket path: `sun_path` holds 104 bytes on macOS and 108
/// on Linux, including the terminating NUL.
pub const MAX_SOCKET_PATH: usize = 103;

/// The most bytes read from the discovery file.
pub const MAX_DISCOVERY_BYTES: usize = 4096;

/// Where the desktop listens.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Endpoint {
    /// A Unix domain socket (macOS, Linux). Its directory must be private to
    /// the user.
    UnixSocket(PathBuf),
    /// A Windows named pipe, `\\.\pipe\mosaic-browser-<32 hex>`.
    NamedPipe(String),
}

/// A fresh pipe name: [`PIPE_PREFIX`] and `random(16)` as lowercase hex.
///
/// # Errors
/// [`Error::Channel`] if the OS CSPRNG fails.
pub fn new_windows_pipe_name() -> Result<String, Error> {
    new_windows_pipe_name_for(ChannelKind::Browser)
}

/// A fresh pipe name of `kind`: its prefix and `random(16)` as lowercase hex.
///
/// # Errors
/// [`Error::Channel`] if the OS CSPRNG fails.
pub fn new_windows_pipe_name_for(kind: ChannelKind) -> Result<String, Error> {
    let bytes = new_endpoint_token()?;
    Ok(format!(
        "{}{}",
        kind.pipe_prefix(),
        &**crate::hex::encode(&bytes)
    ))
}

/// Whether `name` is `\\.\pipe\mosaic-browser-` followed by exactly 32
/// lowercase hex digits.
#[must_use]
pub fn is_valid_pipe_name(name: &str) -> bool {
    is_valid_pipe_name_for(ChannelKind::Browser, name)
}

/// Whether `name` is the prefix of `kind` followed by exactly 32 lowercase
/// hex digits.
#[must_use]
pub fn is_valid_pipe_name_for(kind: ChannelKind, name: &str) -> bool {
    name.strip_prefix(kind.pipe_prefix()).is_some_and(|rest| {
        rest.len() == 32
            && rest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Whether `path` is absolute, also when it is a Unix-style path on Windows
/// (the derivations below are tested for every OS on any OS).
fn is_absolute(path: &Path) -> bool {
    path.is_absolute() || path.to_str().is_some_and(|s| s.starts_with('/'))
}

fn absolute(path: Option<&Path>) -> Option<&Path> {
    path.filter(|p| is_absolute(p))
}

/// A Unix socket path fits `sun_path` and is absolute.
fn checked_socket_path(path: PathBuf) -> Result<PathBuf, Error> {
    if is_absolute(&path) && path.as_os_str().len() <= MAX_SOCKET_PATH {
        Ok(path)
    } else {
        Err(Error::EndpointInvalid)
    }
}

/// The Linux socket path from `XDG_RUNTIME_DIR`, `XDG_DATA_HOME` and `HOME`.
///
/// # Errors
/// [`Error::EndpointInvalid`] if no absolute base directory is available or
/// the path is longer than [`MAX_SOCKET_PATH`].
pub fn linux_socket_path(
    xdg_runtime_dir: Option<&Path>,
    xdg_data_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, Error> {
    linux_socket_path_for(ChannelKind::Browser, xdg_runtime_dir, xdg_data_home, home)
}

/// [`linux_socket_path`] for the channel `kind`.
///
/// # Errors
/// As [`linux_socket_path`].
pub fn linux_socket_path_for(
    kind: ChannelKind,
    xdg_runtime_dir: Option<&Path>,
    xdg_data_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, Error> {
    let path = if let Some(runtime) = absolute(xdg_runtime_dir) {
        runtime.join("mosaic").join(kind.socket_file())
    } else {
        let data = match absolute(xdg_data_home) {
            Some(data) => data.to_path_buf(),
            None => absolute(home)
                .ok_or(Error::EndpointInvalid)?
                .join(".local")
                .join("share"),
        };
        data.join(APP_IDENTIFIER)
            .join(kind.dir_name())
            .join(kind.socket_file())
    };
    checked_socket_path(path)
}

/// The macOS socket path from `HOME`.
///
/// # Errors
/// As [`linux_socket_path`].
pub fn macos_socket_path(home: Option<&Path>) -> Result<PathBuf, Error> {
    macos_socket_path_for(ChannelKind::Browser, home)
}

/// [`macos_socket_path`] for the channel `kind`.
///
/// # Errors
/// As [`linux_socket_path`].
pub fn macos_socket_path_for(kind: ChannelKind, home: Option<&Path>) -> Result<PathBuf, Error> {
    checked_socket_path(
        absolute(home)
            .ok_or(Error::EndpointInvalid)?
            .join("Library")
            .join("Application Support")
            .join(APP_IDENTIFIER)
            .join(kind.dir_name())
            .join(kind.socket_file()),
    )
}

/// The Windows discovery file from `%LOCALAPPDATA%`.
///
/// # Errors
/// [`Error::EndpointInvalid`] if `local_app_data` is missing or relative.
pub fn discovery_file_path(local_app_data: Option<&Path>) -> Result<PathBuf, Error> {
    discovery_file_path_for(ChannelKind::Browser, local_app_data)
}

/// [`discovery_file_path`] for the channel `kind`.
///
/// # Errors
/// As [`discovery_file_path`].
pub fn discovery_file_path_for(
    kind: ChannelKind,
    local_app_data: Option<&Path>,
) -> Result<PathBuf, Error> {
    Ok(absolute(local_app_data)
        .ok_or(Error::EndpointInvalid)?
        .join(APP_IDENTIFIER)
        .join(kind.dir_name())
        .join("endpoint.json"))
}

/// The content of the discovery file for `pipe`:
/// `{"version":1,"pipe":"\\\\.\\pipe\\mosaic-browser-<32hex>"}`.
#[must_use]
pub fn discovery_json(pipe: &str) -> String {
    let pipe = serde_json::Value::from(pipe).to_string();
    format!(r#"{{"version":1,"pipe":{pipe}}}"#)
}

/// Reads the pipe name out of a discovery file's bytes.
///
/// # Errors
/// [`Error::EndpointInvalid`] if the file is longer than
/// [`MAX_DISCOVERY_BYTES`], is not exactly `{"version":1,"pipe":"..."}`, or
/// the name is not a valid pipe name.
pub fn parse_discovery(bytes: &[u8]) -> Result<String, Error> {
    parse_discovery_for(ChannelKind::Browser, bytes)
}

/// [`parse_discovery`] for the channel `kind`.
///
/// # Errors
/// As [`parse_discovery`].
pub fn parse_discovery_for(kind: ChannelKind, bytes: &[u8]) -> Result<String, Error> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        version: u8,
        pipe: String,
    }
    if bytes.len() > MAX_DISCOVERY_BYTES {
        return Err(Error::EndpointInvalid);
    }
    let file: File = serde_json::from_slice(bytes).map_err(|_| Error::EndpointInvalid)?;
    if file.version == 1 && is_valid_pipe_name_for(kind, &file.pipe) {
        Ok(file.pipe)
    } else {
        Err(Error::EndpointInvalid)
    }
}

/// Reads and validates the discovery file at `path`. A missing file means
/// the desktop is not running.
///
/// # Errors
/// [`Error::DesktopNotRunning`] if the file does not exist,
/// [`Error::EndpointInvalid`] if it is malformed or too long.
pub fn read_discovery_file(path: &Path) -> Result<String, Error> {
    read_discovery_file_for(ChannelKind::Browser, path)
}

/// [`read_discovery_file`] for the channel `kind`.
///
/// # Errors
/// As [`read_discovery_file`].
pub fn read_discovery_file_for(kind: ChannelKind, path: &Path) -> Result<String, Error> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(Error::DesktopNotRunning),
        Err(e) => return Err(e.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_DISCOVERY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    parse_discovery_for(kind, &bytes)
}

/// Writes the discovery file for `pipe` atomically (a temporary file in the
/// same directory, then a rename), creating the directory if needed.
///
/// On Windows the desktop uses `DiscoveryLease::create` instead, which
/// also keeps the file locked for its lifetime; this function alone leaves a
/// file the host treats as stale.
///
/// # Errors
/// [`Error::EndpointInvalid`] for a bad pipe name or a path without a
/// directory, [`Error::Io`] on a write failure.
pub fn write_discovery_file(path: &Path, pipe: &str) -> Result<(), Error> {
    write_discovery_file_for(ChannelKind::Browser, path, pipe)
}

/// [`write_discovery_file`] for the channel `kind`.
///
/// # Errors
/// As [`write_discovery_file`].
pub fn write_discovery_file_for(kind: ChannelKind, path: &Path, pipe: &str) -> Result<(), Error> {
    if !is_valid_pipe_name_for(kind, pipe) || path.parent().is_none() {
        return Err(Error::EndpointInvalid);
    }
    write_discovery_file_io(path, pipe).map_err(Error::from)
}

/// The write of [`write_discovery_file`], with the raw I/O error.
fn write_discovery_file_io(path: &Path, pipe: &str) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension("json.tmp");
    let write = || -> io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(discovery_json(pipe).as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    };
    write().inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Removes the discovery file; a missing file is fine.
///
/// # Errors
/// [`Error::Io`] on a failure other than a missing file.
pub fn remove_discovery_file(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// The pipe name of the discovery file at `path`, trusted only while a
/// desktop holds the file (Windows; elsewhere there is no discovery file).
fn read_trusted_discovery_file(kind: ChannelKind, path: &Path) -> Result<String, Error> {
    #[cfg(windows)]
    {
        read_live_discovery_file_for(kind, path)
    }
    #[cfg(not(windows))]
    {
        read_discovery_file_for(kind, path)
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// The discovery file of this user (Windows only).
///
/// # Errors
/// [`Error::EndpointInvalid`] off Windows or without `%LOCALAPPDATA%`.
pub fn discovery_file() -> Result<PathBuf, Error> {
    discovery_file_for(ChannelKind::Browser)
}

/// The discovery file of this user for the channel `kind` (Windows only).
///
/// # Errors
/// As [`discovery_file`].
pub fn discovery_file_for(kind: ChannelKind) -> Result<PathBuf, Error> {
    if cfg!(windows) {
        discovery_file_path_for(kind, env_path("LOCALAPPDATA").as_deref())
    } else {
        Err(Error::EndpointInvalid)
    }
}

/// The endpoint the host connects to: on Unix the socket path of this OS, on
/// Windows the pipe named in the discovery file, but only while a desktop
/// holds that file open (`DiscoveryLease`); a stale file is
/// `DesktopNotRunning`.
///
/// In debug builds `MOSAIC_BROWSER_ENDPOINT` (a socket path or a pipe name)
/// replaces it, for integration tests. Release builds ignore the variable.
///
/// # Errors
/// [`Error::DesktopNotRunning`] on Windows without a discovery file,
/// [`Error::EndpointInvalid`] for anything unusable.
pub fn default_endpoint() -> Result<Endpoint, Error> {
    default_endpoint_for(ChannelKind::Browser)
}

/// [`default_endpoint`] for the channel `kind`. In debug builds the agent
/// channel's override variable is `MOSAIC_AGENT_ENDPOINT`.
///
/// # Errors
/// As [`default_endpoint`].
pub fn default_endpoint_for(kind: ChannelKind) -> Result<Endpoint, Error> {
    if cfg!(debug_assertions)
        && let Some(value) = std::env::var_os(kind.override_var())
    {
        return endpoint_from_override_for(kind, &value.to_string_lossy());
    }
    if cfg!(windows) {
        let pipe = read_trusted_discovery_file(kind, &discovery_file_for(kind)?)?;
        Ok(Endpoint::NamedPipe(pipe))
    } else if cfg!(target_os = "macos") {
        macos_socket_path_for(kind, env_path("HOME").as_deref()).map(Endpoint::UnixSocket)
    } else {
        linux_socket_path_for(
            kind,
            env_path("XDG_RUNTIME_DIR").as_deref(),
            env_path("XDG_DATA_HOME").as_deref(),
            env_path("HOME").as_deref(),
        )
        .map(Endpoint::UnixSocket)
    }
}

/// The endpoint named by the debug override.
/// The endpoint of `kind` named by the debug override.
fn endpoint_from_override_for(kind: ChannelKind, value: &str) -> Result<Endpoint, Error> {
    if cfg!(windows) {
        if is_valid_pipe_name_for(kind, value) {
            Ok(Endpoint::NamedPipe(value.to_owned()))
        } else {
            Err(Error::EndpointInvalid)
        }
    } else {
        checked_socket_path(PathBuf::from(value)).map(Endpoint::UnixSocket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_endpoint_paths_per_os() {
        let a = ChannelKind::Agent;
        assert_eq!(
            linux_socket_path_for(a, Some(Path::new("/run/user/1000")), None, None).unwrap(),
            PathBuf::from("/run/user/1000/mosaic/agent.sock")
        );
        assert_eq!(
            linux_socket_path_for(a, None, None, Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/home/a/.local/share/io.github.stephanbaum.mosaic/agent/agent.sock")
        );
        assert_eq!(
            macos_socket_path_for(a, Some(Path::new("/Users/a"))).unwrap(),
            PathBuf::from(
                "/Users/a/Library/Application Support/io.github.stephanbaum.mosaic/agent/agent.sock"
            )
        );
        let local = std::env::temp_dir();
        assert_eq!(
            discovery_file_path_for(a, Some(&local)).unwrap(),
            local
                .join("io.github.stephanbaum.mosaic")
                .join("agent")
                .join("endpoint.json")
        );
        let pipe = new_windows_pipe_name_for(a).unwrap();
        assert!(pipe.starts_with(AGENT_PIPE_PREFIX), "{pipe}");
        assert!(is_valid_pipe_name_for(a, &pipe));
        assert!(!is_valid_pipe_name_for(ChannelKind::Browser, &pipe));
        assert!(!is_valid_pipe_name(&pipe));
        let browser = new_windows_pipe_name().unwrap();
        assert!(!is_valid_pipe_name_for(a, &browser));
        assert_eq!(
            parse_discovery_for(a, discovery_json(&pipe).as_bytes()),
            Ok(pipe.clone())
        );
        assert_eq!(
            parse_discovery(discovery_json(&pipe).as_bytes()),
            Err(Error::EndpointInvalid)
        );
        assert_eq!(
            endpoint_from_override_for(a, "relative"),
            Err(Error::EndpointInvalid)
        );
    }

    #[test]
    fn browser_paths_are_unchanged() {
        assert_eq!(
            linux_socket_path(Some(Path::new("/run/user/1000")), None, None).unwrap(),
            PathBuf::from("/run/user/1000/mosaic/browser.sock")
        );
        assert!(PIPE_PREFIX.ends_with("mosaic-browser-"));
        assert_eq!(ChannelKind::Browser.dir_name(), "browser");
    }

    #[test]
    fn pipe_names_are_random_and_validated() {
        let a = new_windows_pipe_name().unwrap();
        let b = new_windows_pipe_name().unwrap();
        assert_ne!(a, b);
        assert!(is_valid_pipe_name(&a), "{a}");
        assert_eq!(a.len(), PIPE_PREFIX.len() + 32);
        let good = format!("{PIPE_PREFIX}{}", "0f".repeat(16));
        assert!(is_valid_pipe_name(&good));
        for bad in [
            r"\\.\pipe\openssh-ssh-agent".to_owned(),
            format!("{PIPE_PREFIX}{}", "0f".repeat(15)),
            format!("{PIPE_PREFIX}{}", "0f".repeat(17)),
            format!("{PIPE_PREFIX}{}", "0F".repeat(16)),
            format!("{PIPE_PREFIX}{}x", "0f".repeat(16)),
            format!(r"\\server\pipe\mosaic-browser-{}", "0f".repeat(16)),
            format!("{good}\\x"),
            String::new(),
        ] {
            assert!(!is_valid_pipe_name(&bad), "{bad}");
        }
    }

    #[test]
    fn linux_paths() {
        let runtime = Path::new("/run/user/1000");
        assert_eq!(
            linux_socket_path(Some(runtime), None, Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/run/user/1000/mosaic/browser.sock")
        );
        assert_eq!(
            linux_socket_path(None, None, Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/home/a/.local/share/io.github.stephanbaum.mosaic/browser/browser.sock")
        );
        assert_eq!(
            linux_socket_path(None, Some(Path::new("/data")), Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/data/io.github.stephanbaum.mosaic/browser/browser.sock")
        );
        // A relative runtime directory is ignored.
        assert_eq!(
            linux_socket_path(Some(Path::new("run")), None, Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/home/a/.local/share/io.github.stephanbaum.mosaic/browser/browser.sock")
        );
        assert_eq!(
            linux_socket_path(None, Some(Path::new("data")), Some(Path::new("/home/a"))).unwrap(),
            PathBuf::from("/home/a/.local/share/io.github.stephanbaum.mosaic/browser/browser.sock")
        );
        assert_eq!(
            linux_socket_path(None, None, None),
            Err(Error::EndpointInvalid)
        );
        assert_eq!(
            linux_socket_path(None, None, Some(Path::new("home"))),
            Err(Error::EndpointInvalid)
        );
    }

    #[test]
    fn unix_paths_must_fit_sun_path() {
        let long = format!("/{}", "a".repeat(MAX_SOCKET_PATH));
        assert_eq!(
            linux_socket_path(Some(Path::new(&long)), None, None),
            Err(Error::EndpointInvalid)
        );
        let fits = "a".repeat(MAX_SOCKET_PATH - "/mosaic/browser.sock".len() - 1);
        let runtime = format!("/{fits}");
        let path = linux_socket_path(Some(Path::new(&runtime)), None, None).unwrap();
        assert_eq!(path.as_os_str().len(), MAX_SOCKET_PATH);
        assert!(checked_socket_path(PathBuf::from("relative/browser.sock")).is_err());
    }

    #[test]
    fn macos_paths() {
        assert_eq!(
            macos_socket_path(Some(Path::new("/Users/a"))).unwrap(),
            PathBuf::from(
                "/Users/a/Library/Application Support/io.github.stephanbaum.mosaic/browser/browser.sock"
            )
        );
        assert_eq!(macos_socket_path(None), Err(Error::EndpointInvalid));
        assert_eq!(
            macos_socket_path(Some(Path::new("Users"))),
            Err(Error::EndpointInvalid)
        );
    }

    #[test]
    fn windows_discovery_path() {
        let local = std::env::temp_dir();
        let path = discovery_file_path(Some(&local)).unwrap();
        assert_eq!(
            path,
            local
                .join("io.github.stephanbaum.mosaic")
                .join("browser")
                .join("endpoint.json")
        );
        assert_eq!(discovery_file_path(None), Err(Error::EndpointInvalid));
        assert_eq!(
            discovery_file_path(Some(Path::new("Local"))),
            Err(Error::EndpointInvalid)
        );
    }

    #[test]
    fn the_discovery_validator() {
        let pipe = format!("{PIPE_PREFIX}{}", "ab".repeat(16));
        let good = discovery_json(&pipe);
        assert_eq!(parse_discovery(good.as_bytes()).unwrap(), pipe);
        assert_eq!(
            good,
            format!(
                r#"{{"version":1,"pipe":{}}}"#,
                serde_json::to_string(&pipe).unwrap()
            )
        );
        let bad_name = discovery_json(r"\\.\pipe\openssh-ssh-agent");
        let long = format!(
            r#"{{"version":1,"pipe":"{}"}}"#,
            "a".repeat(MAX_DISCOVERY_BYTES)
        );
        for bad in [
            bad_name.as_str(),
            r#"{"version":2,"pipe":"x"}"#,
            r#"{"version":1}"#,
            r#"{"pipe":"x"}"#,
            "{}",
            "null",
            "",
            long.as_str(),
        ] {
            assert_eq!(
                parse_discovery(bad.as_bytes()),
                Err(Error::EndpointInvalid),
                "{bad}"
            );
        }
        let extra = good.replacen('{', r#"{"extra":1,"#, 1);
        assert_eq!(
            parse_discovery(extra.as_bytes()),
            Err(Error::EndpointInvalid)
        );
    }

    #[test]
    fn the_discovery_file_round_trips_and_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("browser").join("endpoint.json");
        assert_eq!(read_discovery_file(&path), Err(Error::DesktopNotRunning));
        let pipe = new_windows_pipe_name().unwrap();
        write_discovery_file(&path, &pipe).unwrap();
        assert_eq!(read_discovery_file(&path).unwrap(), pipe);
        let pipe2 = new_windows_pipe_name().unwrap();
        write_discovery_file(&path, &pipe2).unwrap();
        assert_eq!(read_discovery_file(&path).unwrap(), pipe2, "replaced");
        assert!(!path.with_extension("json.tmp").exists());
        assert_eq!(
            write_discovery_file(&path, "not a pipe"),
            Err(Error::EndpointInvalid)
        );
        remove_discovery_file(&path).unwrap();
        remove_discovery_file(&path).unwrap();
        assert_eq!(read_discovery_file(&path), Err(Error::DesktopNotRunning));
        fs::write(&path, vec![b' '; MAX_DISCOVERY_BYTES + 1]).unwrap();
        assert_eq!(read_discovery_file(&path), Err(Error::EndpointInvalid));
    }

    #[test]
    fn the_debug_override_is_checked() {
        if cfg!(windows) {
            let pipe = format!("{PIPE_PREFIX}{}", "cd".repeat(16));
            assert_eq!(
                endpoint_from_override_for(ChannelKind::Browser, &pipe),
                Ok(Endpoint::NamedPipe(pipe))
            );
            assert_eq!(
                endpoint_from_override_for(ChannelKind::Browser, "anything"),
                Err(Error::EndpointInvalid)
            );
        } else {
            assert_eq!(
                endpoint_from_override_for(ChannelKind::Browser, "/tmp/x/browser.sock"),
                Ok(Endpoint::UnixSocket(PathBuf::from("/tmp/x/browser.sock")))
            );
            assert_eq!(
                endpoint_from_override_for(ChannelKind::Browser, "relative"),
                Err(Error::EndpointInvalid)
            );
        }
    }
}

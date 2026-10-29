//! OS endpoints: a Unix domain socket on macOS and Linux, a named pipe on
//! Windows. Both admit the current user only.
//!
//! - **Unix:** the socket lives in a directory that must be owned by the
//!   current user with mode `0700` (it is created so if missing); the socket
//!   itself is `0600`, and a peer with another UID (`SO_PEERCRED` /
//!   `LOCAL_PEERCRED`) is disconnected before it can send anything. An
//!   existing socket is replaced only if nothing answers on it; any other
//!   file at the path is left alone and the start fails.
//! - **Windows:** the pipe is created as the first instance of its name, so
//!   the start fails if another agent (for example the Windows "OpenSSH
//!   Authentication Agent" service) already serves it, and remote clients
//!   are rejected. The pipe keeps Windows' default security descriptor:
//!   full access for the creating user, SYSTEM and Administrators, read-only
//!   access for everyone else, which cannot send a request.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::approval::ClientInfo;
use crate::error::Error;

/// Where the agent listens. Clients find it through `SSH_AUTH_SOCK`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Endpoint {
    /// A Unix domain socket path (macOS, Linux). Its directory must be
    /// private to the user.
    UnixSocket(PathBuf),
    /// A Windows named pipe, `\\.\pipe\<name>`.
    NamedPipe(String),
}

impl Endpoint {
    /// The pipe the Windows OpenSSH client uses when `SSH_AUTH_SOCK` is not
    /// set.
    pub const WINDOWS_OPENSSH_PIPE: &'static str = r"\\.\pipe\openssh-ssh-agent";

    /// The default endpoint of this platform: [`Self::WINDOWS_OPENSSH_PIPE`]
    /// on Windows, `<private_dir>/agent.sock` elsewhere. `private_dir`
    /// should be a per-user runtime directory (for example
    /// `$XDG_RUNTIME_DIR/mosaic` or the app's data directory).
    #[must_use]
    pub fn platform_default(private_dir: &std::path::Path) -> Self {
        if cfg!(windows) {
            Self::NamedPipe(Self::WINDOWS_OPENSSH_PIPE.to_owned())
        } else {
            Self::UnixSocket(private_dir.join("agent.sock"))
        }
    }

    /// The value for `SSH_AUTH_SOCK`.
    #[must_use]
    pub fn auth_sock(&self) -> String {
        match self {
            Self::UnixSocket(p) => p.to_string_lossy().into_owned(),
            Self::NamedPipe(name) => name.clone(),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.auth_sock())
    }
}

#[cfg(unix)]
use unix::Listener;
/// A connected client stream.
#[cfg(unix)]
pub(crate) type PlatformStream = tokio::net::UnixStream;
#[cfg(windows)]
use windows::Listener;
/// A connected client stream.
#[cfg(windows)]
pub(crate) type PlatformStream = tokio::net::windows::named_pipe::NamedPipeServer;

/// Maps an I/O error to the crate error.
fn io_error(e: &io::Error) -> Error {
    Error::Io(e.kind())
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::io;
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    use tokio::net::{UnixListener, UnixStream};

    use super::{Endpoint, io_error};
    use crate::approval::ClientInfo;
    use crate::error::Error;

    /// `sun_path` holds 104 bytes on macOS and 108 on Linux, including the
    /// terminating NUL.
    const MAX_SOCKET_PATH: usize = 103;

    pub(crate) struct Listener {
        inner: UnixListener,
        path: PathBuf,
        uid: u32,
        dev_ino: (u64, u64),
    }

    impl Listener {
        pub(crate) fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
            let Endpoint::UnixSocket(path) = endpoint else {
                return Err(Error::EndpointInvalid);
            };
            if path.as_os_str().len() > MAX_SOCKET_PATH || !path.is_absolute() {
                return Err(Error::EndpointInvalid);
            }
            let dir = path.parent().ok_or(Error::EndpointInvalid)?;
            prepare_dir(dir)?;
            clear_stale(path)?;
            let inner = UnixListener::bind(path).map_err(|e| io_error(&e))?;
            let meta = fs::symlink_metadata(path).map_err(|e| io_error(&e))?;
            // From here on, dropping the listener removes the socket again.
            let listener = Self {
                inner,
                path: path.clone(),
                uid: meta.uid(),
                dev_ino: (meta.dev(), meta.ino()),
            };
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .map_err(|e| io_error(&e))?;
            let dir_meta = fs::symlink_metadata(dir).map_err(|e| io_error(&e))?;
            if dir_meta.uid() != listener.uid {
                return Err(Error::EndpointInsecure);
            }
            Ok(listener)
        }

        pub(crate) async fn accept(&mut self) -> io::Result<(UnixStream, ClientInfo)> {
            loop {
                let (stream, _) = self.inner.accept().await?;
                let Ok(cred) = stream.peer_cred() else {
                    continue;
                };
                if cred.uid() != self.uid {
                    continue;
                }
                let pid = cred.pid().and_then(|p| u32::try_from(p).ok());
                return Ok((stream, client_info(pid)));
            }
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            // Remove the socket only if it is still ours.
            if let Ok(meta) = fs::symlink_metadata(&self.path)
                && (meta.dev(), meta.ino()) == self.dev_ino
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }

    /// Creates `dir` with mode 0700 if missing, then requires a real
    /// directory (not a symbolic link) that nobody else can enter.
    fn prepare_dir(dir: &Path) -> Result<(), Error> {
        match fs::DirBuilder::new().mode(0o700).create(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(&e)),
        }
        let meta = fs::symlink_metadata(dir).map_err(|e| io_error(&e))?;
        if !meta.file_type().is_dir() || meta.mode() & 0o077 != 0 {
            return Err(Error::EndpointInsecure);
        }
        Ok(())
    }

    /// Removes a socket nobody listens on; refuses a live socket or any
    /// other file.
    fn clear_stale(path: &Path) -> Result<(), Error> {
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(&e)),
            Ok(meta) if meta.file_type().is_socket() => {
                if std::os::unix::net::UnixStream::connect(path).is_ok() {
                    Err(Error::EndpointInUse)
                } else {
                    fs::remove_file(path).map_err(|e| io_error(&e))
                }
            }
            Ok(_) => Err(Error::EndpointInUse),
        }
    }

    /// The executable of `pid` where the OS exposes it without unsafe code
    /// (Linux `/proc`). The process may have exited or replaced its image
    /// since it connected; the result is for display and for scoping
    /// grants, not proof of identity.
    fn client_info(pid: Option<u32>) -> ClientInfo {
        let executable = pid.and_then(|p| fs::read_link(format!("/proc/{p}/exe")).ok());
        let process_name = executable
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned());
        ClientInfo {
            pid,
            process_name,
            executable,
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::io;

    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

    use super::{Endpoint, io_error};
    use crate::approval::ClientInfo;
    use crate::error::Error;

    pub(crate) struct Listener {
        name: String,
        next: NamedPipeServer,
    }

    impl Listener {
        pub(crate) fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
            let Endpoint::NamedPipe(name) = endpoint else {
                return Err(Error::EndpointInvalid);
            };
            let valid = name
                .strip_prefix(r"\\.\pipe\")
                .is_some_and(|rest| !rest.is_empty() && !rest.contains('\\'));
            if !valid {
                return Err(Error::EndpointInvalid);
            }
            let next = ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .create(name)
                .map_err(|e| match e.kind() {
                    // ERROR_ACCESS_DENIED: an instance of the name exists.
                    io::ErrorKind::PermissionDenied => Error::EndpointInUse,
                    _ => io_error(&e),
                })?;
            Ok(Self {
                name: name.clone(),
                next,
            })
        }

        pub(crate) async fn accept(&mut self) -> io::Result<(NamedPipeServer, ClientInfo)> {
            self.next.connect().await?;
            // The next instance exists before this one is handed out, so a
            // client never finds the name missing.
            let next = ServerOptions::new()
                .reject_remote_clients(true)
                .create(&self.name)?;
            let connected = std::mem::replace(&mut self.next, next);
            // The client's process ID needs GetNamedPipeClientProcessId,
            // which only unsafe FFI reaches; see the crate docs.
            Ok((connected, ClientInfo::UNKNOWN))
        }
    }
}

/// Binds `endpoint` and yields client streams.
pub(crate) struct Transport {
    inner: Listener,
}

impl Transport {
    pub(crate) fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
        Ok(Self {
            inner: Listener::bind(endpoint)?,
        })
    }

    pub(crate) async fn accept(&mut self) -> io::Result<(PlatformStream, ClientInfo)> {
        self.inner.accept().await
    }
}

//! OS transports of the browser channel: a Unix domain socket on macOS and
//! Linux, a named pipe on Windows. Both admit the current user only.
//!
//! The bind and stale-socket rules are copied from
//! `crates/ssh-agent/src/transport.rs` (ADR-0036 item 3; M9 extracts one
//! shared crate). Differences: no client identity (the browser channel
//! authenticates with the pairing key instead), a client side with its own
//! checks, and the pipe name is the random one of the discovery file.
//!
//! - **Unix, server:** the socket lives in a directory that must be owned by
//!   the current user with mode `0700` (it is created so if missing); the
//!   socket itself is `0600`, and a peer with another UID (`SO_PEERCRED` /
//!   `LOCAL_PEERCRED`) is disconnected before it can send anything. An
//!   existing socket is replaced only if nothing answers on it; any other
//!   file at the path is left alone and the bind fails.
//! - **Unix, client:** the parent directory must be a real directory with
//!   mode `& 0o077 == 0` owned by the current user, the path must be a
//!   socket owned by the directory's owner, and after connecting the peer's
//!   UID must be the directory's owner. The current UID has no safe std
//!   call: it is the owner of `/proc/self` where that exists (Linux), else
//!   the owner of `$HOME` (macOS). Unix peer identity is available without
//!   unsafe code; Windows has none (THREAT_MODEL R42).
//! - **Windows:** the pipe is created as the first instance of its name (so
//!   the bind fails if the name is taken) and remote clients are rejected.
//!   The pipe keeps Windows' default security descriptor: full access for
//!   the creating user, SYSTEM and Administrators, read-only access for
//!   everyone else, which cannot send a request.

use core::time::Duration;
use std::io;

use tokio::time::timeout;

use crate::endpoint::Endpoint;
use crate::error::Error;
use crate::protocol::CONNECT_TIMEOUT_SECS;

/// How long [`connect`] waits for the endpoint.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(CONNECT_TIMEOUT_SECS);

/// A connection accepted by a [`Listener`].
#[cfg(unix)]
pub type ServerStream = tokio::net::UnixStream;
/// A connection accepted by a [`Listener`].
#[cfg(windows)]
pub type ServerStream = tokio::net::windows::named_pipe::NamedPipeServer;

/// A connection made by [`connect`].
#[cfg(unix)]
pub type ClientStream = tokio::net::UnixStream;
/// A connection made by [`connect`].
#[cfg(windows)]
pub type ClientStream = tokio::net::windows::named_pipe::NamedPipeClient;

/// Maps an I/O error to the crate error.
fn io_error(e: &io::Error) -> Error {
    Error::Io(e.kind())
}

/// What a failed connect means: nobody serves the endpoint.
fn connect_error(e: &io::Error) -> Error {
    match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => Error::DesktopNotRunning,
        _ => io_error(e),
    }
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::io;
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    use tokio::net::{UnixListener, UnixStream};

    use super::{Endpoint, connect_error, io_error};
    use crate::endpoint::MAX_SOCKET_PATH;
    use crate::error::Error;

    pub(super) struct Listener {
        inner: UnixListener,
        path: PathBuf,
        uid: u32,
        dev_ino: (u64, u64),
    }

    impl Listener {
        pub(super) fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
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

        pub(super) async fn accept(&mut self) -> io::Result<UnixStream> {
            loop {
                let (stream, _) = self.inner.accept().await?;
                let Ok(cred) = stream.peer_cred() else {
                    continue;
                };
                if cred.uid() != self.uid {
                    continue;
                }
                return Ok(stream);
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
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| io_error(&e))?;
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

    /// The UID of the current user without `unsafe` (std has no `getuid`):
    /// the owner of `/proc/self`, which the kernel gives the process's UID,
    /// and where there is no procfs (macOS) the owner of `$HOME`, which must
    /// be an absolute path to a real directory. `None` if neither works; the
    /// client then refuses to connect.
    fn current_uid() -> Option<u32> {
        if let Ok(meta) = fs::metadata("/proc/self") {
            return Some(meta.uid());
        }
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        if !home.is_absolute() {
            return None;
        }
        let meta = fs::metadata(home).ok()?;
        meta.is_dir().then(|| meta.uid())
    }

    /// The client's checks before connecting: returns the directory's owner.
    pub(super) fn check_client_path(path: &Path) -> Result<u32, Error> {
        if !path.is_absolute() || path.as_os_str().len() > MAX_SOCKET_PATH {
            return Err(Error::EndpointInvalid);
        }
        let dir = path.parent().ok_or(Error::EndpointInvalid)?;
        let dir_meta = match fs::symlink_metadata(dir) {
            Ok(meta) => meta,
            Err(e) => return Err(connect_error(&e)),
        };
        if !dir_meta.file_type().is_dir() || dir_meta.mode() & 0o077 != 0 {
            return Err(Error::EndpointInsecure);
        }
        // A private directory of somebody else is not ours: that user could
        // run a socket there that speaks to this one.
        if current_uid() != Some(dir_meta.uid()) {
            return Err(Error::EndpointInsecure);
        }
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) => return Err(connect_error(&e)),
        };
        if !meta.file_type().is_socket() || meta.uid() != dir_meta.uid() {
            return Err(Error::EndpointInsecure);
        }
        Ok(dir_meta.uid())
    }

    pub(super) async fn connect(endpoint: &Endpoint) -> Result<UnixStream, Error> {
        let Endpoint::UnixSocket(path) = endpoint else {
            return Err(Error::EndpointInvalid);
        };
        let owner = check_client_path(path)?;
        let stream = UnixStream::connect(path)
            .await
            .map_err(|e| connect_error(&e))?;
        let cred = stream.peer_cred().map_err(|e| io_error(&e))?;
        if cred.uid() != owner {
            return Err(Error::EndpointInsecure);
        }
        Ok(stream)
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::time::Duration;

    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };

    use super::{Endpoint, connect_error, io_error};
    use crate::endpoint::is_valid_pipe_name;
    use crate::error::Error;

    /// `ERROR_PIPE_BUSY`: every instance is in use; try again shortly.
    const ERROR_PIPE_BUSY: i32 = 231;

    pub(super) struct Listener {
        name: String,
        next: NamedPipeServer,
    }

    impl Listener {
        pub(super) fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
            let Endpoint::NamedPipe(name) = endpoint else {
                return Err(Error::EndpointInvalid);
            };
            if !is_valid_pipe_name(name) {
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

        pub(super) async fn accept(&mut self) -> io::Result<NamedPipeServer> {
            self.next.connect().await?;
            // The next instance exists before this one is handed out, so a
            // client never finds the name missing.
            let next = ServerOptions::new()
                .reject_remote_clients(true)
                .create(&self.name)?;
            Ok(std::mem::replace(&mut self.next, next))
        }
    }

    pub(super) async fn connect(endpoint: &Endpoint) -> Result<NamedPipeClient, Error> {
        let Endpoint::NamedPipe(name) = endpoint else {
            return Err(Error::EndpointInvalid);
        };
        if !is_valid_pipe_name(name) {
            return Err(Error::EndpointInvalid);
        }
        loop {
            match ClientOptions::new().open(name) {
                Ok(client) => return Ok(client),
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(e) => return Err(connect_error(&e)),
            }
        }
    }
}

/// Binds an [`Endpoint`] and yields client connections of the current user.
pub struct Listener {
    #[cfg(unix)]
    inner: unix::Listener,
    #[cfg(windows)]
    inner: windows::Listener,
}

impl Listener {
    /// Binds `endpoint`.
    ///
    /// # Errors
    /// [`Error::EndpointInUse`] if another program serves it (or another
    /// file sits at the socket path), [`Error::EndpointInsecure`] if the
    /// socket's directory is not private, [`Error::EndpointInvalid`] if the
    /// endpoint does not fit this platform, [`Error::Io`] otherwise.
    pub fn bind(endpoint: &Endpoint) -> Result<Self, Error> {
        Ok(Self {
            inner: {
                #[cfg(unix)]
                {
                    unix::Listener::bind(endpoint)?
                }
                #[cfg(windows)]
                {
                    windows::Listener::bind(endpoint)?
                }
            },
        })
    }

    /// Waits for the next connection (Unix: from the socket owner's UID).
    ///
    /// # Errors
    /// An I/O error of the listener itself.
    pub async fn accept(&mut self) -> io::Result<ServerStream> {
        self.inner.accept().await
    }
}

/// Connects to `endpoint` within [`CONNECT_TIMEOUT`], with the client-side
/// checks of the platform.
///
/// # Errors
/// [`Error::DesktopNotRunning`] if nothing serves the endpoint,
/// [`Error::EndpointInsecure`] if a Unix socket or its directory fails the
/// checks, [`Error::Timeout`], [`Error::EndpointInvalid`], [`Error::Io`].
pub async fn connect(endpoint: &Endpoint) -> Result<ClientStream, Error> {
    #[cfg(unix)]
    let attempt = unix::connect(endpoint);
    #[cfg(windows)]
    let attempt = windows::connect(endpoint);
    timeout(CONNECT_TIMEOUT, attempt)
        .await
        .map_err(|_| Error::Timeout)?
}

#[cfg(test)]
mod tests;

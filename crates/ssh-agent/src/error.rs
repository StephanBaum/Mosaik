//! Errors of starting the agent.

use std::io;

use thiserror::Error;

/// Why the agent could not start. Carries no key material and no paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Another program serves the endpoint (another agent on the socket, or
    /// the Windows "OpenSSH Authentication Agent" service on its pipe), or
    /// another file sits at the socket path.
    #[error("another program uses the SSH agent endpoint")]
    EndpointInUse,
    /// The endpoint does not fit this platform (a socket path on Windows, a
    /// pipe name elsewhere), is not absolute, or is too long.
    #[error("the SSH agent endpoint is not valid on this platform")]
    EndpointInvalid,
    /// The socket's directory is not a private directory of the current
    /// user (another owner, group or world access, or a symbolic link).
    #[error("the SSH agent socket directory is not private")]
    EndpointInsecure,
    /// Called outside a Tokio runtime.
    #[error("the SSH agent needs a Tokio runtime")]
    NoRuntime,
    /// An OS error while binding.
    #[error("SSH agent I/O error: {0}")]
    Io(io::ErrorKind),
}

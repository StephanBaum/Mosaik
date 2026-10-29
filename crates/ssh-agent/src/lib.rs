//! An SSH agent (brief §5 "SSH agent") that serves the SSH keys of the
//! unlocked vault, with an approval for every use.
//!
//! Mosaic is pre-audit; nothing here has been externally reviewed.
//!
//! # Protocol
//!
//! draft-miller-ssh-agent over a Unix socket (macOS, Linux) or a Windows
//! named pipe ([`Endpoint`]); see [`wire`] for the encoding.
//!
//! - `SSH_AGENTC_REQUEST_IDENTITIES` lists the public keys of the vault's
//!   SSH key items, without asking.
//! - `SSH_AGENTC_SIGN_REQUEST` signs with `ssh-ed25519` or
//!   `ecdsa-sha2-nistp256` after the user approved it through the
//!   [`Approver`], unless an earlier "Allow for 15 minutes" answer
//!   ([`Decision::AllowFor`], at most [`AgentConfig::max_grant`], default
//!   [`DEFAULT_GRANT`]) covers the same key and client executable. Requests without an answer
//!   are refused after 25 s ([`AgentConfig::approval_timeout`]). RSA keys
//!   are not served in this build (the `rsa` crate's RUSTSEC-2023-0071 has no
//!   fix); [`rsa_signature_algorithm`] holds the RFC 8332 flag rule
//!   (`rsa-sha2-256` / `rsa-sha2-512`, SHA-1 `ssh-rsa` refused) for when they
//!   are.
//! - Adding, removing, locking and unlocking from clients, smartcard keys and
//!   every extension are refused with `SSH_AGENT_FAILURE`: the keys come from
//!   the vault only.
//!
//! # Lifecycle
//!
//! Start an [`AgentHandle`] after unlock with a [`KeySet`] built from the
//! vault's items ([`KeySet::from_items`]); call [`AgentCore::set_keys`] when
//! items change; [`AgentHandle::stop`] on lock. Stopping refuses the open
//! approvals ([`CloseReason::Locked`]), closes every connection, removes the
//! socket and drops the keys and grants.
//! [`AgentCore::set_keys`] after a stop keeps nothing.
//!
//! # Secrets
//!
//! [`AgentKey`] keeps the OpenSSH private key text and the item passphrase
//! in zeroizing memory. The private key is parsed, decrypted and used only
//! after an approval, off the async executor, and dropped (wiped) right
//! after signing. Nothing secret reaches an error, `Debug` output or the
//! [`Approver`]. The public key comes from the item's public key text (or,
//! if that is missing, from the private key file's clear header).
//!
//! # Client identity
//!
//! On Linux the client's PID and executable come from the socket's peer
//! credentials and `/proc`; on macOS only the PID is known. On Windows the pipe
//! client's PID needs `GetNamedPipeClientProcessId`, which only unsafe FFI
//! reaches (the crate forbids unsafe code), so nothing is known. In both
//! cases [`ApprovalRequest::can_remember`] is `false`, so
//! [`Decision::AllowFor`] counts as [`Decision::AllowOnce`].
#![forbid(unsafe_code)]

mod approval;
mod core;
mod error;
mod keys;
mod server;
#[cfg(test)]
mod testkeys;
mod transport;
pub mod wire;

pub use approval::{
    ApprovalEvent, ApprovalPrompts, ApprovalRequest, Approver, ClientInfo, CloseReason,
    DEFAULT_GRANT, Decision, InvalidRequestId, RequestExpired, RequestId, SignFailed,
};
pub use core::{AgentConfig, AgentCore};
pub use error::Error;
pub use keys::{
    AgentKey, DEFAULT_MAX_KDF_ROUNDS, KeyAlgorithm, KeyError, KeySet, SignFailure, SkipReason,
    SkippedKey, rsa_signature_algorithm,
};
pub use server::AgentHandle;
pub use transport::Endpoint;

//! An SSH agent (brief §5 "SSH agent") that serves the SSH keys of the
//! unlocked vault, with an approval for every use.
//!
//! Status: scaffold only. Implementation lands in M3 (SSH agent track); see
//! `docs/PROGRESS.md` and `docs/DESKTOP_IPC.md` ("SSH agent approvals").
//!
//! Rules for the implementation:
//!
//! - The agent protocol (draft-miller-ssh-agent) runs over a Unix socket or
//!   a Windows named pipe that only the current user can open.
//! - Listing identities shows public keys only. Every sign request waits for
//!   an approval from the desktop UI (the `sshApprovalRequested` event and
//!   the `ssh_respond` command); without an answer it is refused after a
//!   timeout. A locked vault refuses every request.
//! - Private keys stay in zeroizing memory, are parsed only when a request
//!   is approved and are dropped right after signing.
//! - Key parsing and signing use reviewed crates only, chosen with an ADR
//!   (`docs/CRYPTO_SPEC.md` §2 lists no SSH crate yet).
#![forbid(unsafe_code)]

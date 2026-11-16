//! Windows program fill, phase 1 (ADR-0157, ADR-0158): which program a window
//! belongs to, whether a login may be linked to it, and whether the target is
//! still the one the user picked before anything is written.
//!
//! Phase 1 writes nothing into any program: it only captures and verifies.
//! Everything here is pure and tested on any OS. The Windows side reaches the
//! operating system through a probe trait.
//!
//! - [`identity`]: the captured identity of a process (exe, package, trusted
//!   signer or file hash) and why a capture is refused.
//! - [`policy`]: the link policy: what a login is bound to, and the two
//!   confirmations an unsigned program needs.
//! - [`verify`]: the re-verification before every write (focus, window,
//!   process, image path, field, occlusion, time).
//! - `windows` (Windows only): the probe that reads the real operating system
//!   through `mosaic-winshim`, and the capture of the focused field's program.
#![forbid(unsafe_code)]

pub mod binding;
pub mod detect;
pub mod fill;
pub mod host;
pub mod identity;
pub mod placement;
pub mod policy;
pub mod suggest;
pub mod verify;

#[cfg(any(test, feature = "fake"))]
pub mod fake;

#[cfg(windows)]
pub mod windows;

pub use identity::{
    CaptureRefusal, CapturedIdentity, HashError, IdentitySource, MAX_UNSIGNED_IMAGE_BYTES,
    OwnProcess, RawProcess, SignatureState, SignerDisplay, assemble_identity,
    choose_identity_process, exe_name_of, hash_file_capped, signer_id,
};

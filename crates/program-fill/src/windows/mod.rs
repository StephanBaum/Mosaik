//! The Windows probe: reads the real operating system through `mosaic-winshim`
//! for the pure model of this crate (ADR-0157, ADR-0158). Phase 1 only
//! captures and verifies; nothing is written into any program.

mod uia;

use std::time::{Duration, Instant};

use mosaic_winshim::{
    ImageFile, ProcessHandle, ShimError, Signature, Uia, UiaElement, current_integrity_rid,
    final_path, foreground_window, window_facts,
};

use crate::identity::{
    CaptureRefusal, CapturedIdentity, HashError, IdentitySource, MAX_UNSIGNED_IMAGE_BYTES,
    OwnProcess, RawProcess, SignatureState, SignerDisplay, assemble_identity,
    choose_identity_process, hash_file_capped,
};
use crate::verify::{
    PickSession, ProbeError, ProgramProbe, Rect, Snapshot, WindowFacts, probe_points,
};
use uia::{clear_at, focused_field, probe_error, same};

/// Connection and transaction timeout of the UI Automation client
/// (ADR-0157 D5).
const UIA_TIMEOUT_MS: u32 = 2000;

/// The time one snapshot may take in all: one deadline for every UI Automation
/// call and parent walk of it. A single call can still run to its own
/// [`UIA_TIMEOUT_MS`] past the deadline before it returns, so a snapshot ends
/// within the budget plus one call timeout (ADR-0157 D5).
const SNAPSHOT_BUDGET: Duration = Duration::from_secs(2);

/// The time one capture may take in all: the UI Automation reads, the
/// signature check and the hash of an unsigned image (up to 512 MiB).
const CAPTURE_BUDGET: Duration = Duration::from_secs(10);

/// What a pick session keeps: the identity process's open handle, so its ID
/// cannot be reused while the session lives, and the picked element for
/// `CompareElements`.
#[derive(Debug)]
pub struct WinHeld {
    process: ProcessHandle,
    element: UiaElement,
}

/// A captured program and the session that picked its field.
pub struct Captured {
    /// The program identity.
    pub identity: CapturedIdentity,
    /// The pick, for [`ProgramProbe::snapshot`] and `check`.
    pub session: PickSession<WinHeld>,
}

/// Why a capture failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CaptureError {
    /// Nothing, or something that is not an edit control, has focus.
    #[error("the focused element is not a text field")]
    NotAField,
    /// The program cannot be captured (excluded, elevated, own process, ...).
    #[error("capture refused: {0}")]
    Refused(#[from] CaptureRefusal),
    /// The operating system could not be read.
    #[error("probe failed: {0}")]
    Probe(#[from] ProbeError),
}

/// Reads the focused field and its program, and re-reads them before every
/// write. Not `Send`: it owns a UI Automation client of its thread.
pub struct WindowsProbe {
    uia: Uia,
    own: OwnProcess,
}

impl WindowsProbe {
    /// Starts the UI Automation client and reads Mosaic's own integrity level.
    /// Call it on a thread in the COM multithreaded apartment (a plain worker
    /// thread; the client initialises the apartment itself): on a
    /// single-threaded apartment thread, such as a UI thread, it fails.
    ///
    /// # Errors
    /// [`ProbeError`] if the client cannot start.
    pub fn new() -> Result<Self, ProbeError> {
        let uia = Uia::new(UIA_TIMEOUT_MS, UIA_TIMEOUT_MS).map_err(probe_error)?;
        let integrity_rid = current_integrity_rid().map_err(probe_error)?;
        Ok(Self {
            uia,
            own: OwnProcess {
                pid: std::process::id(),
                integrity_rid,
            },
        })
    }

    /// Reads the focused edit and its program now. `picked_at_ms` is the
    /// caller's monotonic clock.
    ///
    /// # Errors
    /// [`CaptureError`].
    pub fn capture_focused(&self, picked_at_ms: u64) -> Result<Captured, CaptureError> {
        let deadline = Instant::now() + CAPTURE_BUDGET;
        let (element, field, window) =
            focused_field(&self.uia, deadline)?.ok_or(CaptureError::NotAField)?;
        let window = window_facts(window).map_err(probe_error)?;
        let element_process = ProcessHandle::open(field.pid).map_err(probe_error)?;
        let owner_process = ProcessHandle::open(window.owner_pid).map_err(probe_error)?;
        let (identity, held) = identify(self.own, element_process, owner_process, deadline)?;
        let session = PickSession::new(
            WinHeld {
                process: held,
                element,
            },
            &identity,
            field.pid,
            window.owner_pid,
            window.root,
            field.runtime_id,
            field.kind,
            picked_at_ms,
        );
        Ok(Captured { identity, session })
    }
}

impl ProgramProbe for WindowsProbe {
    type Held = WinHeld;

    fn snapshot(&self, session: &PickSession<WinHeld>) -> Result<Snapshot, ProbeError> {
        // Nothing is cached between snapshots: every fact is read now.
        let deadline = Instant::now() + SNAPSHOT_BUDGET;
        let focus = focused_field(&self.uia, deadline)?;
        let focused_is_target = match &focus {
            Some((element, _, _)) => same(&self.uia, element, &session.held.element, deadline)?,
            None => false,
        };
        let focused = focus.map(|(_, facts, _)| facts);

        let held = &session.held.process;
        let process_running = held.is_running().map_err(probe_error)?;
        let image = held
            .image_path()
            .and_then(|path| final_path(&path))
            .map_err(probe_error);
        // A process that ended cannot report its image; the verdict is then
        // `ProcessGone`, decided before the path is looked at.
        let image_final_path = match image {
            Ok(path) => path,
            Err(_) if !process_running => String::new(),
            Err(err) => return Err(err),
        };

        let foreground = foreground_window()
            .and_then(|hwnd| window_facts(hwnd).ok())
            .map(|w| WindowFacts {
                root: w.root,
                pid: w.owner_pid,
                cloaked: w.cloaked,
                minimized: w.minimized,
            });

        // The occlusion probes only matter (and only cost UIA calls) when the
        // target has focus; otherwise the check fails earlier anyway.
        let occlusion_clear = if focused_is_target {
            self.occlusion(
                focused.as_ref().map(|f| f.rect),
                &session.held.element,
                deadline,
            )?
        } else {
            [false; 5]
        };
        Ok(Snapshot {
            focused,
            focused_is_target,
            process_running,
            image_final_path,
            foreground,
            occlusion_clear,
        })
    }
}

impl WindowsProbe {
    /// For each of the five probe points of `rect`: whether the element there
    /// is the target or inside it. All `false` for a rectangle with no points.
    fn occlusion(
        &self,
        rect: Option<Rect>,
        target: &UiaElement,
        deadline: Instant,
    ) -> Result<[bool; 5], ProbeError> {
        let Some(points) = rect.and_then(probe_points) else {
            return Ok([false; 5]);
        };
        let mut clear = [false; 5];
        for (slot, point) in clear.iter_mut().zip(points) {
            *slot = clear_at(&self.uia, target, point, deadline)?;
        }
        Ok(clear)
    }
}

/// The identity of the program behind a focused element: chooses the process
/// (window owner or element process), opens its image once, verifies the
/// signature and hashes an unsigned image (within the cap) through that one
/// handle, and assembles the identity. Returns the chosen process's handle
/// for the session to keep. An image on a network path is refused.
fn identify(
    own: OwnProcess,
    element: ProcessHandle,
    owner: ProcessHandle,
    deadline: Instant,
) -> Result<(CapturedIdentity, ProcessHandle), CaptureError> {
    let raw_element = raw_process(&element)?;
    let raw_owner = raw_process(&owner)?;
    let (chosen, mut raw) = match choose_identity_process(&raw_element, &raw_owner)? {
        IdentitySource::WindowOwner => (owner, raw_owner),
        IdentitySource::ElementProcess => (element, raw_element),
    };
    let image = chosen.open_image().map_err(image_error)?;
    // The open file is the process image (checked by `open_image`); its final
    // path is the identity's path.
    image.final_path().clone_into(&mut raw.final_path);
    // The size cap comes first: the signature checks and the catalog hashes
    // read the whole image and must not run past the deadline on a huge one.
    let image_len = image
        .file()
        .metadata()
        .map_err(|_| CaptureError::Probe(ProbeError::Os))?
        .len();
    if image_len > MAX_UNSIGNED_IMAGE_BYTES {
        return Err(CaptureError::Refused(CaptureRefusal::TooLargeToBind));
    }
    let signature = signature_state(&image, deadline)?;
    let file_sha256 = if matches!(signature, SignatureState::Unsigned) {
        hash_unsigned(&image, deadline)?
    } else {
        None
    };
    let identity = assemble_identity(own, &raw, signature, file_sha256)?;
    Ok((identity, chosen))
}

/// The OS's view of one process, read through its held handle.
fn raw_process(handle: &ProcessHandle) -> Result<RawProcess, ProbeError> {
    let final_path = handle
        .image_path()
        .and_then(|path| final_path(&path))
        .map_err(probe_error)?;
    Ok(RawProcess {
        pid: handle.pid(),
        final_path,
        package_family: handle.package_family_name().map_err(probe_error)?,
        integrity_rid: handle.integrity_rid().map_err(probe_error)?,
    })
}

/// A failure to open the image: a network path is a refusal, anything else a
/// probe failure.
fn image_error(err: ShimError) -> CaptureError {
    match err {
        ShimError::NetworkImage => CaptureError::Refused(CaptureRefusal::NetworkImage),
        other => CaptureError::Probe(probe_error(other)),
    }
}

/// The Authenticode state of the open image, as the model's.
fn signature_state(image: &ImageFile, deadline: Instant) -> Result<SignatureState, ProbeError> {
    Ok(
        match image.verify_authenticode(deadline).map_err(probe_error)? {
            Signature::Trusted {
                subject_der,
                root_der,
                common_name,
                organization,
            } => SignatureState::Trusted {
                subject_der,
                root_der,
                display: SignerDisplay {
                    common_name,
                    organization,
                },
            },
            Signature::Unsigned => SignatureState::Unsigned,
            Signature::Revoked => SignatureState::Revoked,
            Signature::Untrusted => SignatureState::Untrusted,
        },
    )
}

/// The hash of an unsigned image read through its open handle, `None` when it
/// is over the cap.
fn hash_unsigned(image: &ImageFile, deadline: Instant) -> Result<Option<[u8; 32]>, ProbeError> {
    match hash_file_capped(image.file(), MAX_UNSIGNED_IMAGE_BYTES, deadline) {
        Ok(hash) => Ok(Some(hash)),
        Err(HashError::TooLarge) => Ok(None),
        Err(HashError::Timeout) => Err(ProbeError::Timeout),
        Err(HashError::Io) => Err(ProbeError::Os),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::{FieldKind, WriteKind, check};
    use mosaic_winshim::ShimError;

    /// A child process that is killed and reaped on drop.
    struct Child(std::process::Child);

    impl Drop for Child {
        fn drop(&mut self) {
            // Nothing more to do if the child is already gone.
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(30)
    }

    fn own() -> OwnProcess {
        OwnProcess {
            pid: std::process::id(),
            integrity_rid: current_integrity_rid().unwrap(),
        }
    }

    #[test]
    fn the_probe_starts() {
        std::thread::spawn(|| {
            WindowsProbe::new().unwrap();
        })
        .join()
        .unwrap();
    }

    #[test]
    fn capturing_without_a_focused_edit_never_panics() {
        std::thread::spawn(|| {
            let probe = WindowsProbe::new().unwrap();
            // Whatever has focus on this desktop, the answer is a value.
            match probe.capture_focused(0) {
                Ok(_)
                | Err(
                    CaptureError::NotAField | CaptureError::Refused(_) | CaptureError::Probe(_),
                ) => {}
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn mosaics_own_process_cannot_be_captured() {
        let me = std::process::id();
        let result = identify(
            own(),
            ProcessHandle::open(me).unwrap(),
            ProcessHandle::open(me).unwrap(),
            soon(),
        );
        assert!(matches!(
            result,
            Err(CaptureError::Refused(CaptureRefusal::OwnProcess))
        ));
    }

    #[test]
    fn a_signed_system_program_is_captured_with_its_signer() {
        // ping.exe is a Microsoft-signed system binary that waits for a while.
        let child = Child(
            std::process::Command::new("ping")
                .args(["-n", "30", "127.0.0.1"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let pid = child.0.id();
        let (identity, held) = identify(
            own(),
            ProcessHandle::open(pid).unwrap(),
            ProcessHandle::open(pid).unwrap(),
            soon(),
        )
        .unwrap();
        assert_eq!(held.pid(), pid);
        assert_eq!(identity.pid, pid);
        assert_eq!(identity.exe_name, "ping.exe");
        assert!(identity.signer_id.is_some());
        assert_eq!(identity.file_sha256, None);
        assert!(
            identity
                .signer_display
                .as_ref()
                .and_then(|d| d.organization.as_deref())
                .is_some_and(|o| o.contains("Microsoft"))
        );
        let proposed = crate::policy::propose_binding(&identity, &[]).unwrap();
        assert!(
            proposed
                .url
                .canonical()
                .starts_with("winapp://signed/ping.exe")
        );
    }

    #[test]
    fn an_image_on_a_network_path_is_refused_not_a_probe_failure() {
        assert!(matches!(
            image_error(ShimError::NetworkImage),
            CaptureError::Refused(CaptureRefusal::NetworkImage)
        ));
        assert!(matches!(
            image_error(ShimError::ImageChanged),
            CaptureError::Probe(ProbeError::Os)
        ));
        assert!(matches!(
            image_error(ShimError::Timeout),
            CaptureError::Probe(ProbeError::Timeout)
        ));
    }

    #[test]
    fn an_unsigned_image_is_hashed_through_its_open_handle() {
        // This test binary is unsigned: the capture path hashes it. Mosaic's
        // own pid is made to differ so the test process can be the target.
        let me = std::process::id();
        let other_own = OwnProcess {
            pid: me.wrapping_add(1),
            integrity_rid: current_integrity_rid().unwrap(),
        };
        let (identity, _) = identify(
            other_own,
            ProcessHandle::open(me).unwrap(),
            ProcessHandle::open(me).unwrap(),
            soon(),
        )
        .unwrap();
        let bytes = std::fs::read(&identity.final_path).unwrap();
        let expected: [u8; 32] = <sha2::Sha256 as sha2::Digest>::digest(&bytes).into();
        assert_eq!(identity.file_sha256, Some(expected));
        assert_eq!(identity.signer_id, None);
    }

    #[test]
    fn shim_errors_map_to_probe_errors() {
        assert_eq!(probe_error(ShimError::Timeout), ProbeError::Timeout);
        assert_eq!(probe_error(ShimError::NotFound), ProbeError::Gone);
        assert_eq!(probe_error(ShimError::AccessDenied), ProbeError::Os);
        assert_eq!(probe_error(ShimError::Os(5)), ProbeError::Os);
    }

    #[test]
    #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]
    fn the_focused_field_is_captured_and_verified() {
        // Manual checklist only: CI runs ignored tests in release, so opt in explicitly.
        if std::env::var_os("MOSAIC_MANUAL_TESTS").is_none() {
            return;
        }
        let probe = WindowsProbe::new().unwrap();
        let captured = probe.capture_focused(0).unwrap();
        let write = match captured.session.target_kind {
            FieldKind::Password => WriteKind::Password,
            FieldKind::Text => WriteKind::Username,
        };
        check(&probe, &captured.session, write, 1).unwrap();
    }
}

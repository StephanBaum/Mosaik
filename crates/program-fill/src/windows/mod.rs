//! The Windows probe: reads the real operating system through `mosaic-winshim`
//! for the pure model of this crate (ADR-0157, ADR-0158, ADR-0163, ADR-0164).
//! The probe captures and verifies; the fill is driven by [`WindowsHost`]
//! through [`driver::WindowsDriver`].

mod driver;
mod host;
mod uia;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mosaic_winshim::{
    ImageFile, ProcessHandle, ShimError, Signature, Uia, UiaElement, current_integrity_rid,
    final_path, foreground_window, process_chain, window_facts,
};

pub use host::WindowsHost;

use crate::detect::{CONTAINER_LEVELS, EditInfo, MAX_EDITS_SCANNED, is_web_content, usable};
use crate::identity::{
    CaptureRefusal, CapturedIdentity, ChainProcess, HashError, IdentitySource,
    MAX_UNSIGNED_IMAGE_BYTES, MAX_WEBVIEW2_CHAIN, OwnProcess, RawProcess, SignatureState,
    SignerDisplay, WEBVIEW2_EXE, assemble_identity, assemble_webview2_host_identity,
    choose_identity_process_webview2, exe_name_of, hash_file_capped, product_class, screen_process,
};
use crate::verify::{
    FieldFacts, PickSession, ProbeError, ProgramProbe, Rect, RuntimeId, Snapshot, WindowFacts,
    probe_points,
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
/// cannot be reused while the session lives, the picked element for
/// `CompareElements`, the other edits of its container (found when a fill
/// starts), and, for `WebView2` content, the handles of every process from the
/// element's to the host's child.
#[derive(Debug)]
pub struct WinHeld {
    pub(crate) process: ProcessHandle,
    pub(crate) element: UiaElement,
    pub(crate) others: Vec<UiaElement>,
    pub(crate) chain: Vec<ProcessHandle>,
}

impl WinHeld {
    /// The element for a runtime ID: the picked one or one of its siblings.
    pub(crate) fn element_for(&self, id: &RuntimeId) -> Option<&UiaElement> {
        std::iter::once(&self.element)
            .chain(self.others.iter())
            .find(|e| e.facts().runtime_id == id.0)
    }

    /// Whether the identity process and every held `WebView2` process still run.
    fn all_running(&self) -> Result<bool, ProbeError> {
        if !self.process.is_running().map_err(probe_error)? {
            return Ok(false);
        }
        for held in &self.chain {
            if !held.is_running().map_err(probe_error)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// A captured program and the session that picked its field.
pub struct Captured {
    /// The program identity.
    pub identity: CapturedIdentity,
    /// The focused field as read at capture time.
    pub field: FieldFacts,
    /// The pick, for [`ProgramProbe::snapshot`] and `check`.
    pub session: PickSession<WinHeld>,
    /// Whether the field is web content (a browser engine inside the program).
    pub web_content: bool,
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

/// A program already identified, so a focus change inside it costs no new
/// signature check or hash. Its held handle keeps the process ID from being
/// reused while the entry lives.
struct CacheEntry {
    held: ProcessHandle,
    webview: bool,
    identity: CapturedIdentity,
}

/// At most [`host::IDENTITY_CACHE`] identities, oldest out first.
#[derive(Default)]
struct IdentityCache(VecDeque<CacheEntry>);

impl IdentityCache {
    fn get(&mut self, pid: u32, webview: bool) -> Option<CapturedIdentity> {
        let index = self
            .0
            .iter()
            .position(|e| e.held.pid() == pid && e.webview == webview)?;
        let alive = self
            .0
            .get(index)
            .is_some_and(|e| e.held.is_running().unwrap_or(false));
        if alive {
            self.0.get(index).map(|e| e.identity.clone())
        } else {
            self.0.remove(index);
            None
        }
    }

    fn put(&mut self, entry: CacheEntry) {
        self.0.push_back(entry);
        while self.0.len() > host::IDENTITY_CACHE {
            self.0.pop_front();
        }
    }
}

/// Reads the focused field and its program, and re-reads them before every
/// write. Not `Send`: it owns a UI Automation client of its thread.
pub struct WindowsProbe {
    uia: Uia,
    own: OwnProcess,
    cache: RefCell<IdentityCache>,
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
            cache: RefCell::new(IdentityCache::default()),
        })
    }

    /// The UI Automation client (for the driver and the host).
    pub(crate) fn uia(&self) -> &Uia {
        &self.uia
    }

    /// Mosaic's own process ID.
    pub(crate) fn own_pid(&self) -> u32 {
        self.own.pid
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
        let plan = plan_identity(self.own, element_process, owner_process)?;
        let identity = self.identity_of(&plan, deadline)?;
        let web_content = is_web_content(
            &element.facts().framework_id,
            field.pid,
            window.owner_pid,
            plan.webview,
        );
        let session = PickSession::new(
            WinHeld {
                process: plan.chosen,
                element,
                others: Vec::new(),
                chain: plan.chain,
            },
            &identity,
            field.pid,
            window.owner_pid,
            window.root,
            field.runtime_id.clone(),
            field.kind,
            picked_at_ms,
        );
        Ok(Captured {
            identity,
            field,
            session,
            web_content,
        })
    }

    /// The rectangle of whatever has focus, `None` when nothing does or it
    /// cannot be read (the hotkey places its notice near it).
    pub(crate) fn focused_rect(&self) -> Option<Rect> {
        let deadline = Instant::now() + SNAPSHOT_BUDGET;
        let element = self.uia.focused(deadline).ok()??;
        let [left, top, right, bottom] = element.facts().rect;
        Some(Rect {
            left,
            top,
            right,
            bottom,
        })
    }

    /// The identity of a planned capture, from the cache when this program
    /// was identified before and its process still runs.
    fn identity_of(
        &self,
        plan: &Plan,
        deadline: Instant,
    ) -> Result<CapturedIdentity, CaptureError> {
        let pid = plan.chosen.pid();
        let cached = self.cache.borrow_mut().get(pid, plan.webview);
        if let Some(identity) = cached {
            return Ok(identity);
        }
        let identity = finish_identity(self.own, plan, deadline)?;
        // A second handle keeps the process ID reserved while the entry lives.
        if let Ok(held) = ProcessHandle::open(pid) {
            self.cache.borrow_mut().put(CacheEntry {
                held,
                webview: plan.webview,
                identity: identity.clone(),
            });
        }
        Ok(identity)
    }

    /// The Edits of the picked field's container (shim `edits_near`, at most
    /// [`MAX_EDITS_SCANNED`], [`CONTAINER_LEVELS`] up) as elements.
    fn scan(&self, captured: &Captured) -> Result<(Vec<UiaElement>, usize), ProbeError> {
        let elements = self
            .uia
            .edits_near(
                &captured.session.held.element,
                CONTAINER_LEVELS,
                MAX_EDITS_SCANNED,
            )
            .map_err(probe_error)?;
        let focused = elements
            .iter()
            .position(|e| e.facts().runtime_id == captured.session.target.0)
            .ok_or(ProbeError::Gone)?;
        Ok((elements, focused))
    }

    /// The Edits of the focused field's container (shim
    /// `edits_near(.., CONTAINER_LEVELS, MAX_EDITS_SCANNED)`), as `EditInfo`,
    /// and the focused one's index.
    ///
    /// # Errors
    /// [`ProbeError`]; [`ProbeError::Gone`] when the focused field is not among
    /// them.
    pub fn field_set(&self, captured: &Captured) -> Result<(Vec<EditInfo>, usize), ProbeError> {
        let (elements, focused) = self.scan(captured)?;
        Ok((elements.iter().map(edit_info).collect(), focused))
    }

    /// Like [`WindowsProbe::field_set`], and keeps the elements in the session
    /// so a fill can reach the user-name and password fields.
    pub(crate) fn field_set_attached(
        &self,
        captured: &mut Captured,
    ) -> Result<(Vec<EditInfo>, usize), ProbeError> {
        let (elements, focused) = self.scan(captured)?;
        let infos = elements.iter().map(edit_info).collect();
        captured.session.held.others = elements;
        Ok((infos, focused))
    }

    /// The Edits of the window `root`, searched by a bounded walk that stops at
    /// the second edit `scan` counts (a lone text field, or a sole password
    /// field, needs no more to be decided).
    pub(crate) fn window_edits(
        &self,
        root: isize,
        scan: WindowScan,
    ) -> Result<Vec<EditInfo>, ProbeError> {
        let deadline = Instant::now() + SNAPSHOT_BUDGET;
        let counts = |facts: &mosaic_winshim::ElementFacts| {
            let info = edit_info_of(facts);
            usable(&info) && (scan == WindowScan::Usable || info.is_password)
        };
        let elements = self
            .uia
            .edits_in_window_until(root, MAX_EDITS_SCANNED, deadline, 2, &counts)
            .map_err(probe_error)?;
        Ok(elements.iter().map(edit_info).collect())
    }

    /// Whether the five probe points of `rect` resolve to the top-level window
    /// `own_hwnd` (nothing else covers it).
    pub(crate) fn window_clear(&self, own_hwnd: isize, rect: Rect) -> Result<bool, ProbeError> {
        let Some(points) = probe_points(rect) else {
            return Ok(false);
        };
        let deadline = Instant::now() + SNAPSHOT_BUDGET;
        for point in points {
            let Some(element) = self
                .uia
                .element_from_point(point.x, point.y, deadline)
                .map_err(probe_error)?
            else {
                return Ok(false);
            };
            let hwnd = match self.uia.nearest_window(&element, deadline) {
                Ok(hwnd) => hwnd,
                Err(ShimError::NotFound) => return Ok(false),
                Err(err) => return Err(probe_error(err)),
            };
            match window_facts(hwnd) {
                Ok(facts) if facts.root == own_hwnd => {}
                Ok(_) | Err(ShimError::NotFound) => return Ok(false),
                Err(err) => return Err(probe_error(err)),
            }
        }
        Ok(true)
    }
}

/// Which edits of a window a bounded search counts towards its early stop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WindowScan {
    /// Usable edits (a lone text field).
    Usable,
    /// Usable password edits (the sole password field of a second step).
    Password,
}

/// The model's view of an edit element.
fn edit_info(element: &UiaElement) -> EditInfo {
    edit_info_of(element.facts())
}

fn edit_info_of(facts: &mosaic_winshim::ElementFacts) -> EditInfo {
    let [left, top, right, bottom] = facts.rect;
    EditInfo {
        runtime_id: RuntimeId(facts.runtime_id.clone()),
        is_password: facts.is_password,
        enabled: facts.enabled,
        offscreen: facts.offscreen,
        rect: Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}

impl ProgramProbe for WindowsProbe {
    type Held = WinHeld;

    fn snapshot(&self, session: &PickSession<WinHeld>) -> Result<Snapshot, ProbeError> {
        // Nothing is cached between snapshots: every fact is read now.
        let deadline = Instant::now() + SNAPSHOT_BUDGET;
        let focus = focused_field(&self.uia, deadline)?;
        let target = session.held.element_for(&session.target);
        let focused_is_target = match (&focus, target) {
            (Some((element, _, _)), Some(target)) => same(&self.uia, element, target, deadline)?,
            _ => false,
        };
        let focused = focus.map(|(_, facts, _)| facts);

        let process_running = session.held.all_running()?;
        let image = session
            .held
            .process
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
        let occlusion_clear = match (focused_is_target, target) {
            (true, Some(target)) => {
                self.occlusion(focused.as_ref().map(|f| f.rect), target, deadline)?
            }
            _ => [false; 5],
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

/// A program identified up to the point where the image is opened: the chosen
/// process, whether it is the host of `WebView2` content, and the handles of
/// the `WebView2` processes to keep for the session.
struct Plan {
    chosen: ProcessHandle,
    raw: RawProcess,
    webview: bool,
    chain: Vec<ProcessHandle>,
}

/// A captured identity with the handles a session keeps (tests; the probe goes
/// through its cache instead).
#[cfg(test)]
struct Identified {
    identity: CapturedIdentity,
    held: ProcessHandle,
}

/// The parent chain of a `WebView2` element process up to the window owner,
/// read through open handles: the descriptions for the pure check and the
/// handles to keep (the element's process first, then the ones in between; the
/// owner's handle is the caller's). Empty unless the element's process is a
/// `WebView2` process other than the owner.
fn webview2_chain(
    element: &ProcessHandle,
    raw_element: &RawProcess,
    owner: &ProcessHandle,
) -> (Vec<ChainProcess>, Vec<ProcessHandle>) {
    let none = (Vec::new(), Vec::new());
    if element.pid() == owner.pid()
        || exe_name_of(&raw_element.final_path).is_none_or(|n| n != WEBVIEW2_EXE)
    {
        return none;
    }
    let Ok(pids) = process_chain(element.pid(), MAX_WEBVIEW2_CHAIN) else {
        return none;
    };
    // Up to and including the owner; a chain that never reaches it is no chain.
    let Some(owner_at) = pids.iter().position(|&p| p == owner.pid()) else {
        return none;
    };
    let describe = |handle: &ProcessHandle| -> Option<ChainProcess> {
        let path = final_path(&handle.image_path().ok()?).ok()?;
        Some(ChainProcess {
            pid: handle.pid(),
            exe_name: exe_name_of(&path)?,
            start_time: handle.start_time().ok()?,
        })
    };
    let mut described = Vec::new();
    let mut held = Vec::new();
    for (index, &pid) in pids.iter().take(owner_at + 1).enumerate() {
        let entry = if index == 0 {
            describe(element)
        } else if pid == owner.pid() {
            describe(owner)
        } else if let Ok(handle) = ProcessHandle::open(pid) {
            let entry = describe(&handle);
            held.push(handle);
            entry
        } else {
            None
        };
        match entry {
            Some(entry) => described.push(entry),
            // A process that cannot be read cannot be vouched for.
            None => return none,
        }
    }
    (described, held)
}

/// Chooses the identity process (the window owner, a UWP content process, or
/// the host of `WebView2` content), screens it, and keeps the handles a session
/// needs. Nothing is signed or hashed yet.
fn plan_identity(
    own: OwnProcess,
    element: ProcessHandle,
    owner: ProcessHandle,
) -> Result<Plan, CaptureError> {
    let raw_element = raw_process(&element)?;
    let raw_owner = raw_process(&owner)?;
    let (described, between) = webview2_chain(&element, &raw_element, &owner);
    let plan = match choose_identity_process_webview2(&raw_element, &raw_owner, &described)? {
        IdentitySource::WindowOwner => Plan {
            chosen: owner,
            raw: raw_owner,
            webview: false,
            chain: Vec::new(),
        },
        IdentitySource::ElementProcess => Plan {
            chosen: element,
            raw: raw_element,
            webview: false,
            chain: Vec::new(),
        },
        IdentitySource::WebView2Host => {
            let mut chain = vec![element];
            chain.extend(between);
            Plan {
                chosen: owner,
                raw: raw_owner,
                webview: true,
                chain,
            }
        }
    };
    screen_process(own, &plan.raw)?;
    Ok(plan)
}

/// Opens the chosen process's image once, verifies the signature and hashes an
/// unsigned image (within the cap) through that one handle, and assembles the
/// identity. An image on a network path is refused.
fn finish_identity(
    own: OwnProcess,
    plan: &Plan,
    deadline: Instant,
) -> Result<CapturedIdentity, CaptureError> {
    let image = plan.chosen.open_image().map_err(image_error)?;
    // A browser renamed to another file name is still a browser (ADR-0164).
    if let Ok(Some(product)) = image.product_name()
        && let Some(class) = product_class(&product)
    {
        return Err(CaptureError::Refused(CaptureRefusal::Excluded(class)));
    }
    let mut raw = plan.raw.clone();
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
    let identity = if plan.webview {
        assemble_webview2_host_identity(own, &raw, signature, file_sha256)?
    } else {
        assemble_identity(own, &raw, signature, file_sha256)?
    };
    Ok(identity)
}

/// The identity of the program behind a focused element, without the cache.
#[cfg(test)]
fn identify(
    own: OwnProcess,
    element: ProcessHandle,
    owner: ProcessHandle,
    deadline: Instant,
) -> Result<Identified, CaptureError> {
    let plan = plan_identity(own, element, owner)?;
    let identity = finish_identity(own, &plan, deadline)?;
    Ok(Identified {
        identity,
        held: plan.chosen,
    })
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
        let Identified { identity, held, .. } = identify(
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
        let Identified { identity, .. } = identify(
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

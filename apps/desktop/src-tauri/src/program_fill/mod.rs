//! The desktop end of Windows program fill (ADR-0164): what the host reports becomes a chip next
//! to the field; a click opens the list; a pick in Mosaic's own list is the only thing that
//! fills; a program without a link can be linked on the spot.
//!
//! ```text
//! focus -> [host] -- HostEvent --> [service: offer, model] -- windows --> chip / list (webviews)
//!                                      ^   |                                   | picker_* commands
//!                                      |   +-- vault reads (titles, user names) <+
//!                                      +----- host.fill(FillRequest) <---- pick (generation, guard time,
//!                                                                          window in front, place unchanged)
//! ```
//!
//! No fill happens without a pick (`picks`): a focus event only ever shows the chip. The password
//! goes from the vault (`SecretString`) to the host's channel and into the shim's zeroizing input
//! buffer; no webview sees it, and the picker state carries titles, user names and program names
//! only. The mutex of the model is never held across a call into the host, the vault or a window.

pub mod blocklist;
mod link;
mod offer;
mod picker_windows;
mod picks;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use mosaic_core::{ItemId, VaultId};
use mosaic_origin::program::ProgramClass;
use mosaic_program_fill::binding::{block_key, display_for};
use mosaic_program_fill::detect::FieldRole;
use mosaic_program_fill::host::{FieldOffer, Host, HostEvent, HostSink, Token, monotonic_ms};
use mosaic_program_fill::identity::{CaptureRefusal, CapturedIdentity};
use mosaic_program_fill::placement::{chip_rect, list_rect};
use mosaic_program_fill::policy::LinkConfirmation;
use mosaic_program_fill::verify::{CONTINUATION_TTL_MS, Point, Rect, RuntimeId};
use tauri::ipc::Channel;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::picker::{
    LinkStep, PickerEvent, PickerGenerationRequest, PickerMode, PickerNotice, PickerState,
    ProgramKind, ProgramView,
};
use crate::contract::program_fill::{
    ProgramFillStatus, ProgramFillUnavailable, ProgramFillUnblockRequest,
};
use crate::shortcut::HotkeyRegistrar;
use crate::state::{Backend, Slot};

pub use blocklist::{BLOCKLIST_FILE, BlockList, MAX_BLOCKED};
pub use offer::{MAX_MATCHES, MAX_QUERY_CHARS, MAX_SEARCH};
#[cfg(windows)]
pub use picker_windows::TauriPickerWindows;
pub use picker_windows::{NoPickerWindows, PickerLabel, PickerWindows};

/// A pick earlier than this after the list appeared is refused (milliseconds).
pub const PICK_MIN_DELAY_MS: u64 = 300;
/// A link confirmation earlier than this after the step appeared is refused (milliseconds).
pub const LINK_MIN_DELAY_MS: u64 = 1_000;
/// A click on the locked chip stays valid for this long, so unlocking brings the list up
/// (milliseconds).
pub const UNLOCK_RESUME_MS: u64 = 60_000;
/// How often the chip's field is looked at while the chip shows (milliseconds).
pub const RECT_POLL_MS: u64 = 250;
/// How long a notice stays (milliseconds).
pub const NOTICE_MS: u64 = 6_000;
/// How long the focus settles after the unlock before the field is looked at again.
const RESUME_SETTLE_MS: u64 = 200;

/// Milliseconds on the host's monotonic clock.
pub type PfClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Whether Mosaic runs above the normal integrity level (program fill then stays off: Mosaic
/// never asks for elevation).
#[cfg(windows)]
#[must_use]
pub fn mosaic_elevated() -> bool {
    /// The medium integrity level's RID.
    const MEDIUM_RID: u32 = 0x2000;
    // A failed read counts as elevated: nothing starts that cannot be checked.
    mosaic_winshim::current_integrity_rid().map_or(true, |rid| rid > MEDIUM_RID)
}

/// Whether Mosaic runs above the normal integrity level (never, off Windows: it reads nothing).
#[cfg(not(windows))]
#[must_use]
pub const fn mosaic_elevated() -> bool {
    false
}

/// A login that was filled with its user name only, for the second step of a two-step login.
#[derive(Clone)]
pub(crate) struct Cont {
    pub(crate) vault: VaultId,
    pub(crate) item: ItemId,
    pub(crate) identity: CapturedIdentity,
    pub(crate) picked_at: u64,
    pub(crate) used: bool,
}

/// A click on the locked chip: unlocking reopens the list for the same field.
#[derive(Clone)]
pub(crate) struct Resume {
    /// The clicked offer's token and program: the list reopens only against the same offer.
    pub(crate) token: Token,
    pub(crate) identity: CapturedIdentity,
    pub(crate) target: RuntimeId,
    pub(crate) until: u64,
}

/// The link step in progress.
pub(crate) struct LinkState {
    pub(crate) view: LinkStep,
    pub(crate) confirmation: LinkConfirmation,
    pub(crate) vault: VaultId,
    pub(crate) item: ItemId,
    pub(crate) shown_at: u64,
}

/// What the pickers show, and what they were offered.
pub(crate) struct Model {
    pub(crate) generation: u64,
    pub(crate) mode: PickerMode,
    pub(crate) locked: bool,
    pub(crate) offer: Option<FieldOffer>,
    pub(crate) program: Option<ProgramView>,
    pub(crate) data: offer::Offer,
    pub(crate) link: Option<LinkState>,
    pub(crate) notice: Option<PickerNotice>,
    pub(crate) chip_at: Option<Rect>,
    pub(crate) chip_visible: bool,
    /// Where the field was seen once after it moved; the chip returns when the next look agrees.
    pub(crate) pending_rect: Option<Rect>,
    pub(crate) list_at: Option<Rect>,
    pub(crate) list_shown_at: u64,
    pub(crate) fill_running: bool,
    pub(crate) cont: Option<Cont>,
    pub(crate) resume: Option<Resume>,
    /// The login the list opens on: the one of the first step of a two-step login whose second
    /// step was not written without a pick.
    pub(crate) preselect: Option<(VaultId, ItemId)>,
}

impl Model {
    const fn new() -> Self {
        Self {
            generation: 0,
            mode: PickerMode::Hidden,
            locked: false,
            offer: None,
            program: None,
            data: offer::Offer {
                matches: Vec::new(),
                link_again: None,
                suggestions: Vec::new(),
            },
            link: None,
            notice: None,
            chip_at: None,
            chip_visible: false,
            pending_rect: None,
            list_at: None,
            list_shown_at: 0,
            fill_running: false,
            cont: None,
            resume: None,
            preselect: None,
        }
    }

    /// Nothing is on offer or shown (the continuation, the resume and a running fill stay).
    pub(crate) fn clear_view(&mut self) -> Vec<Effect> {
        self.mode = PickerMode::Hidden;
        self.locked = false;
        self.offer = None;
        self.program = None;
        self.data = offer::Offer::default();
        self.link = None;
        self.notice = None;
        self.chip_at = None;
        self.chip_visible = false;
        self.pending_rect = None;
        self.list_at = None;
        self.preselect = None;
        self.generation += 1;
        vec![
            Effect::Hide(PickerLabel::Chip),
            Effect::Hide(PickerLabel::List),
        ]
    }
}

/// A call into the windows, made after the model's lock is released.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Effect {
    ShowChip(Rect),
    ShowList(Rect, bool),
    Hide(PickerLabel),
    Changed(PickerLabel, u64),
    FocusMain,
    /// Destroy both windows (a lock, or program fill stopped).
    CloseAll,
}

/// The program fill service.
pub struct ProgramFillService {
    blocklist: Mutex<BlockList>,
    host: Option<Arc<dyn Host>>,
    windows: Arc<dyn PickerWindows>,
    clock: PfClock,
    elevated: Arc<dyn Fn() -> bool + Send + Sync>,
    running: AtomicBool,
    host_failed: AtomicBool,
    hotkey: Mutex<Option<HotkeyRegistrar>>,
    model: Mutex<Model>,
    backend: Mutex<Weak<Backend>>,
    /// Whether the background timers (rectangle poll, notice expiry, unlock settle) run.
    timers: bool,
    poller_live: AtomicBool,
    /// The channel of each picker window's `changed` events (the latest subscription wins).
    channels: Mutex<HashMap<PickerLabel, Channel<PickerEvent>>>,
}

impl ProgramFillService {
    /// A service over `host` (`None`: this device cannot read programs) and the picker windows.
    #[must_use]
    pub fn new(
        host: Option<Arc<dyn Host>>,
        windows: Arc<dyn PickerWindows>,
        blocklist: BlockList,
        clock: PfClock,
    ) -> Self {
        Self {
            blocklist: Mutex::new(blocklist),
            host,
            windows,
            clock,
            elevated: Arc::new(mosaic_elevated),
            running: AtomicBool::new(false),
            host_failed: AtomicBool::new(false),
            hotkey: Mutex::new(None),
            model: Mutex::new(Model::new()),
            backend: Mutex::new(Weak::new()),
            timers: true,
            poller_live: AtomicBool::new(false),
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// A service without a host and windows that keeps its block list nowhere (the default of a
    /// backend that does not run program fill, and tests).
    #[must_use]
    pub fn memory() -> Self {
        Self::new(
            None,
            Arc::new(NoPickerWindows),
            BlockList::memory(),
            Arc::new(monotonic_ms),
        )
    }

    /// Replaces the check whether Mosaic itself runs elevated (tests).
    #[must_use]
    pub fn with_elevation_check(mut self, check: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        self.elevated = check;
        self
    }

    /// Turns the background timers off (tests drive them by hand).
    #[must_use]
    pub const fn without_timers(mut self) -> Self {
        self.timers = false;
        self
    }

    /// Lets the host's events reach the backend.
    pub(crate) fn attach(&self, backend: Weak<Backend>) {
        *self.backend.lock().unwrap_or_else(PoisonError::into_inner) = backend;
    }

    /// A picker window's channel for `changed` events (the latest of a window wins).
    pub fn subscribe(&self, which: PickerLabel, channel: Channel<PickerEvent>) {
        self.channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(which, channel);
    }

    /// Tells a window its state has a new generation. A failed send is a window that is gone.
    fn notify(&self, which: PickerLabel, generation: u64) {
        let channel = self
            .channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&which)
            .cloned();
        if let Some(channel) = channel {
            let _ = channel.send(PickerEvent::Changed { generation });
        }
    }

    pub(crate) fn now(&self) -> u64 {
        (self.clock)()
    }

    pub(crate) fn model(&self) -> MutexGuard<'_, Model> {
        self.model.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn blocklist(&self) -> MutexGuard<'_, BlockList> {
        self.blocklist
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn host(&self) -> Option<&Arc<dyn Host>> {
        self.host.as_ref()
    }

    pub(crate) fn windows(&self) -> &Arc<dyn PickerWindows> {
        &self.windows
    }

    /// Sets the registrar of the program fill hotkey.
    pub fn set_hotkey_registrar(&self, registrar: Option<HotkeyRegistrar>) {
        *self.hotkey.lock().unwrap_or_else(PoisonError::into_inner) = registrar;
    }

    pub(crate) fn hotkey_registrar(&self) -> Option<HotkeyRegistrar> {
        self.hotkey
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Why program fill cannot run on this device (the watcher's failure is not counted).
    fn base_reason(&self) -> Option<ProgramFillUnavailable> {
        if self.host.is_none() {
            return Some(ProgramFillUnavailable::NotWindows);
        }
        (self.elevated)().then_some(ProgramFillUnavailable::MosaicElevated)
    }

    pub(crate) fn status(&self) -> ProgramFillStatus {
        let reason = self.base_reason().or_else(|| {
            self.host_failed
                .load(Ordering::SeqCst)
                .then_some(ProgramFillUnavailable::HostFailed)
        });
        ProgramFillStatus {
            available: reason.is_none(),
            reason,
            running: self.running.load(Ordering::SeqCst),
            blocked: self.blocklist().entries().to_vec(),
        }
    }

    /// Makes the calls into the windows a model change asked for.
    pub(crate) fn apply(&self, effects: &[Effect]) {
        for effect in effects {
            match *effect {
                Effect::ShowChip(at) => {
                    self.windows.show_chip(at);
                }
                Effect::ShowList(at, focus) => {
                    self.windows.show_list(at, focus);
                }
                Effect::Hide(which) => self.windows.hide(which),
                Effect::Changed(which, generation) => {
                    self.windows.changed(which, generation);
                    self.notify(which, generation);
                }
                Effect::FocusMain => self.windows.focus_main(),
                Effect::CloseAll => {
                    self.windows.close_all();
                    self.channels
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clear();
                }
            }
        }
    }

    /// Runs `work` after `ms` on a short-lived thread, when timers are on.
    fn after(&self, ms: u64, work: impl FnOnce() + Send + 'static) {
        if !self.timers {
            return;
        }
        let spawned = std::thread::Builder::new()
            .name("mosaic-program-fill-timer".to_owned())
            .spawn(move || {
                std::thread::sleep(Duration::from_millis(ms));
                work();
            });
        // No thread, no timer: a notice then stays until the next event replaces it.
        drop(spawned);
    }

    fn weak_backend(&self) -> Weak<Backend> {
        self.backend
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The program, as the picker shows it.
pub(crate) fn program_view(identity: &CapturedIdentity) -> ProgramView {
    let display = display_for(identity);
    let kind = if identity.package_family.is_some() {
        ProgramKind::Package
    } else if identity.signer_id.is_some() {
        ProgramKind::Signed
    } else {
        ProgramKind::Unsigned
    };
    ProgramView {
        kind,
        exe: identity.exe_name.clone(),
        name: display.name,
        signer: display.signer,
        path: (kind == ProgramKind::Unsigned).then(|| identity.final_path.clone()),
    }
}

pub(crate) fn center(rect: Rect) -> Point {
    Point {
        x: rect.left + (rect.right - rect.left) / 2,
        y: rect.top + (rect.bottom - rect.top) / 2,
    }
}

pub(crate) fn stale() -> IpcError {
    IpcError::new(ErrorCode::PickerStale)
}

/// What one look at the chip's field decided.
enum PollAction {
    Nothing,
    Hide,
    Reposition(Rect),
}

impl Backend {
    /// `program_fill_status`.
    #[must_use]
    pub fn program_fill_status(&self) -> ProgramFillStatus {
        self.program_fill.status()
    }

    /// `program_fill_unblock`: offers the program's logins again.
    ///
    /// # Errors
    /// `storeUnavailable` when the list cannot be written (the entry is off the list for this run).
    pub fn program_fill_unblock(
        &self,
        request: &ProgramFillUnblockRequest,
    ) -> IpcResult<ProgramFillStatus> {
        self.touch();
        self.program_fill.blocklist().remove(&request.key)?;
        Ok(self.program_fill.status())
    }

    /// Starts or stops the watcher to match the settings. Called by `update_settings` and after
    /// every unlock; the settings of a file that did not verify read as off, so nothing starts
    /// from one.
    pub fn program_fill_apply_settings(&self) {
        let svc = &self.program_fill;
        let Some(host) = svc.host().cloned() else {
            return;
        };
        let wanted = self.settings_snapshot().program_fill && svc.base_reason().is_none();
        let running = svc.running.load(Ordering::SeqCst);
        if wanted && !running {
            svc.host_failed.store(false, Ordering::SeqCst);
            let weak = svc.weak_backend();
            let sink: HostSink = Arc::new(move |event| {
                // The host checks its epoch before calling, but not atomically with the
                // call: an event that raced a stop is dropped here.
                if let Some(backend) = weak.upgrade()
                    && backend.program_fill.running.load(Ordering::SeqCst)
                {
                    backend.program_fill_on_event(event);
                }
            });
            // Set first: the sink drops events while it is false.
            svc.running.store(true, Ordering::SeqCst);
            if host.start(sink).is_err() {
                svc.running.store(false, Ordering::SeqCst);
                svc.host_failed.store(true, Ordering::SeqCst);
            }
        } else if !wanted && running {
            svc.running.store(false, Ordering::SeqCst);
            host.stop();
            let effects = {
                let mut m = svc.model();
                m.cont = None;
                m.resume = None;
                let mut effects = m.clear_view();
                effects.push(Effect::CloseAll);
                effects
            };
            svc.apply(&effects);
        }
    }

    /// The vault locked: both windows close, and the offer, a link step, a continuation and a
    /// pending resume are dropped.
    pub fn program_fill_on_lock(&self) {
        let svc = &self.program_fill;
        let effects = {
            let mut m = svc.model();
            m.cont = None;
            m.resume = None;
            let mut effects = m.clear_view();
            effects.push(Effect::CloseAll);
            effects
        };
        svc.apply(&effects);
    }

    /// The vault unlocked: the watcher starts when it is on, and a click on the locked chip
    /// reopens the list for its field.
    pub fn program_fill_on_unlock(&self) {
        self.program_fill_apply_settings();
        let svc = &self.program_fill;
        let now = svc.now();
        let resume = {
            let mut m = svc.model();
            m.resume.take().filter(|r| r.until >= now)
        };
        let Some(resume) = resume else { return };
        if svc.host().is_none() {
            return;
        }
        // The unlock still holds the session: the vault is read on another thread.
        let weak = svc.weak_backend();
        let spawned = std::thread::Builder::new()
            .name("mosaic-program-fill-resume".to_owned())
            .spawn(move || {
                if let Some(backend) = weak.upgrade() {
                    if backend.program_fill.timers {
                        std::thread::sleep(Duration::from_millis(RESUME_SETTLE_MS));
                    }
                    backend.program_fill_resume(&resume);
                }
            });
        drop(spawned);
    }

    fn program_fill_resume(&self, resume: &Resume) {
        let svc = &self.program_fill;
        let Some(host) = svc.host() else { return };
        // The offer that was clicked must still be the current one: another offer since then
        // (another field, another program) cancels the resume, with no list at all.
        let current = svc
            .model()
            .offer
            .as_ref()
            .map(|o| (o.token, o.identity.clone()));
        let Some((token, identity)) = current else {
            return;
        };
        if token != resume.token || identity != resume.identity {
            return;
        }
        // Back into the program's field, then look at it again.
        let _ = host.refocus(token);
        if let Ok(HostEvent::Field(offer)) = host.capture_now()
            && offer.target == resume.target
            && offer.identity == resume.identity
        {
            self.program_fill_field(offer, true);
        }
    }

    /// What the host reported.
    pub fn program_fill_on_event(&self, event: HostEvent) {
        match event {
            HostEvent::Field(offer) => self.program_fill_field(offer, false),
            HostEvent::Gone | HostEvent::NoField { .. } | HostEvent::Refused { .. } => {
                self.program_fill_hide_chip();
            }
            HostEvent::Continue { token, near } => self.program_fill_second_step(token, near),
            HostEvent::ContinueOffer(offer) => self.program_fill_continue_offer(offer),
        }
    }

    /// The second step of a two-step login that may not be written without a pick (web content,
    /// or more than one password field in the window): the list opens on the first step's login,
    /// and one Enter or click fills. A continuation that is not live is an ordinary offer.
    fn program_fill_continue_offer(&self, offer: FieldOffer) {
        let svc = &self.program_fill;
        let now = svc.now();
        let preselect = {
            let mut m = svc.model();
            m.cont
                .as_mut()
                .filter(|c| {
                    !c.used
                        && now >= c.picked_at
                        && now - c.picked_at < CONTINUATION_TTL_MS
                        && c.identity == offer.identity
                })
                .map(|c| {
                    c.used = true;
                    (c.vault, c.item)
                })
        };
        match preselect {
            Some(login) => self.program_fill_field_with(offer, true, Some(login)),
            None => self.program_fill_field(offer, false),
        }
    }

    /// The program fill hotkey was pressed: look at the focused field now.
    pub fn program_fill_hotkey_pressed(&self) {
        let svc = &self.program_fill;
        if !svc.running.load(Ordering::SeqCst) {
            return;
        }
        let Some(host) = svc.host() else { return };
        match host.capture_now() {
            Ok(HostEvent::Field(offer) | HostEvent::ContinueOffer(offer)) => {
                self.program_fill_field(offer, true);
            }
            Ok(HostEvent::Refused { refusal, near }) => {
                let notice = if refusal == CaptureRefusal::Elevated {
                    PickerNotice::Elevated
                } else {
                    PickerNotice::NoField
                };
                self.program_fill_notice(notice, near);
            }
            Ok(HostEvent::NoField { near }) => {
                self.program_fill_notice(PickerNotice::NoField, near);
            }
            Ok(_) | Err(_) => {}
        }
    }

    /// Hides the chip (the list stays while it is in front).
    fn program_fill_hide_chip(&self) {
        let svc = &self.program_fill;
        let effects = {
            let mut m = svc.model();
            if m.mode != PickerMode::Chip {
                return;
            }
            m.clear_view()
        };
        svc.apply(&effects);
    }

    /// Shows a notice near `near`.
    pub(crate) fn program_fill_notice(&self, notice: PickerNotice, near: Option<Rect>) {
        let svc = &self.program_fill;
        let Some(near) = near else { return };
        let Some((monitor, scale)) = svc.windows().monitor_at(center(near)) else {
            return;
        };
        let Some(at) = list_rect(near, monitor, scale) else {
            return;
        };
        let (effects, generation) = {
            let mut m = svc.model();
            let mut effects = m.clear_view();
            m.mode = PickerMode::Notice;
            m.notice = Some(notice);
            m.list_at = Some(at);
            m.generation += 1;
            effects.push(Effect::ShowList(at, false));
            effects.push(Effect::Changed(PickerLabel::List, m.generation));
            (effects, m.generation)
        };
        svc.apply(&effects);
        let weak = svc.weak_backend();
        svc.after(NOTICE_MS, move || {
            if let Some(backend) = weak.upgrade() {
                backend.program_fill_notice_expired(generation);
            }
        });
    }

    pub(crate) fn program_fill_notice_expired(&self, generation: u64) {
        let svc = &self.program_fill;
        let effects = {
            let mut m = svc.model();
            if m.generation != generation || m.mode != PickerMode::Notice {
                return;
            }
            m.clear_view()
        };
        svc.apply(&effects);
    }

    /// A login field has focus. With `open_list` (the hotkey, or an unlock that was waited for)
    /// the list opens at once; else the chip shows.
    pub(crate) fn program_fill_field(&self, offer: FieldOffer, open_list: bool) {
        self.program_fill_field_with(offer, open_list, None);
    }

    /// [`Self::program_fill_field`] with a login the list opens on.
    fn program_fill_field_with(
        &self,
        offer: FieldOffer,
        open_list: bool,
        preselect: Option<(VaultId, ItemId)>,
    ) {
        let svc = &self.program_fill;
        if svc.model().fill_running {
            return;
        }
        // Browsers, script hosts, brokers and the like are never offered (the host refuses them
        // already; this is the second check).
        let blocked = offer.identity.class != ProgramClass::Ordinary
            || block_key(&offer.identity).is_some_and(|key| svc.blocklist().contains(&key));
        if blocked {
            self.program_fill_hide_chip();
            return;
        }
        let Ok(slot) = self.slot() else { return };
        let (locked, data) = match &*slot {
            Slot::Unlocked(session) => (false, offer::build(session, &offer.identity)),
            _ => (true, offer::Offer::default()),
        };
        drop(slot);
        let role_ok = match offer.role {
            FieldRole::Password { .. } | FieldRole::Username { .. } => true,
            // A lone text field is the first step of a two-step login: only for a linked program,
            // and only when it is the one edit of its window.
            FieldRole::Text => {
                !locked
                    && !data.matches.is_empty()
                    && svc
                        .host()
                        .is_some_and(|h| h.lone_text(offer.token) == Ok(true))
            }
        };
        if !role_ok {
            self.program_fill_hide_chip();
            return;
        }
        let Some((monitor, scale)) = svc.windows().monitor_at(center(offer.rect)) else {
            self.program_fill_hide_chip();
            return;
        };
        let now = svc.now();
        if open_list && locked {
            self.program_fill_locked_click(offer, now);
            return;
        }
        let place = if open_list {
            list_rect(offer.rect, monitor, scale)
        } else {
            chip_rect(offer.rect, monitor, scale)
        };
        let Some(at) = place else {
            self.program_fill_hide_chip();
            return;
        };
        let program = program_view(&offer.identity);
        let effects = {
            let mut m = svc.model();
            if m.fill_running {
                return;
            }
            let mut effects = m.clear_view();
            m.locked = locked;
            m.program = Some(program);
            m.data = data;
            m.offer = Some(offer);
            m.preselect = preselect;
            m.generation += 1;
            if open_list {
                m.mode = PickerMode::List;
                m.list_at = Some(at);
                m.list_shown_at = now;
                effects.push(Effect::ShowList(at, true));
                effects.push(Effect::Changed(PickerLabel::List, m.generation));
            } else {
                m.mode = PickerMode::Chip;
                m.chip_at = Some(at);
                m.chip_visible = true;
                effects.push(Effect::ShowChip(at));
                effects.push(Effect::Changed(PickerLabel::Chip, m.generation));
            }
            effects
        };
        svc.apply(&effects);
        if !open_list {
            self.program_fill_start_poller();
        }
    }

    /// A click on the locked chip, or the hotkey while locked: the unlock screen comes forward
    /// and unlocking reopens the list for this field.
    fn program_fill_locked_click(&self, offer: FieldOffer, now: u64) {
        let svc = &self.program_fill;
        if !matches!(
            offer.role,
            FieldRole::Password { .. } | FieldRole::Username { .. }
        ) {
            return;
        }
        let program = program_view(&offer.identity);
        let target = offer.target.clone();
        let effects = {
            let mut m = svc.model();
            let mut effects = m.clear_view();
            m.locked = true;
            m.program = Some(program);
            m.resume = Some(Resume {
                token: offer.token,
                identity: offer.identity.clone(),
                target,
                until: now + UNLOCK_RESUME_MS,
            });
            m.offer = Some(offer);
            effects.push(Effect::FocusMain);
            effects
        };
        svc.apply(&effects);
    }

    /// The chip was clicked.
    ///
    /// # Errors
    /// `pickerStale` (the chip changed since the click was made).
    pub fn picker_chip_click(&self, request: &PickerGenerationRequest) -> IpcResult<()> {
        let svc = &self.program_fill;
        let now = svc.now();
        let (offer, locked) = {
            let m = svc.model();
            if m.generation != request.generation || m.mode != PickerMode::Chip {
                return Err(stale());
            }
            (m.offer.clone().ok_or_else(stale)?, m.locked)
        };
        if locked {
            self.program_fill_locked_click(offer, now);
            return Ok(());
        }
        // The offer is read again: the vault may have changed since the chip appeared.
        let data = {
            let slot = self.slot()?;
            offer::build(slot.unlocked()?, &offer.identity)
        };
        let at = svc
            .windows()
            .monitor_at(center(offer.rect))
            .and_then(|(monitor, scale)| list_rect(offer.rect, monitor, scale));
        let Some(at) = at else {
            self.program_fill_hide_chip();
            return Ok(());
        };
        let effects = {
            let mut m = svc.model();
            if m.generation != request.generation || m.mode != PickerMode::Chip {
                return Err(stale());
            }
            m.data = data;
            m.mode = PickerMode::List;
            m.chip_visible = false;
            m.chip_at = None;
            m.list_at = Some(at);
            m.list_shown_at = now;
            m.generation += 1;
            vec![
                Effect::Hide(PickerLabel::Chip),
                Effect::ShowList(at, true),
                Effect::Changed(PickerLabel::List, m.generation),
            ]
        };
        svc.apply(&effects);
        Ok(())
    }

    /// The list window lost the focus: it closes, unless a fill runs.
    pub fn picker_list_blurred(&self) {
        let svc = &self.program_fill;
        let effects = {
            let mut m = svc.model();
            if m.fill_running || !matches!(m.mode, PickerMode::List | PickerMode::Notice) {
                return;
            }
            m.clear_view()
        };
        svc.apply(&effects);
    }

    /// What the picker windows show now. Never holds a secret.
    ///
    /// # Errors
    /// `internal`.
    pub fn picker_state(&self) -> IpcResult<PickerState> {
        let settings = self.settings_snapshot();
        let m = self.program_fill.model();
        Ok(PickerState {
            generation: m.generation,
            mode: m.mode,
            locked: m.locked,
            language: settings.language,
            theme: settings.theme,
            program: m.program.clone(),
            matches: m.data.matches.iter().map(|x| x.login.clone()).collect(),
            preselected: m
                .preselect
                .and_then(|(vault, item)| {
                    m.data.matches.iter().position(|x| {
                        x.login.vault_id.get() == vault && x.login.item_id.get() == item
                    })
                })
                .and_then(|index| u32::try_from(index).ok())
                .or_else(|| (m.data.matches.len() == 1).then_some(0)),
            suggestions: m.data.suggestions.clone(),
            link_again: m.data.link_again,
            link: m.link.as_ref().map(|l| l.view.clone()),
            notice: m.notice,
        })
    }

    /// Looks at the chip's field every [`RECT_POLL_MS`] while the chip shows.
    fn program_fill_start_poller(&self) {
        let svc = &self.program_fill;
        if !svc.timers || svc.poller_live.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = svc.weak_backend();
        let spawned = std::thread::Builder::new()
            .name("mosaic-program-fill-poll".to_owned())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(RECT_POLL_MS));
                    let Some(backend) = weak.upgrade() else {
                        return;
                    };
                    if !backend.program_fill_poll_step() {
                        backend
                            .program_fill
                            .poller_live
                            .store(false, Ordering::SeqCst);
                        return;
                    }
                }
            });
        if spawned.is_err() {
            svc.poller_live.store(false, Ordering::SeqCst);
        }
    }

    /// One look at the chip's field: a field that moved or is gone hides the chip; the chip comes
    /// back at the new place when two looks in a row agree. `false` once the chip is not on offer.
    pub(crate) fn program_fill_poll_step(&self) -> bool {
        let svc = &self.program_fill;
        let token = {
            let m = svc.model();
            match (m.mode == PickerMode::Chip)
                .then_some(m.offer.as_ref())
                .flatten()
            {
                Some(offer) => offer.token,
                None => return false,
            }
        };
        let seen = svc.host().and_then(|h| h.field_rect(token).ok()).flatten();
        let (action, effects) = {
            let mut m = svc.model();
            if m.mode != PickerMode::Chip || m.offer.as_ref().map(|o| o.token) != Some(token) {
                return false;
            }
            let known = m.offer.as_ref().map(|o| o.rect);
            let action = match seen {
                None => {
                    m.pending_rect = None;
                    PollAction::Hide
                }
                Some(rect) if Some(rect) == known && m.chip_visible => {
                    m.pending_rect = None;
                    PollAction::Nothing
                }
                Some(rect) if m.pending_rect == Some(rect) => {
                    m.pending_rect = None;
                    PollAction::Reposition(rect)
                }
                Some(rect) => {
                    m.pending_rect = Some(rect);
                    PollAction::Hide
                }
            };
            let effects = if matches!(action, PollAction::Hide) && m.chip_visible {
                m.chip_visible = false;
                vec![Effect::Hide(PickerLabel::Chip)]
            } else {
                Vec::new()
            };
            (action, effects)
        };
        svc.apply(&effects);
        if let PollAction::Reposition(rect) = action {
            let at = svc
                .windows()
                .monitor_at(center(rect))
                .and_then(|(monitor, scale)| chip_rect(rect, monitor, scale));
            let Some(at) = at else {
                self.program_fill_hide_chip();
                return false;
            };
            let shown = {
                let mut m = svc.model();
                let chip = m.mode == PickerMode::Chip;
                match m.offer.as_mut().filter(|o| o.token == token) {
                    Some(offer) if chip => {
                        offer.rect = rect;
                        m.chip_at = Some(at);
                        m.chip_visible = true;
                        m.generation += 1;
                        Some(m.generation)
                    }
                    _ => None,
                }
            };
            if let Some(generation) = shown {
                svc.apply(&[
                    Effect::ShowChip(at),
                    Effect::Changed(PickerLabel::Chip, generation),
                ]);
            }
        }
        true
    }
}

/// The hotkey's action: asks the backend to look at the focused field.
pub fn on_program_fill_hotkey<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager as _;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let backend = state.backend();
        backend.program_fill_hotkey_pressed();
    }
}

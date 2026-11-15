//! Program fill, the desktop side (ADR-0164): offers, picks, links and the two-step login, over a
//! fake host and fake picker windows and a really unlocked in-memory account.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mosaic_core::model::{ItemType, LoginUrl, SecretField, TextField};
use mosaic_core::{Argon2Profile, ItemId, VaultId};
use mosaic_origin::program::{ProgramClass, ProgramUrl};
use mosaic_program_fill::binding::{program_display, set_submit, submit_enabled};
use mosaic_program_fill::detect::FieldRole;
use mosaic_program_fill::fake::{FakeHost, HostCall};
use mosaic_program_fill::fill::{FillAbort, FillOutcome};
use mosaic_program_fill::host::{FieldOffer, Host, HostEvent};
use mosaic_program_fill::identity::{CaptureRefusal, CapturedIdentity, SignerDisplay};
use mosaic_program_fill::verify::{Point, ProbeError, Rect, RuntimeId, WriteKind};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use crate::contract::errors::{ErrorCode, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::ids::{WireItemId, WireVaultId};
use crate::contract::picker::{
    LinkAgainReason, LinkConfirmResult, PickerGenerationRequest, PickerItemRequest, PickerMode,
    PickerNotice, PickerSearchRequest,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, UnlockRequest};
use crate::keystore::MemoryKeystore;
use crate::program_fill::{
    BlockList, LINK_MIN_DELAY_MS, MAX_MATCHES, PICK_MIN_DELAY_MS, PickerLabel, PickerWindows,
    ProgramFillService, UNLOCK_RESUME_MS,
};
use crate::state::{AppState, Backend};

const PASSWORD: &str = "program fill master password";
const SLACK_PW: &str = "pw-SLACK-0x5f9e-secret";
const BANK_PW: &str = "pw-BANK-0xa1c3-secret";
const TOOL_PW: &str = "pw-TOOL-0x77d2-secret";
const H1: [u8; 32] = [0x41; 32];
const H2: [u8; 32] = [0x42; 32];
const SLACK_SIGNER: [u8; 32] = [0x11; 32];
const OTHER_SIGNER: [u8; 32] = [0x22; 32];

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// What a window was asked to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Win {
    ShowChip(Rect),
    ShowList(Rect, bool),
    Hide(PickerLabel),
    Changed(PickerLabel, u64),
    FocusMain,
}

struct FakeWindows {
    log: Mutex<Vec<Win>>,
    chip: Mutex<Option<Rect>>,
    list: Mutex<Option<Rect>>,
    foreground: AtomicBool,
}

impl FakeWindows {
    fn new() -> Self {
        Self {
            log: Mutex::new(Vec::new()),
            chip: Mutex::new(None),
            list: Mutex::new(None),
            foreground: AtomicBool::new(false),
        }
    }

    fn calls(&self) -> Vec<Win> {
        self.log.lock().unwrap().clone()
    }

    fn count(&self, wanted: impl Fn(&Win) -> bool) -> usize {
        self.calls().iter().filter(|w| wanted(w)).count()
    }

    fn chip_shown(&self) -> Option<Rect> {
        *self.chip.lock().unwrap()
    }

    fn list_shown(&self) -> Option<Rect> {
        *self.list.lock().unwrap()
    }

    fn move_list(&self, to: Rect) {
        *self.list.lock().unwrap() = Some(to);
    }
}

impl PickerWindows for FakeWindows {
    fn show_chip(&self, at: Rect) -> bool {
        self.log.lock().unwrap().push(Win::ShowChip(at));
        *self.chip.lock().unwrap() = Some(at);
        true
    }

    fn show_list(&self, at: Rect, focus: bool) -> bool {
        self.log.lock().unwrap().push(Win::ShowList(at, focus));
        *self.list.lock().unwrap() = Some(at);
        self.foreground.store(focus, Ordering::SeqCst);
        true
    }

    fn hide(&self, which: PickerLabel) {
        self.log.lock().unwrap().push(Win::Hide(which));
        match which {
            PickerLabel::Chip => *self.chip.lock().unwrap() = None,
            PickerLabel::List => {
                *self.list.lock().unwrap() = None;
                self.foreground.store(false, Ordering::SeqCst);
            }
        }
    }

    fn placed(&self, which: PickerLabel) -> Option<Rect> {
        match which {
            PickerLabel::Chip => self.chip_shown(),
            PickerLabel::List => self.list_shown(),
        }
    }

    fn hwnd(&self, which: PickerLabel) -> Option<isize> {
        Some(match which {
            PickerLabel::Chip => 0xC1,
            PickerLabel::List => 0xB1,
        })
    }

    fn is_foreground(&self, which: PickerLabel) -> bool {
        which == PickerLabel::List && self.foreground.load(Ordering::SeqCst)
    }

    fn monitor_at(&self, _p: Point) -> Option<(Rect, f64)> {
        Some((
            Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            1.0,
        ))
    }

    fn changed(&self, which: PickerLabel, generation: u64) {
        self.log
            .lock()
            .unwrap()
            .push(Win::Changed(which, generation));
    }

    fn focus_main(&self) {
        self.log.lock().unwrap().push(Win::FocusMain);
    }
}

fn signed(exe: &str, signer: [u8; 32], organization: &str) -> CapturedIdentity {
    CapturedIdentity {
        pid: 4242,
        final_path: format!(r"C:\Program Files\{exe}"),
        exe_name: exe.to_owned(),
        package_family: None,
        signer_id: Some(signer),
        signer_display: Some(SignerDisplay {
            common_name: None,
            organization: Some(organization.to_owned()),
        }),
        file_sha256: None,
        class: ProgramClass::Ordinary,
    }
}

fn unsigned(path: &str, hash: [u8; 32]) -> CapturedIdentity {
    CapturedIdentity {
        pid: 4343,
        final_path: path.to_owned(),
        exe_name: path.rsplit('\\').next().unwrap().to_lowercase(),
        package_family: None,
        signer_id: None,
        signer_display: None,
        file_sha256: Some(hash),
        class: ProgramClass::Ordinary,
    }
}

const TOOL_PATH: &str = r"C:\Users\a\AppData\Local\Tool\tool.exe";
const FIELD: Rect = Rect {
    left: 500,
    top: 300,
    right: 700,
    bottom: 330,
};

fn offer(token: u64, identity: CapturedIdentity, role: FieldRole) -> FieldOffer {
    FieldOffer {
        token,
        identity,
        role,
        rect: FIELD,
        target: RuntimeId(vec![42, 7]),
    }
}

fn password_field(token: u64, identity: CapturedIdentity) -> FieldOffer {
    offer(token, identity, FieldRole::Password { username: None })
}

fn signed_url(exe: &str, signer: [u8; 32]) -> LoginUrl {
    LoginUrl::new(&ProgramUrl::signed(exe, signer).unwrap().canonical())
}

// ---------------------------------------------------------------------------
// The environment
// ---------------------------------------------------------------------------

struct Env {
    state: AppState,
    host: Arc<FakeHost>,
    windows: Arc<FakeWindows>,
    clock: Arc<AtomicU64>,
    secret_key: String,
    vault: VaultId,
    _kdf: std::sync::MutexGuard<'static, ()>,
}

struct Items {
    slack: ItemId,
    bank: ItemId,
    tool: ItemId,
}

impl Env {
    fn new() -> Self {
        Self::with(|service| service)
    }

    fn with(tune: impl FnOnce(ProgramFillService) -> ProgramFillService) -> Self {
        let kdf = crate::test_support::kdf_permit();
        let host = Arc::new(FakeHost::default());
        let windows = Arc::new(FakeWindows::new());
        let clock = Arc::new(AtomicU64::new(1_000_000));
        let service = tune(
            ProgramFillService::new(
                Some(Arc::clone(&host) as Arc<dyn Host>),
                Arc::clone(&windows) as Arc<dyn PickerWindows>,
                BlockList::memory(),
                {
                    let clock = Arc::clone(&clock);
                    Arc::new(move || clock.load(Ordering::SeqCst))
                },
            )
            .without_timers(),
        );
        let backend = Backend::new(
            crate::state::Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_program_fill(service);
        let state = AppState::new(backend);
        let created = state
            .backend()
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        let secret_key = serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
            .as_str()
            .unwrap()
            .to_owned();
        // The hooks of `run`.
        let weak = state.downgrade();
        state.backend().on_unlock(Box::new(move || {
            if let Some(b) = weak.upgrade() {
                b.program_fill_on_unlock();
            }
        }));
        let weak = state.downgrade();
        state.backend().on_lock(Box::new(move |_| {
            if let Some(b) = weak.upgrade() {
                b.program_fill_on_lock();
            }
        }));
        let vault = state.backend().slot().unwrap().unlocked().unwrap().vaults()[0].vault_id;
        Self {
            state,
            host,
            windows,
            clock,
            secret_key,
            vault,
            _kdf: kdf,
        }
    }

    fn b(&self) -> &Backend {
        self.state.backend()
    }

    fn advance(&self, ms: u64) {
        self.clock.fetch_add(ms, Ordering::SeqCst);
    }

    fn now(&self) -> u64 {
        self.clock.load(Ordering::SeqCst)
    }

    /// Turns program fill on (as a verified file would) and starts the fake host.
    fn start(&self) {
        self.b()
            .update_stored(|s| s.settings.program_fill = true)
            .unwrap();
        self.b().program_fill_apply_settings();
    }

    fn add_login(&self, title: &str, user: &str, password: &str, urls: &[LoginUrl]) -> ItemId {
        let mut slot = self.b().slot().unwrap();
        slot.unlocked_mut()
            .unwrap()
            .add_item(self.vault, &ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_text(TextField::Username, user)?;
                e.set_secret(SecretField::Password, password)?;
                for url in urls {
                    e.add_url(url)?;
                }
                Ok(())
            })
            .unwrap()
    }

    /// "Slack" (linked to `slack.exe`), "Bank" (a web login, not linked) and "Tool" (linked to
    /// an unsigned file with hash `H1`, Enter after the fill off).
    fn seed(&self) -> Items {
        let slack = self.add_login(
            "Slack",
            "slack-user",
            SLACK_PW,
            &[signed_url("slack.exe", SLACK_SIGNER)],
        );
        let bank = self.add_login(
            "Bank",
            "bank-user",
            BANK_PW,
            &[LoginUrl::new("https://bank.example")],
        );
        let mut tool_url = LoginUrl::new(&ProgramUrl::path(TOOL_PATH, H1).unwrap().canonical());
        set_submit(&mut tool_url, false);
        let tool = self.add_login("Tool", "tool-user", TOOL_PW, &[tool_url]);
        Items { slack, bank, tool }
    }

    fn slack_identity() -> CapturedIdentity {
        signed("slack.exe", SLACK_SIGNER, "Slack Technologies")
    }

    fn state_now(&self) -> crate::contract::picker::PickerState {
        self.b().picker_state().unwrap()
    }

    fn generation(&self) -> u64 {
        self.state_now().generation
    }

    fn click_chip(&self) -> IpcResult<()> {
        self.b().picker_chip_click(&PickerGenerationRequest {
            generation: self.generation(),
        })
    }

    /// A field with focus, the chip click, and the 300 ms guard time.
    fn open_list(&self, offer: FieldOffer) {
        self.host.emit(HostEvent::Field(offer));
        assert_eq!(self.state_now().mode, PickerMode::Chip, "chip first");
        self.click_chip().unwrap();
        assert_eq!(self.state_now().mode, PickerMode::List);
        self.advance(PICK_MIN_DELAY_MS);
    }

    fn item_request(&self, item: ItemId) -> PickerItemRequest {
        self.item_request_at(self.generation(), item)
    }

    fn item_request_at(&self, generation: u64, item: ItemId) -> PickerItemRequest {
        PickerItemRequest {
            generation,
            vault_id: WireVaultId::from(self.vault),
            item_id: WireItemId::from(item),
        }
    }

    fn fills(&self) -> Vec<HostCall> {
        self.host
            .calls()
            .into_iter()
            .filter(|c| matches!(c, HostCall::Fill { .. }))
            .collect()
    }

    fn urls_of(&self, item: ItemId) -> Vec<LoginUrl> {
        let slot = self.b().slot().unwrap();
        slot.unlocked()
            .unwrap()
            .item(self.vault, item)
            .unwrap()
            .urls()
            .into_iter()
            .map(|(_, u)| u)
            .collect()
    }
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "timed out: {what}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn no_fill_without_a_pick() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    for i in 0..100 {
        e.host
            .emit(HostEvent::Field(password_field(i, Env::slack_identity())));
        e.host.emit(HostEvent::Continue {
            token: i + 1000,
            near: FIELD,
        });
        e.host.emit(HostEvent::Gone);
    }
    e.host
        .emit(HostEvent::Field(password_field(500, Env::slack_identity())));
    e.click_chip().unwrap();
    let state = e.state_now();
    e.b()
        .picker_search(&PickerSearchRequest {
            generation: state.generation,
            query: "sla".into(),
        })
        .unwrap();
    let _ = e.b().picker_link_begin(&e.item_request(items.bank));
    assert!(
        e.fills().is_empty(),
        "no fill without a pick: {:?}",
        e.host.calls()
    );
}

fn record(seen: &mut Vec<String>, e: &Env) {
    seen.push(serde_json::to_string(&e.state_now()).unwrap());
}

#[test]
fn picker_webview_never_receives_a_secret() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    let mut seen: Vec<String> = Vec::new();
    e.host
        .emit(HostEvent::Field(password_field(1, Env::slack_identity())));
    record(&mut seen, &e);
    e.click_chip().unwrap();
    record(&mut seen, &e);
    let found = e
        .b()
        .picker_search(&PickerSearchRequest {
            generation: e.generation(),
            query: "bank".into(),
        })
        .unwrap();
    seen.push(serde_json::to_string(&found).unwrap());
    let step = e.b().picker_link_begin(&e.item_request(items.bank));
    seen.push(format!("{step:?}"));
    record(&mut seen, &e);
    e.advance(LINK_MIN_DELAY_MS);
    let result = e.b().picker_link_confirm(&PickerGenerationRequest {
        generation: e.generation(),
    });
    seen.push(format!("{result:?}"));
    // A notice.
    e.host
        .emit(HostEvent::Field(password_field(2, Env::slack_identity())));
    *e.host.capture.lock().unwrap() = Ok(HostEvent::Refused {
        refusal: CaptureRefusal::Elevated,
        near: Some(FIELD),
    });
    e.b().program_fill_hotkey_pressed();
    record(&mut seen, &e);
    for call in e.windows.calls() {
        seen.push(format!("{call:?}"));
    }
    let all = seen.join("\n");
    for secret in [SLACK_PW, BANK_PW, TOOL_PW, "pw-"] {
        assert!(!all.contains(secret), "{secret} reached a picker");
    }
    // The shapes that matter are there.
    assert!(all.contains("Slack") && all.contains("slack-user"));
}

#[test]
fn a_bound_program_gets_its_login_preselected() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.host
        .emit(HostEvent::Field(password_field(1, Env::slack_identity())));
    assert!(e.windows.chip_shown().is_some());
    e.click_chip().unwrap();
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::List);
    assert_eq!(state.matches.len(), 1);
    assert_eq!(state.matches[0].item_id.get(), items.slack);
    assert_eq!(state.matches[0].title, "Slack");
    assert_eq!(state.preselected, Some(0));
    assert_eq!(state.program.as_ref().unwrap().name, "slack.exe");
    assert!(
        e.windows.chip_shown().is_none(),
        "the chip gave way to the list"
    );
}

#[test]
fn a_pick_fills_with_the_login_and_the_submit_settings() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.open_list(password_field(1, Env::slack_identity()));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    assert_eq!(
        e.fills(),
        vec![HostCall::Fill {
            token: 1,
            username: Some("slack-user".into()),
            has_password: true,
            submit: true,
            continuation: false,
        }]
    );
    assert!(e.windows.list_shown().is_none() && e.windows.chip_shown().is_none());

    // The setting off: no Enter.
    e.b()
        .update_stored(|s| s.settings.program_fill_submit = false)
        .unwrap();
    e.open_list(password_field(2, Env::slack_identity()));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    assert!(matches!(
        e.fills().last(),
        Some(HostCall::Fill { submit: false, .. })
    ));

    // The binding's own switch off: no Enter either.
    e.b()
        .update_stored(|s| s.settings.program_fill_submit = true)
        .unwrap();
    e.open_list(password_field(3, unsigned(TOOL_PATH, H1)));
    e.b().picker_choose(&e.item_request(items.tool)).unwrap();
    assert!(matches!(
        e.fills().last(),
        Some(HostCall::Fill {
            token: 3,
            submit: false,
            ..
        })
    ));
}

#[test]
fn picks_too_soon_stale_or_outside_the_offer_are_refused() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.host
        .emit(HostEvent::Field(password_field(1, Env::slack_identity())));
    e.click_chip().unwrap();
    // The list has just appeared.
    e.advance(PICK_MIN_DELAY_MS - 1);
    let early = e
        .b()
        .picker_choose(&e.item_request(items.slack))
        .unwrap_err();
    assert_eq!(early.code, ErrorCode::PickerTooSoon);
    e.advance(1);
    // Another generation.
    let generation = e.generation();
    let old = e
        .b()
        .picker_choose(&e.item_request_at(generation - 1, items.slack))
        .unwrap_err();
    assert_eq!(old.code, ErrorCode::PickerStale);
    // Not one of the logins on offer.
    let outside = e
        .b()
        .picker_choose(&e.item_request(items.bank))
        .unwrap_err();
    assert_eq!(outside.code, ErrorCode::InvalidInput);
    // A chip click with an old generation.
    assert_eq!(e.fills().len(), 0);
    let request = e.item_request(items.slack);
    e.b().picker_choose(&request).unwrap();
    assert_eq!(e.fills().len(), 1);
    // A second Enter for the same list fills nothing more.
    let again = e.b().picker_choose(&request).unwrap_err();
    assert_eq!(again.code, ErrorCode::PickerStale);
    assert_eq!(e.fills().len(), 1);
}

#[test]
fn a_moved_or_covered_list_refuses_picks_and_links() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.open_list(password_field(1, Env::slack_identity()));
    let at = e.windows.list_shown().unwrap();

    // Moved.
    e.windows.move_list(Rect {
        left: at.left + 5,
        ..at
    });
    let moved = e
        .b()
        .picker_choose(&e.item_request(items.slack))
        .unwrap_err();
    assert_eq!(moved.code, ErrorCode::PickerStale);
    e.windows.move_list(at);

    // Not in front.
    e.windows.foreground.store(false, Ordering::SeqCst);
    let behind = e
        .b()
        .picker_choose(&e.item_request(items.slack))
        .unwrap_err();
    assert_eq!(behind.code, ErrorCode::PickerStale);
    e.windows.foreground.store(true, Ordering::SeqCst);

    // Covered (the five probe points do not all resolve to the list).
    *e.host.clear.lock().unwrap() = Ok(false);
    let covered = e
        .b()
        .picker_choose(&e.item_request(items.slack))
        .unwrap_err();
    assert_eq!(covered.code, ErrorCode::PickerStale);
    // The same for a link.
    e.b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap();
    e.advance(LINK_MIN_DELAY_MS);
    let link = e
        .b()
        .picker_link_confirm(&PickerGenerationRequest {
            generation: e.generation(),
        })
        .unwrap_err();
    assert_eq!(link.code, ErrorCode::PickerStale);
    assert_eq!(e.fills().len(), 0);
    // A probe that fails closes it too.
    *e.host.clear.lock().unwrap() = Err(ProbeError::Timeout);
    assert_eq!(
        e.b()
            .picker_choose(&e.item_request(items.slack))
            .unwrap_err()
            .code,
        ErrorCode::PickerStale
    );
}

#[test]
fn elevated_target_gets_no_chip_and_no_fill() {
    let e = Env::new();
    e.seed();
    e.start();
    e.host.emit(HostEvent::Refused {
        refusal: CaptureRefusal::Elevated,
        near: Some(FIELD),
    });
    assert!(e.windows.chip_shown().is_none());
    assert_eq!(e.state_now().mode, PickerMode::Hidden);
    // The hotkey says why.
    *e.host.capture.lock().unwrap() = Ok(HostEvent::Refused {
        refusal: CaptureRefusal::Elevated,
        near: Some(FIELD),
    });
    e.b().program_fill_hotkey_pressed();
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::Notice);
    assert_eq!(state.notice, Some(PickerNotice::Elevated));
    assert_eq!(e.windows.count(|w| matches!(w, Win::ShowList(_, false))), 1);
    // A field that is no field says so, too.
    *e.host.capture.lock().unwrap() = Ok(HostEvent::NoField { near: Some(FIELD) });
    e.b().program_fill_hotkey_pressed();
    assert_eq!(e.state_now().notice, Some(PickerNotice::NoField));
    assert_eq!(e.fills().len(), 0);
}

#[test]
fn blocked_program_gets_no_chip() {
    let e = Env::new();
    e.seed();
    e.start();
    let tool = unsigned(TOOL_PATH, H1);
    e.host
        .emit(HostEvent::Field(password_field(1, tool.clone())));
    e.click_chip().unwrap();
    e.b()
        .picker_block(&PickerGenerationRequest {
            generation: e.generation(),
        })
        .unwrap();
    assert_eq!(e.state_now().mode, PickerMode::Hidden);
    assert_eq!(e.b().program_fill_status().blocked.len(), 1);
    // The same program: nothing.
    e.host.emit(HostEvent::Field(password_field(2, tool)));
    assert!(e.windows.chip_shown().is_none());
    // Another file at the same path (an update): still blocked.
    e.host
        .emit(HostEvent::Field(password_field(3, unsigned(TOOL_PATH, H2))));
    assert!(e.windows.chip_shown().is_none());
    // Offered again after "Offer again".
    let key = e.b().program_fill_status().blocked[0].key.clone();
    e.b()
        .program_fill_unblock(&crate::contract::program_fill::ProgramFillUnblockRequest { key })
        .unwrap();
    e.host
        .emit(HostEvent::Field(password_field(4, unsigned(TOOL_PATH, H2))));
    assert!(e.windows.chip_shown().is_some());
}

#[test]
fn a_lone_text_field_is_offered_only_for_a_linked_program_with_one_edit() {
    let e = Env::new();
    e.seed();
    e.start();
    // No link: nothing.
    let other = signed("other.exe", OTHER_SIGNER, "Other");
    *e.host.lone.lock().unwrap() = Ok(true);
    e.host
        .emit(HostEvent::Field(offer(1, other, FieldRole::Text)));
    assert!(e.windows.chip_shown().is_none());
    // Linked, but not the only edit of its window.
    *e.host.lone.lock().unwrap() = Ok(false);
    e.host.emit(HostEvent::Field(offer(
        2,
        Env::slack_identity(),
        FieldRole::Text,
    )));
    assert!(e.windows.chip_shown().is_none());
    // Linked and alone.
    *e.host.lone.lock().unwrap() = Ok(true);
    e.host.emit(HostEvent::Field(offer(
        3,
        Env::slack_identity(),
        FieldRole::Text,
    )));
    assert!(e.windows.chip_shown().is_some());
}

#[test]
fn two_step_login_continues_once_in_the_same_session() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    *e.host.lone.lock().unwrap() = Ok(true);
    e.open_list(offer(1, Env::slack_identity(), FieldRole::Text));
    let picked_at = e.now();
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    // The user name only; the password is not even handed over.
    assert_eq!(
        e.fills(),
        vec![HostCall::Fill {
            token: 1,
            username: Some("slack-user".into()),
            has_password: false,
            submit: true,
            continuation: false,
        }]
    );
    let armed = e
        .host
        .calls()
        .into_iter()
        .find_map(|c| match c {
            HostCall::ArmContinuation(token, c) => Some((token, c)),
            _ => None,
        })
        .expect("armed");
    assert_eq!(armed.0, 1);
    assert_eq!(armed.1.picked_at_ms, picked_at);

    // The password field of the second step.
    e.advance(5_000);
    e.host.emit(HostEvent::Continue {
        token: 2,
        near: FIELD,
    });
    let fills = e.fills();
    assert_eq!(fills.len(), 2);
    assert_eq!(
        fills[1],
        HostCall::Fill {
            token: 2,
            username: None,
            has_password: true,
            submit: true,
            continuation: true,
        }
    );
    // Once.
    e.host.emit(HostEvent::Continue {
        token: 3,
        near: FIELD,
    });
    assert_eq!(e.fills().len(), 2);

    // A second login, and a minute later: nothing.
    e.open_list(offer(10, Env::slack_identity(), FieldRole::Text));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    assert_eq!(e.fills().len(), 3);
    e.advance(61_000);
    e.host.emit(HostEvent::Continue {
        token: 11,
        near: FIELD,
    });
    assert_eq!(e.fills().len(), 3);
    // Nothing armed at all: nothing.
    e.host.emit(HostEvent::Continue {
        token: 12,
        near: FIELD,
    });
    assert_eq!(e.fills().len(), 3);
}

#[test]
fn a_second_step_that_may_not_continue_alone_offers_the_list_with_the_login_preselected() {
    let e = Env::new();
    let items = e.seed();
    // A second login of the same program, listed before Slack by its title.
    e.add_login(
        "Aaa other",
        "other-user",
        "pw-OTHER",
        &[signed_url("slack.exe", SLACK_SIGNER)],
    );
    e.start();
    *e.host.lone.lock().unwrap() = Ok(true);
    e.open_list(offer(1, Env::slack_identity(), FieldRole::Text));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    assert_eq!(e.fills().len(), 1);

    // The password field of the second step is web content or one of several: no write.
    e.advance(5_000);
    e.host.emit(HostEvent::ContinueOffer(password_field(
        2,
        Env::slack_identity(),
    )));
    assert_eq!(e.fills().len(), 1, "nothing is written without a pick");
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::List);
    let slack_row = state
        .matches
        .iter()
        .position(|m| m.item_id.get() == items.slack)
        .unwrap();
    assert!(state.matches.len() >= 2);
    assert_eq!(state.preselected, u32::try_from(slack_row).ok());

    // One pick fills the password, as an ordinary fill.
    e.advance(PICK_MIN_DELAY_MS);
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    let fills = e.fills();
    assert_eq!(fills.len(), 2);
    assert_eq!(
        fills[1],
        HostCall::Fill {
            token: 2,
            username: Some("slack-user".into()),
            has_password: true,
            submit: true,
            continuation: false,
        }
    );
    // The continuation is spent: a second such event is an ordinary chip.
    e.advance(PICK_MIN_DELAY_MS);
    e.host.emit(HostEvent::ContinueOffer(password_field(
        3,
        Env::slack_identity(),
    )));
    assert_eq!(e.state_now().mode, PickerMode::Chip);
    assert_eq!(e.fills().len(), 2);
}

#[test]
fn a_continue_offer_of_another_program_or_after_a_minute_is_an_ordinary_offer() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    *e.host.lone.lock().unwrap() = Ok(true);
    e.open_list(offer(1, Env::slack_identity(), FieldRole::Text));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    // Another program: a chip, not a list.
    e.host.emit(HostEvent::ContinueOffer(password_field(
        2,
        signed("slack.exe", OTHER_SIGNER, "Elsewhere"),
    )));
    assert_ne!(e.state_now().mode, PickerMode::List);
    // The same program a minute later: a chip.
    e.advance(61_000);
    e.host.emit(HostEvent::ContinueOffer(password_field(
        3,
        Env::slack_identity(),
    )));
    assert_eq!(e.state_now().mode, PickerMode::Chip);
    assert_eq!(e.fills().len(), 1);
}

#[test]
fn linking_a_signed_program_needs_one_guarded_confirmation() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    let bank = signed("bank.exe", OTHER_SIGNER, "Bank AG");
    e.open_list(password_field(1, bank));
    assert_eq!(e.state_now().matches.len(), 0);
    let step = e
        .b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap();
    assert_eq!(step.confirmations, 1);
    assert_eq!(step.program.name, "bank.exe");
    assert!(step.submit);
    let generation = e.generation();
    let confirm = || {
        e.b()
            .picker_link_confirm(&PickerGenerationRequest { generation })
    };
    e.advance(LINK_MIN_DELAY_MS - 1);
    assert_eq!(confirm().unwrap_err().code, ErrorCode::PickerTooSoon);
    assert_eq!(e.fills().len(), 0);
    e.advance(1);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::Linked);
    // Bank now holds the binding next to its web address, with the display data.
    let urls = e.urls_of(items.bank);
    assert_eq!(urls.len(), 2);
    let binding = urls
        .iter()
        .find(|u| u.url.as_str().starts_with("winapp://signed/"))
        .unwrap();
    assert_eq!(program_display(binding).unwrap().name, "bank.exe");
    assert_eq!(
        program_display(binding).unwrap().signer.as_deref(),
        Some("Bank AG")
    );
    assert!(submit_enabled(binding));
    // And the login was filled.
    assert_eq!(
        e.fills(),
        vec![HostCall::Fill {
            token: 1,
            username: Some("bank-user".into()),
            has_password: true,
            submit: true,
            continuation: false,
        }]
    );
}

#[test]
fn linking_an_unsigned_program_needs_two_confirmations() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.open_list(password_field(
        1,
        unsigned(r"C:\Users\a\Downloads\newtool.exe", H1),
    ));
    let step = e
        .b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap();
    assert_eq!(step.confirmations, 2);
    assert_eq!(step.confirmed, 0);
    assert_eq!(
        step.program.path.as_deref(),
        Some(r"C:\Users\a\Downloads\newtool.exe")
    );
    // The window re-reads the state after each step, so every call carries the current generation.
    let confirm = || {
        e.b().picker_link_confirm(&PickerGenerationRequest {
            generation: e.generation(),
        })
    };
    e.advance(LINK_MIN_DELAY_MS);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::NeedAnother);
    assert_eq!(e.state_now().link.unwrap().confirmed, 1);
    e.advance(500);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::TooSoon);
    assert_eq!(e.fills().len(), 0);
    e.advance(500);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::Linked);
    assert_eq!(e.fills().len(), 1);
    assert!(
        e.urls_of(items.bank)
            .iter()
            .any(|u| u.url.as_str().starts_with("winapp://path/"))
    );
}

#[test]
fn link_again_replaces_the_changed_binding() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    // The linked file changed: same path, hash H2.
    e.open_list(password_field(1, unsigned(TOOL_PATH, H2)));
    let state = e.state_now();
    assert_eq!(state.matches.len(), 0);
    assert_eq!(state.link_again, Some(LinkAgainReason::FileChanged));
    let step = e
        .b()
        .picker_link_begin(&e.item_request(items.tool))
        .unwrap();
    let change = step.hash_change.expect("hash change");
    assert_eq!(change.old_prefix, "4141414141414141");
    assert_eq!(change.new_prefix, "4242424242424242");
    assert_eq!(step.confirmations, 2);
    let confirm = || {
        e.b().picker_link_confirm(&PickerGenerationRequest {
            generation: e.generation(),
        })
    };
    e.advance(LINK_MIN_DELAY_MS);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::NeedAnother);
    e.advance(LINK_MIN_DELAY_MS);
    assert_eq!(confirm().unwrap(), LinkConfirmResult::Linked);
    // One binding, the new one.
    let urls = e.urls_of(items.tool);
    assert_eq!(urls.len(), 1, "no dead binding next to the new one");
    assert_eq!(
        ProgramUrl::parse(urls[0].url.as_str()).unwrap(),
        ProgramUrl::path(TOOL_PATH, H2).unwrap()
    );
}

#[test]
fn the_link_flow_cannot_bind_excluded_programs() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    let mut browser = signed("chrome.exe", OTHER_SIGNER, "Google");
    browser.class = ProgramClass::Browser;
    // No chip for it.
    e.host
        .emit(HostEvent::Field(password_field(1, browser.clone())));
    assert!(e.windows.chip_shown().is_none());
    // A forged request in a list that is open for another program.
    e.open_list(password_field(2, Env::slack_identity()));
    e.b().program_fill.model().offer.as_mut().unwrap().identity = browser;
    let error = e
        .b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    // A link to a trashed or missing login is refused as well.
    e.b()
        .slot()
        .unwrap()
        .unlocked_mut()
        .unwrap()
        .trash_item(e.vault, items.bank)
        .unwrap();
    e.b().program_fill.model().offer.as_mut().unwrap().identity = Env::slack_identity();
    let trashed = e
        .b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap_err();
    assert_eq!(trashed.code, ErrorCode::InvalidInput);
}

#[test]
fn lock_ends_offers_links_and_continuations() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    *e.host.lone.lock().unwrap() = Ok(true);
    // A continuation in flight.
    e.open_list(offer(1, Env::slack_identity(), FieldRole::Text));
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    // A link step open.
    e.open_list(password_field(
        2,
        signed("bank.exe", OTHER_SIGNER, "Bank AG"),
    ));
    e.b()
        .picker_link_begin(&e.item_request(items.bank))
        .unwrap();
    let generation = e.generation();
    e.advance(LINK_MIN_DELAY_MS);

    e.b().lock(LockReason::Manual).unwrap();
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::Hidden);
    assert!(state.link.is_none() && state.matches.is_empty());
    assert!(e.windows.list_shown().is_none() && e.windows.chip_shown().is_none());
    let fills = e.fills().len();
    let refused = e
        .b()
        .picker_link_confirm(&PickerGenerationRequest { generation })
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::PickerStale);
    e.host.emit(HostEvent::Continue {
        token: 5,
        near: FIELD,
    });
    assert_eq!(e.fills().len(), fills, "no continuation after a lock");
}

#[test]
fn a_locked_chip_click_unlocks_then_reopens_only_the_same_field() {
    let e = Env::new();
    e.seed();
    e.start();
    e.b().lock(LockReason::Manual).unwrap();
    let field = password_field(1, Env::slack_identity());
    e.host.emit(HostEvent::Field(field.clone()));
    let locked = e.state_now();
    assert_eq!(locked.mode, PickerMode::Chip);
    assert!(locked.locked);
    assert!(locked.matches.is_empty(), "a locked chip lists nothing");
    e.click_chip().unwrap();
    assert_eq!(e.windows.count(|w| *w == Win::FocusMain), 1);

    // Unlocking: the same field is looked at again and the list opens.
    *e.host.capture.lock().unwrap() = Ok(HostEvent::Field(field.clone()));
    e.b()
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&e.secret_key)),
        })
        .unwrap();
    wait_for("the list after the unlock", || {
        e.windows.list_shown().is_some()
    });
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::List);
    assert_eq!(state.matches.len(), 1);
    assert!(e.host.calls().contains(&HostCall::Refocus(1)));

    // Another field after the unlock: nothing opens.
    e.b().lock(LockReason::Manual).unwrap();
    e.host.emit(HostEvent::Field(field.clone()));
    e.click_chip().unwrap();
    let mut other = field.clone();
    other.target = RuntimeId(vec![9, 9]);
    *e.host.capture.lock().unwrap() = Ok(HostEvent::Field(other));
    let shown = e.windows.count(|w| matches!(w, Win::ShowList(..)));
    e.b()
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&e.secret_key)),
        })
        .unwrap();
    wait_for("the capture after the unlock", || {
        e.host
            .calls()
            .iter()
            .filter(|c| **c == HostCall::CaptureNow)
            .count()
            >= 2
    });
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(e.windows.count(|w| matches!(w, Win::ShowList(..))), shown);

    // A minute later: nothing is looked at.
    e.b().lock(LockReason::Manual).unwrap();
    e.host.emit(HostEvent::Field(field.clone()));
    e.click_chip().unwrap();
    e.advance(UNLOCK_RESUME_MS + 1);
    let captures = e
        .host
        .calls()
        .iter()
        .filter(|c| **c == HostCall::CaptureNow)
        .count();
    e.b()
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&e.secret_key)),
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        e.host
            .calls()
            .iter()
            .filter(|c| **c == HostCall::CaptureNow)
            .count(),
        captures
    );
}

#[test]
fn an_unlock_resume_is_dropped_when_another_offer_came_in_meanwhile() {
    let e = Env::new();
    e.seed();
    e.start();
    e.b().lock(LockReason::Manual).unwrap();
    let field = password_field(1, Env::slack_identity());
    e.host.emit(HostEvent::Field(field.clone()));
    e.click_chip().unwrap();
    // Another offer (another program, another token) replaces the clicked one while locked.
    e.host.emit(HostEvent::Field(offer(
        2,
        signed("bank.exe", OTHER_SIGNER, "Bank AG"),
        FieldRole::Password { username: None },
    )));
    // Even a capture that matches the old field by its runtime ID gets no list.
    *e.host.capture.lock().unwrap() = Ok(HostEvent::Field(field));
    let shown = e.windows.count(|w| matches!(w, Win::ShowList(..)));
    e.b()
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&e.secret_key)),
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(e.windows.count(|w| matches!(w, Win::ShowList(..))), shown);
    let calls = e.host.calls();
    assert!(!calls.contains(&HostCall::CaptureNow));
    assert!(!calls.contains(&HostCall::Refocus(2)) && !calls.contains(&HostCall::Refocus(1)));
}

#[test]
fn offer_caps_matches_and_handles_many_bindings() {
    let e = Env::new();
    for i in 0..400 {
        let url = if i < 120 {
            signed_url("many.exe", SLACK_SIGNER)
        } else {
            LoginUrl::new(&format!("https://site{i}.example"))
        };
        e.add_login(&format!("Login {i:04}"), "u", "p", &[url]);
    }
    e.start();
    let identity = signed("many.exe", SLACK_SIGNER, "Many");
    e.host.emit(HostEvent::Field(password_field(1, identity)));
    e.click_chip().unwrap();
    let state = e.state_now();
    assert_eq!(state.matches.len(), MAX_MATCHES);
    assert_eq!(state.preselected, None);
    let titles: Vec<&str> = state.matches.iter().map(|m| m.title.as_str()).collect();
    let mut sorted = titles.clone();
    sorted.sort_unstable();
    assert_eq!(titles, sorted, "sorted by title");
    assert_eq!(titles[0], "Login 0000");
}

#[test]
fn an_elevated_mosaic_does_not_watch_programs() {
    let e = Env::with(|s| s.with_elevation_check(Arc::new(|| true)));
    e.seed();
    e.start();
    let status = e.b().program_fill_status();
    assert!(!status.available && !status.running);
    assert_eq!(
        status.reason,
        Some(crate::contract::program_fill::ProgramFillUnavailable::MosaicElevated)
    );
    assert!(!e.host.calls().contains(&HostCall::Start));
}

#[test]
fn a_host_timeout_shows_not_responding_and_writes_nothing_more() {
    let e = Env::new();
    let items = e.seed();
    e.start();
    e.open_list(password_field(1, Env::slack_identity()));
    *e.host.fill_result.lock().unwrap() = Err(ProbeError::Timeout);
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    let state = e.state_now();
    assert_eq!(state.mode, PickerMode::Notice);
    assert_eq!(state.notice, Some(PickerNotice::NotResponding));
    assert_eq!(e.fills().len(), 1, "one attempt, nothing more");

    // The other endings.
    let cases = [
        (
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::ModifiersHeld,
                cleared: false,
            },
            PickerNotice::ModifiersHeld,
        ),
        (
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::InputBlocked,
                cleared: true,
            },
            PickerNotice::AbortedCleared,
        ),
        (
            FillOutcome::Aborted {
                at: WriteKind::Username,
                reason: FillAbort::ReadBackMismatch,
                cleared: false,
            },
            PickerNotice::Aborted,
        ),
    ];
    for (outcome, notice) in cases {
        e.open_list(password_field(2, Env::slack_identity()));
        *e.host.fill_result.lock().unwrap() = Ok(outcome);
        e.b().picker_choose(&e.item_request(items.slack)).unwrap();
        assert_eq!(e.state_now().notice, Some(notice));
    }
    // Nothing to fill: nothing shown.
    e.open_list(password_field(3, Env::slack_identity()));
    *e.host.fill_result.lock().unwrap() = Ok(FillOutcome::NothingToFill);
    e.b().picker_choose(&e.item_request(items.slack)).unwrap();
    assert_eq!(e.state_now().mode, PickerMode::Hidden);
}

#[test]
fn a_field_that_moves_hides_the_chip_until_two_looks_agree() {
    let e = Env::new();
    e.seed();
    e.start();
    e.host
        .emit(HostEvent::Field(password_field(1, Env::slack_identity())));
    let first = e.windows.chip_shown().unwrap();
    // Unchanged: nothing happens.
    *e.host.rect.lock().unwrap() = Ok(Some(FIELD));
    assert!(e.b().program_fill_poll_step());
    assert_eq!(e.windows.chip_shown(), Some(first));
    // Moved: hidden at the first look, back at the second.
    let moved = Rect {
        left: 800,
        top: 500,
        right: 1000,
        bottom: 530,
    };
    *e.host.rect.lock().unwrap() = Ok(Some(moved));
    assert!(e.b().program_fill_poll_step());
    assert!(e.windows.chip_shown().is_none());
    assert!(e.b().program_fill_poll_step());
    let again = e.windows.chip_shown().expect("back at the new place");
    assert!(again.left > first.left);
    // Gone: hidden.
    *e.host.rect.lock().unwrap() = Ok(None);
    assert!(e.b().program_fill_poll_step());
    assert!(e.windows.chip_shown().is_none());
    // Not on offer any more: the poller ends.
    e.host.emit(HostEvent::Gone);
    assert!(!e.b().program_fill_poll_step());
}

#[test]
fn a_chip_click_with_an_old_generation_is_stale() {
    let e = Env::new();
    e.seed();
    e.start();
    e.host
        .emit(HostEvent::Field(password_field(1, Env::slack_identity())));
    let generation = e.generation();
    e.host
        .emit(HostEvent::Field(password_field(2, Env::slack_identity())));
    let error = e
        .b()
        .picker_chip_click(&PickerGenerationRequest { generation })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::PickerStale);
    // The guard time constants are the plan's.
    assert_eq!(
        (PICK_MIN_DELAY_MS, LINK_MIN_DELAY_MS, UNLOCK_RESUME_MS),
        (300, 1_000, 60_000)
    );
}

#[test]
fn search_finds_logins_by_title_user_or_host_and_is_capped() {
    let e = Env::new();
    e.seed();
    e.start();
    e.open_list(password_field(
        1,
        signed("bank.exe", OTHER_SIGNER, "Bank AG"),
    ));
    let search = |q: &str| {
        e.b().picker_search(&PickerSearchRequest {
            generation: e.generation(),
            query: q.to_owned(),
        })
    };
    assert_eq!(search("BANK.exa").unwrap().len(), 1, "host, folded");
    assert_eq!(search("tool-us").unwrap().len(), 1, "user name");
    assert_eq!(search("sla").unwrap()[0].title, "Slack");
    assert_eq!(search("").unwrap().len(), 0);
    assert_eq!(
        search(&"x".repeat(129)).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let stale = e
        .b()
        .picker_search(&PickerSearchRequest {
            generation: e.generation() + 1,
            query: "sla".into(),
        })
        .unwrap_err();
    assert_eq!(stale.code, ErrorCode::PickerStale);
}

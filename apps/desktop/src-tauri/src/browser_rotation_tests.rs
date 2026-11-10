//! Tests of the guided password change over the browser channel (M7b T4): which attempts a page
//! sees, the two values a change form gets and the checks before them (exact origin, https,
//! frames), the three desktop dialogs (the 1 s Allow guard, the timeout, one at a time, closed on
//! lock) and what each answer writes. They call the endpoint's backend and prompt methods
//! directly (the wire envelope and its allowlist are covered in `browser_tests`).

#![expect(
    clippy::too_many_lines,
    clippy::assert_is_empty,
    reason = "each test reads as one story; an empty list is the answer that is checked"
)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mosaic_core::model::{HistoryReason, ItemType, LoginUrl, SecretField, TextField, UrlMatch};
use mosaic_core::{Argon2Profile, ItemId, VaultId};
use mosaic_native_messaging::bodies::{
    AddOriginOutcome, AttemptStage, BeginOutcome, ChangeShape, FillChangeBody, FillPendingBody,
    FrameKind, ListForOriginBody, ReportedOutcome, ResultOutcome, RotationAddOriginBody,
    RotationBeginBody, RotationResultBody,
};
use mosaic_native_messaging::protocol::ErrorCode as Wire;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::Value;

use crate::browser::{BrowserService, Clock, Limits, RotationAnswer, RotationTicket, Shared};
use crate::clipboard::MemoryClipboard;
use crate::contract::browser::{
    BrowserFamily, BrowserRotationDecision, BrowserRotationRespondRequest,
    BrowserRotationRevealRequest,
};
use crate::contract::credential_rotation::RotationBeginRequest;
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::secret::SecretIn;
use crate::contract::session::CreateAccountRequest;
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

const PASSWORD: &str = "correct horse battery staple";
const CURRENT: &str = "old-Pass-1!xQ";
const NEW: &str = "Xy7@mB3^dF6!hJ9#zC2$";
const SITE: &str = "https://example.com";

#[derive(Clone)]
struct TestClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl TestClock {
    fn handle(&self) -> Clock {
        let clock = self.clone();
        Arc::new(move || clock.base + *clock.offset.lock().unwrap())
    }

    fn advance(&self, by: Duration) {
        *self.offset.lock().unwrap() += by;
    }
}

struct Env {
    state: AppState,
    events: Log,
    clock: TestClock,
    pairing: [u8; 16],
    conn: u64,
    _kdf: std::sync::MutexGuard<'static, ()>,
}

fn run<F: std::future::Future>(future: F) -> F::Output {
    tauri::async_runtime::block_on(future)
}

impl Env {
    fn new() -> Self {
        Self::with_limits(Limits::default())
    }

    fn with_limits(limits: Limits) -> Self {
        let kdf = crate::test_support::kdf_permit();
        let clock = TestClock {
            base: Instant::now(),
            offset: Arc::default(),
        };
        let mut backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_clipboard(MemoryClipboard::default().factory());
        backend.browser = BrowserService::with_clock(clock.handle(), limits);
        let (channel, events) = recording_channel();
        backend.subscribe(channel).unwrap();
        let state = AppState::new(backend);
        state.backend().browser.attach(state.downgrade());
        state
            .backend()
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        let shared = state.backend().browser.shared();
        let (pairing, _key) = shared
            .registry
            .lock()
            .unwrap()
            .add("Chrome".into(), BrowserFamily::Chromium, 1_000)
            .unwrap();
        let conn = shared.register_idle(pairing).unwrap();
        Self {
            state,
            events,
            clock,
            pairing,
            conn,
            _kdf: kdf,
        }
    }

    fn be(&self) -> &Backend {
        self.state.backend()
    }

    fn shared(&self) -> &Arc<Shared> {
        self.be().browser.shared()
    }

    fn vault(&self) -> VaultId {
        self.be().list_vaults().unwrap()[0].vault_id.get()
    }

    fn login_with(&self, tweak: impl FnOnce(&mut LoginUrl), url: &str) -> ItemId {
        let vault = self.vault();
        let mut slot = self.be().slot().unwrap();
        slot.unlocked_mut()
            .unwrap()
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title("Example")?;
                e.set_text(TextField::Username, "alex@example.com")?;
                e.set_secret(SecretField::Password, CURRENT)?;
                let mut entry = LoginUrl::new(url);
                tweak(&mut entry);
                e.add_url(&entry)?;
                Ok(())
            })
            .unwrap()
    }

    fn login(&self) -> ItemId {
        self.login_with(|_| {}, SITE)
    }

    /// Starts an attempt through the desktop's own command (as the panel does).
    fn begin(&self, item: ItemId) -> String {
        self.be()
            .rotation_begin(&RotationBeginRequest {
                vault_id: self.vault().into(),
                item_id: item.into(),
                new_password: SecretIn::test(NEW),
                rules_source: None,
            })
            .unwrap()
            .attempt_id
    }

    fn password(&self, item: ItemId) -> String {
        let vault = self.vault();
        let slot = self.be().slot().unwrap();
        slot.unlocked()
            .unwrap()
            .item(vault, item)
            .unwrap()
            .secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret()
            .to_owned()
    }

    fn history(&self, item: ItemId) -> Vec<HistoryReason> {
        let vault = self.vault();
        let slot = self.be().slot().unwrap();
        slot.unlocked()
            .unwrap()
            .item(vault, item)
            .unwrap()
            .password_history()
            .iter()
            .map(|e| e.reason)
            .collect()
    }

    fn events_of(&self, kind: &str) -> Vec<Value> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|e| serde_json::from_str::<Value>(e).unwrap())
            .filter(|e| e["type"] == kind)
            .collect()
    }

    fn fill_change(
        &self,
        item: ItemId,
        attempt: &str,
        origin: &str,
        top: &str,
        frame: FrameKind,
        shape: ChangeShape,
    ) -> Result<(Option<String>, String), Wire> {
        self.be()
            .browser_fill_change(&FillChangeBody {
                vault_id: self.vault().to_string(),
                item_id: item.to_string(),
                attempt_id: attempt.to_owned(),
                origin: origin.to_owned(),
                top_origin: top.to_owned(),
                frame,
                shape,
            })
            .map(|r| {
                (
                    r.current.map(|c| c.expose_secret().to_owned()),
                    r.new.expose_secret().to_owned(),
                )
            })
    }

    fn open_begin(&self, item: ItemId) -> RotationTicket {
        let plan = self
            .be()
            .browser_plan_rotation_begin(&RotationBeginBody {
                vault_id: self.vault().to_string(),
                item_id: item.to_string(),
                origin: SITE.into(),
                top_origin: SITE.into(),
                rules: None,
            })
            .unwrap();
        self.shared()
            .open_rotation(&self.pairing, self.conn, plan)
            .unwrap()
    }

    fn open_result(&self, item: ItemId, attempt: &str, outcome: ReportedOutcome) -> RotationTicket {
        let plan = self
            .be()
            .browser_plan_rotation_result(&RotationResultBody {
                vault_id: self.vault().to_string(),
                item_id: item.to_string(),
                attempt_id: attempt.to_owned(),
                origin: SITE.into(),
                top_origin: SITE.into(),
                outcome,
            })
            .unwrap();
        self.shared()
            .open_rotation(&self.pairing, self.conn, plan)
            .unwrap()
    }

    fn respond(&self, id: &str, decision: BrowserRotationDecision) -> Result<(), IpcError> {
        self.be()
            .browser_rotation_respond(&BrowserRotationRespondRequest {
                request_id: id.to_owned(),
                decision,
            })
    }

    fn request_id(&self) -> String {
        let events = self.events_of("browserRotationRequested");
        events.last().unwrap()["request"]["requestId"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

/// A well-formed attempt ID that names no attempt.
fn unknown_attempt(real: &str) -> String {
    let (base, _) = real.rsplit_once('.').unwrap();
    format!("{base}.77")
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

#[test]
fn browser_rotation_for_origin_lists_exact_attempts_without_values() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let list = |origin: &str| {
        env.be()
            .browser_rotation_for_origin(&ListForOriginBody {
                origin: origin.into(),
                top_origin: origin.into(),
            })
            .unwrap()
            .attempts
    };
    let found = list(SITE);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].attempt_id, attempt);
    assert_eq!(found[0].host, "example.com");
    assert_eq!(found[0].title.as_deref(), Some("Example"));
    assert_eq!(
        found[0].change_url.as_deref(),
        Some("https://example.com/.well-known/change-password")
    );
    assert!(!serde_json::to_string(&found).unwrap().contains(NEW));
    // No eTLD+1 match, no sibling host, no plain http.
    assert!(list("https://accounts.example.com").is_empty());
    assert!(list("https://example.com.evil.test").is_empty());
    assert!(list("http://example.com").is_empty());
}

#[test]
fn browser_fill_change_requires_exact_origin() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let (current, new) = env
        .fill_change(
            item,
            &attempt,
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::CurrentAndNew,
        )
        .unwrap();
    assert_eq!(current.as_deref(), Some(CURRENT));
    assert_eq!(new, NEW);
    let (none, new) = env
        .fill_change(
            item,
            &attempt,
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly,
        )
        .unwrap();
    assert_eq!(
        none, None,
        "a new-only form never gets the current password"
    );
    assert_eq!(new, NEW);
    for other in [
        "https://www.example.com",
        "https://example.com:8443",
        "https://example.org",
    ] {
        assert_eq!(
            env.fill_change(
                item,
                &attempt,
                other,
                other,
                FrameKind::Top,
                ChangeShape::NewOnly
            )
            .unwrap_err(),
            Wire::OriginMismatch,
            "{other}"
        );
    }
    assert_eq!(
        env.fill_change(
            item,
            &unknown_attempt(&attempt),
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::RotationNotFound
    );
    assert_eq!(
        env.fill_change(
            item,
            &"a".repeat(65),
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::InvalidRequest
    );
    assert_eq!(
        env.fill_change(
            item,
            "nope",
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::InvalidRequest,
        "a malformed attempt ID is not an attempt"
    );
}

#[test]
fn browser_fill_change_refuses_etld1_match() {
    let env = Env::new();
    let item = env.login_with(|u| u.match_mode = UrlMatch::Etld1, SITE);
    let attempt = env.begin(item);
    let sibling = "https://accounts.example.com";
    // A normal fill would match the sibling; a change fill never does.
    assert_eq!(
        env.fill_change(
            item,
            &attempt,
            sibling,
            sibling,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::OriginMismatch
    );
    assert!(
        env.be()
            .browser_rotation_for_origin(&ListForOriginBody {
                origin: sibling.into(),
                top_origin: sibling.into(),
            })
            .unwrap()
            .attempts
            .is_empty()
    );
}

#[test]
fn browser_fill_change_refuses_http_even_with_item_override() {
    let env = Env::new();
    let item = env.login_with(|u| u.allow_http = true, "http://example.com");
    // An attempt can only be started on a login with an https URL; add one so the item is valid.
    let vault = env.vault();
    env.be()
        .slot()
        .unwrap()
        .unlocked_mut()
        .unwrap()
        .update_item(vault, item, |e| e.add_url(&LoginUrl::new(SITE)).map(|_| ()))
        .unwrap();
    let attempt = env.begin(item);
    assert_eq!(
        env.fill_change(
            item,
            &attempt,
            "http://example.com",
            "http://example.com",
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::InsecureOrigin
    );
}

#[test]
fn browser_fill_change_refuses_cross_origin_frame() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let top = "https://evil.test";
    assert_eq!(
        env.fill_change(
            item,
            &attempt,
            SITE,
            top,
            FrameKind::SameOriginFrame,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::CrossOriginFrame
    );
    assert_eq!(
        env.fill_change(
            item,
            &attempt,
            SITE,
            top,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .unwrap_err(),
        Wire::InvalidRequest
    );
    // The dialog ops refuse a frame that is not the top one, too.
    let refused = env.be().browser_plan_rotation_begin(&RotationBeginBody {
        vault_id: env.vault().to_string(),
        item_id: item.to_string(),
        origin: SITE.into(),
        top_origin: top.into(),
        rules: None,
    });
    assert_eq!(refused.err(), Some(Wire::CrossOriginFrame));
    assert_eq!(
        env.fill_change(
            item,
            &attempt,
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly
        )
        .map(|(_, new)| new),
        Ok(NEW.to_owned()),
        "the same attempt still fills on the top frame"
    );
}

#[test]
fn browser_fill_change_marks_delivered_before_answer() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let stage = || {
        env.be()
            .browser_rotation_for_origin(&ListForOriginBody {
                origin: SITE.into(),
                top_origin: SITE.into(),
            })
            .unwrap()
            .attempts[0]
            .stage
    };
    assert_eq!(stage(), AttemptStage::Prepared);
    env.fill_change(
        item,
        &attempt,
        SITE,
        SITE,
        FrameKind::Top,
        ChangeShape::NewOnly,
    )
    .unwrap();
    assert_eq!(stage(), AttemptStage::Delivered);
    // The pending sign-in fill gets the user name and the new value, never the old password.
    let pending = env
        .be()
        .browser_fill_pending(&FillPendingBody {
            vault_id: env.vault().to_string(),
            item_id: item.to_string(),
            attempt_id: attempt,
            origin: SITE.into(),
            top_origin: SITE.into(),
            frame: FrameKind::Top,
        })
        .unwrap();
    assert_eq!(pending.username.as_deref(), Some("alex@example.com"));
    assert_eq!(pending.password.expose_secret(), NEW);
    assert_eq!(
        env.password(item),
        CURRENT,
        "a fill never changes the password"
    );
}

#[test]
fn browser_rotation_result_commits_only_on_confirm() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let ticket = env.open_result(item, &attempt, ReportedOutcome::Succeeded);
    let id = env.request_id();
    assert_eq!(
        env.password(item),
        CURRENT,
        "the request alone changes nothing"
    );
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Confirm {})
                .unwrap_err()
        ),
        ErrorCode::BrowserAllowTooSoon
    );
    assert_eq!(env.password(item), CURRENT);
    // The wrong kind of answer is refused and the dialog stays.
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Allow {})
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );
    env.clock.advance(Duration::from_secs(2));
    env.respond(&id, BrowserRotationDecision::Confirm {})
        .unwrap();
    let answer = run(env.shared().await_rotation(ticket)).unwrap();
    assert_eq!(answer_of(&answer), "confirmed");
    assert_eq!(env.password(item), NEW);
    assert_eq!(env.history(item), vec![HistoryReason::Replaced]);
    assert_eq!(
        env.events_of("browserRotationClosed").last().unwrap()["reason"],
        "answered"
    );

    // "It didn't work": the user keeps the old password; the value stays in the history.
    let attempt = env.begin(item);
    let ticket = env.open_result(item, &attempt, ReportedOutcome::Failed);
    let id = env.request_id();
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Confirm {})
                .unwrap_err()
        ),
        ErrorCode::InvalidInput,
        "a reported failure cannot be confirmed"
    );
    // Keeping the old password needs no wait: it is the safe answer.
    env.respond(&id, BrowserRotationDecision::KeepOld {})
        .unwrap();
    assert_eq!(
        answer_of(&run(env.shared().await_rotation(ticket)).unwrap()),
        "keptOld"
    );
    assert_eq!(
        env.password(item),
        NEW,
        "the password the last confirmation set stays"
    );
    assert!(env.history(item).contains(&HistoryReason::Unconfirmed));

    // A forged result for an attempt that does not exist opens no dialog.
    assert_eq!(
        env.be()
            .browser_plan_rotation_result(&RotationResultBody {
                vault_id: env.vault().to_string(),
                item_id: item.to_string(),
                attempt_id: unknown_attempt(&attempt),
                origin: SITE.into(),
                top_origin: SITE.into(),
                outcome: ReportedOutcome::Succeeded,
            })
            .err(),
        Some(Wire::RotationNotFound)
    );
}

#[test]
fn browser_rotation_dialog_allow_guard_and_timeout() {
    let env = Env::new();
    let item = env.login();
    let ticket = env.open_begin(item);
    let id = env.request_id();
    let request = &env.events_of("browserRotationRequested")[0]["request"];
    assert_eq!(request["kind"], "begin");
    assert!(
        !request.to_string().contains(CURRENT),
        "the event carries no password"
    );
    // The password is fetched on request and is the item's new attempt value only after Allow.
    let first = env
        .be()
        .browser_rotation_reveal(&BrowserRotationRevealRequest {
            request_id: id.clone(),
        })
        .unwrap();
    let first = serde_json::to_value(&first).unwrap()["value"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(first.chars().count() >= 12);
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Allow {})
                .unwrap_err()
        ),
        ErrorCode::BrowserAllowTooSoon
    );
    assert!(
        env.be()
            .browser_rotation_for_origin(&ListForOriginBody {
                origin: SITE.into(),
                top_origin: SITE.into(),
            })
            .unwrap()
            .attempts
            .is_empty(),
        "nothing is stored before Allow"
    );
    // Generating another one shows a new password and starts the guard again.
    env.clock.advance(Duration::from_secs(2));
    env.respond(&id, BrowserRotationDecision::Regenerate {})
        .unwrap();
    let second = serde_json::to_value(
        env.be()
            .browser_rotation_reveal(&BrowserRotationRevealRequest {
                request_id: id.clone(),
            })
            .unwrap(),
    )
    .unwrap()["value"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(first, second);
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Allow {})
                .unwrap_err()
        ),
        ErrorCode::BrowserAllowTooSoon
    );
    env.clock.advance(Duration::from_secs(2));
    env.respond(&id, BrowserRotationDecision::Allow {}).unwrap();
    let answer = run(env.shared().await_rotation(ticket)).unwrap();
    assert_eq!(answer_of(&answer), "started");
    assert_eq!(
        env.password(item),
        CURRENT,
        "the current password is untouched"
    );
    // The stored attempt value is what the dialog showed last.
    let (_, new) = env
        .fill_change(
            item,
            &env.be()
                .browser_rotation_for_origin(&ListForOriginBody {
                    origin: SITE.into(),
                    top_origin: SITE.into(),
                })
                .unwrap()
                .attempts[0]
                .attempt_id,
            SITE,
            SITE,
            FrameKind::Top,
            ChangeShape::NewOnly,
        )
        .unwrap();
    assert_eq!(new, second);
    // A closed dialog cannot be answered or revealed again.
    assert_eq!(
        code(
            env.respond(&id, BrowserRotationDecision::Cancel {})
                .unwrap_err()
        ),
        ErrorCode::BrowserRequestExpired
    );
    assert_eq!(
        code(
            env.be()
                .browser_rotation_reveal(&BrowserRotationRevealRequest { request_id: id })
                .unwrap_err()
        ),
        ErrorCode::BrowserRequestExpired
    );
}

#[test]
fn browser_rotation_dialog_expires_and_answers_cancelled() {
    let env = Env::with_limits(Limits {
        save_timeout: Duration::from_millis(60),
        ..Limits::default()
    });
    let item = env.login();
    let ticket = env.open_begin(item);
    let answer = run(env.shared().await_rotation(ticket)).unwrap();
    assert_eq!(answer_of(&answer), "cancelled");
    assert_eq!(
        env.events_of("browserRotationClosed").last().unwrap()["reason"],
        "expired"
    );
    assert!(!env.shared().rotation_is_open());
}

#[test]
fn browser_rotation_one_dialog_at_a_time_and_closed_on_lock() {
    let env = Env::new();
    let item = env.login();
    let ticket = env.open_begin(item);
    let again = env
        .be()
        .browser_plan_rotation_begin(&RotationBeginBody {
            vault_id: env.vault().to_string(),
            item_id: item.to_string(),
            origin: SITE.into(),
            top_origin: SITE.into(),
            rules: None,
        })
        .unwrap();
    assert_eq!(
        env.shared()
            .open_rotation(&env.pairing, env.conn, again)
            .err(),
        Some(Wire::Busy)
    );
    env.be().lock(LockReason::Manual).unwrap();
    assert_eq!(
        run(env.shared().await_rotation(ticket)).err(),
        Some(Wire::Locked)
    );
    assert_eq!(
        env.events_of("browserRotationClosed").last().unwrap()["reason"],
        "locked"
    );
    assert!(!env.shared().rotation_is_open());
}

#[test]
fn browser_rotation_dialog_closes_when_the_pairing_goes() {
    let env = Env::new();
    let item = env.login();
    let ticket = env.open_begin(item);
    env.shared().remove_pairing(&env.pairing, None).unwrap();
    assert!(run(env.shared().await_rotation(ticket)).is_err());
    assert_eq!(
        env.events_of("browserRotationClosed").last().unwrap()["reason"],
        "disconnected"
    );
}

#[test]
fn browser_rotation_begin_refuses_at_the_limit_and_for_other_origins() {
    let env = Env::new();
    let item = env.login();
    for _ in 0..4 {
        env.begin(item);
    }
    let plan = |origin: &str| {
        env.be().browser_plan_rotation_begin(&RotationBeginBody {
            vault_id: env.vault().to_string(),
            item_id: item.to_string(),
            origin: origin.into(),
            top_origin: origin.into(),
            rules: None,
        })
    };
    assert_eq!(plan(SITE).err(), Some(Wire::RotationLimit));
    assert_eq!(
        plan("https://www.example.com").err(),
        Some(Wire::OriginMismatch)
    );
    assert_eq!(plan("http://example.com").err(), Some(Wire::InsecureOrigin));
    assert_eq!(
        env.be()
            .browser_plan_rotation_begin(&RotationBeginBody {
                vault_id: env.vault().to_string(),
                item_id: item.to_string(),
                origin: SITE.into(),
                top_origin: SITE.into(),
                rules: Some("x".repeat(4097)),
            })
            .err(),
        Some(Wire::InvalidRequest)
    );
}

fn add_origin_body(
    env: &Env,
    item: ItemId,
    attempt: &str,
    from: &str,
    to: &str,
) -> RotationAddOriginBody {
    RotationAddOriginBody {
        vault_id: env.vault().to_string(),
        item_id: item.to_string(),
        attempt_id: attempt.to_owned(),
        from_origin: from.into(),
        origin: to.into(),
        top_origin: to.into(),
    }
}

#[test]
fn browser_add_origin_refuses_unrelated_public_suffix_and_loopback() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    for (from, to) in [
        (SITE, "https://example.org"),
        (SITE, "https://example.com.evil.test"),
        (SITE, "https://evil-example.com"),
        (SITE, "http://accounts.example.com"),
        (SITE, "https://localhost"),
        (SITE, "https://co.uk"),
        (SITE, "https://127.0.0.1"),
        ("https://evil.test", "https://accounts.example.com"),
        ("http://example.com", "https://accounts.example.com"),
    ] {
        assert_eq!(
            env.be()
                .browser_plan_rotation_add_origin(&add_origin_body(&env, item, &attempt, from, to))
                .err(),
            Some(Wire::OriginNotRelated),
            "{from} -> {to}"
        );
    }
    assert_eq!(
        env.be()
            .browser_plan_rotation_add_origin(&add_origin_body(
                &env,
                item,
                &unknown_attempt(&attempt),
                SITE,
                "https://accounts.example.com"
            ))
            .err(),
        Some(Wire::RotationNotFound)
    );

    // A related https origin goes to the dialog; only the answer adds it.
    let plan = env
        .be()
        .browser_plan_rotation_add_origin(&add_origin_body(
            &env,
            item,
            &attempt,
            SITE,
            "https://accounts.example.com",
        ))
        .unwrap();
    let ticket = env
        .shared()
        .open_rotation(&env.pairing, env.conn, plan)
        .unwrap();
    let id = env.request_id();
    let request = &env.events_of("browserRotationRequested")[0]["request"];
    assert_eq!(request["kind"], "addOrigin");
    assert_eq!(request["newOrigin"], "https://accounts.example.com");
    let urls = |env: &Env| {
        let vault = env.vault();
        let slot = env.be().slot().unwrap();
        slot.unlocked()
            .unwrap()
            .item(vault, item)
            .unwrap()
            .urls()
            .len()
    };
    assert_eq!(urls(&env), 1);
    env.clock.advance(Duration::from_secs(2));
    env.respond(&id, BrowserRotationDecision::AddOrigin {})
        .unwrap();
    assert_eq!(
        answer_of(&run(env.shared().await_rotation(ticket)).unwrap()),
        "added"
    );
    assert_eq!(urls(&env), 2);
    // It is now an exact origin of the login, and a second add is refused.
    assert_eq!(
        env.be()
            .browser_rotation_for_origin(&ListForOriginBody {
                origin: "https://accounts.example.com".into(),
                top_origin: "https://accounts.example.com".into(),
            })
            .unwrap()
            .attempts
            .len(),
        1
    );
    assert_eq!(
        env.be()
            .browser_plan_rotation_add_origin(&add_origin_body(
                &env,
                item,
                &attempt,
                SITE,
                "https://accounts.example.com"
            ))
            .err(),
        Some(Wire::InvalidRequest)
    );
}

#[test]
fn rotation_ops_are_refused_in_a_locked_vault() {
    let env = Env::new();
    let item = env.login();
    let attempt = env.begin(item);
    let vault_id = env.vault().to_string();
    env.be().lock(LockReason::Manual).unwrap();
    let refused = env.be().browser_fill_change(&FillChangeBody {
        vault_id,
        item_id: item.to_string(),
        attempt_id: attempt,
        origin: SITE.into(),
        top_origin: SITE.into(),
        frame: FrameKind::Top,
        shape: ChangeShape::NewOnly,
    });
    assert_eq!(refused.err(), Some(Wire::Locked));
}

/// What an answer says on the wire.
fn answer_of(answer: &RotationAnswer) -> &'static str {
    match answer {
        RotationAnswer::Begin(r) => match r.outcome {
            BeginOutcome::Started => "started",
            BeginOutcome::Cancelled => "cancelled",
        },
        RotationAnswer::Result(r) => match r.outcome {
            ResultOutcome::Confirmed => "confirmed",
            ResultOutcome::KeptOld => "keptOld",
            ResultOutcome::Cancelled => "cancelled",
        },
        RotationAnswer::AddOrigin(r) => match r.outcome {
            AddOriginOutcome::Added => "added",
            AddOriginOutcome::Cancelled => "cancelled",
        },
    }
}

//! M10-G5b: adversarial sync scenarios against the phone core. A hostile
//! server (`common::hostile`) lies to the phone over the real HTTP transport
//! the phone uses; the phone's lifecycle (lock, store close, process death)
//! is cut into a sync at an exact answer; and three devices (a desktop and two
//! phones) edit, delete, move and rotate concurrently, with clock skew.
//!
//! The engine-level hosted chaos lives in `crates/server/tests/it/chaos_hosted.rs`;
//! these scenarios cover what the phone adds: `Core::sync_now`, the
//! `SyncOverview` it reports, the error codes, the store and session state
//! afterwards, and the lifecycle calls of the apps.
//!
//! Every scenario is deterministic: answers are held by a channel handshake,
//! clocks are the server's manual clock and the core's offset, and the
//! schedules come from fixed seeds. For each one: no silent data loss, no
//! accepted rollback or fork (it is detected and reported), no secret in an
//! error or in the overview, and a store and session that still lock and
//! unlock to the same contents.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use mosaic_core::ItemId;
use mosaic_core::model::{ItemType, SecretField};
use mosaic_mobile::{
    CoreContext, ItemFilter, LoginDraft, MobileError, OtpEdit, Platform, SessionState,
    SyncOverview, TrashMode, UrlDraft,
};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::transport::Transport as _;
use mosaic_vault::session::{ItemFilter as DeskFilter, TrashFilter};
use serde_json::{Value, json};

use crate::common::hostile::{Exchange, HostileServer, Reply};
use crate::common::{BOOT, Desktop, PASSWORD, Phone, Server as _, pw};

const DAY_MS: i64 = 86_400_000;
const HOLD: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A deterministic generator (`SplitMix64`) for the seeded schedules.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A desktop with one login ("Desk login", `pw-desk-1`) and a phone that
/// joined and synced once, over a hostile server that is honest for now.
struct World {
    server: HostileServer,
    desktop: Desktop,
    phone: Phone,
    /// The desktop's login.
    x: ItemId,
}

impl World {
    fn new(email: &str) -> Self {
        let server = HostileServer::start();
        let desktop = Desktop::register(&server, email);
        let x = desktop.add_login("Desk login", "bob", "pw-desk-1", "https://desk.example");
        desktop.sync(&server);
        let phone = Phone::new();
        phone.join(&server, email, &desktop.sk_text());
        sync(&server, &phone).unwrap();
        Self {
            server,
            desktop,
            phone,
            x,
        }
    }

    /// Another phone of the account, joined and synced.
    fn second_phone(&self) -> Phone {
        let phone = Phone::new();
        phone.join(&self.server, &self.desktop.email, &self.desktop.sk_text());
        sync(&self.server, &phone).unwrap();
        phone
    }

    fn x(&self) -> String {
        self.x.to_string()
    }
}

/// One phone run with the server armed (the tamper, if any, applies).
fn sync(server: &HostileServer, phone: &Phone) -> mosaic_mobile::Result<SyncOverview> {
    server.arm(true);
    let result = phone.core.sync_now();
    server.arm(false);
    check_clean(&format!("{result:?}"));
    check_clean(&format!("{:?}", phone.core.sync_overview()));
    result
}

/// No password, no Secret Key text and no account password in a rendering
/// of an error or overview. Every test password starts with `pw-`.
fn check_clean(text: &str) {
    assert!(!text.contains("pw-"), "a password leaked: {text}");
    assert!(
        !text.contains(PASSWORD),
        "the master password leaked: {text}"
    );
    assert!(!text.contains("A3-"), "a Secret Key leaked: {text}");
}

fn draft(vault: &str, item: Option<&str>, title: &str, password: &str) -> LoginDraft {
    LoginDraft {
        vault_id: vault.to_owned(),
        item_id: item.map(str::to_owned),
        title: title.to_owned(),
        username: String::new(),
        password: Some(password.as_bytes().to_vec()),
        urls: Vec::new(),
        notes: None,
        otp: OtpEdit::Keep,
        favorite: false,
    }
}

/// A new login on the phone.
fn phone_add(phone: &Phone, vault: &str, title: &str, password: &str) -> String {
    phone
        .core
        .save_login(draft(vault, None, title, password))
        .unwrap()
        .item_id
}

/// Changes the password of a login on the phone, keeping everything else.
fn phone_set_password(phone: &Phone, vault: &str, item: &str, password: &str) {
    let detail = phone.core.item(vault, item).unwrap();
    let username = detail
        .fields
        .iter()
        .find(|f| f.key.ends_with("username"))
        .and_then(|f| f.value.clone())
        .unwrap_or_default();
    let mut d = draft(vault, Some(item), &detail.summary.title, password);
    d.username = username;
    d.favorite = detail.summary.favorite;
    d.urls = detail
        .urls
        .iter()
        .map(|u| UrlDraft {
            url: u.url.clone(),
            match_etld1: u.match_etld1,
            allow_http: u.allow_http,
        })
        .collect();
    phone.core.save_login(d).unwrap();
}

/// The vault of an item on the phone.
fn phone_vault_of(phone: &Phone, item: &str) -> String {
    let mut filter = ItemFilter::default();
    filter.trash = TrashMode::Include;
    phone
        .core
        .items(filter)
        .unwrap()
        .into_iter()
        .find(|i| i.item_id == item)
        .map(|i| i.vault_id)
        .unwrap()
}

/// Every item of the phone (trash included): ID to (title, password).
fn snapshot(phone: &Phone) -> BTreeMap<String, (String, String)> {
    let mut filter = ItemFilter::default();
    filter.trash = TrashMode::Include;
    phone
        .core
        .items(filter)
        .unwrap()
        .into_iter()
        .map(|i| {
            let password = match phone.core.reveal(&i.vault_id, &i.item_id, "login.password") {
                Ok(value) => String::from_utf8(value.expose_bytes()).unwrap(),
                Err(_) => String::new(),
            };
            (i.item_id, (i.title, password))
        })
        .collect()
}

fn titles(phone: &Phone) -> Vec<String> {
    snapshot(phone).into_values().map(|(t, _)| t).collect()
}

fn password_on(phone: &Phone, item: &str) -> String {
    snapshot(phone)
        .get(item)
        .map_or_else(String::new, |(_, password)| password.clone())
}

/// Every item of the desktop (trash included), as [`snapshot`].
fn desk_snapshot(desktop: &Desktop) -> BTreeMap<String, (String, String)> {
    desktop.with(|session| {
        let mut filter = DeskFilter::default();
        filter.trash = TrashFilter::Include;
        session
            .items(&filter)
            .iter()
            .map(|row| {
                let password = row
                    .item
                    .secret(SecretField::Password)
                    .ok()
                    .flatten()
                    .map_or_else(String::new, |secret| secret.expose_secret().to_owned());
                (
                    row.item_id.to_string(),
                    (row.item.title().unwrap_or("").to_owned(), password),
                )
            })
            .collect()
    })
}

fn desk_titles(desktop: &Desktop) -> Vec<String> {
    desk_snapshot(desktop)
        .into_values()
        .map(|(t, _)| t)
        .collect()
}

/// Every value an item of the desktop still holds for any field: the shown
/// ones, the conflict losers and the password history.
fn desk_values(desktop: &Desktop, id: ItemId) -> BTreeSet<String> {
    desktop.with(|session| {
        let mut filter = DeskFilter::default();
        filter.trash = TrashFilter::Include;
        let mut out = BTreeSet::new();
        let items = session.items(&filter);
        let Some(row) = items.iter().find(|row| row.item_id == id) else {
            return out;
        };
        if let Some(secret) = row.item.secret(SecretField::Password).ok().flatten() {
            out.insert(secret.expose_secret().to_owned());
        }
        for conflict in row.item.conflicts() {
            let entries = std::iter::once(conflict.winner).chain(conflict.losers.iter().copied());
            out.extend(entries.filter_map(|entry| entry.value().as_str().map(str::to_owned)));
        }
        for entry in row.item.password_history() {
            out.insert(entry.value.expose_secret().to_owned());
        }
        out
    })
}

fn desk_vault_of(desktop: &Desktop, id: ItemId) -> mosaic_core::VaultId {
    desktop.with(|session| {
        let mut filter = DeskFilter::default();
        filter.trash = TrashFilter::Include;
        session
            .items(&filter)
            .iter()
            .find(|row| row.item_id == id)
            .map(|row| row.vault_id)
            .unwrap()
    })
}

fn desk_set_password(desktop: &Desktop, id: ItemId, password: &str) {
    let vault = desk_vault_of(desktop, id);
    desktop.with(|s| {
        s.update_item(vault, id, |e| e.set_secret(SecretField::Password, password))
            .unwrap();
    });
}

fn desk_add_in(desktop: &Desktop, vault: mosaic_core::VaultId, title: &str, pw: &str) -> ItemId {
    desktop.with(|s| {
        s.add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, pw)?;
            Ok(())
        })
        .unwrap()
    })
}

fn halted(phone: &Phone) -> bool {
    phone.core.sync_overview().halted
}

/// Locks and unlocks with the password: the same contents come back, and
/// the session is usable.
fn assert_consistent(phone: &Phone) {
    let before = snapshot(phone);
    phone.core.lock();
    assert!(
        matches!(phone.core.state(), SessionState::Locked { .. }),
        "{:?}",
        phone.core.state()
    );
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(snapshot(phone), before, "lock and unlock changed the store");
}

/// The process dies: a new core over the same files and storage, unlocked
/// with the password.
fn restart(phone: &mut Phone) {
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
}

/// A phone run whose answer `matches` holds is cut by `cut` (a lock or a
/// store close) while the server holds that answer; the run's result.
fn sync_cut_at(
    server: &HostileServer,
    phone: &Phone,
    matches: impl Fn(&Exchange) -> bool + Send + 'static,
    cut: impl FnOnce(),
) -> mosaic_mobile::Result<SyncOverview> {
    let (reached, go) = server.hold(matches);
    server.arm(true);
    let result = std::thread::scope(|scope| {
        let run = scope.spawn(|| phone.core.sync_now());
        reached.recv_timeout(HOLD).expect("the answer was held");
        cut();
        go.send(()).unwrap();
        run.join().unwrap()
    });
    server.arm(false);
    check_clean(&format!("{result:?}"));
    result
}

/// A changes page that withholds everything after `since` (the head stays
/// where the phone is).
fn withheld(ex: &Exchange) -> Option<Reply> {
    ex.changes_vault()?;
    let since = ex.since()?;
    let mut v = ex.json()?;
    v["headSeq"] = json!(since);
    v["nextSince"] = json!(since);
    v["more"] = json!(false);
    v["records"] = json!([]);
    v["keys"] = json!([]);
    v["meta"] = Value::Null;
    v["rosters"] = json!([]);
    Some(Reply::json(&v))
}

/// Replaces a changes record by another one (`id`, `kind`, `ciphertext`)
/// under the same server sequence.
fn substitute(record: &mut Value, id: &str, kind: &Value, ciphertext: &Value) {
    let seq = record["serverSeq"].clone();
    *record = json!({ "recordId": id, "kind": kind, "ciphertext": ciphertext, "serverSeq": seq });
}

/// A tamper that serves, in the changes pages of `vault` (any vault for
/// `None`), the first record as `id` with another record's `kind` and
/// `ciphertext`.
fn swap_first(
    vault: Option<String>,
    id: String,
    kind: Value,
    ciphertext: Value,
) -> impl FnMut(&Exchange) -> Option<Reply> + Send + 'static {
    move |ex| {
        let at = ex.changes_vault()?;
        if vault.as_deref().is_some_and(|v| v != at) {
            return None;
        }
        let mut page = ex.json()?;
        substitute(
            page["records"].as_array_mut()?.first_mut()?,
            &id,
            &kind,
            &ciphertext,
        );
        Some(Reply::json(&page))
    }
}

/// A tamper that flips one character near the end of the first record's
/// ciphertext in `vault`'s pages (still valid base64url: the AEAD tag fails).
fn flip_first(vault: String) -> impl FnMut(&Exchange) -> Option<Reply> + Send + 'static {
    move |ex| {
        if ex.changes_vault()? != vault {
            return None;
        }
        let mut page = ex.json()?;
        let first = page["records"].as_array_mut()?.first_mut()?;
        let mut chars: Vec<char> = first["ciphertext"].as_str()?.chars().collect();
        let at = chars.len().checked_sub(12)?;
        chars[at] = if chars[at] == 'A' { 'B' } else { 'A' };
        let flipped: String = chars.into_iter().collect();
        first["ciphertext"] = json!(flipped);
        Some(Reply::json(&page))
    }
}

/// A tamper that leaves every record out of a changes page that has some
/// (the head stays: the page is cut short).
fn cut_short(ex: &Exchange) -> Option<Reply> {
    ex.changes_vault()?;
    let mut page = ex.json()?;
    if page["records"].as_array()?.is_empty() {
        return None;
    }
    page["records"] = json!([]);
    Some(Reply::json(&page))
}

/// A tamper that truncates (even rounds) or breaks the UTF-8 of (odd
/// rounds) a changes page with records, at a seeded position.
fn mangle(round: u64, pick: u64) -> impl FnMut(&Exchange) -> Option<Reply> + Send + 'static {
    move |ex| {
        ex.changes_vault()?;
        if ex.records().is_empty() {
            return None;
        }
        let len = u64::try_from(ex.body.len()).unwrap();
        let at = usize::try_from(1 + pick % (len - 1)).unwrap();
        let mut body = ex.body.clone();
        if round.is_multiple_of(2) {
            body.truncate(at);
        } else {
            body.insert(at, 0xFF);
        }
        Some(Reply { status: 200, body })
    }
}

/// What the engine recorded in the phone's store, read beside the core (the
/// core has no call for it): lag entries, vault flags, no-common-view
/// entries, and whether sync is halted. The core is closed meanwhile and
/// unlocked again after.
fn engine_facts(phone: &Phone, desktop: &Desktop) -> (usize, usize, usize, bool) {
    phone.core.close_store();
    let facts = {
        let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
        let Ok(session) = mosaic_vault::session::LockedSession::open(store)
            .unlock(&pw(PASSWORD), &desktop.secret_key)
        else {
            panic!("the store does not open beside the core");
        };
        let findings = session.sync_findings().unwrap();
        (
            session.sync_lag().unwrap().len(),
            findings.vault_flags.len(),
            findings.no_common_view.len(),
            findings.halt.is_some(),
        )
    };
    assert!(matches!(
        phone.core.reopen_store().unwrap(),
        SessionState::Locked { .. }
    ));
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    facts
}

// ---------------------------------------------------------------------------
// A malicious or broken server against the phone
// ---------------------------------------------------------------------------

/// The server rolls the vault back (a restored backup): its head is below
/// the phone's cursor. The phone flags the vault (`halted`), keeps its data,
/// and keeps a later edit queued instead of uploading it into the
/// rolled-back vault. The flag survives a lock and a process restart.
#[test]
fn g5b_rollback_below_the_cursor_is_flagged_and_keeps_local_data() {
    let mut w = World::new("g5b-rollback@example.com");
    let before = snapshot(&w.phone);
    w.server.set_tamper(|ex| {
        ex.changes_vault()?;
        let since = ex.since().filter(|s| *s > 0)?;
        let mut v = ex.json()?;
        v["headSeq"] = json!(since - 1);
        v["nextSince"] = json!(0);
        v["more"] = json!(false);
        v["records"] = json!([]);
        v["keys"] = json!([]);
        v["meta"] = Value::Null;
        v["rosters"] = json!([]);
        v["shares"] = json!([]);
        Some(Reply::json(&v))
    });
    let _ = sync(&w.server, &w.phone);
    assert!(halted(&w.phone), "{:?}", w.phone.core.sync_overview());
    assert_eq!(snapshot(&w.phone), before, "nothing local changed");

    let vault = w.phone.vault_id();
    phone_add(&w.phone, &vault, "After rollback", "pw-after-rollback");
    let _ = sync(&w.server, &w.phone);
    w.server.clear_tamper();
    let _ = sync(&w.server, &w.phone);
    let overview = w.phone.core.sync_overview();
    assert!(overview.halted, "the flag stays until acknowledged");
    assert!(
        overview.pending_uploads > 0,
        "the edit stays queued: {overview:?}"
    );
    assert!(titles(&w.phone).contains(&"After rollback".to_owned()));
    assert_consistent(&w.phone);
    restart(&mut w.phone);
    assert!(halted(&w.phone), "a new core still reports the flag");
    assert!(titles(&w.phone).contains(&"After rollback".to_owned()));
}

/// The server serves the phone an old copy of its own checkpoint (a rolled
/// back checkpoint table). The second strike halts sync with `syncFailed`
/// and `halted`; the halt survives an honest server, a lock and a restart,
/// and nothing local is lost.
#[test]
fn g5b_rolled_back_own_checkpoint_halts_and_the_halt_survives_restart() {
    let mut w = World::new("g5b-checkpoint@example.com");
    sync(&w.server, &w.phone).unwrap();
    sync(&w.server, &w.phone).unwrap();
    let before = snapshot(&w.phone);
    let old = w
        .server
        .exchanges()
        .into_iter()
        .find(|ex| ex.armed && ex.method == "GET" && ex.path == "/v1/checkpoints")
        .unwrap()
        .body;
    w.server.set_tamper(move |ex| {
        (ex.method == "GET" && ex.path == "/v1/checkpoints").then(|| Reply {
            status: 200,
            body: old.clone(),
        })
    });
    let mut failure = None;
    for _ in 0..4 {
        if let Err(e) = sync(&w.server, &w.phone) {
            failure = Some(e);
            break;
        }
    }
    assert_eq!(failure, Some(MobileError::SyncFailed));
    assert!(halted(&w.phone));
    w.server.clear_tamper();
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncFailed),
        "an honest server does not end the halt"
    );
    assert_eq!(snapshot(&w.phone), before);
    assert_consistent(&w.phone);
    restart(&mut w.phone);
    assert!(halted(&w.phone));
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncFailed)
    );
    assert_eq!(snapshot(&w.phone), before);
}

/// The server withholds the desktop's new login from the phone (a freeze)
/// but serves the desktop's checkpoint. The engine detects the lag and,
/// after the 10-minute grace, warns `FreezeSuspected`; the phone's
/// `sync_now` drops the run's report, and `SyncOverview` has no field for
/// it, so the user is never told.
#[test]
#[ignore = "G5b-1: a freeze (FreezeSuspected) is detected by the engine but never surfaced by sync_now/SyncOverview"]
fn g5b_freeze_is_surfaced_on_the_phone_after_the_grace() {
    let w = World::new("g5b-freeze@example.com");
    w.desktop
        .add_login("Withheld", "eve", "pw-withheld", "https://w.example");
    w.desktop.sync(&w.server);
    w.server.set_tamper(withheld);
    sync(&w.server, &w.phone).unwrap();
    w.phone.core.test_hooks().set_clock_offset_ms(11 * 60_000);
    sync(&w.server, &w.phone).unwrap();
    w.phone.core.test_hooks().set_clock_offset_ms(30 * 60_000);
    let overview = sync(&w.server, &w.phone).unwrap();
    assert!(!titles(&w.phone).contains(&"Withheld".to_owned()));
    assert_consistent(&w.phone);
    let facts = engine_facts(&w.phone, &w.desktop);
    assert!(facts.0 > 0, "the engine recorded no lag: {facts:?}");
    assert!(
        overview.halted || overview.last_error.is_some(),
        "a freeze past the grace must reach the user: {overview:?}; engine (lag, flags, no common view, halt) = {facts:?}"
    );
}

/// The server cuts a page short: the head moves past the desktop's new
/// login but the record is left out (a fork: the phone and the desktop are
/// shown different contents for the same head). The view digests differ,
/// and the phone reports it (`halted`) instead of silently missing the item.
#[test]
fn g5b_page_cut_short_is_a_fork_finding() {
    let w = World::new("g5b-fork@example.com");
    w.desktop
        .add_login("Skipped", "eve", "pw-skipped", "https://s.example");
    w.desktop.sync(&w.server);
    w.server.set_tamper(cut_short);
    let _ = sync(&w.server, &w.phone);
    w.server.clear_tamper();
    let _ = sync(&w.server, &w.phone);
    let shows = titles(&w.phone).contains(&"Skipped".to_owned());
    assert!(
        shows || halted(&w.phone),
        "a record the server skipped is either shown or reported"
    );
    assert!(halted(&w.phone), "{:?}", w.phone.core.sync_overview());
    assert_consistent(&w.phone);
}

/// After a finding (the fork above) the server is honest again. The
/// desktop offers an acknowledgement (`clear_vault_findings`), which
/// rebuilds the vault and fetches what was withheld. The phone core has no
/// such call: the vault stays flagged and the skipped login never arrives,
/// short of a device reset.
#[test]
#[ignore = "G5b-2: no clear_vault_findings/clear_sync_halt on the phone core; a flagged vault never recovers"]
fn g5b_a_finding_can_be_acknowledged_and_the_phone_recovers() {
    let w = World::new("g5b-ack@example.com");
    w.desktop
        .add_login("Skipped", "eve", "pw-skipped", "https://s.example");
    w.desktop.sync(&w.server);
    w.server.set_tamper(cut_short);
    let _ = sync(&w.server, &w.phone);
    w.server.clear_tamper();
    for _ in 0..3 {
        let _ = sync(&w.server, &w.phone);
    }
    assert!(
        titles(&w.phone).contains(&"Skipped".to_owned()) && !halted(&w.phone),
        "the phone must be able to recover from a finding: {:?}",
        w.phone.core.sync_overview()
    );
}

/// The server replays the desktop's first version of a login as a new
/// change. The phone never rolls the login back (the replay is dominated),
/// and the change the replay displaced is either shown or reported.
#[test]
fn g5b_replayed_old_record_does_not_roll_back_the_phone() {
    let w = World::new("g5b-replay@example.com");
    desk_set_password(&w.desktop, w.x, "pw-desk-2");
    w.desktop.sync(&w.server);
    sync(&w.server, &w.phone).unwrap();
    assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-2");
    let (kind, ciphertext) = w.server.first_upload(&w.x()).unwrap();
    w.desktop
        .add_login("Displaced", "eve", "pw-displaced", "https://d.example");
    w.desktop.sync(&w.server);
    w.server
        .set_tamper(swap_first(None, w.x(), kind, ciphertext));
    let _ = sync(&w.server, &w.phone);
    assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-2", "rolled back");
    w.server.clear_tamper();
    let _ = sync(&w.server, &w.phone);
    assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-2");
    assert!(titles(&w.phone).contains(&"Displaced".to_owned()) || halted(&w.phone));
    assert_consistent(&w.phone);
}

/// Reordered records, a duplicated record and a replayed page are refused
/// whole (`syncServerInvalid`): nothing of the page is applied, and the next
/// honest run delivers each record exactly once.
#[test]
fn g5b_reordered_duplicated_and_replayed_pages_are_refused_whole() {
    let w = World::new("g5b-order@example.com");
    w.desktop
        .add_login("R1", "eve", "pw-r1", "https://r1.example");
    w.desktop
        .add_login("R2", "eve", "pw-r2", "https://r2.example");
    w.desktop.sync(&w.server);
    let before = snapshot(&w.phone);

    w.server.set_tamper(|ex| {
        ex.changes_vault()?;
        let mut v = ex.json()?;
        let records = v["records"].as_array_mut()?;
        if records.len() < 2 {
            return None;
        }
        records.reverse();
        Some(Reply::json(&v))
    });
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncServerInvalid)
    );
    assert_eq!(snapshot(&w.phone), before, "a reordered page applied");

    w.server.set_tamper(|ex| {
        ex.changes_vault()?;
        let mut v = ex.json()?;
        let records = v["records"].as_array_mut()?;
        let first = records.first()?.clone();
        records.insert(0, first);
        Some(Reply::json(&v))
    });
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncServerInvalid)
    );
    assert_eq!(snapshot(&w.phone), before, "a duplicated record applied");

    w.server.clear_tamper();
    sync(&w.server, &w.phone).unwrap();
    let page = w
        .server
        .exchanges()
        .into_iter()
        .rev()
        .find(|ex| ex.armed && ex.changes_vault().is_some() && ex.records().len() >= 2)
        .unwrap()
        .body;
    w.server.set_tamper(move |ex| {
        ex.changes_vault()?;
        Some(Reply {
            status: 200,
            body: page.clone(),
        })
    });
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncServerInvalid),
        "a replayed page"
    );
    w.server.clear_tamper();
    sync(&w.server, &w.phone).unwrap();
    let mut t = titles(&w.phone);
    t.sort();
    assert_eq!(t, ["Desk login", "R1", "R2"]);
    assert!(!halted(&w.phone));
    assert_consistent(&w.phone);
}

/// Truncated, garbled (not UTF-8) and oversized answers (16 MiB + 1) fail
/// the run with `syncServerInvalid` and change nothing; a truncated answer
/// to an upload the server committed neither loses nor duplicates it.
#[test]
fn g5b_truncated_garbled_and_oversized_answers_change_nothing() {
    let w = World::new("g5b-garbled@example.com");
    desk_set_password(&w.desktop, w.x, "pw-desk-2");
    w.desktop.sync(&w.server);
    let mut rng = Rng(0x5EED_0005);
    for round in 0..6_u64 {
        w.server.set_tamper(mangle(round, rng.next()));
        assert_eq!(
            sync(&w.server, &w.phone).err(),
            Some(MobileError::SyncServerInvalid),
            "round {round}"
        );
        assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-1", "round {round}");
    }
    for path in ["changes", "checkpoints"] {
        w.server.set_tamper(move |ex| {
            let hit = match path {
                "changes" => ex.changes_vault().is_some(),
                _ => ex.method == "GET" && ex.path == "/v1/checkpoints",
            };
            hit.then(|| Reply {
                status: 200,
                body: vec![b' '; 16 * 1024 * 1024 + 1],
            })
        });
        assert_eq!(
            sync(&w.server, &w.phone).err(),
            Some(MobileError::SyncServerInvalid),
            "oversized {path}"
        );
        assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-1");
    }
    w.server.clear_tamper();
    sync(&w.server, &w.phone).unwrap();
    assert_eq!(password_on(&w.phone, &w.x()), "pw-desk-2");

    // The server commits an upload and its answer arrives cut in half.
    let vault = w.phone.vault_id();
    phone_add(&w.phone, &vault, "Phone login", "pw-phone-1");
    w.server.set_tamper(|ex| {
        ex.push_vault()?;
        let mut body = ex.body.clone();
        body.truncate(body.len() / 2);
        Some(Reply { status: 200, body })
    });
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncServerInvalid)
    );
    w.server.clear_tamper();
    let overview = sync(&w.server, &w.phone).unwrap();
    assert_eq!(overview.pending_uploads, 0);
    assert!(!overview.halted);
    w.desktop.sync(&w.server);
    let count = |t: Vec<String>| t.iter().filter(|t| *t == "Phone login").count();
    assert_eq!(count(desk_titles(&w.desktop)), 1);
    assert_eq!(count(titles(&w.phone)), 1);
    assert_consistent(&w.phone);
}

/// Records swapped in from another item of the vault, from another vault of
/// the account and from another account, and a record with a flipped
/// ciphertext byte, never open on the phone: no foreign value appears and no
/// item changes. The changes they displaced are either shown or reported.
#[test]
fn g5b_records_swapped_between_items_vaults_and_accounts_do_not_open() {
    let server = HostileServer::start();
    let desktop = Desktop::register(&server, "g5b-swap@example.com");
    let personal = desktop.vault();
    let login = desk_add_in(&desktop, personal, "Desk login", "pw-desk-1");
    let second = desk_add_in(&desktop, personal, "Second login", "pw-second");
    let work = desktop.with(|session| session.create_vault("Work").unwrap());
    let work_login = desk_add_in(&desktop, work, "Work login", "pw-work");
    desktop.sync(&server);
    let other = Desktop::register(&server, "g5b-swap-other@example.com");
    let foreign_login = other.add_login("Other account", "zed", "pw-other", "https://o.example");
    other.sync(&server);
    let phone = Phone::new();
    phone.join(&server, "g5b-swap@example.com", &desktop.sk_text());
    sync(&server, &phone).unwrap();
    let before = snapshot(&phone);
    assert_eq!(before.len(), 3);

    let foreign = |id: ItemId, as_id: ItemId| {
        let (kind, ciphertext) = server.first_upload(&id.to_string()).unwrap();
        (as_id.to_string(), kind, ciphertext)
    };
    let cases = [
        ("item", foreign(second, login)),
        ("vault", foreign(work_login, work_login)),
        ("account", foreign(foreign_login, foreign_login)),
    ];
    let vault = Some(personal.to_string());
    for (round, (label, (as_id, kind, ciphertext))) in cases.into_iter().enumerate() {
        desk_set_password(&desktop, login, &format!("pw-desk-{}", round + 2));
        desktop.sync(&server);
        server.set_tamper(swap_first(vault.clone(), as_id, kind, ciphertext));
        let _ = sync(&server, &phone);
        assert_eq!(
            snapshot(&phone),
            before,
            "a swapped record opened ({label})"
        );
    }
    desk_set_password(&desktop, login, "pw-desk-last");
    desktop.sync(&server);
    server.set_tamper(flip_first(personal.to_string()));
    let _ = sync(&server, &phone);
    assert_eq!(snapshot(&phone), before, "a flipped ciphertext opened");
    server.clear_tamper();
    let _ = sync(&server, &phone);
    assert!(
        password_on(&phone, &login.to_string()) == "pw-desk-last" || halted(&phone),
        "a displaced change is either delivered or reported"
    );
    assert!(
        !snapshot(&phone)
            .values()
            .any(|(title, password)| title == "Other account" || password == "pw-other"),
        "another account's record appeared"
    );
    assert_eq!(
        phone_vault_of(&phone, &work_login.to_string()),
        work.to_string()
    );
    assert_consistent(&phone);
}

// ---------------------------------------------------------------------------
// Phone lifecycle under sync
// ---------------------------------------------------------------------------

/// A lock, then a store close (iOS in the background), while a page with a
/// new record is on the wire: the run ends `locked`, nothing is half
/// applied, the phone's own queued edit stays queued, and after the unlock
/// (and the reopen) one honest run delivers both ways.
#[test]
fn g5b_lock_and_store_close_between_download_and_apply() {
    let w = World::new("g5b-cut@example.com");
    let vault = w.phone.vault_id();
    phone_add(&w.phone, &vault, "Phone pending", "pw-pending");
    w.desktop
        .add_login("Arrives later", "eve", "pw-later", "https://l.example");
    w.desktop.sync(&w.server);
    let page_with_records =
        |ex: &Exchange| ex.changes_vault().is_some() && !ex.records().is_empty();

    let before = snapshot(&w.phone);
    let result = sync_cut_at(&w.server, &w.phone, page_with_records, || {
        w.phone.core.lock();
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    assert!(matches!(w.phone.core.state(), SessionState::Locked { .. }));
    w.phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let after = snapshot(&w.phone);
    assert!(
        after == before || after.len() == before.len() + 1,
        "a page is applied whole or not at all"
    );
    assert!(w.phone.core.sync_overview().pending_uploads > 0);

    let result = sync_cut_at(&w.server, &w.phone, page_with_records, || {
        w.phone.core.close_store();
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    assert_eq!(w.phone.core.state(), SessionState::StoreClosed);
    assert!(matches!(
        w.phone.core.reopen_store().unwrap(),
        SessionState::Locked { .. }
    ));
    w.phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let overview = sync(&w.server, &w.phone).unwrap();
    assert_eq!(overview.pending_uploads, 0);
    assert!(!overview.halted);
    let mut t = titles(&w.phone);
    t.sort();
    assert_eq!(t, ["Arrives later", "Desk login", "Phone pending"]);
    w.desktop.sync(&w.server);
    let desk = desk_titles(&w.desktop);
    assert_eq!(desk.iter().filter(|t| *t == "Phone pending").count(), 1);
    assert_consistent(&w.phone);
}

/// The server commits the phone's upload and the process dies before the
/// answer is applied (the core is dropped). The new process still has the
/// change queued; its next run settles it without a duplicate.
#[test]
fn g5b_process_death_after_the_server_committed_an_upload() {
    let mut w = World::new("g5b-death-push@example.com");
    let vault = w.phone.vault_id();
    let id = phone_add(&w.phone, &vault, "Phone death", "pw-death");
    let result = sync_cut_at(
        &w.server,
        &w.phone,
        |ex| ex.push_vault().is_some(),
        || w.phone.core.close_store(),
    );
    assert_eq!(result.err(), Some(MobileError::Locked));
    restart(&mut w.phone);
    assert_eq!(password_on(&w.phone, &id), "pw-death");
    let overview = sync(&w.server, &w.phone).unwrap();
    assert_eq!(overview.pending_uploads, 0);
    assert!(!overview.halted);
    w.desktop.sync(&w.server);
    let desk = desk_snapshot(&w.desktop);
    assert_eq!(
        desk.get(&id),
        Some(&("Phone death".to_owned(), "pw-death".to_owned()))
    );
    assert_eq!(desk.values().filter(|(t, _)| t == "Phone death").count(), 1);
    sync(&w.server, &w.phone).unwrap();
    assert_eq!(snapshot(&w.phone), desk, "the phone and the desktop agree");
    assert_consistent(&w.phone);
}

/// The server accepts the phone's checkpoint and the process dies before
/// the phone records it as published. The server now holds a counter the
/// phone never confirmed: that is not a rollback and not a duplicated
/// install, so later runs neither strike nor halt.
#[test]
fn g5b_process_death_after_a_checkpoint_upload_is_not_a_rollback() {
    let mut w = World::new("g5b-death-cp@example.com");
    let result = sync_cut_at(
        &w.server,
        &w.phone,
        |ex| ex.method == "PUT" && ex.path.starts_with("/v1/checkpoints/"),
        || w.phone.core.close_store(),
    );
    assert_eq!(result.err(), Some(MobileError::Locked));
    restart(&mut w.phone);
    for _ in 0..3 {
        let overview = sync(&w.server, &w.phone).unwrap();
        assert!(!overview.halted, "{overview:?}");
        assert_eq!(overview.last_error, None);
    }
    assert_consistent(&w.phone);
}

/// A sync while a join fetches its account (held at the server), and while
/// an unlock derives its key, is refused without side effects; both finish
/// normally and the next sync succeeds.
#[test]
fn g5b_sync_while_a_join_or_an_unlock_is_in_flight() {
    let server = HostileServer::start();
    let desktop = Desktop::register(&server, "g5b-inflight@example.com");
    desktop.add_login("Desk login", "bob", "pw-desk-1", "https://desk.example");
    desktop.sync(&server);
    let phone = Phone::new();
    let (reached, go) = server.hold(|ex| ex.method == "GET" && ex.path == "/v1/account");
    server.advance_ms(60_000);
    let request = mosaic_mobile::JoinHostedRequest {
        server_url: server.base().to_owned(),
        email: "g5b-inflight@example.com".to_owned(),
        password: PASSWORD.as_bytes().to_vec(),
        secret_key: desktop.sk_text().into_bytes(),
        device_name: None,
    };
    let joined = std::thread::scope(|scope| {
        let join = scope.spawn(|| phone.core.join_hosted(request));
        reached.recv_timeout(HOLD).expect("the join fetch was held");
        assert!(phone.core.test_hooks().join_in_flight());
        let during = phone.core.sync_now();
        assert!(
            matches!(
                during,
                Err(MobileError::NoAccount | MobileError::Busy | MobileError::Locked)
            ),
            "{during:?}"
        );
        go.send(()).unwrap();
        join.join().unwrap()
    });
    assert!(
        matches!(joined, Ok(mosaic_mobile::JoinOutcome::Unlocked(_))),
        "{joined:?}"
    );
    let overview = sync(&server, &phone).unwrap();
    assert_eq!(overview.last_error, None);
    assert_eq!(titles(&phone), ["Desk login"]);

    phone.core.lock();
    phone.core.test_hooks().set_work_delay_ms(500);
    let unlocked = std::thread::scope(|scope| {
        let unlock = scope.spawn(|| {
            phone
                .core
                .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        });
        let start = std::time::Instant::now();
        while phone.core.state() != SessionState::Busy {
            assert!(start.elapsed() < HOLD, "never busy");
            std::thread::yield_now();
        }
        assert_eq!(phone.core.sync_now().err(), Some(MobileError::Busy));
        unlock.join().unwrap()
    });
    phone.core.test_hooks().set_work_delay_ms(0);
    assert!(matches!(unlocked, Ok(SessionState::Unlocked { .. })));
    let overview = sync(&server, &phone).unwrap();
    assert_eq!(overview.last_error, None);
    assert_consistent(&phone);
}

/// The stored session token is no longer valid (another device rotated it,
/// or a broken server answered 401): the phone signs out, deletes the
/// token, makes no further request, and keeps its queued edit. Signing in
/// again with the (unchanged) password uploads the edit.
#[test]
fn g5b_unknown_token_signs_out_and_a_sign_in_uploads_the_edit() {
    let w = World::new("g5b-token@example.com");
    let vault = w.phone.vault_id();
    phone_add(&w.phone, &vault, "Pending edit", "pw-pending");
    {
        let mut map = w.phone.storage.map.lock().unwrap();
        for (name, value) in map.iter_mut() {
            if name.starts_with("sync-token/") {
                *value = vec![0xA5; 32];
            }
        }
    }
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncSignedOut)
    );
    let overview = w.phone.core.sync_overview();
    assert!(!overview.signed_in);
    assert!(overview.pending_uploads > 0);
    assert!(
        !w.phone
            .storage
            .names()
            .iter()
            .any(|n| n.starts_with("sync-token/"))
    );
    let requests = w.server.request_count();
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncSignedOut)
    );
    assert_eq!(
        w.server.request_count(),
        requests,
        "no request without a token"
    );
    assert_consistent(&w.phone);

    w.server.advance_ms(60_000);
    let overview = w
        .phone
        .core
        .recover_password(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert!(overview.signed_in);
    let overview = sync(&w.server, &w.phone).unwrap();
    assert_eq!(overview.pending_uploads, 0);
    w.desktop.sync(&w.server);
    assert!(desk_titles(&w.desktop).contains(&"Pending edit".to_owned()));
}

/// The desktop removes the phone from the account. The phone's next run
/// reports `syncSignedOut`, deletes the token and keeps its local edit
/// readable across a lock.
#[test]
fn g5b_device_removed_elsewhere_signs_out_and_keeps_local_edits() {
    let w = World::new("g5b-removed@example.com");
    let vault = w.phone.vault_id();
    phone_add(&w.phone, &vault, "Unsent", "pw-unsent");
    let mut engine = w.desktop.engine(&w.server);
    let devices = engine.transport_mut().devices().unwrap();
    let phone_install = devices
        .iter()
        .map(|d| d.device_id)
        .find(|d| *d != w.desktop.install)
        .unwrap();
    w.server.advance_ms(60_000);
    let grant = mosaic_sync::http::hosted::login(
        &w.desktop.base,
        &w.desktop.email,
        w.desktop.account,
        &w.desktop.opw(),
        w.desktop.install,
        "Desktop",
    )
    .unwrap();
    mosaic_sync::http::hosted::device_remove(&w.desktop.base, &grant.token, phone_install).unwrap();
    assert_eq!(
        sync(&w.server, &w.phone).err(),
        Some(MobileError::SyncSignedOut)
    );
    let overview = w.phone.core.sync_overview();
    assert!(!overview.signed_in);
    assert!(overview.pending_uploads > 0);
    assert!(titles(&w.phone).contains(&"Unsent".to_owned()));
    assert_consistent(&w.phone);
    assert!(titles(&w.phone).contains(&"Unsent".to_owned()));
}

// ---------------------------------------------------------------------------
// Three devices
// ---------------------------------------------------------------------------

/// Syncs every device once, in the order `order` (0 desktop, 1 and 2 the
/// phones).
fn deliver(w: &World, p2: &Phone, order: &[usize]) {
    for d in order {
        match d {
            0 => w.desktop.sync(&w.server),
            1 => {
                sync(&w.server, &w.phone).unwrap();
            }
            _ => {
                sync(&w.server, p2).unwrap();
            }
        }
    }
}

/// Full delivery: three rounds over all devices.
fn settle(w: &World, p2: &Phone) {
    for _ in 0..3 {
        deliver(w, p2, &[0, 1, 2]);
    }
}

fn assert_converged(w: &World, p2: &Phone) {
    let desk = desk_snapshot(&w.desktop);
    assert_eq!(snapshot(&w.phone), desk, "phone 1 and the desktop differ");
    assert_eq!(snapshot(p2), desk, "phone 2 and the desktop differ");
    for phone in [&w.phone, p2] {
        let overview = phone.core.sync_overview();
        assert!(!overview.halted, "{overview:?}");
        assert_eq!(overview.last_error, None);
        assert_eq!(overview.pending_uploads, 0);
    }
}

/// The same login edited concurrently on the desktop and on two phones whose
/// clocks are days apart (one behind, one ahead), delivered in seeded
/// orders: all three converge on one value and every edit is still held
/// (shown, a conflict loser or in the history). No false finding.
#[test]
fn g5b_three_devices_edit_one_login_with_clock_skew() {
    for seed in [3_u64, 11] {
        let w = World::new(&format!("g5b-three-{seed}@example.com"));
        let p2 = w.second_phone();
        w.phone.core.test_hooks().set_clock_offset_ms(-3 * DAY_MS);
        p2.core.test_hooks().set_clock_offset_ms(4 * DAY_MS);
        let vault = w.phone.vault_id();
        phone_set_password(&w.phone, &vault, &w.x(), "pw-p1");
        phone_set_password(&p2, &vault, &w.x(), "pw-p2");
        desk_set_password(&w.desktop, w.x, "pw-d");
        let mut rng = Rng(seed);
        let mut order = [0_usize, 1, 2];
        for i in (1..order.len()).rev() {
            let j = usize::try_from(rng.below(u64::try_from(i).unwrap() + 1)).unwrap();
            order.swap(i, j);
        }
        deliver(&w, &p2, &order);
        settle(&w, &p2);
        assert_converged(&w, &p2);
        let held = desk_values(&w.desktop, w.x);
        for value in ["pw-desk-1", "pw-p1", "pw-p2", "pw-d"] {
            assert!(held.contains(value), "seed {seed}: {value} lost");
        }
    }
}

/// Delete and move versus edit: the desktop deletes a login that phone 1
/// edits meanwhile (the delete loses, the edit is kept everywhere), moves a
/// login to another vault while phone 2 edits it (the edit is kept), and
/// deletes a third login nobody touches (it stays deleted on all devices,
/// even after the phone that still held it syncs again).
#[test]
fn g5b_delete_and_move_versus_edit_on_three_devices() {
    let w = World::new("g5b-delete@example.com");
    let personal = w.desktop.vault();
    let doomed = desk_add_in(&w.desktop, personal, "Doomed", "pw-doomed");
    let mover = desk_add_in(&w.desktop, personal, "Mover", "pw-mover");
    let work = w.desktop.with(|s| s.create_vault("Work").unwrap());
    w.desktop.sync(&w.server);
    sync(&w.server, &w.phone).unwrap();
    let p2 = w.second_phone();
    let vault = personal.to_string();

    phone_set_password(&w.phone, &vault, &w.x(), "pw-p1-edit");
    phone_set_password(&p2, &vault, &mover.to_string(), "pw-p2-edit");
    w.desktop.with(|s| {
        s.purge_item(personal, w.x).unwrap();
        s.purge_item(personal, doomed).unwrap();
        s.move_item(personal, mover, work).unwrap();
    });
    deliver(&w, &p2, &[0, 2, 1]);
    settle(&w, &p2);
    assert_converged(&w, &p2);
    let desk = desk_snapshot(&w.desktop);
    assert_eq!(
        desk.get(&w.x()).map(|(_, p)| p.as_str()),
        Some("pw-p1-edit"),
        "an edit concurrent with a delete is kept: {desk:?}"
    );
    assert!(
        desk.values().any(|(_, p)| p == "pw-p2-edit"),
        "an edit concurrent with a move is kept: {desk:?}"
    );
    assert!(
        !desk.contains_key(&doomed.to_string()),
        "a delete without a concurrent edit stays"
    );
    settle(&w, &p2);
    assert!(
        !snapshot(&p2).contains_key(&doomed.to_string()),
        "resurrected"
    );
    assert_consistent(&w.phone);
}

/// The desktop rotates the vault key while phone 1 is offline with an edit
/// under the old key, then writes under the new key. Phone 1 adopts the new
/// key, re-seals and uploads its edit; phone 2 (offline throughout) catches
/// up. Nothing is lost and nobody halts.
#[test]
fn g5b_vault_key_rotation_while_a_phone_is_offline() {
    let w = World::new("g5b-rotate@example.com");
    let p2 = w.second_phone();
    let vault = w.phone.vault_id();
    phone_set_password(&w.phone, &vault, &w.x(), "pw-offline-edit");
    let personal = w.desktop.vault();
    let mut engine = w.desktop.engine(&w.server);
    let mut access = w.desktop.owned.clone();
    engine
        .rotate_vault(&mut access, &SystemClock, personal)
        .unwrap();
    let after = desk_add_in(&w.desktop, personal, "After rotation", "pw-after");
    w.desktop.sync(&w.server);
    settle(&w, &p2);
    assert_converged(&w, &p2);
    let desk = desk_snapshot(&w.desktop);
    assert_eq!(desk[&w.x()].1, "pw-offline-edit");
    assert_eq!(desk[&after.to_string()].1, "pw-after");
    for phone in [&w.phone, &p2] {
        assert!(phone.core.vaults().unwrap().iter().all(|v| v.available));
    }
}

/// Seeded random schedules over three devices (model-checker style): edits
/// of two logins on any device, syncs of any device, phones with clocks days
/// apart. After full delivery all devices agree and every value any device
/// committed is still held by its login.
#[test]
fn g5b_seeded_three_device_schedules_converge_without_loss() {
    for seed in [7_u64, 19] {
        let w = World::new(&format!("g5b-schedule-{seed}@example.com"));
        let second = w
            .desktop
            .add_login("Second", "bob", "pw-second-1", "https://second.example");
        w.desktop.sync(&w.server);
        sync(&w.server, &w.phone).unwrap();
        let p2 = w.second_phone();
        let mut rng = Rng(seed);
        let skew = |rng: &mut Rng| (i64::try_from(rng.below(9)).unwrap() - 4) * DAY_MS;
        w.phone
            .core
            .test_hooks()
            .set_clock_offset_ms(skew(&mut rng));
        p2.core.test_hooks().set_clock_offset_ms(skew(&mut rng));
        let items = [w.x, second];
        let mut written: [BTreeSet<String>; 2] = [
            BTreeSet::from(["pw-desk-1".to_owned()]),
            BTreeSet::from(["pw-second-1".to_owned()]),
        ];
        let vault = w.phone.vault_id();
        let mut schedule = Vec::new();
        for step in 0..12 {
            let device = usize::try_from(rng.below(3)).unwrap();
            let op = rng.below(3);
            if op == 2 {
                schedule.push(format!("sync{device}"));
                deliver(&w, &p2, &[device]);
                continue;
            }
            let k = usize::try_from(op).unwrap();
            let value = format!("pw-s{seed}-{step}-d{device}");
            schedule.push(format!("edit{device}:{k}"));
            match device {
                0 => desk_set_password(&w.desktop, items[k], &value),
                1 => phone_set_password(&w.phone, &vault, &items[k].to_string(), &value),
                _ => phone_set_password(&p2, &vault, &items[k].to_string(), &value),
            }
            written[k].insert(value);
        }
        settle(&w, &p2);
        assert_converged(&w, &p2);
        for (k, id) in items.iter().enumerate() {
            let held = desk_values(&w.desktop, *id);
            let lost: Vec<&String> = written[k]
                .iter()
                .filter(|value| !held.contains(*value))
                .collect();
            assert!(
                lost.is_empty(),
                "seed {seed} schedule {schedule:?}: lost {lost:?}"
            );
        }
    }
}

/// Three honest rounds in which the desktop and phone 2 both write before
/// phone 1 syncs, so phone 1's head is one ahead of the desktop's checkpoint
/// at each of its runs (nothing is withheld). Phone 1's clock moves to
/// `offsets` before each run. Every run must stay free of findings.
fn heads_apart_rounds(email: &str, offsets: [i64; 3]) {
    let w = World::new(email);
    let p2 = w.second_phone();
    let vault = w.phone.vault_id();
    let second = w
        .desktop
        .add_login("Second", "bob", "pw-second-1", "https://second.example");
    w.desktop.sync(&w.server);
    sync(&w.server, &p2).unwrap();
    for (round, offset) in offsets.into_iter().enumerate() {
        desk_set_password(&w.desktop, w.x, &format!("pw-d-{round}"));
        w.desktop.sync(&w.server);
        phone_set_password(&p2, &vault, &second.to_string(), &format!("pw-p2-{round}"));
        sync(&w.server, &p2).unwrap();
        w.phone.core.test_hooks().set_clock_offset_ms(offset);
        let overview = sync(&w.server, &w.phone).unwrap();
        assert!(
            !overview.halted,
            "round {round}: {overview:?}; engine (lag, flags, no common view, halt) = {:?}",
            engine_facts(&w.phone, &w.desktop)
        );
    }
    settle(&w, &p2);
    assert_converged(&w, &p2);
}

/// The control of the next scenario: with the phone's clock steady, heads
/// that are apart at every run raise nothing.
#[test]
fn g5b_heads_apart_without_a_clock_jump_raise_no_finding() {
    heads_apart_rounds("g5b-apart@example.com", [0, 0, 0]);
}

/// The same honest rounds with phone 1's clock jumping three days forward
/// between its runs (a phone whose clock was wrong and is corrected). The
/// "heads that never meet" rule measures its 48 hours by the phone's clock,
/// so two runs a moment apart in real time span 72 hours, and phone 1
/// records a no-common-view finding against the honest desktop: a false
/// finding that sets `halted` (and, with G5b-2, never clears on the phone).
#[test]
#[ignore = "G5b-3: a forward clock jump of days raises a false NoCommonView finding between honest devices"]
fn g5b_a_clock_jump_with_heads_apart_raises_no_false_finding() {
    heads_apart_rounds("g5b-jump@example.com", [0, 3 * DAY_MS, 6 * DAY_MS]);
}

//! Tests of the origin index: it follows every kind of change, is dropped at lock, and above
//! all gives the same answers as a full scan (`origin_index.rs` explains why it may).

use std::cell::RefCell;
use std::collections::BTreeSet;

use mosaic_core::model::{Item, ItemType, LoginUrl, UrlMatch};
use mosaic_core::{Argon2Profile, DeviceId, ItemId, MasterPassword, SecretKey, Timestamp, VaultId};
use mosaic_origin::app::{AndroidAppUrl, app_fill_allowed};
use mosaic_origin::{
    HttpPolicy, MatchKind, PageOrigin, Refusal, fill_allowed_with, lookalike_of, saved_origin,
};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;

use super::sync::RemoteRecord;
use super::{ItemFilter, ItemRef, LockedSession, OriginKey, TrashFilter, UnlockedSession};
use crate::storage::{RecordBody, Store};

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(Store::open_in_memory().unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

fn add_login(s: &mut UnlockedSession, vault: VaultId, urls: &[&str]) -> ItemId {
    s.add_item(vault, &ItemType::Login, |e| {
        e.set_title("login")?;
        for url in urls {
            e.add_url(&LoginUrl::new(url))?;
        }
        Ok(())
    })
    .unwrap()
}

fn page(origin: &str) -> PageOrigin {
    PageOrigin::parse(origin).unwrap()
}

fn found(s: &UnlockedSession, origin: &str, filter: &ItemFilter) -> Vec<ItemId> {
    s.items_for_origin(&OriginKey::web(&page(origin)), filter)
        .into_iter()
        .map(|r| r.item_id)
        .collect()
}

#[test]
fn the_index_is_built_by_the_first_lookup_and_follows_edits() {
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let a = add_login(&mut s, vault, &["https://example.com"]);
    let all = ItemFilter::default();
    assert!(!s.origin_index_is_built());
    assert_eq!(found(&s, "https://example.com", &all), [a]);
    assert!(s.origin_index_is_built());

    // put_item (add and update): a new login is found, an edited URL moves the login.
    let b = add_login(&mut s, vault, &["https://www.example.com/login"]);
    assert_eq!(found(&s, "https://example.com", &all), [a, b]);
    let other = LoginUrl::new("https://other.org");
    let (url_id, _) = s.item(vault, a).unwrap().urls().into_iter().next().unwrap();
    s.update_item(vault, a, |e| e.set_url(&url_id, &other))
        .unwrap();
    assert_eq!(found(&s, "https://example.com", &all), [b]);
    assert_eq!(found(&s, "https://other.org", &all), [a]);
    // A web key never finds an item that is only saved under another registrable domain.
    assert!(found(&s, "https://example.org", &all).is_empty());

    // A purge removes it.
    s.purge_item(vault, b).unwrap();
    assert!(found(&s, "https://example.com", &all).is_empty());
}

#[test]
fn a_received_record_is_found() {
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let all = ItemFilter::default();
    assert!(found(&s, "https://remote.example", &all).is_empty());
    let id = ItemId::new_v7().unwrap();
    let device = DeviceId::new_v7().unwrap();
    let at = Timestamp::from_unix_millis(1);
    let mut item = Item::new(&ItemType::Login, device, at).unwrap();
    item.edit(device, at)
        .add_url(&LoginUrl::new("https://remote.example"))
        .unwrap();
    let vk = s.vaults.get(&vault).unwrap().current_key().unwrap();
    let rec = RemoteRecord {
        vault_id: vault,
        record_id: id,
        body: RecordBody::Item(item.seal(vk, &vault, &id).unwrap()),
        server_seq: 1,
    };
    s.apply_remote_record(&rec).unwrap();
    assert_eq!(found(&s, "https://remote.example", &all), [id]);
    assert_eq!(
        s.lookalike_of_saved("remote.exampel", true, |_| true)
            .as_deref(),
        Some("remote.example")
    );
}

#[test]
fn a_trashed_item_is_only_found_when_the_filter_asks_for_it() {
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let a = add_login(&mut s, vault, &["https://example.com"]);
    let all = ItemFilter::default();
    let with_trash = ItemFilter {
        trash: TrashFilter::Include,
        ..ItemFilter::default()
    };
    assert_eq!(found(&s, "https://example.com", &all), [a]);

    s.trash_item(vault, a).unwrap();
    assert!(found(&s, "https://example.com", &all).is_empty());
    assert_eq!(found(&s, "https://example.com", &with_trash), [a]);
    // A trashed login is not a saved host to imitate.
    assert_eq!(s.lookalike_of_saved("exampel.com", true, |_| true), None);

    s.restore_item(vault, a).unwrap();
    assert_eq!(found(&s, "https://example.com", &all), [a]);
    assert_eq!(
        s.lookalike_of_saved("exampel.com", true, |_| true)
            .as_deref(),
        Some("example.com")
    );
}

#[test]
fn locking_drops_the_index() {
    let (mut s, key) = account();
    let vault = s.create_vault("Personal").unwrap();
    let a = add_login(&mut s, vault, &["https://example.com"]);
    assert_eq!(
        found(&s, "https://example.com", &ItemFilter::default()),
        [a]
    );
    assert!(s.origin_index_is_built());

    let s = s.lock().unlock(&pw(), &key).unwrap();
    assert!(!s.origin_index_is_built());
    assert_eq!(
        found(&s, "https://example.com", &ItemFilter::default()),
        [a]
    );
    assert!(s.origin_index_is_built());
}

#[test]
fn app_links_and_look_alike_hosts_are_indexed() {
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let cert = AndroidAppUrl::new("com.example.app", Some([7; 32]))
        .unwrap()
        .canonical();
    let app = add_login(&mut s, vault, &[cert.as_str()]);
    let unbound = add_login(&mut s, vault, &["androidapp://com.example.app"]);
    let web = add_login(&mut s, vault, &["https://github.com"]);
    let all = ItemFilter::default();
    let ids = |key: &OriginKey| -> Vec<ItemId> {
        s.items_for_origin(key, &all)
            .into_iter()
            .map(|r| r.item_id)
            .collect()
    };
    // Both app URLs are candidates (the cert rule is the caller's check); a web page is not.
    assert_eq!(ids(&OriginKey::app("com.example.app")), [app, unbound]);
    assert!(ids(&OriginKey::app("com.example.other")).is_empty());
    assert_eq!(ids(&OriginKey::web(&page("https://github.com"))), [web]);
    assert_eq!(
        s.lookalike_of_saved("githuh.com", true, |_| true)
            .as_deref(),
        Some("github.com")
    );
    // The vault filter of the mobile core.
    assert_eq!(
        s.lookalike_of_saved("githuh.com", false, |v| v != vault),
        None
    );
}

// The property test: random vaults, random pages and apps.

const HOSTS: &[&str] = &[
    "example.com",
    "www.example.com",
    "login.example.com",
    "EXAMPLE.COM",
    "example.com.",
    "example.co.uk",
    "example.co",
    "exampel.com",
    "exmple.com",
    "examplee.com",
    "github.com",
    "githuh.com",
    "github.io",
    "a.github.io",
    "b.github.io",
    "kawasaki.jp",
    "a.kawasaki.jp",
    "localhost",
    "app.localhost",
    "127.0.0.1",
    "10.0.0.1",
    "[::1]",
    "[2001:db8::1]",
    "xn--mnchen-3ya.de",
    "m\u{fc}nchen.de",
    "xn--mnchen-3yb.de",
    "com",
    "co.uk",
    "nolabel",
];
const SCHEMES: &[&str] = &["https://", "http://", "", "HTTPS://", "ftp://", "file:///"];
const PORTS: &[&str] = &["", "", ":8443", ":443", ":80", ":3000"];
const PATHS: &[&str] = &["", "", "/login", "/a?b=c#d", "/"];
const ODD: &[&str] = &[
    "",
    "   ",
    "not a url",
    "https://",
    "file:///etc/hosts",
    "winapp:com.example.app",
    "androidapp://com.example.app?cert=zzzz",
    "androidapp://bad package?cert=AAAA",
    "javascript:alert(1)",
    "mailto:a@example.com",
    "http://[::1]:3000/",
];
const PACKAGES: &[&str] = &[
    "com.example.app",
    "com.example.other",
    "org.mozilla.firefox",
];

#[derive(Clone, Debug)]
struct UrlSpec {
    text: String,
    etld1: bool,
    allow_http: bool,
}

#[derive(Clone, Debug)]
struct ItemSpec {
    login: bool,
    trashed: bool,
    vault: usize,
    urls: Vec<UrlSpec>,
}

fn url_text() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => (select(SCHEMES), select(HOSTS), select(PORTS), select(PATHS))
            .prop_map(|(s, h, p, t)| format!("{s}{h}{p}{t}")),
        2 => select(ODD).prop_map(str::to_owned),
        2 => (select(PACKAGES), any::<bool>(), 1u8..=2).prop_map(|(package, bound, cert)| {
            AndroidAppUrl::new(package, bound.then_some([cert; 32]))
                .unwrap()
                .canonical()
        }),
    ]
}

fn item_spec() -> impl Strategy<Value = ItemSpec> {
    let url =
        (url_text(), any::<bool>(), any::<bool>()).prop_map(|(text, etld1, allow_http)| UrlSpec {
            text,
            etld1,
            allow_http,
        });
    (
        proptest::bool::weighted(0.85),
        proptest::bool::weighted(0.15),
        0usize..2,
        vec(url, 0..4),
    )
        .prop_map(|(login, trashed, vault, urls)| ItemSpec {
            login,
            trashed,
            vault,
            urls,
        })
}

/// A page origin built from a saved-URL spelling, so odd spellings (uppercase, IDN, a trailing
/// dot) reach `saved_origin` first. `None` when it is no `http(s)` origin.
fn page_origin() -> impl Strategy<Value = Option<PageOrigin>> {
    (
        select(&["http://", "https://"][..]),
        select(HOSTS),
        select(PORTS),
    )
        .prop_map(|(scheme, host, port)| saved_origin(&format!("{scheme}{host}{port}")))
}

/// The full scan's verdict of one item, as `match_item` of the mobile core and `best_match` of
/// the desktop decide it (the same `mosaic-origin` calls).
#[derive(Clone)]
enum Target {
    Web(PageOrigin, HttpPolicy),
    App(String, [u8; 32]),
}

fn verdict(item: &Item, target: &Target, shared: bool) -> Result<MatchKind, Refusal> {
    let mut insecure = false;
    let mut best: Option<MatchKind> = None;
    for (_, url) in item.urls() {
        let one = match target {
            Target::Web(page, policy) => fill_allowed_with(&url, page, *policy),
            Target::App(..) if shared => Err(Refusal::NoMatch),
            Target::App(package, cert) => app_fill_allowed(&url, package, &[*cert]),
        };
        match one {
            Ok(MatchKind::ExactOrigin) => best = Some(MatchKind::ExactOrigin),
            Ok(kind) => best = best.or(Some(kind)),
            Err(Refusal::Insecure) => insecure = true,
            Err(Refusal::NoMatch) => {}
        }
    }
    best.ok_or(if insecure {
        Refusal::Insecure
    } else {
        Refusal::NoMatch
    })
}

type Verdicts = Vec<(VaultId, ItemId, Result<MatchKind, Refusal>)>;

/// The logins of `refs` that match at all, with their verdict.
fn scan_verdicts<'a>(
    refs: impl IntoIterator<Item = ItemRef<'a>>,
    target: &Target,
    shared: &BTreeSet<VaultId>,
) -> Verdicts {
    refs.into_iter()
        .filter(|r| r.item.item_type() == Some(ItemType::Login))
        .filter_map(
            |r| match verdict(r.item, target, shared.contains(&r.vault_id)) {
                Err(Refusal::NoMatch) => None,
                v => Some((r.vault_id, r.item_id, v)),
            },
        )
        .collect()
}

fn saved_hosts(refs: impl IntoIterator<Item = ItemRef<'_>>) -> Vec<String> {
    refs.into_iter()
        .flat_map(|r| r.item.urls())
        .filter_map(|(_, u)| saved_origin(u.url.as_str()).map(|o| o.host_str()))
        .collect()
}

fn add_spec(s: &mut UnlockedSession, vault: VaultId, spec: &ItemSpec) -> ItemId {
    let item_type = if spec.login {
        ItemType::Login
    } else {
        ItemType::SecureNote
    };
    let id = s
        .add_item(vault, &item_type, |e| {
            e.set_title("t")?;
            for url in &spec.urls {
                let mut saved = LoginUrl::new(&url.text);
                if url.etld1 {
                    saved.match_mode = UrlMatch::Etld1;
                }
                saved.allow_http = url.allow_http;
                e.add_url(&saved)?;
            }
            Ok(())
        })
        .unwrap();
    if spec.trashed {
        s.trash_item(vault, id).unwrap();
    }
    id
}

struct Fixture {
    s: UnlockedSession,
    vaults: [VaultId; 2],
}

thread_local! {
    // One session for all cases (the account's key derivation is slow); every case purges its items.
    static FIXTURE: RefCell<Fixture> = RefCell::new({
        let (mut s, _) = account();
        let vaults = [s.create_vault("A").unwrap(), s.create_vault("B").unwrap()];
        Fixture { s, vaults }
    });
}

fn check_web(
    s: &UnlockedSession,
    page: &PageOrigin,
    shared: &BTreeSet<VaultId>,
    usable_vault: VaultId,
) -> Result<(), TestCaseError> {
    let default = ItemFilter::default();
    for policy in [
        HttpPolicy::LoopbackExempt,
        HttpPolicy::LoopbackNeedsAllowHttp,
    ] {
        let target = Target::Web(page.clone(), policy);
        let full = scan_verdicts(s.items(&default), &target, shared);
        let indexed = scan_verdicts(
            s.items_for_origin(&OriginKey::web(page), &default),
            &target,
            shared,
        );
        prop_assert_eq!(full, indexed, "page {} {:?}", page.serialize(), policy);
    }
    // The warnings: the desktop looks at the logins, the mobile core at every item of the
    // usable vaults.
    let host = page.host_str();
    let logins = saved_hosts(
        s.items(&default)
            .into_iter()
            .filter(|r| r.item.item_type() == Some(ItemType::Login)),
    );
    let full = lookalike_of(&host, logins.iter().map(String::as_str)).map(str::to_owned);
    prop_assert_eq!(
        full,
        s.lookalike_of_saved(&host, true, |_| true),
        "{}",
        host
    );
    let every = saved_hosts(
        s.items(&default)
            .into_iter()
            .filter(|r| r.vault_id == usable_vault),
    );
    let full = lookalike_of(&host, every.iter().map(String::as_str)).map(str::to_owned);
    prop_assert_eq!(
        full,
        s.lookalike_of_saved(&host, false, |v| v == usable_vault),
        "{}",
        host
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Indexed candidates followed by the existing checks equal the full scan, for web pages
    /// under both HTTP policies, for Android apps (a vault of shared items among them) and for
    /// the look-alike warning.
    #[test]
    fn indexed_candidates_equal_the_full_scan(
        specs in vec(item_spec(), 0..8),
        pages in vec(page_origin(), 1..6),
        package in select(PACKAGES),
        cert in 1u8..=2,
    ) {
        let outcome = FIXTURE.with(|f| {
            let mut f = f.borrow_mut();
            let vaults = f.vaults;
            let ids: Vec<(VaultId, ItemId)> = specs
                .iter()
                .map(|spec| (vaults[spec.vault], add_spec(&mut f.s, vaults[spec.vault], spec)))
                .collect();
            let shared = BTreeSet::from([vaults[1]]);
            let mut outcome: Result<(), TestCaseError> = Ok(());
            for page in pages.iter().flatten() {
                outcome = outcome.and_then(|()| check_web(&f.s, page, &shared, vaults[0]));
            }
            let target = Target::App(package.to_owned(), [cert; 32]);
            let default = ItemFilter::default();
            let full = scan_verdicts(f.s.items(&default), &target, &shared);
            let indexed = scan_verdicts(
                f.s.items_for_origin(&OriginKey::app(package), &default),
                &target,
                &shared,
            );
            let outcome = outcome.and_then(|()| {
                prop_assert_eq!(full, indexed, "app {}", package);
                Ok(())
            });
            for (vault, id) in ids {
                f.s.purge_item(vault, id).unwrap();
            }
            outcome
        });
        outcome?;
    }
}

/// `cargo test -p mosaic-vault --release origin_index_bench -- --ignored --nocapture`
#[test]
#[ignore = "timing test"]
fn origin_index_bench_at_10000_items() {
    use std::time::Instant;

    let (mut s, _) = account();
    let vault = s.create_vault("Big").unwrap();
    s.batch(|s| {
        for i in 0..10_000 {
            let a = format!("https://site{i}.example.com/login");
            let b = format!("https://www.shop{i}.org");
            s.add_item(vault, &ItemType::Login, |e| {
                e.set_title("t")?;
                e.add_url(&LoginUrl::new(&a))?;
                e.add_url(&LoginUrl::new(&b))?;
                Ok(())
            })?;
        }
        Ok(())
    })
    .unwrap();
    let target = Target::Web(
        page("https://site4242.example.com"),
        HttpPolicy::LoopbackExempt,
    );
    let key = OriginKey::web(&page("https://site4242.example.com"));
    let default = ItemFilter::default();
    let shared = BTreeSet::new();

    let start = Instant::now();
    let full = scan_verdicts(s.items(&default), &target, &shared);
    let scan = start.elapsed();
    let start = Instant::now();
    let first = scan_verdicts(s.items_for_origin(&key, &default), &target, &shared);
    let build = start.elapsed();
    let start = Instant::now();
    let again = scan_verdicts(s.items_for_origin(&key, &default), &target, &shared);
    let warm = start.elapsed();
    assert_eq!(full, first);
    assert_eq!(full, again);

    let start = Instant::now();
    let scan_warning = {
        let hosts = saved_hosts(s.items(&default));
        lookalike_of("site4242.exampel.com", hosts.iter().map(String::as_str)).map(str::to_owned)
    };
    let scan_warning_time = start.elapsed();
    let start = Instant::now();
    let warm_warning = s.lookalike_of_saved("site4242.exampel.com", false, |_| true);
    let warm_warning_time = start.elapsed();
    assert_eq!(scan_warning, warm_warning);
    eprintln!(
        "10000 items: candidates full scan {scan:?}, first lookup (builds) {build:?}, warm {warm:?}; \
         look-alike full scan {scan_warning_time:?}, warm {warm_warning_time:?}"
    );
}

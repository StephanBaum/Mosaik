//! `analyze` over real `mosaic_core` items.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    reason = "integration tests"
)]

use mosaic_core::model::{
    CustomField, CustomFieldValue, Item, ItemType, LoginUrl, OtpConfig, PasskeyCredential,
    SecretField, Text, TextField,
};
use mosaic_core::{DeviceId, ItemId, SecretVec, Timestamp, VaultId};
use mosaic_insights::{
    Action, Candidate, Counts, FindingKind, ItemKey, Options, Report, Severity, analyze,
};
use proptest::prelude::*;

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_800_000_000_000;
const STRONG: &str = "Vw9!rT2#kL5$pM8&nQ4*";

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

struct Fixture {
    vault: VaultId,
    device: DeviceId,
    items: Vec<(ItemId, Item)>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            vault: VaultId::new_v7().unwrap(),
            device: DeviceId::new_v7().unwrap(),
            items: Vec::new(),
        }
    }

    fn push(&mut self, item: Item) -> ItemKey {
        let item_id = ItemId::new_v7().unwrap();
        self.items.push((item_id, item));
        ItemKey {
            vault_id: self.vault,
            item_id,
        }
    }

    fn login(&mut self, password: Option<&str>, urls: &[&str]) -> ItemKey {
        self.login_at(password, urls, NOW - DAY)
    }

    fn login_at(&mut self, password: Option<&str>, urls: &[&str], ms: i64) -> ItemKey {
        let item = self.make(&ItemType::Login, ms, |e| {
            if let Some(p) = password {
                e.set_secret(SecretField::Password, p).unwrap();
            }
            for u in urls {
                e.add_url(&LoginUrl::new(u)).unwrap();
            }
        });
        self.push(item)
    }

    fn make(
        &self,
        kind: &ItemType,
        ms: i64,
        f: impl FnOnce(&mut mosaic_core::ItemEdit<'_>),
    ) -> Item {
        let mut item = Item::new(kind, self.device, at(ms)).unwrap();
        {
            let mut e = item.edit(self.device, at(ms));
            e.set_title("Sample").unwrap();
            f(&mut e);
        }
        item
    }

    fn edit(&mut self, key: ItemKey, ms: i64, f: impl FnOnce(&mut mosaic_core::ItemEdit<'_>)) {
        let device = self.device;
        let (_, item) = self
            .items
            .iter_mut()
            .find(|(id, _)| *id == key.item_id)
            .unwrap();
        let mut e = item.edit(device, at(ms));
        f(&mut e);
    }

    fn report(&self) -> Report {
        self.report_with(Options::default())
    }

    fn report_with(&self, options: Options) -> Report {
        analyze(
            self.items.iter().map(|(item_id, item)| Candidate {
                vault_id: self.vault,
                item_id: *item_id,
                item,
            }),
            at(NOW),
            &options,
        )
        .unwrap()
    }
}

fn otp(issuer: Option<&str>) -> OtpConfig {
    let mut config = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
    config.issuer = issuer.map(Text::new);
    config
}

fn kinds(report: &Report, key: ItemKey) -> Vec<FindingKind> {
    report
        .item(key)
        .map(|i| i.findings.iter().map(|f| f.kind).collect())
        .unwrap_or_default()
}

#[test]
fn empty_vault() {
    let r = Fixture::new().report();
    assert_eq!(r.items_checked, 0);
    assert_eq!(r.counts, Counts::default());
    assert!(r.items.is_empty());
    assert!(r.reuse_groups.is_empty());
}

#[test]
fn weak_passwords_by_bucket() {
    let mut fx = Fixture::new();
    let very_weak = fx.login(Some("password"), &[]);
    let weak = fx.login(Some("q7Rk2mZp"), &[]);
    let strong = fx.login(Some(STRONG), &[]);
    let none = fx.login(None, &[]);
    let empty = fx.login(Some(""), &[]);
    let r = fx.report();

    let f = &r.item(very_weak).unwrap().findings[0];
    assert_eq!(
        (f.kind, f.severity, f.action),
        (FindingKind::Weak, Severity::High, Action::ChangePassword)
    );
    let f = &r.item(weak).unwrap().findings[0];
    assert_eq!((f.kind, f.severity), (FindingKind::Weak, Severity::Medium));
    for key in [strong, none, empty] {
        assert!(r.item(key).is_none());
    }
    assert_eq!(r.counts.weak, 2);
    assert_eq!(r.items_checked, 5);
}

#[test]
fn wifi_passwords_are_checked_but_notes_are_not() {
    let mut fx = Fixture::new();
    let wifi = fx.make(&ItemType::Wifi, NOW, |e| {
        e.set_secret(SecretField::WifiPassword, "12345678").unwrap();
    });
    let wifi = fx.push(wifi);
    let note = fx.make(&ItemType::SecureNote, NOW, |e| {
        e.set_text(TextField::Notes, "password").unwrap();
    });
    let note = fx.push(note);
    let card = fx.make(&ItemType::CreditCard, NOW, |e| {
        e.set_secret(SecretField::CardPin, "1234").unwrap();
    });
    let card = fx.push(card);
    let r = fx.report();
    assert_eq!(kinds(&r, wifi), [FindingKind::Weak]);
    assert!(r.item(note).is_none());
    assert!(r.item(card).is_none());
    assert_eq!(r.items_checked, 3);
}

#[test]
fn reused_groups_and_partners() {
    let mut fx = Fixture::new();
    let a = fx.login(Some(STRONG), &["https://a.example"]);
    let b = fx.login(Some(STRONG), &["https://b.example"]);
    let wifi = fx.make(&ItemType::Wifi, NOW, |e| {
        e.set_secret(SecretField::WifiPassword, STRONG).unwrap();
    });
    let c = fx.push(wifi);
    let d = fx.login(Some("another-Strong-pass-2#x9"), &[]);
    let e = fx.login(Some("another-Strong-pass-2#x9"), &[]);
    let lone = fx.login(Some("yet-another-Strong-9#q"), &[]);
    let r = fx.report();

    let mut group = vec![a, b, c];
    group.sort();
    let mut pair = vec![d, e];
    pair.sort();
    let mut want = vec![group.clone(), pair.clone()];
    want.sort();
    assert_eq!(r.reuse_groups, want);
    assert_eq!(r.counts.reused, 5);
    assert_eq!(r.counts.reused_groups, 2);
    for key in &group {
        let item = r.item(*key).unwrap();
        assert!(item.has(FindingKind::Reused));
        let others: Vec<ItemKey> = group.iter().copied().filter(|k| k != key).collect();
        assert_eq!(item.reused_with, others);
        assert_eq!(item.findings[0].severity, Severity::High);
    }
    assert!(r.item(lone).is_none());
}

#[test]
fn old_passwords() {
    let mut fx = Fixture::new();
    let old = fx.login_at(Some(STRONG), &[], NOW - 400 * DAY);
    let recent = fx.login_at(Some("Kd8$mWq2#Lp9vRt4Zz"), &[], NOW - 300 * DAY);
    let changed = fx.login_at(Some("hT3%nB8@wQ6^zY1&Ab"), &[], NOW - 900 * DAY);
    fx.edit(changed, NOW - 10 * DAY, |e| {
        e.set_secret(SecretField::Password, "Zx8!pL3#vN6$qR1&Cd")
            .unwrap();
    });
    let no_password = fx.login_at(None, &[], NOW - 900 * DAY);
    let future = fx.login_at(Some("Mm4^tY7*bV2(eW9)Gh"), &[], NOW + 30 * DAY);
    let r = fx.report();

    let f = &r.item(old).unwrap().findings[0];
    assert_eq!(
        (f.kind, f.severity, f.action),
        (FindingKind::Old, Severity::Low, Action::ChangePassword)
    );
    for key in [recent, changed, no_password, future] {
        assert!(r.item(key).is_none(), "{key:?}");
    }
    assert_eq!(r.counts.old, 1);

    let mut options = Options::default();
    options.max_password_age_days = 30;
    let r = fx.report_with(options);
    assert!(r.item(recent).unwrap().has(FindingKind::Old));
    assert!(r.item(changed).is_none());
}

#[test]
fn insecure_urls() {
    let mut fx = Fixture::new();
    let public = fx.login(Some(STRONG), &["http://shop.example.com/login"]);
    let router = fx.login(Some("Kd8$mWq2#Lp9vRt4Zz"), &["http://192.168.178.1"]);
    let mixed = fx.login(
        Some("hT3%nB8@wQ6^zY1&Ab"),
        &["http://fritz.box", "HTTP://bank.example.net"],
    );
    let loopback = fx.login(
        Some("Zx8!pL3#vN6$qR1&Cd"),
        &["http://localhost:8080", "http://127.0.0.1"],
    );
    let https = fx.login(
        Some("Mm4^tY7*bV2(eW9)Gh"),
        &["https://shop.example.com", "example.org"],
    );
    let r = fx.report();

    let f = &r.item(public).unwrap().findings[0];
    assert_eq!(
        (f.kind, f.severity, f.action),
        (
            FindingKind::InsecureUrl,
            Severity::Medium,
            Action::ReviewUrl
        )
    );
    assert_eq!(r.item(router).unwrap().findings[0].severity, Severity::Low);
    assert_eq!(
        r.item(mixed).unwrap().findings[0].severity,
        Severity::Medium
    );
    assert!(r.item(loopback).is_none());
    assert!(r.item(https).is_none());
    assert_eq!(r.counts.insecure_url, 3);
    // The host the dashboard names: a public one first.
    let host = |key| r.item(key).unwrap().insecure_host.as_deref();
    assert_eq!(host(public), Some("shop.example.com"));
    assert_eq!(host(router), Some("192.168.178.1"));
    assert_eq!(host(mixed), Some("bank.example.net"));
}

#[test]
fn missing_totp() {
    let mut fx = Fixture::new();
    let github = fx.login(Some(STRONG), &["https://github.com/login"]);
    let unknown = fx.login(
        Some("Kd8$mWq2#Lp9vRt4Zz"),
        &["https://unknown-site.example"],
    );
    let with_otp = fx.login(Some("hT3%nB8@wQ6^zY1&Ab"), &["https://gitlab.com"]);
    fx.edit(with_otp, NOW, |e| e.set_otp(&otp(None)).unwrap());
    let r = fx.report();

    let item = r.item(github).unwrap();
    let f = item.findings[0];
    assert_eq!(
        (f.kind, f.severity, f.action),
        (
            FindingKind::MissingTotp,
            Severity::Medium,
            Action::SetUpTotp
        )
    );
    let site = item.totp_site.unwrap();
    assert_eq!(site.domain(), "github.com");
    assert!(site.documentation().is_some());
    assert!(r.item(unknown).is_none());
    assert!(r.item(with_otp).is_none());
    assert_eq!(r.counts.missing_totp, 1);

    // A separate one-time-code item for GitHub covers the login, by issuer.
    let mut covered_by_issuer = Fixture::new();
    let login = covered_by_issuer.login(Some(STRONG), &["https://github.com"]);
    let totp = covered_by_issuer.make(&ItemType::Totp, NOW, |e| {
        e.set_otp(&otp(Some("GitHub"))).unwrap();
    });
    covered_by_issuer.push(totp);
    assert!(covered_by_issuer.report().item(login).is_none());

    // ... or by URL.
    let mut covered_by_url = Fixture::new();
    let login = covered_by_url.login(Some(STRONG), &["https://gist.github.com"]);
    let totp = covered_by_url.make(&ItemType::Totp, NOW, |e| {
        e.set_otp(&otp(None)).unwrap();
        e.add_url(&LoginUrl::new("https://github.com")).unwrap();
    });
    covered_by_url.push(totp);
    assert!(covered_by_url.report().item(login).is_none());
}

fn otp_field(issuer: Option<&str>) -> CustomField {
    CustomField::new(Text::new("2FA"), CustomFieldValue::Otp(otp(issuer)))
}

#[test]
fn custom_field_one_time_codes_count() {
    // Regression: only `Item::otp` counted, so an imported login with its
    // OTP generator in a custom field was flagged.
    let mut fx = Fixture::new();
    let login = fx.login(Some(STRONG), &["https://github.com/login"]);
    fx.edit(login, NOW, |e| {
        e.add_custom_field(&otp_field(None)).unwrap();
    });
    let r = fx.report();
    assert!(r.item(login).is_none(), "{:?}", r.item(login));
    assert_eq!(r.counts.missing_totp, 0);

    // A custom-field OTP on another item covers the site by issuer ...
    let mut by_issuer = Fixture::new();
    let login = by_issuer.login(Some(STRONG), &["https://github.com"]);
    let note = by_issuer.make(&ItemType::SecureNote, NOW, |e| {
        e.add_custom_field(&otp_field(Some("GitHub"))).unwrap();
    });
    by_issuer.push(note);
    assert!(by_issuer.report().item(login).is_none());

    // ... and by URL.
    let mut by_url = Fixture::new();
    let login = by_url.login(Some(STRONG), &["https://gist.github.com"]);
    let other = by_url.login(Some("hT3%nB8@wQ6^zY1&Ab"), &["https://github.com"]);
    by_url.edit(other, NOW, |e| {
        e.add_custom_field(&otp_field(None)).unwrap();
    });
    let r = by_url.report();
    assert!(r.item(login).is_none());
    assert!(r.item(other).is_none());

    // A custom field of another kind does not count.
    let mut plain = Fixture::new();
    let login = plain.login(Some(STRONG), &["https://github.com"]);
    plain.edit(login, NOW, |e| {
        e.add_custom_field(&CustomField::new(
            Text::new("note"),
            CustomFieldValue::Text(Text::new("otpauth://totp/x")),
        ))
        .unwrap();
    });
    assert_eq!(kinds(&plain.report(), login), [FindingKind::MissingTotp]);
}

#[test]
fn a_passkey_for_the_site_lowers_missing_totp_to_low() {
    let mut fx = Fixture::new();
    let login = fx.login(Some(STRONG), &["https://github.com/login"]);
    let elsewhere = fx.login(Some("hT3%nB8@wQ6^zY1&Ab"), &["https://gitlab.com"]);
    let both = fx.login(
        Some("Rk7&uP2!xW9#cJ4$Lm"),
        &["https://github.com", "https://gitlab.com"],
    );
    let passkey = fx.make(&ItemType::Passkey, NOW, |e| {
        e.set_passkey(&PasskeyCredential::new(
            vec![1, 2, 3],
            Text::new("github.com"),
            vec![4, 5, 6],
            SecretVec::from_slice(&[7u8; 32]).unwrap(),
            -8,
            at(NOW),
        ))
        .unwrap();
    });
    fx.push(passkey);
    let r = fx.report();
    // Still reported (the password alone still signs in), but `Low`.
    let f = r.item(login).unwrap().findings[0];
    assert_eq!(
        (f.kind, f.severity),
        (FindingKind::MissingTotp, Severity::Low)
    );
    let f = r.item(elsewhere).unwrap().findings[0];
    assert_eq!(
        (f.kind, f.severity),
        (FindingKind::MissingTotp, Severity::Medium)
    );
    // A passkey for one of an item's sites does not lower the finding for
    // another site of the same item: the unprotected site is named.
    let entry = r.item(both).unwrap();
    assert_eq!(entry.findings[0].severity, Severity::Medium);
    assert_eq!(entry.totp_site.unwrap().domain(), "gitlab.com");
    assert_eq!(r.counts.missing_totp, 3);
}

#[test]
fn trashed_and_deleted_items_are_skipped() {
    let mut fx = Fixture::new();
    let trashed = fx.login(Some("password"), &["http://example.com"]);
    fx.edit(trashed, NOW, |e| e.trash().unwrap());
    let deleted = fx.login(Some("password"), &[]);
    let device = fx.device;
    fx.items
        .iter_mut()
        .find(|(id, _)| *id == deleted.item_id)
        .unwrap()
        .1
        .delete(device, at(NOW))
        .unwrap();
    let live = fx.login(Some("password"), &[]);
    let r = fx.report();
    assert_eq!(r.items_checked, 1);
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].item, live);
    assert!(
        r.reuse_groups.is_empty(),
        "skipped items never count as reuse"
    );
}

#[test]
fn ordering_and_severity_counts() {
    let mut fx = Fixture::new();
    let low = fx.login(Some(STRONG), &["http://nas.local"]);
    let high = fx.login(
        Some("password"),
        &["http://example.com", "https://github.com"],
    );
    let medium = fx.login(Some("Kd8$mWq2#Lp9vRt4Zz"), &["http://example.org"]);
    let r = fx.report();
    let order: Vec<ItemKey> = r.items.iter().map(|i| i.item).collect();
    assert_eq!(order, [high, medium, low]);
    assert_eq!(
        kinds(&r, high),
        [
            FindingKind::Weak,
            FindingKind::MissingTotp,
            FindingKind::InsecureUrl
        ]
    );
    assert_eq!((r.counts.high, r.counts.medium, r.counts.low), (1, 1, 1));
    for kind in FindingKind::ALL {
        let n = r.items.iter().filter(|i| i.has(kind)).count();
        assert_eq!(r.counts.of(kind) as usize, n, "{kind:?}");
    }
}

#[test]
fn the_report_holds_no_secret() {
    let mut fx = Fixture::new();
    let secrets = ["password", "Sommer2024!", STRONG];
    for s in secrets {
        fx.login(Some(s), &["http://example.com"]);
        fx.login(Some(s), &["https://github.com"]);
    }
    let r = fx.report();
    let shown = format!("{r:?}");
    for s in secrets {
        assert!(!shown.contains(s), "{s} in the report");
    }
    assert_eq!(r.reuse_groups.len(), 3);
}

#[test]
fn i18n_keys() {
    let keys = [
        FindingKind::Weak.i18n_key(),
        FindingKind::Reused.i18n_key(),
        FindingKind::Old.i18n_key(),
        FindingKind::MissingTotp.i18n_key(),
        FindingKind::InsecureUrl.i18n_key(),
        Severity::Low.i18n_key(),
        Severity::Medium.i18n_key(),
        Severity::High.i18n_key(),
        Action::ChangePassword.i18n_key(),
        Action::SetUpTotp.i18n_key(),
        Action::ReviewUrl.i18n_key(),
    ];
    // Every key exists in both catalogs of packages/i18n.
    let en = include_str!("../../../packages/i18n/src/catalog/en/insights.ts");
    let de = include_str!("../../../packages/i18n/src/catalog/de/insights.ts");
    for key in keys {
        assert!(key.starts_with("insights."), "{key}");
        let quoted = format!("\"{key}\":");
        assert!(
            en.contains(&quoted),
            "{key} missing in catalog/en/insights.ts"
        );
        assert!(
            de.contains(&quoted),
            "{key} missing in catalog/de/insights.ts"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Reuse findings are exactly the classes of equal passwords: every
    /// member of a class of two or more is `Reused` with the other members
    /// as partners, and nothing else is.
    #[test]
    fn reuse_matches_password_equality(
        passwords in proptest::collection::vec(proptest::option::of("[xy]{1,2}"), 0..12)
    ) {
        let mut fx = Fixture::new();
        let keys: Vec<ItemKey> = passwords
            .iter()
            .map(|p| fx.login(p.as_deref(), &[]))
            .collect();
        let r = fx.report();
        let mut reused = 0;
        for (i, key) in keys.iter().enumerate() {
            let mut partners: Vec<ItemKey> = keys
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i && passwords[i].is_some() && passwords[j] == passwords[i])
                .map(|(_, k)| *k)
                .collect();
            partners.sort();
            let item = r.item(*key);
            prop_assert_eq!(item.is_some_and(|f| f.has(FindingKind::Reused)), !partners.is_empty());
            if !partners.is_empty() {
                reused += 1;
                prop_assert_eq!(&item.unwrap().reused_with, &partners);
            }
        }
        prop_assert_eq!(r.counts.reused as usize, reused);
        let members: usize = r.reuse_groups.iter().map(Vec::len).sum();
        prop_assert_eq!(members, reused);
    }
}

//! Items, reveal, OTP and the generator.

use mosaic_mobile::{
    GeneratorMode, GeneratorOptions, ItemFilter, ItemKey, ItemKind, LoginDraft, MobileError,
    NoteDraft, OtpEdit, OtpScheme, TrashMode, UrlDraft,
};
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};

use crate::common::Phone;

const PASSWORD: &str = "s3cr3t-Passw0rd!";

fn url(u: &str) -> UrlDraft {
    UrlDraft {
        url: u.to_owned(),
        match_etld1: false,
        allow_http: false,
    }
}

fn draft(vault: &str, title: &str, password: Option<&str>) -> LoginDraft {
    LoginDraft {
        vault_id: vault.to_owned(),
        item_id: None,
        title: title.to_owned(),
        username: "alice".to_owned(),
        password: password.map(|p| p.as_bytes().to_vec()),
        urls: vec![url("https://example.com/login")],
        notes: Some(b"note-with-text".to_vec()),
        otp: OtpEdit::Keep,
        favorite: false,
    }
}

fn save(phone: &Phone, title: &str, password: &str) -> ItemKey {
    phone
        .core
        .save_login(draft(&phone.vault_id(), title, Some(password)))
        .unwrap()
}

#[test]
fn items_and_detail_never_contain_concealed_values() {
    let (phone, _) = Phone::with_account();
    let key = save(&phone, "Example", PASSWORD);
    let list = phone.core.items(ItemFilter::default()).unwrap();
    let found = phone.core.search("alice".to_owned()).unwrap();
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    let vaults = phone.core.vaults().unwrap();
    for debug in [
        format!("{list:?}"),
        format!("{found:?}"),
        format!("{detail:?}"),
        format!("{vaults:?}"),
    ] {
        assert!(!debug.contains(PASSWORD), "a secret leaked: {debug}");
    }
    assert_eq!(list.len(), 1);
    assert_eq!(found.len(), 1);
    assert_eq!(list[0].kind, ItemKind::Login);
    assert_eq!(list[0].title, "Example");
    assert_eq!(list[0].subtitle.as_deref(), Some("alice"));
    assert_eq!(list[0].primary_host.as_deref(), Some("example.com"));
    let password = detail
        .fields
        .iter()
        .find(|f| f.key == "login.password")
        .unwrap();
    assert!(password.concealed);
    assert_eq!(password.value, None);
    assert!(password.copyable);
    let username = detail
        .fields
        .iter()
        .find(|f| f.key == "login.username")
        .unwrap();
    assert_eq!(username.value.as_deref(), Some("alice"));
    assert_eq!(detail.urls[0].url, "https://example.com/login");
}

#[test]
fn reveal_returns_secret_value_and_wipe_empties_it() {
    let (phone, _) = Phone::with_account();
    let key = save(&phone, "Example", PASSWORD);
    let value = phone
        .core
        .reveal(&key.vault_id, &key.item_id, "login.password")
        .unwrap();
    assert_eq!(value.expose_bytes(), PASSWORD.as_bytes());
    assert_eq!(value.len(), PASSWORD.len());
    assert_eq!(format!("{value:?}"), "SecretValue([REDACTED])");
    value.wipe();
    assert_eq!(value.len(), 0);
    assert!(value.expose_bytes().is_empty());
    assert!(value.is_empty());
    assert_eq!(
        phone
            .core
            .reveal(&key.vault_id, &key.item_id, "no.such.field")
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .reveal(&key.vault_id, &key.item_id, "card.number")
            .err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn save_login_creates_and_edits_with_history() {
    let (phone, _) = Phone::with_account();
    let vault = phone.vault_id();
    let key = save(&phone, "Example", PASSWORD);
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    assert_eq!(detail.history_count, 0);

    let mut edit = draft(&vault, "Example renamed", Some("an-even-better-Passw0rd"));
    edit.item_id = Some(key.item_id.clone());
    edit.username = "bob".to_owned();
    edit.urls = vec![
        url("https://other.example"),
        url("https://example.com/login"),
    ];
    edit.favorite = true;
    let same = phone.core.save_login(edit).unwrap();
    assert_eq!(same, key);

    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    assert_eq!(detail.summary.title, "Example renamed");
    assert_eq!(detail.summary.subtitle.as_deref(), Some("bob"));
    assert!(detail.summary.favorite);
    assert_eq!(
        detail.history_count, 1,
        "the old password is in the history"
    );
    assert_eq!(detail.urls.len(), 2);
    let revealed = phone
        .core
        .reveal(&key.vault_id, &key.item_id, "login.password")
        .unwrap();
    assert_eq!(revealed.expose_bytes(), b"an-even-better-Passw0rd");
    // An empty title is refused.
    assert_eq!(
        phone.core.save_login(draft(&vault, "  ", None)).err(),
        Some(MobileError::InvalidInput)
    );
    // A note.
    let note = phone
        .core
        .save_note(NoteDraft {
            vault_id: vault,
            item_id: None,
            title: "Wi-Fi code".to_owned(),
            notes: Some(b"hello".to_vec()),
            favorite: false,
        })
        .unwrap();
    assert_eq!(
        phone
            .core
            .item(&note.vault_id, &note.item_id)
            .unwrap()
            .summary
            .kind,
        ItemKind::SecureNote
    );
}

#[test]
fn save_login_password_none_keeps_password() {
    let (phone, _) = Phone::with_account();
    let vault = phone.vault_id();
    let key = save(&phone, "Example", PASSWORD);
    let mut edit = draft(&vault, "Example", None);
    edit.item_id = Some(key.item_id.clone());
    edit.username = "carol".to_owned();
    edit.notes = None;
    phone.core.save_login(edit).unwrap();
    let revealed = phone
        .core
        .reveal(&key.vault_id, &key.item_id, "login.password")
        .unwrap();
    assert_eq!(revealed.expose_bytes(), PASSWORD.as_bytes());
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    assert_eq!(detail.history_count, 0);
    assert_eq!(detail.summary.subtitle.as_deref(), Some("carol"));
    // The notes were left alone too.
    assert!(
        detail
            .fields
            .iter()
            .any(|f| f.key == "notes" && f.value.as_deref() == Some("note-with-text"))
    );
}

#[test]
fn trash_item_moves_to_trash() {
    let (phone, _) = Phone::with_account();
    let key = save(&phone, "Example", PASSWORD);
    phone.core.trash_item(&key.vault_id, &key.item_id).unwrap();
    assert!(phone.core.items(ItemFilter::default()).unwrap().is_empty());
    let mut only = ItemFilter::default();
    only.trash = TrashMode::Only;
    assert_eq!(phone.core.items(only).unwrap().len(), 1);
    assert!(phone.core.search("example".to_owned()).unwrap().is_empty());
    phone
        .core
        .set_favorite(&key.vault_id, &key.item_id, true)
        .unwrap();
}

fn totp_code(secret: &str, at: u64) -> String {
    Otp {
        secret: OtpSecret::from_base32(secret).unwrap(),
        algorithm: Algorithm::Sha1,
        kind: OtpKind::Totp {
            digits: Digits::DEFAULT,
            period: Period::DEFAULT,
        },
    }
    .code_at(at)
    .unwrap()
    .expose_secret()
    .to_owned()
}

#[test]
fn otp_code_and_remaining_seconds() {
    let (phone, _) = Phone::with_account();
    let vault = phone.vault_id();
    let preview = phone
        .core
        .otp_from_text(b"JBSW Y3DP-EHPK 3PXP".to_vec())
        .unwrap();
    assert_eq!(preview.scheme, OtpScheme::Totp);
    let mut d = draft(&vault, "With OTP", Some(PASSWORD));
    d.otp = OtpEdit::SetPending {
        id: preview.pending_id,
    };
    let key = phone.core.save_login(d).unwrap();
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    assert!(detail.summary.has_otp);
    assert_eq!(detail.otp.as_ref().unwrap().digits, 6);
    assert_eq!(detail.otp.as_ref().unwrap().period_s, Some(30));

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let code = phone.core.otp_code(&key.vault_id, &key.item_id).unwrap();
    let after = before + 2;
    assert_eq!(code.period_s, 30);
    assert_eq!(code.digits, 6);
    assert!((1..=30).contains(&code.remaining_s));
    let text = String::from_utf8(code.code.expose_bytes()).unwrap();
    let expected: Vec<String> = (before..=after)
        .map(|t| totp_code("JBSWY3DPEHPK3PXP", t))
        .collect();
    assert!(expected.contains(&text), "{text} not in {expected:?}");
    // The code is a secret value like any other.
    code.code.wipe();
    assert_eq!(code.code.len(), 0);
}

#[test]
fn otp_pending_preview_has_no_secret_and_expires() {
    let (phone, _) = Phone::with_account();
    let vault = phone.vault_id();
    let uri = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example";
    let preview = phone.core.otp_from_text(uri.as_bytes().to_vec()).unwrap();
    let shown = format!("{preview:?}");
    assert!(!shown.contains("JBSWY3DPEHPK3PXP"));
    assert_eq!(preview.issuer.as_deref(), Some("Example"));
    assert_eq!(preview.account_name.as_deref(), Some("alice"));
    assert_eq!(preview.digits, 6);
    assert_eq!(preview.expires_in_s, 600);
    // Garbage is refused with the matching code.
    assert_eq!(
        phone
            .core
            .otp_from_text(b"otpauth://nonsense".to_vec())
            .err(),
        Some(MobileError::OtpUriInvalid)
    );
    assert_eq!(
        phone.core.otp_from_text(b"not base32 !!".to_vec()).err(),
        Some(MobileError::OtpKeyInvalid)
    );
    assert_eq!(
        phone
            .core
            .otp_from_text(b"otpauth-migration://offline?data=AAAA".to_vec())
            .err(),
        Some(MobileError::OtpQrMigration)
    );
    // At most four wait; the oldest goes first.
    let mut ids = vec![preview.pending_id.clone()];
    for _ in 0..4 {
        ids.push(
            phone
                .core
                .otp_from_text(uri.as_bytes().to_vec())
                .unwrap()
                .pending_id,
        );
    }
    let mut d = draft(&vault, "First", None);
    d.otp = OtpEdit::SetPending { id: ids[0].clone() };
    assert_eq!(phone.core.save_login(d).err(), Some(MobileError::NotFound));
    // Discarding forgets one; the rest expire after ten minutes.
    phone.core.otp_pending_discard(ids[1].clone());
    let mut d = draft(&vault, "Second", None);
    d.otp = OtpEdit::SetPending { id: ids[1].clone() };
    assert_eq!(phone.core.save_login(d).err(), Some(MobileError::NotFound));
    phone.core.test_hooks().set_clock_offset_ms(601_000);
    let mut d = draft(&vault, "Third", None);
    d.otp = OtpEdit::SetPending { id: ids[2].clone() };
    assert_eq!(phone.core.save_login(d).err(), Some(MobileError::NotFound));
}

#[test]
fn generate_password_respects_options() {
    let (phone, _) = Phone::with_account();
    let mut options = GeneratorOptions::default();
    options.length = 32;
    options.symbols = false;
    let generated = phone.core.generate_password(options).unwrap();
    let text = String::from_utf8(generated.value.expose_bytes()).unwrap();
    assert_eq!(text.chars().count(), 32);
    assert!(text.chars().all(|c| c.is_ascii_alphanumeric()), "{text}");
    assert!(text.chars().any(|c| c.is_ascii_lowercase()));
    assert!(text.chars().any(|c| c.is_ascii_uppercase()));
    assert!(text.chars().any(|c| c.is_ascii_digit()));
    assert!(generated.entropy_bits > 150.0);

    let mut options = GeneratorOptions::default();
    options.mode = GeneratorMode::Passphrase;
    options.words = 5;
    options.separator = ".".to_owned();
    let generated = phone.core.generate_password(options).unwrap();
    let text = String::from_utf8(generated.value.expose_bytes()).unwrap();
    assert_eq!(text.split('.').count(), 5, "{text}");
    assert!(generated.entropy_bits > 50.0);

    let mut options = GeneratorOptions::default();
    options.length = 1;
    assert_eq!(
        phone.core.generate_password(options).err(),
        Some(MobileError::InvalidInput)
    );
    // The generator needs no session.
    phone.core.lock();
    assert!(
        phone
            .core
            .generate_password(GeneratorOptions::default())
            .is_ok()
    );
}

//! Golden files for the plaintext formats of the item model (CRYPTO_SPEC
//! §14): one item that uses every field kind, a tombstone and vault
//! metadata, built from fixed devices and times. Any format change shows up
//! as a diff of `tests/golden/*.json`, and the golden files are decoded
//! again, so old plaintext stays readable. Regenerate after an intended,
//! reviewed change with:
//!
//! ```sh
//! MOSAIC_BLESS=1 cargo test -p mosaic-core --test model_golden
//! ```
//!
//! The files are also the seeds of the `item_plaintext` and `item_merge`
//! fuzz targets.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] functions"
)]

use std::path::PathBuf;

use mosaic_core::model::{
    AttachmentRef, CustomField, CustomFieldValue, Item, ItemType, LoginUrl, OtpConfig, OtpKind,
    PasskeyCredential, SecretField, SshKeyMaterial, Text, TextField, Timestamp, Tombstone,
    UrlMatch, VaultMeta,
};
use mosaic_core::{AttachmentId, AttachmentKey, DeviceId, SecretBytes, SecretVec};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 16]).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

/// Compares `bytes` with the golden file and returns the golden bytes.
fn check(name: &str, bytes: &[u8]) -> Vec<u8> {
    let p = path(name);
    if std::env::var_os("MOSAIC_BLESS").is_some() {
        std::fs::write(&p, bytes).unwrap();
    }
    let golden = std::fs::read(&p).unwrap();
    assert_eq!(
        String::from_utf8_lossy(bytes),
        String::from_utf8_lossy(&golden),
        "{name} changed; bless only for a reviewed format change"
    );
    golden
}

fn secret(s: &str) -> mosaic_core::SecretString {
    let mut tmp = Item::new(&ItemType::SecureNote, dev(9), at(0)).unwrap();
    tmp.edit(dev(9), at(0))
        .set_secret(SecretField::CardPin, s)
        .unwrap();
    tmp.secret(SecretField::CardPin).unwrap().unwrap()
}

/// An item with every field kind, written by two devices, with one
/// concurrent conflict on the title.
fn golden_item() -> Item {
    let t0 = 1_761_000_000_000;
    let mut item = Item::new(&ItemType::Login, dev(1), at(t0)).unwrap();
    {
        let mut e = item.edit(dev(1), at(t0 + 1));
        for f in TextField::ALL {
            e.set_text(*f, &format!("{} value", f.key())).unwrap();
        }
        for f in SecretField::ALL {
            e.set_secret(*f, &format!("{} secret", f.key())).unwrap();
        }
        e.set_secret(SecretField::Password, "correct horse battery staple")
            .unwrap();
        e.set_favorite(true).unwrap();
        e.add_tag("work").unwrap();
        e.add_tag("Fünf").unwrap();
        e.add_url(&LoginUrl::new("https://github.com")).unwrap();
        let mut u = LoginUrl::new("http://intranet.example:8080/login");
        u.match_mode = UrlMatch::Etld1;
        u.allow_http = true;
        e.add_url(&u).unwrap();
        e.add_custom_field(&CustomField::new(
            Text::new("Recovery code"),
            CustomFieldValue::Hidden(secret("abcd-efgh")),
        ))
        .unwrap();
        for (label, value) in [
            ("Note", CustomFieldValue::Text(Text::new("text"))),
            (
                "Site",
                CustomFieldValue::Url(Text::new("https://x.example")),
            ),
            ("Mail", CustomFieldValue::Email(Text::new("a@x.example"))),
            ("Phone", CustomFieldValue::Phone(Text::new("+49 30 1234"))),
            ("Born", CustomFieldValue::Date(Text::new("1990-01-31"))),
        ] {
            e.add_custom_field(&CustomField::new(Text::new(label), value))
                .unwrap();
        }
        let mut otp = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
        otp.issuer = Some(Text::new("GitHub"));
        otp.label = Some(Text::new("octocat"));
        e.set_otp(&otp).unwrap();
        let mut steam = OtpConfig::totp(SecretVec::from_slice(b"steam-secret").unwrap());
        steam.kind = OtpKind::Steam;
        steam.digits = 5;
        steam.hardware_protected = true;
        e.add_custom_field(&CustomField::new(
            Text::new("Steam"),
            CustomFieldValue::Otp(steam),
        ))
        .unwrap();
        e.set_otp_counter(3).unwrap();
        e.set_attachment(&AttachmentRef::new(
            AttachmentId::from_bytes([0x0D; 16]).unwrap(),
            Text::new("recovery.pdf"),
            Text::new("application/pdf"),
            12_345,
            AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32])),
        ))
        .unwrap();
        let mut pk = PasskeyCredential::new(
            vec![0xC1; 16],
            Text::new("github.com"),
            vec![0xA5; 32],
            SecretVec::from_slice(&[0x77; 32]).unwrap(),
            -7,
            at(t0),
        );
        pk.user_name = Text::new("octocat");
        pk.user_display_name = Text::new("The Octocat");
        e.set_passkey(&pk).unwrap();
        e.set_passkey_sign_count(42).unwrap();
        e.set_ssh_key(&SshKeyMaterial::new(
            secret("-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaA==\n-----END OPENSSH PRIVATE KEY-----\n"),
            Text::new("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA octocat"),
            Text::new("SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU"),
        ))
        .unwrap();
    }
    item.edit(dev(1), at(t0 + 2))
        .set_secret(SecretField::Password, "Tr0ub4dor&3 \"quoted\" \\ \u{1F511}")
        .unwrap();
    item.edit(dev(1), at(t0 + 3)).trash().unwrap();
    item.edit(dev(1), at(t0 + 4)).restore_from_trash().unwrap();
    // A concurrent title edit on a second device.
    let mut other = Item::from_json(item.to_json().unwrap().expose_secret()).unwrap();
    item.edit(dev(1), at(t0 + 5))
        .set_title("GitHub (laptop)")
        .unwrap();
    other
        .edit(dev(2), at(t0 + 5))
        .set_title("GitHub (phone)")
        .unwrap();
    item.merge(&other)
}

#[test]
fn item_plaintext() {
    let item = golden_item();
    let golden = check(
        "item_plaintext.json",
        item.to_json().unwrap().expose_secret(),
    );
    let back = Item::from_json(&golden).unwrap();
    assert_eq!(back, item);
    assert_eq!(back.unreadable_fields(), Vec::<&str>::new());
    assert_eq!(back.item_type(), Some(ItemType::Login));
    assert_eq!(back.title(), Some("GitHub (phone)"));
    assert_eq!(back.conflicts().len(), 1);
    assert_eq!(back.urls().len(), 2);
    assert_eq!(back.custom_fields().len(), 7);
    assert_eq!(back.attachments().len(), 1);
    assert_eq!(back.password_history().len(), 2);
    assert_eq!(back.otp_counter(), 3);
    assert_eq!(back.passkey_sign_count(), 42);
    assert!(back.otp().is_some() && back.passkey().is_some() && back.ssh_key().is_some());
    assert!(back.favorite());
    assert_eq!(back.trashed_at(), None);
    assert_eq!(back.tags(), vec!["Fünf", "work"]);
    assert_eq!(
        back.secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret(),
        "Tr0ub4dor&3 \"quoted\" \\ \u{1F511}"
    );
}

#[test]
fn tombstone_plaintext() {
    let mut item = golden_item();
    item.delete(dev(2), at(1_761_000_100_000)).unwrap();
    let t = item.to_tombstone().unwrap();
    let golden = check(
        "tombstone_plaintext.json",
        t.to_json().unwrap().expose_secret(),
    );
    let back = Tombstone::from_json(&golden).unwrap();
    assert_eq!(back, t);
    assert_eq!(back.deleted_at(), at(1_761_000_100_000));
    assert_eq!(back.deleted_by(), vec![dev(2)]);
    assert!(Item::from(back).is_deleted());
}

#[test]
fn vault_meta_plaintext() {
    let mut meta = VaultMeta::new("Personal", dev(1), at(1_761_000_000_000)).unwrap();
    {
        let mut e = meta.edit(dev(1), at(1_761_000_000_001));
        e.set_icon(Some("house")).unwrap();
        e.set_description(Some("Everyday logins")).unwrap();
    }
    let golden = check(
        "vault_meta_plaintext.json",
        meta.to_json().unwrap().expose_secret(),
    );
    let back = VaultMeta::from_json(&golden).unwrap();
    assert_eq!(back, meta);
    assert_eq!(back.name(), Some("Personal"));
    assert_eq!(back.icon(), Some("house"));
    assert_eq!(back.description(), Some("Everyday logins"));
}

/// The committed `item_plaintext` fuzz seeds stay valid inputs, so the
/// fuzzer starts behind the decoder instead of at its first error. Seeds
/// named `reject_*` sit just outside the format (a layout carrying the
/// other layout's reserved member) and must be rejected by every decoder.
#[test]
fn fuzz_seeds_decode() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/item_plaintext");
    let (mut valid, mut rejected) = (0, 0);
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let bytes = std::fs::read(entry.path()).unwrap();
        let decodes = Item::from_json(&bytes).is_ok()
            || Tombstone::from_json(&bytes).is_ok()
            || VaultMeta::from_json(&bytes).is_ok();
        let name = entry.file_name().into_string().unwrap();
        if name.starts_with("reject_") {
            assert!(!decodes, "{name}");
            rejected += 1;
        } else {
            assert!(decodes, "{name}");
            valid += 1;
        }
    }
    assert!(valid >= 5);
    assert!(rejected >= 2);
}

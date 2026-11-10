//! Mosaic's encrypted export (CRYPTO_SPEC §11.2): everything a vault holds
//! survives export and import, and a wrong passphrase, a cut-off file or a
//! modified one commits nothing. Exports use the standard Argon2id profile
//! (the lowest §11.2 accepts: 256 MiB per run), so each test keeps the number
//! of exports low.

#![expect(clippy::unwrap_used, clippy::too_many_lines, reason = "tests")]

use std::path::Path;

use mosaic_core::model::{
    CustomField, CustomFieldValue, ItemType, LoginUrl, OtpConfig, PasskeyCredential, SecretField,
    SshKeyMaterial, Text, TextField,
};
use mosaic_core::{
    Argon2Profile, ExportPassphrase, MasterPassword, SecretString, SecretVec, Timestamp, VaultId,
};
use mosaic_import::mosaic_export::{parse_bytes, parse_reader};
use mosaic_import::{EntryWarning, Error, commit};
use mosaic_passkey::Es256Key;
use mosaic_vault::session::{ExportSelection, ItemFilter, ItemRef, LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;

const PASSPHRASE: &str = "an export passphrase for the tests";

fn secret(s: &str) -> SecretString {
    SecretString::from_str_copy(s).unwrap()
}

fn passphrase() -> ExportPassphrase {
    ExportPassphrase::new(&secret(PASSPHRASE)).unwrap()
}

fn session(dir: &Path, name: &str) -> (UnlockedSession, VaultId) {
    let (mut s, _) = LockedSession::open(Store::open(dir.join(name)).unwrap())
        .create_account(
            &MasterPassword::new("export test".to_owned()),
            Argon2Profile::Baseline,
        )
        .unwrap();
    let vault = s.create_vault("Personal").unwrap();
    (s, vault)
}

fn text(label: &str, value: &str) -> CustomField {
    CustomField::new(Text::new(label), CustomFieldValue::Text(Text::new(value)))
}

fn totp() -> OtpConfig {
    let mut c = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
    c.issuer = Some(Text::new("Example"));
    c.label = Some(Text::new("alice"));
    c
}

/// One item of every type, with every kind of field.
fn fill(s: &mut UnlockedSession, v: VaultId) -> usize {
    s.add_item(v, &ItemType::Login, |e| {
        e.set_title("Login")?;
        e.set_text(TextField::Username, "alice@example.com")?;
        e.set_secret(SecretField::Password, "pw-one")?;
        e.set_secret(SecretField::Password, "pw-two")?;
        e.set_secret(SecretField::Password, "pw-three")?;
        e.add_url(&LoginUrl::new("https://login.example.com/start"))?;
        e.add_url(&LoginUrl::new("https://example.org"))?;
        e.set_text(TextField::Notes, "line one\nline \"two\" \u{1F511}")?;
        e.add_tag("Work")?;
        e.add_tag("Mail")?;
        e.set_favorite(true)?;
        e.set_otp(&totp())?;
        e.add_custom_field(&text("Security question", "first pet"))?;
        e.add_custom_field(&CustomField::new(
            Text::new("Recovery code"),
            CustomFieldValue::Hidden(secret("abcd-efgh")),
        ))?;
        e.add_custom_field(&CustomField::new(
            Text::new("Backup 2FA"),
            CustomFieldValue::Otp(totp()),
        ))?;
        e.add_custom_field(&CustomField::new(
            Text::new("Started"),
            CustomFieldValue::Date(Text::new("2024-05-17")),
        ))?;
        Ok(())
    })
    .unwrap();
    let key = Es256Key::generate().unwrap();
    let mut credential = PasskeyCredential::new(
        vec![1, 2, 3, 4, 5],
        Text::new("example.com"),
        vec![9; 8],
        key.to_pkcs8_der().unwrap(),
        -7,
        Timestamp::from_unix_millis(1_700_000_000_000),
    );
    credential.user_name = Text::new("alice");
    s.add_item(v, &ItemType::Passkey, |e| {
        e.set_title("Passkey")?;
        e.set_passkey(&credential)?;
        e.set_passkey_sign_count(5)
    })
    .unwrap();
    s.add_item(v, &ItemType::Totp, |e| {
        e.set_title("Authenticator")?;
        e.set_otp(&totp())
    })
    .unwrap();
    s.add_item(v, &ItemType::SecureNote, |e| {
        e.set_title("Note")?;
        e.set_text(TextField::Notes, "only a note")
    })
    .unwrap();
    s.add_item(v, &ItemType::CreditCard, |e| {
        e.set_title("Card")?;
        e.set_text(TextField::CardholderName, "Alice Example")?;
        e.set_text(TextField::CardExpiry, "12/2030")?;
        e.set_secret(SecretField::CardNumber, "4111111111111111")?;
        e.set_secret(SecretField::CardVerificationCode, "123")
    })
    .unwrap();
    s.add_item(v, &ItemType::Identity, |e| {
        e.set_title("Identity")?;
        e.set_text(TextField::FirstName, "Alice")?;
        e.set_text(TextField::BirthDate, "1990-01-02")?;
        e.set_secret(SecretField::NationalId, "123-45-6789")
    })
    .unwrap();
    s.add_item(v, &ItemType::SshKey, |e| {
        e.set_title("Deploy key")?;
        e.set_ssh_key(&SshKeyMaterial::new(
            secret(
                "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----\n",
            ),
            Text::new("ssh-ed25519 AAAAC3Nz alice"),
            Text::new("SHA256:abc"),
        ))
    })
    .unwrap();
    s.add_item(v, &ItemType::ApiKey, |e| {
        e.set_title("API")?;
        e.set_text(TextField::ApiKeyId, "key-id")?;
        e.set_secret(SecretField::ApiKeySecret, "sk_live_123")
    })
    .unwrap();
    s.add_item(v, &ItemType::Wifi, |e| {
        e.set_title("Home wifi")?;
        e.set_text(TextField::WifiSsid, "Home")?;
        e.set_secret(SecretField::WifiPassword, "wifi-secret")
    })
    .unwrap();
    let doc = s
        .add_item(v, &ItemType::Document, |e| e.set_title("Scan"))
        .unwrap();
    let scan: Vec<u8> = (0..70_000u32).map(|i| (i % 251) as u8).collect();
    s.add_attachment(v, doc, "scan.bin", "application/octet-stream", &scan)
        .unwrap();
    // A trashed item is not exported.
    let gone = s
        .add_item(v, &ItemType::SecureNote, |e| e.set_title("Trashed"))
        .unwrap();
    s.trash_item(v, gone).unwrap();
    10
}

fn export(s: &UnlockedSession, attachments: bool) -> Vec<u8> {
    let mut file = Vec::new();
    let sel = ExportSelection {
        attachments,
        ..ExportSelection::default()
    };
    s.write_export(&sel, &mut file, &passphrase()).unwrap();
    file
}

fn secrets(s: &UnlockedSession, v: VaultId, id: mosaic_core::ItemId) -> Vec<Option<String>> {
    let item = s.item(v, id).unwrap();
    SecretField::ALL
        .iter()
        .map(|f| {
            item.secret(*f)
                .unwrap()
                .map(|x| x.expose_secret().to_owned())
        })
        .collect()
}

fn customs(item: &mosaic_core::Item) -> Vec<(String, String, String)> {
    item.custom_fields()
        .into_iter()
        .map(|(_, c)| {
            let (kind, value) = match &c.value {
                CustomFieldValue::Text(t) => ("text", t.as_str().to_owned()),
                CustomFieldValue::Hidden(s) => ("hidden", s.expose_secret().to_owned()),
                CustomFieldValue::Url(t) => ("url", t.as_str().to_owned()),
                CustomFieldValue::Email(t) => ("email", t.as_str().to_owned()),
                CustomFieldValue::Phone(t) => ("phone", t.as_str().to_owned()),
                CustomFieldValue::Date(t) => ("date", t.as_str().to_owned()),
                CustomFieldValue::Otp(o) => ("otp", format!("{:?}", o.secret.expose_secret())),
            };
            (c.label.as_str().to_owned(), kind.to_owned(), value)
        })
        .collect()
}

fn assert_same(src: &UnlockedSession, dst: &UnlockedSession, a: &ItemRef<'_>, b: &ItemRef<'_>) {
    let title = a.item.title().unwrap();
    let what = |m: &str| format!("{title}: {m}");
    assert_eq!(a.item.item_type(), b.item.item_type(), "{}", what("type"));
    for f in TextField::ALL {
        assert_eq!(a.item.text(*f), b.item.text(*f), "{}", what(f.key()));
    }
    assert_eq!(
        secrets(src, a.vault_id, a.item_id),
        secrets(dst, b.vault_id, b.item_id),
        "{}",
        what("secrets")
    );
    let urls = |i: &mosaic_core::Item| -> Vec<String> {
        i.urls()
            .iter()
            .map(|(_, u)| u.url.as_str().to_owned())
            .collect()
    };
    assert_eq!(urls(a.item), urls(b.item), "{}", what("urls"));
    let mut tags_a = a.item.tags();
    let mut tags_b = b.item.tags();
    tags_a.sort_unstable();
    tags_b.sort_unstable();
    assert_eq!(tags_a, tags_b, "{}", what("tags"));
    assert_eq!(a.item.favorite(), b.item.favorite(), "{}", what("favorite"));
    assert_eq!(
        customs(a.item),
        customs(b.item),
        "{}",
        what("custom fields")
    );
    let otp = |i: &mosaic_core::Item| {
        i.otp().map(|c| {
            (
                c.secret.expose_secret().to_vec(),
                format!("{:?}{:?}", c.kind, c.algorithm),
                c.digits,
                c.period,
                c.issuer.map(|t| t.as_str().to_owned()),
                c.label.map(|t| t.as_str().to_owned()),
            )
        })
    };
    assert_eq!(otp(a.item), otp(b.item), "{}", what("otp"));
    // Replaced within one edit, so the entries share a time: compare as sets.
    let history = |i: &mosaic_core::Item| -> Vec<String> {
        let mut h: Vec<String> = i
            .password_history()
            .iter()
            .map(|h| h.value.expose_secret().to_owned())
            .collect();
        h.sort();
        h
    };
    assert_eq!(history(a.item), history(b.item), "{}", what("history"));
    let ssh = |i: &mosaic_core::Item| {
        i.ssh_key().map(|k| {
            (
                k.private_key.expose_secret().to_owned(),
                k.public_key.as_str().to_owned(),
                k.fingerprint.as_str().to_owned(),
            )
        })
    };
    assert_eq!(ssh(a.item), ssh(b.item), "{}", what("ssh key"));
    let passkey = |i: &mosaic_core::Item| {
        i.passkey().map(|p| {
            (
                p.credential_id.clone(),
                p.rp_id.as_str().to_owned(),
                p.user_handle.clone(),
                p.user_name.as_str().to_owned(),
                p.private_key.expose_secret().to_vec(),
                p.algorithm,
                p.discoverable,
            )
        })
    };
    assert_eq!(passkey(a.item), passkey(b.item), "{}", what("passkey"));
    assert_eq!(
        a.item.passkey_sign_count(),
        b.item.passkey_sign_count(),
        "{}",
        what("sign count")
    );
    let files_a = a.item.attachments();
    let files_b = b.item.attachments();
    assert_eq!(files_a.len(), files_b.len(), "{}", what("attachments"));
    for (fa, fb) in files_a.iter().zip(&files_b) {
        assert_eq!(fa.file_name.as_str(), fb.file_name.as_str());
        assert_eq!(fa.mime.as_str(), fb.mime.as_str());
        let da = src
            .attachment(a.vault_id, a.item_id, fa.attachment_id)
            .unwrap();
        let db = dst
            .attachment(b.vault_id, b.item_id, fb.attachment_id)
            .unwrap();
        assert_eq!(da.expose_secret(), db.expose_secret());
    }
}

#[test]
fn export_then_import_round_trip_all_item_types() {
    let dir = tempfile::tempdir().unwrap();
    let (mut src, sv) = session(dir.path(), "src.db");
    let expected = fill(&mut src, sv);
    let file = export(&src, true);

    let import = parse_bytes(&file, &secret(PASSPHRASE)).unwrap();
    assert_eq!(import.entries.len(), expected);
    assert_eq!(import.skipped.len(), 0);
    assert!(
        import.entries.iter().all(|e| e.warnings.is_empty()),
        "{:?}",
        import
            .entries
            .iter()
            .map(|e| &e.warnings)
            .collect::<Vec<_>>()
    );
    let (mut dst, dv) = session(dir.path(), "dst.db");
    let report = commit(import, &mut dst, dv, &[]).unwrap();
    assert_eq!((report.imported, report.failed, report.skipped), (10, 0, 0));
    assert_eq!((report.attachments, report.attachments_failed), (1, 0));

    let src_items = src.items(&ItemFilter::default());
    let dst_items = dst.items(&ItemFilter::default());
    assert_eq!((src_items.len(), dst_items.len()), (10, 10));
    for a in src_items {
        let title = a.item.title().unwrap();
        let b = dst_items
            .iter()
            .find(|b| b.item.title() == Some(title))
            .unwrap_or_else(|| panic!("no item {title} after the import"));
        assert_same(&src, &dst, &a, b);
    }

    // Without attachment data the import says so, and adds no file.
    let file = export(&src, false);
    let import = parse_bytes(&file, &secret(PASSPHRASE)).unwrap();
    assert!(
        import
            .entries
            .iter()
            .any(|e| e.warnings == [EntryWarning::AttachmentMissing])
    );
    let (mut dst, dv) = session(dir.path(), "dst2.db");
    let report = commit(import, &mut dst, dv, &[]).unwrap();
    assert_eq!((report.imported, report.attachments), (10, 0));
}

#[test]
fn mosaic_export_wrong_passphrase_or_damage_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut src, sv) = session(dir.path(), "src.db");
    // Three chunks of plaintext (64 KiB each): a cut after the second one
    // still lets the first verify, and only the last chunk's check fails.
    for n in 0..3 {
        src.add_item(sv, &ItemType::SecureNote, |e| {
            e.set_title(&format!("Note {n}"))?;
            e.set_text(TextField::Notes, &"x".repeat(80_000))
        })
        .unwrap();
    }
    let file = export(&src, false);
    let (dst, _) = session(dir.path(), "dst.db");
    let header = 59;
    let chunk = 65_536 + 16;
    assert!(
        file.len() > header + 2 * chunk,
        "the export has three chunks"
    );

    // The whole file reads.
    assert_eq!(
        parse_bytes(&file, &secret(PASSPHRASE))
            .unwrap()
            .entries
            .len(),
        3
    );
    // A wrong passphrase.
    assert_eq!(
        parse_bytes(&file, &secret("another passphrase")).unwrap_err(),
        Error::WrongCredentials
    );
    // Cut at a chunk boundary (every chunk read so far verified), cut inside a
    // chunk, and extended.
    for len in [header + chunk, header + 2 * chunk, file.len() - 10] {
        assert_eq!(
            parse_bytes(file.get(..len).unwrap(), &secret(PASSPHRASE)).unwrap_err(),
            Error::WrongCredentials,
            "truncated to {len}"
        );
    }
    let mut longer = file.clone();
    longer.extend_from_slice(&[0; 5]);
    assert_eq!(
        parse_bytes(&longer, &secret(PASSPHRASE)).unwrap_err(),
        Error::WrongCredentials
    );
    // A flipped bit in the middle chunk and in the header (the AAD).
    for at in [header + chunk + 100, 20] {
        let mut bad = file.clone();
        *bad.get_mut(at).unwrap() ^= 1;
        assert!(
            parse_bytes(&bad, &secret(PASSPHRASE)).is_err(),
            "flip at {at}"
        );
    }
    // Not an export at all.
    assert_eq!(
        parse_bytes(b"{\"format\":\"mosaic-export\"}", &secret(PASSPHRASE)).unwrap_err(),
        Error::Malformed
    );
    // Nothing reached the vault: no `Import` exists to commit.
    assert!(dst.items(&ItemFilter::default()).is_empty());
    // The streaming entry point reads the same file.
    let streamed = parse_reader(file.as_slice(), &passphrase()).unwrap();
    assert_eq!(streamed.entries.len(), 3);
}

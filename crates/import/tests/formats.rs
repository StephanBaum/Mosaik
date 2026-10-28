//! Every importer against synthetic export files (`tests/fixtures`, no
//! real data). KDBX files written by pykeepass, an independent
//! implementation, check the KDBX reader against more than this crate's
//! own writer.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::format_push_string,
    clippy::assert_is_empty,
    reason = "tests"
)]

use std::io::Write;

use mosaic_core::SecretString;
use mosaic_core::model::{CustomFieldValue, ItemType, OtpKind, SecretField, Text, TextField};
use mosaic_import::csv::{CsvColumn, CsvMapping};
use mosaic_import::{
    EntryWarning, Error, Format, Import, ImportEntry, ImportWarning, ItemDraft, SkipReason,
    SkippedEntry, parse,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn entry(import: &Import, index: u32) -> &ImportEntry {
    import.entries.iter().find(|e| e.index == index).unwrap()
}

fn draft(import: &Import, index: u32) -> &ItemDraft {
    &entry(import, index).draft
}

fn secret(d: &ItemDraft, f: SecretField) -> &str {
    d.secret(f).unwrap().expose_secret()
}

fn labels(d: &ItemDraft) -> Vec<&str> {
    d.custom_fields.iter().map(|c| c.label.as_str()).collect()
}

fn tags(d: &ItemDraft) -> Vec<&str> {
    d.tags.iter().map(Text::as_str).collect()
}

fn urls(d: &ItemDraft) -> Vec<&str> {
    d.urls.iter().map(Text::as_str).collect()
}

fn history(d: &ItemDraft) -> Vec<&str> {
    d.password_history
        .iter()
        .map(SecretString::expose_secret)
        .collect()
}

#[test]
fn enpass_export() {
    let import = parse(Format::Enpass, &fixture("enpass.json"), None).unwrap();
    assert_eq!(import.format, Format::Enpass);
    assert_eq!(
        import.skipped,
        [
            SkippedEntry {
                index: 9,
                reason: SkipReason::Malformed
            },
            SkippedEntry {
                index: 10,
                reason: SkipReason::Empty
            }
        ]
    );
    assert_eq!(import.entries.len(), 9);

    let e = entry(&import, 0);
    let d = &e.draft;
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.title(), Some("Example Mail"));
    assert_eq!(d.text(TextField::Username), Some("alice@example.com"));
    assert_eq!(secret(d, SecretField::Password), "s3cret pass ");
    assert_eq!(urls(d), ["https://mail.example.com"]);
    assert!(d.otp.is_some());
    assert_eq!(d.text(TextField::Notes), Some("Shared with the team"));
    assert_eq!(
        labels(d),
        [
            "Security question",
            "Phone",
            "Since",
            "Backup code",
            "Number field",
            "Recovery mail"
        ]
    );
    assert!(matches!(
        d.custom_fields[0].value,
        CustomFieldValue::Hidden(_)
    ));
    assert!(matches!(
        d.custom_fields[2].value,
        CustomFieldValue::Date(_)
    ));
    assert!(matches!(
        d.custom_fields[5].value,
        CustomFieldValue::Email(_)
    ));
    assert_eq!(tags(d), ["Business/Clients", "mail", "work"]);
    assert!(d.favorite && !d.trashed);
    assert_eq!(d.attachments.len(), 1);
    assert_eq!(d.attachments[0].data.expose_secret(), b"hello attachment");
    assert_eq!(d.attachments[0].mime.as_str(), "text/plain");
    assert_eq!(
        e.warnings,
        [
            EntryWarning::UnmappedFields,
            EntryWarning::OtpUnreadable,
            EntryWarning::AttachmentMissing
        ]
    );

    let d = draft(&import, 1);
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.text(TextField::Username), Some("admin"));
    assert_eq!(secret(d, SecretField::Password), "router-pw");
    assert_eq!(labels(d), ["Password"]);
    assert_eq!(tags(d), ["archived"]);
    assert_eq!(
        entry(&import, 1).warnings,
        [EntryWarning::UnmappedFields, EntryWarning::Archived]
    );

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert!(d.trashed);
    assert_eq!(d.text(TextField::CardholderName), Some("Alice Example"));
    assert_eq!(d.text(TextField::CardBrand), Some("Visa"));
    assert_eq!(d.text(TextField::CardExpiry), Some("08/2029"));
    assert_eq!(secret(d, SecretField::CardNumber), "4111 1111 1111 1111");
    assert_eq!(secret(d, SecretField::CardVerificationCode), "123");
    assert_eq!(secret(d, SecretField::CardPin), "9876");
    assert_eq!(labels(d), ["Valid from", "Transaction password", "Bank"]);
    assert!(matches!(
        d.custom_fields[1].value,
        CustomFieldValue::Hidden(_)
    ));

    let d = draft(&import, 3);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::FirstName), Some("Alice"));
    assert_eq!(d.text(TextField::LastName), Some("Example"));
    assert_eq!(d.text(TextField::Email), Some("alice@example.com"));
    assert_eq!(d.text(TextField::Phone), Some("+1 555 0100"));
    assert_eq!(d.text(TextField::BirthDate), Some("1990-02-03"));
    assert_eq!(d.text(TextField::PostalCode), Some("10115"));
    assert_eq!(labels(d), ["Geburtstag", "Nickname", "Social security"]);
    assert!(matches!(
        d.custom_fields[2].value,
        CustomFieldValue::Hidden(_)
    ));

    let d = draft(&import, 4);
    assert_eq!(d.item_type, ItemType::SecureNote);
    assert_eq!(d.text(TextField::Notes), Some("Flour, water, salt"));

    let d = draft(&import, 5);
    assert_eq!(d.item_type, ItemType::Wifi);
    assert_eq!(d.text(TextField::WifiSsid), Some("example-net"));
    assert_eq!(d.text(TextField::WifiSecurity), Some("WPA3"));
    assert_eq!(secret(d, SecretField::WifiPassword), "wifi-pass");
    assert_eq!(d.subtitle(), Some("example-net"));
    assert_eq!(labels(d), ["Band"]);

    let d = draft(&import, 6);
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(labels(d), ["Server"]);

    let e = entry(&import, 7);
    assert_eq!(e.draft.item_type, ItemType::SecureNote);
    assert_eq!(labels(&e.draft), ["IBAN", "Online PIN"]);
    assert!(e.warnings.is_empty());

    let e = entry(&import, 8);
    assert_eq!(e.draft.item_type, ItemType::SecureNote);
    assert_eq!(e.warnings, [EntryWarning::UnknownCategory]);

    let p = import.preview(&[]);
    assert_eq!(
        p.counts,
        [
            (ItemType::Login, 3),
            (ItemType::CreditCard, 1),
            (ItemType::Identity, 1),
            (ItemType::SecureNote, 3),
            (ItemType::Wifi, 1)
        ]
    );
    assert_eq!((p.attachments, p.attachment_bytes), (1, 16));
    assert_eq!(
        p.warnings,
        [
            ImportWarning::UnmappedFields,
            ImportWarning::AttachmentsSkipped,
            ImportWarning::EntriesSkipped,
            ImportWarning::OtpUnreadable
        ]
    );
    // Every entry makes a valid item.
    for e in &import.entries {
        e.draft
            .to_item(
                mosaic_core::DeviceId::from_bytes([2; 16]).unwrap(),
                mosaic_core::Timestamp::from_unix_millis(0),
            )
            .unwrap();
    }
}

#[test]
fn enpass_errors() {
    for bad in [&b"[]"[..], b"{\"items\":{}}", b"{", b"{\"folders\":[]}"] {
        assert_eq!(
            parse(Format::Enpass, bad, None).unwrap_err(),
            Error::Malformed
        );
    }
    let empty = parse(Format::Enpass, b"{\"items\":[]}", None).unwrap();
    assert!(empty.entries.is_empty() && empty.skipped.is_empty());
}

#[test]
fn bitwarden_export() {
    let import = parse(Format::Bitwarden, &fixture("bitwarden.json"), None).unwrap();
    assert_eq!(
        import.skipped,
        [SkippedEntry {
            index: 8,
            reason: SkipReason::Malformed
        }]
    );
    let e = entry(&import, 0);
    let d = &e.draft;
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.title(), Some("Example Shop"));
    assert_eq!(d.text(TextField::Username), Some("alice"));
    assert_eq!(secret(d, SecretField::Password), "current-pw");
    assert_eq!(
        urls(d),
        [
            "https://shop.example.com/login",
            "https://www.shop.example.com"
        ]
    );
    let (otp, _) = d.otp.as_ref().unwrap();
    assert_eq!(otp.issuer.as_ref().unwrap().as_str(), "Shop");
    assert_eq!(
        labels(d),
        ["Customer number", "Security answer", "Newsletter"]
    );
    assert!(matches!(
        d.custom_fields[1].value,
        CustomFieldValue::Hidden(_)
    ));
    assert_eq!(history(d), ["oldest-pw", "older-pw"]);
    assert_eq!(tags(d), ["Personal", "Team Shared"]);
    assert!(d.favorite);
    assert_eq!(e.warnings, [EntryWarning::PasskeysNotImported]);

    let d = draft(&import, 1);
    assert_eq!(d.item_type, ItemType::SecureNote);
    assert!(d.trashed);

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(d.text(TextField::CardExpiry), Some("07/2031"));
    assert_eq!(d.text(TextField::CardBrand), Some("Mastercard"));
    assert_eq!(secret(d, SecretField::CardVerificationCode), "321");

    let d = draft(&import, 3);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::MiddleName), Some("B"));
    assert_eq!(d.text(TextField::Region), Some("BE"));
    assert_eq!(secret(d, SecretField::NationalId), "123-45-6789");
    assert_eq!(secret(d, SecretField::PassportNumber), "P1234567");
    assert_eq!(secret(d, SecretField::LicenseNumber), "L7654321");
    assert_eq!(labels(d), ["Title", "Address 3", "Username"]);

    let d = draft(&import, 4);
    assert_eq!(d.item_type, ItemType::SshKey);
    let key = d.ssh_key.as_ref().unwrap();
    assert_eq!(key.fingerprint.as_str(), "SHA256:synthetic");

    assert_eq!(labels(draft(&import, 5)), ["Expiry month"]);
    assert_eq!(labels(draft(&import, 6)), ["Expiry year"]);
    assert_eq!(entry(&import, 7).warnings, [EntryWarning::UnknownCategory]);
    let (steam, _) = draft(&import, 9).otp.as_ref().unwrap();
    assert_eq!(steam.kind, OtpKind::Steam);
}

#[test]
fn bitwarden_errors() {
    assert_eq!(
        parse(
            Format::Bitwarden,
            b"{\"encrypted\":true,\"items\":[]}",
            None
        )
        .unwrap_err(),
        Error::Encrypted
    );
    assert_eq!(
        parse(
            Format::Bitwarden,
            b"{\"passwordProtected\":true,\"data\":\"x\"}",
            None
        )
        .unwrap_err(),
        Error::Encrypted
    );
    assert_eq!(
        parse(Format::Bitwarden, b"{\"items\":3}", None).unwrap_err(),
        Error::Malformed
    );
}

fn onepux(extra: &[(&str, &[u8])], data: Option<&[u8]>) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflate = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("export.attributes", deflate).unwrap();
    zip.write_all(b"{\"version\":3,\"description\":\"1Password Unencrypted Export\"}")
        .unwrap();
    if let Some(d) = data {
        zip.start_file("export.data", deflate).unwrap();
        zip.write_all(d).unwrap();
    }
    for (name, bytes) in extra {
        zip.start_file(*name, deflate).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn onepassword_export() {
    let data = fixture("1pux-export.data.json");
    let file = onepux(
        &[
            ("files/doc-manual__manual.pdf", b"manual bytes"),
            ("files/doc-scan__scan.png", b"pngbytes"),
        ],
        Some(&data),
    );
    let import = parse(Format::OnePassword, &file, None).unwrap();
    assert_eq!(
        import.skipped,
        [SkippedEntry {
            index: 12,
            reason: SkipReason::Malformed
        }]
    );

    let e = entry(&import, 0);
    let d = &e.draft;
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.title(), Some("Example Login"));
    assert_eq!(d.text(TextField::Username), Some("alice"));
    assert_eq!(secret(d, SecretField::Password), "login-pw");
    assert_eq!(
        urls(d),
        ["https://login.example.com", "https://alt.example.com"]
    );
    assert!(d.otp.is_some());
    assert_eq!(
        labels(d),
        [
            "pin",
            "mail",
            "next",
            "city",
            "recovery code",
            "member since",
            "card valid",
            "admin console",
            "support",
            "hotline",
            "office",
            "plan",
            "count",
            "key"
        ]
    );
    let text = |i: usize| match &d.custom_fields[i].value {
        CustomFieldValue::Text(t) | CustomFieldValue::Date(t) => t.as_str().to_owned(),
        _ => String::new(),
    };
    assert_eq!(text(5), "2021-01-01");
    assert_eq!(text(6), "12/2025");
    assert_eq!(text(10), "1 Main St, 12345, Springfield, IL, us");
    assert_eq!(text(12), "7");
    assert_eq!(history(d), ["oldest-pw", "newer-old-pw"]);
    assert_eq!(tags(d), ["web", "important", "Private"]);
    assert!(d.favorite);
    assert_eq!(d.attachments.len(), 1);
    assert_eq!(d.attachments[0].file_name.as_str(), "manual.pdf");
    assert_eq!(d.attachments[0].mime.as_str(), "application/pdf");
    assert_eq!(
        e.warnings,
        [
            EntryWarning::UnmappedFields,
            EntryWarning::AttachmentTooLarge,
            EntryWarning::AttachmentMissing
        ]
    );

    let d = draft(&import, 1);
    assert_eq!(secret(d, SecretField::Password), "only-a-password");
    assert_eq!(tags(d), ["archived", "Private"]);

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(d.text(TextField::CardholderName), Some("Alice Example"));
    assert_eq!(d.text(TextField::CardBrand), Some("visa"));
    assert_eq!(d.text(TextField::CardExpiry), Some("01/2030"));
    assert_eq!(secret(d, SecretField::CardNumber), "4111111111111111");
    assert_eq!(secret(d, SecretField::CardVerificationCode), "999");
    assert_eq!(secret(d, SecretField::CardPin), "1234");

    let d = draft(&import, 3);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::MiddleName), Some("B"));
    assert_eq!(d.text(TextField::BirthDate), Some("1990-01-01"));
    assert_eq!(d.text(TextField::City), Some("Exampletown"));
    assert_eq!(d.text(TextField::Email), Some("alice@example.com"));
    assert_eq!(d.text(TextField::Phone), Some("+49 30 1"));
    assert_eq!(labels(d), ["job"]);

    let d = draft(&import, 4);
    assert_eq!(secret(d, SecretField::PassportNumber), "C01X00T47");
    assert_eq!(d.text(TextField::FirstName), Some("Alice Example"));

    let d = draft(&import, 5);
    assert_eq!(d.item_type, ItemType::Wifi);
    assert_eq!(d.text(TextField::WifiSecurity), Some("WPA2"));
    assert_eq!(secret(d, SecretField::WifiPassword), "wifi-secret");

    let d = draft(&import, 6);
    assert_eq!(d.item_type, ItemType::ApiKey);
    assert_eq!(d.text(TextField::ApiKeyId), Some("client-1"));
    assert_eq!(d.text(TextField::ApiKeyHost), Some("api.example.com"));
    assert_eq!(secret(d, SecretField::ApiKeySecret), "api-secret");

    let d = draft(&import, 7);
    assert_eq!(d.item_type, ItemType::SshKey);
    assert_eq!(
        d.ssh_key.as_ref().unwrap().public_key.as_str(),
        "ssh-ed25519 AAAAexample"
    );

    let d = draft(&import, 8);
    assert_eq!(d.item_type, ItemType::Document);
    assert_eq!(d.attachments[0].data.expose_secret(), b"pngbytes");

    let e = entry(&import, 9);
    assert_eq!(urls(&e.draft), ["ssh://server.example.com"]);
    assert_eq!(secret(&e.draft, SecretField::Password), "server-pw");
    assert_eq!(e.warnings, [EntryWarning::PasskeysNotImported]);

    let e = entry(&import, 10);
    assert!(e.warnings.is_empty());
    assert_eq!(labels(&e.draft), ["extra"]);

    let e = entry(&import, 11);
    assert!(e.draft.trashed);
    assert_eq!(e.warnings, [EntryWarning::UnknownCategory]);

    assert_eq!(tags(draft(&import, 13)), ["Shared"]);
}

#[test]
fn onepassword_errors() {
    assert_eq!(
        parse(Format::OnePassword, b"not a zip", None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::OnePassword, &onepux(&[], None), None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::OnePassword, &onepux(&[], Some(b"{}")), None).unwrap_err(),
        Error::Malformed
    );
    let one_vault = br#"{"accounts":[{"vaults":[{"attrs":{"name":"Only"},"items":[
        {"categoryUuid":"003","overview":{"title":"n"},"details":{"notesPlain":"x"}}]}]}]}"#;
    let import = parse(Format::OnePassword, &onepux(&[], Some(one_vault)), None).unwrap();
    assert!(import.entries[0].draft.tags.is_empty());

    // A member that is not deflate or stored.
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("export.data", stored).unwrap();
    zip.write_all(b"{}").unwrap();
    let mut bytes = zip.finish().unwrap().into_inner();
    // Method field of the local and the central header: 0 → 12 (bzip2).
    for sig in [&b"PK\x03\x04"[..], b"PK\x01\x02"] {
        let at = bytes.windows(4).position(|w| w == sig).unwrap();
        let off = if sig == b"PK\x03\x04" { 8 } else { 10 };
        bytes[at + off] = 12;
    }
    assert_eq!(
        parse(Format::OnePassword, &bytes, None).unwrap_err(),
        Error::Unsupported
    );
}

#[test]
fn csv_presets() {
    let chrome = b"name,url,username,password,note\n\
        GitHub,https://github.com/login,alice,pw1,\"multi\nline\"\n\
        ,https://example.org,bob,pw2,\n\
        ,,,,\n";
    let import = parse(Format::Csv, chrome, None).unwrap();
    assert_eq!(import.entries.len(), 2);
    let d = draft(&import, 0);
    assert_eq!(d.title(), Some("GitHub"));
    assert_eq!(d.text(TextField::Notes), Some("multi\nline"));
    assert_eq!(draft(&import, 1).title(), Some("example.org"));

    let firefox = b"\"url\",\"username\",\"password\",\"httpRealm\",\"formActionOrigin\",\"guid\",\"timeCreated\",\"timeLastUsed\",\"timePasswordChanged\"\r\n\
        \"https://www.mozilla.org\",\"carol\",\"pw3\",,\"https://www.mozilla.org\",\"{x}\",\"1\",\"2\",\"3\"\r\n";
    let import = parse(Format::Csv, firefox, None).unwrap();
    let d = draft(&import, 0);
    assert_eq!(d.title(), Some("mozilla.org"));
    assert!(d.custom_fields.is_empty());

    let safari = "\u{feff}Title,URL,Username,Password,Notes,OTPAuth\n\
        Bank,https://bank.example,dave,pw4,,otpauth://totp/Bank:dave?secret=GEZDGNBVGY3TQOJQ\n";
    let import = parse(Format::Csv, safari.as_bytes(), None).unwrap();
    assert!(draft(&import, 0).otp.is_some());

    let bitwarden = b"folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
        Work,1,login,Jira,,,0,https://jira.example,erin,pw5,\n";
    let import = parse(Format::Csv, bitwarden, None).unwrap();
    let d = draft(&import, 0);
    assert!(d.favorite);
    assert_eq!(tags(d), ["Work"]);

    let german = "Titel;Benutzername;Passwort;Webseite;Notizen;Gruppe;Tags;Kundennummer\n\
        Shop;frank;pw6;shop.example;Notiz;Privat;a, b;K-1\n";
    let import = parse(Format::Csv, german.as_bytes(), None).unwrap();
    let d = draft(&import, 0);
    assert_eq!(tags(d), ["Privat", "a", "b"]);
    assert_eq!(labels(d), ["Kundennummer"]);
}

#[test]
fn csv_custom_mapping_and_errors() {
    let data = b"a,b,c,d\nx,secret,ignored,GEZDGNBVGY3TQOJQ\n";
    let mapping = CsvMapping {
        columns: vec![
            CsvColumn::Title,
            CsvColumn::Hidden("Code".into()),
            CsvColumn::Ignore,
            CsvColumn::Otp,
        ],
    };
    let import = mosaic_import::csv::parse(data, Some(&mapping)).unwrap();
    let d = draft(&import, 0);
    assert_eq!(d.title(), Some("x"));
    assert!(matches!(
        d.custom_fields[0].value,
        CustomFieldValue::Hidden(_)
    ));
    assert!(d.otp.is_some());
    assert_eq!(parse(Format::Csv, b"", None).unwrap_err(), Error::Malformed);
    assert_eq!(
        parse(Format::Csv, b"\xff", None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::Csv, b"a,b\n\"open", None).unwrap_err(),
        Error::Malformed
    );
}

#[test]
fn file_size_limit() {
    // A slice longer than the limit is refused before it is read.
    let big = vec![b' '; mosaic_import::MAX_FILE_LEN + 1];
    for format in [
        Format::Enpass,
        Format::Bitwarden,
        Format::OnePassword,
        Format::Csv,
    ] {
        assert_eq!(parse(format, &big, None).unwrap_err(), Error::TooLarge);
    }
    assert_eq!(
        mosaic_import::enpass::parse(&big).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(
        mosaic_import::bitwarden::parse(&big).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(
        mosaic_import::onepux::parse(&big).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(
        mosaic_import::csv::parse(&big, None).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(
        mosaic_import::kdbx::parse(&big, &SecretString::from_str_copy("x").unwrap()).unwrap_err(),
        Error::TooLarge
    );
}

#[test]
fn kdbx_files_written_by_pykeepass() {
    let password = SecretString::from_str_copy("fixture-password").unwrap();
    for name in ["aes-argon2d.kdbx", "chacha-argon2id.kdbx"] {
        let import = parse(Format::Keepass, &fixture(name), Some(&password)).unwrap();
        assert_eq!(import.entries.len(), 3, "{name}");
        let d = import
            .entries
            .iter()
            .map(|e| &e.draft)
            .find(|d| d.title() == Some("Forum & Co"))
            .unwrap();
        assert_eq!(d.item_type, ItemType::Login);
        assert_eq!(d.text(TextField::Username), Some("alice"));
        assert_eq!(secret(d, SecretField::Password), "pässwörd <3>");
        assert_eq!(urls(d), ["https://forum.example.org/login"]);
        assert_eq!(d.text(TextField::Notes), Some("line one\nline two"));
        let (otp, _) = d.otp.as_ref().unwrap();
        assert_eq!(otp.issuer.as_ref().unwrap().as_str(), "Forum");
        assert_eq!(labels(d), ["PIN", "Customer no"]);
        assert!(matches!(
            d.custom_fields[0].value,
            CustomFieldValue::Hidden(_)
        ));
        assert_eq!(history(d), ["first-password", "second-password"]);
        assert_eq!(tags(d), ["team", "web", "Work/Social"]);
        assert_eq!(d.attachments.len(), 1);
        assert_eq!(
            d.attachments[0].data.expose_secret(),
            b"synthetic attachment\n"
        );
        let note = import
            .entries
            .iter()
            .map(|e| &e.draft)
            .find(|d| d.title() == Some("Notes only"))
            .unwrap();
        assert_eq!(note.item_type, ItemType::SecureNote);
        let trashed = import
            .entries
            .iter()
            .map(|e| &e.draft)
            .find(|d| d.title() == Some("Old login"))
            .unwrap();
        assert!(trashed.trashed);
        assert_eq!(
            parse(
                Format::Keepass,
                &fixture(name),
                Some(&SecretString::from_str_copy("wrong").unwrap())
            )
            .unwrap_err(),
            Error::WrongCredentials
        );
    }
}

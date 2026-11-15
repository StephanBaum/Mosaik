//! `LastPass` CSV: logins, secure notes and structured note types (synthetic
//! file, written from public format descriptions).

#![expect(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

use mosaic_core::model::{CustomFieldValue, ItemType, SecretField, TextField};
use mosaic_import::csv::{CsvMapping, CsvPreset};
use mosaic_import::{EntryWarning, Format, parse};

use crate::support::{draft, entry, labels, secret, tags, urls};

const HEADER: &str = "url,username,password,totp,extra,name,grouping,fav";

/// A CSV field in quotes.
fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

#[expect(clippy::too_many_arguments, reason = "one argument per column")]
fn row(url: &str, user: &str, pw: &str, totp: &str, extra: &str, name: &str, group: &str, fav: &str) -> String {
    format!(
        "{url},{user},{pw},{totp},{},{name},{group},{fav}\n",
        q(extra)
    )
}

fn export() -> String {
    let mut csv = format!("{HEADER}\n");
    csv += &row(
        "https://github.com/login",
        "alice",
        "pw1",
        "otpauth://totp/GitHub:alice?secret=GEZDGNBVGY3TQOJQ",
        "my note",
        "GitHub",
        "Work\\Dev",
        "1",
    );
    csv += &row("http://sn", "", "", "", "just text", "Plain note", "", "0");
    csv += &row(
        "http://sn",
        "",
        "",
        "",
        "NoteType:Credit Card\nLanguage:en-US\nName on Card:Alice Doe\nType:Visa\nNumber:4111111111111111\nSecurity Code:123\nStart Date:,\nExpiration Date:January,2030\nNotes:line1\nline2",
        "My Card",
        "",
        "0",
    );
    csv += &row(
        "http://sn",
        "",
        "",
        "",
        "NoteType:Address\nLanguage:en-US\nTitle:\nFirst Name:Alice\nLast Name:Doe\nGender:f\nPhone:{\"num\":\"123\",\"ext\":\"\",\"cc3l\":\"DEU\"}",
        "Home",
        "",
        "0",
    );
    csv += &row(
        "http://sn",
        "",
        "",
        "",
        "NoteType:Wi-Fi Password\nLanguage:en-US\nSSID:home\nPassword:wifipw\nConnection Type:WPA2",
        "Router",
        "",
        "0",
    );
    csv += &row(
        "http://sn",
        "",
        "",
        "",
        "NoteType:Passport\nLanguage:en-US\nNumber:X1234567\nName:Alice",
        "Passport",
        "",
        "0",
    );
    csv += &row("http://", "bob", "pw3", "", "", "Site", "", "0");
    csv
}

#[test]
fn lastpass_export() {
    let import = parse(Format::Csv, export().as_bytes(), None).unwrap();
    assert_eq!(import.entries.len(), 7);
    assert!(import.skipped.is_empty());

    let d = draft(&import, 0);
    assert_eq!(d.item_type, ItemType::Login);
    assert!(d.otp.is_some());
    assert_eq!(d.text(TextField::Notes), Some("my note"));
    assert_eq!(tags(d), ["Work\\Dev"]);
    assert!(d.favorite);

    let d = draft(&import, 1);
    assert_eq!(d.item_type, ItemType::SecureNote);
    assert_eq!(d.title(), Some("Plain note"));
    assert_eq!(d.text(TextField::Notes), Some("just text"));
    assert!(urls(d).is_empty());

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(d.text(TextField::CardholderName), Some("Alice Doe"));
    assert_eq!(d.text(TextField::CardBrand), Some("Visa"));
    assert_eq!(secret(d, SecretField::CardNumber), "4111111111111111");
    assert_eq!(secret(d, SecretField::CardVerificationCode), "123");
    assert_eq!(d.text(TextField::CardExpiry), Some("01/2030"));
    assert_eq!(d.text(TextField::Notes), Some("line1\nline2"));
    assert!(labels(d).is_empty());

    let e = entry(&import, 3);
    assert_eq!(e.draft.item_type, ItemType::Identity);
    assert_eq!(e.draft.text(TextField::FirstName), Some("Alice"));
    assert_eq!(e.draft.text(TextField::Phone), Some("123"));
    assert_eq!(labels(&e.draft), ["Gender"]);
    assert!(e.warnings.contains(&EntryWarning::UnmappedFields));

    let d = draft(&import, 4);
    assert_eq!(d.item_type, ItemType::Wifi);
    assert_eq!(d.text(TextField::WifiSsid), Some("home"));
    assert_eq!(secret(d, SecretField::WifiPassword), "wifipw");
    assert_eq!(d.text(TextField::WifiSecurity), Some("WPA2"));

    let d = draft(&import, 5);
    assert_eq!(d.item_type, ItemType::SecureNote);
    assert_eq!(labels(d), ["Number", "Name"]);
    assert!(matches!(d.custom_fields[0].value, CustomFieldValue::Hidden(_)));
    assert!(matches!(d.custom_fields[1].value, CustomFieldValue::Text(_)));

    let e = entry(&import, 6);
    assert!(urls(&e.draft).is_empty());
    assert!(!e.warnings.contains(&EntryWarning::UrlKeptAsText));
}

#[test]
fn lastpass_header_with_an_extra_column_still_detects() {
    let header = format!("{HEADER},last_touch");
    let names: Vec<&str> = header.split(',').collect();
    assert_eq!(CsvMapping::detect(&names).1, Some(CsvPreset::LastPass));
    let csv = format!("{header}\nhttps://a.example,alice,pw1,,,A,,0,12345\n");
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    assert_eq!(labels(draft(&import, 0)), ["last_touch"]);
}

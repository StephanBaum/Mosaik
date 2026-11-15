//! `NordPass` CSV: item types, additional URLs and custom fields (synthetic
//! file, written from public format descriptions).

#![expect(clippy::unwrap_used, reason = "tests")]

use mosaic_core::model::{CustomFieldValue, ItemType, SecretField, TextField};
use mosaic_import::csv::{CsvMapping, CsvPreset};
use mosaic_import::{EntryWarning, Format, parse};

use crate::support::{draft, entry, labels, secret, tags, urls};

const HEADER: &str = "name,url,additional_urls,username,password,note,cardholdername,cardnumber,cvc,pin,expirydate,zipcode,folder,full_name,phone_number,email,address1,address2,city,country,state,type,custom_fields";

/// One CSV row: the given `(column, value)` pairs, every other column empty.
fn row(pairs: &[(&str, &str)]) -> String {
    HEADER
        .split(',')
        .map(|col| {
            let v = pairs.iter().find(|(c, _)| *c == col).map_or("", |(_, v)| *v);
            format!("\"{}\"", v.replace('"', "\"\""))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn export() -> String {
    let rows = [
        // login with an additional URL and a hidden custom field
        row(&[
            ("name", "GitHub"),
            ("url", "https://a.example"),
            ("additional_urls", "[\"https://b.example\"]"),
            ("username", "alice"),
            ("password", "pw1"),
            ("note", "n1"),
            ("folder", "Work"),
            ("type", "password"),
            (
                "custom_fields",
                "[{\"label\":\"PIN\",\"type\":\"hidden\",\"value\":\"1234\"}]",
            ),
        ]),
        row(&[("name", "Memo"), ("note", "body"), ("type", "note")]),
        row(&[
            ("name", "Visa"),
            ("cardholdername", "Alice Doe"),
            ("cardnumber", "4111111111111111"),
            ("cvc", "123"),
            ("pin", "0000"),
            ("expirydate", "03/29"),
            ("zipcode", "12345"),
            ("type", "credit_card"),
        ]),
        row(&[
            ("name", "Me"),
            ("full_name", "Alice Doe"),
            ("phone_number", "555"),
            ("email", "alice@example.com"),
            ("address1", "1 Main St"),
            ("city", "Springfield"),
            ("country", "US"),
            ("state", "IL"),
            ("type", "identity"),
        ]),
        row(&[("name", "Work"), ("type", "folder")]),
        row(&[("name", "Key"), ("type", "passkey")]),
    ];
    format!("{HEADER}\n{}\n", rows.join("\n"))
}

#[test]
fn nordpass_export() {
    let csv = export();
    let names: Vec<&str> = HEADER.split(',').collect();
    assert_eq!(CsvMapping::detect(&names).1, Some(CsvPreset::NordPass));
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    assert_eq!(import.entries.len(), 5);
    assert!(import.skipped.is_empty());

    let d = draft(&import, 0);
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(urls(d), ["https://a.example", "https://b.example"]);
    assert_eq!(labels(d), ["PIN"]);
    assert!(matches!(d.custom_fields[0].value, CustomFieldValue::Hidden(_)));
    assert_eq!(tags(d), ["Work"]);

    assert_eq!(draft(&import, 1).item_type, ItemType::SecureNote);

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(secret(d, SecretField::CardNumber), "4111111111111111");
    assert_eq!(secret(d, SecretField::CardVerificationCode), "123");
    assert_eq!(secret(d, SecretField::CardPin), "0000");
    assert_eq!(d.text(TextField::CardExpiry), Some("03/2029"));

    let d = draft(&import, 3);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::FirstName), Some("Alice"));
    assert_eq!(d.text(TextField::LastName), Some("Doe"));
    assert_eq!(d.text(TextField::Email), Some("alice@example.com"));
    assert_eq!(d.text(TextField::Phone), Some("555"));

    let e = entry(&import, 4);
    assert_eq!(e.draft.item_type, ItemType::SecureNote);
    assert!(e.warnings.contains(&EntryWarning::UnknownCategory));
}

#[test]
fn nordpass_bad_custom_fields_are_kept_as_text() {
    let csv = format!(
        "{HEADER}\n{}\n",
        row(&[
            ("name", "Site"),
            ("url", "https://a.example"),
            ("username", "alice"),
            ("password", "pw1"),
            ("type", "password"),
            ("custom_fields", "not json"),
        ])
    );
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let e = entry(&import, 0);
    assert_eq!(labels(&e.draft), ["Custom fields"]);
    assert!(e.warnings.contains(&EntryWarning::UnmappedFields));
}

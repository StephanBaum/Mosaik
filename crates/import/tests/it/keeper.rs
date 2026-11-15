//! Keeper JSON and CSV exports (synthetic files, written from public format
//! descriptions).

#![expect(clippy::assert_is_empty, reason = "tests")]

use mosaic_core::model::{CustomFieldValue, ItemType, SecretField, TextField};
use mosaic_import::{EntryWarning, Error, Format, SkipReason, SkippedEntry, parse};

use crate::support::{draft, entry, labels, secret, tags};

const JSON: &str = r#"{"shared_folders":[],"records":[
  {"title":"Site","login":"alice","password":"pw1","login_url":"https://a.example","notes":"n",
   "custom_fields":{"$oneTimeCode":"otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ","$text:Recovery":"abc"},
   "folders":[{"folder":"Personal"},{"shared_folder":"Team","folder":"Ops"}]},
  {"title":"Card","$type":"bankCard","custom_fields":{
   "$paymentCard":{"cardNumber":"4111111111111111","cardExpirationDate":"04/2030","cardSecurityCode":"123"},
   "$pinCode":"0000"}},
  {"title":"Me","$type":"contact","custom_fields":{"$name":{"first":"Alice","last":"Doe"},
   "$email":"alice@example.com","$phone":[{"number":"123"}],"$address":{"city":"Springfield"}}},
  {"title":"Memo","$type":"encryptedNotes","notes":"body text"},
  {"title":"License","$type":"softwareLicense","custom_fields":{"$licenseNumber":"ABC-1"}}
]}"#;

#[test]
fn keeper_json_export() {
    let import = parse(Format::Keeper, JSON.as_bytes(), None).unwrap();
    assert_eq!(import.entries.len(), 5);
    assert!(import.skipped.is_empty());

    let d = draft(&import, 0);
    assert_eq!(d.item_type, ItemType::Login);
    assert!(d.otp.is_some());
    assert_eq!(labels(d), ["Recovery"]);
    assert_eq!(tags(d), ["Personal", "Team/Ops"]);

    let d = draft(&import, 1);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(d.text(TextField::CardExpiry), Some("04/2030"));
    assert_eq!(secret(d, SecretField::CardVerificationCode), "123");
    assert_eq!(secret(d, SecretField::CardPin), "0000");

    let d = draft(&import, 2);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::FirstName), Some("Alice"));
    assert_eq!(d.text(TextField::LastName), Some("Doe"));
    assert_eq!(d.text(TextField::Email), Some("alice@example.com"));
    assert_eq!(d.text(TextField::Phone), Some("123"));
    assert_eq!(d.text(TextField::City), Some("Springfield"));

    let d = draft(&import, 3);
    assert_eq!(d.item_type, ItemType::SecureNote);
    assert_eq!(d.text(TextField::Notes), Some("body text"));

    let e = entry(&import, 4);
    assert_eq!(e.draft.item_type, ItemType::SecureNote);
    assert!(e.warnings.contains(&EntryWarning::UnknownCategory));
    assert_eq!(labels(&e.draft), ["$licenseNumber"]);
    assert!(matches!(
        e.draft.custom_fields[0].value,
        CustomFieldValue::Hidden(_)
    ));
}

#[test]
fn keeper_json_errors() {
    assert_eq!(
        parse(Format::Keeper, br#"{"records":5}"#, None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::Keeper, b"[", None).unwrap_err(),
        Error::Malformed
    );
    let import = parse(Format::Keeper, br#"{"records":[1]}"#, None).unwrap();
    assert!(import.entries.is_empty());
    assert_eq!(
        import.skipped,
        [SkippedEntry {
            index: 0,
            reason: SkipReason::Malformed
        }]
    );
}

#[test]
fn keeper_json_values_nested_deeper_are_dropped_with_a_warning() {
    let json =
        br#"{"records":[{"title":"T","custom_fields":{"$text:A":[["x"]],"B":{"c":{"d":"e"}}}}]}"#;
    let import = parse(Format::Keeper, json, None).unwrap();
    assert!(
        entry(&import, 0)
            .warnings
            .contains(&EntryWarning::FieldsDropped)
    );
}

#[test]
fn keeper_csv_export() {
    let csv = "Personal,GitHub,alice,pw1,https://github.com,n,,TFC:Keeper,otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ,Recovery,abc\n\
        ,Team note,,,,body,Team\n";
    let import = parse(Format::KeeperCsv, csv.as_bytes(), None).unwrap();
    assert_eq!(import.entries.len(), 2);
    let d = draft(&import, 0);
    assert_eq!(d.title(), Some("GitHub"));
    assert_eq!(tags(d), ["Personal"]);
    assert!(d.otp.is_some());
    assert_eq!(labels(d), ["Recovery"]);
    let d = draft(&import, 1);
    assert_eq!(tags(d), ["Team"]);
    assert_eq!(d.text(TextField::Notes), Some("body"));
    assert_eq!(
        parse(Format::KeeperCsv, b"", None).unwrap_err(),
        Error::Malformed
    );
}

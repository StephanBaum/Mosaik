//! Proton Pass export (synthetic file, written from public format
//! descriptions).

use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_import::{EntryWarning, Error, Format, parse};

use crate::support::{draft, entry, labels, secret, tags, zip_of};

const DATA: &str = r#"{"version":"1.0","userId":"u","encrypted":false,"vaults":{
 "v1":{"name":"Personal","items":[
  {"data":{"metadata":{"name":"GitHub","note":"n"},"type":"login","content":{
    "itemUsername":"alice","itemEmail":"alice@example.com","password":"pw1",
    "urls":["https://github.com","https://gist.github.com"],
    "totpUri":"otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ","passkeys":[]},
   "extraFields":[{"fieldName":"pin","type":"hidden","data":{"content":"1234"}},
    {"fieldName":"2fa","type":"totp","data":{"content":"otpauth://totp/y?secret=GEZDGNBVGY3TQOJQ"}}]},
   "state":1,"pinned":true},
  {"data":{"metadata":{"name":"Old","note":""},"type":"login","content":{
    "itemUsername":"bob","password":"pw2","urls":[],"totpUri":""}},"state":2,"pinned":false},
  {"data":{"metadata":{"name":"Alias","note":""},"type":"alias","content":{}},
   "aliasEmail":"a.b@alias.example","state":1},
  {"data":{"metadata":{"name":"Key","note":""},"type":"login","content":{
    "itemUsername":"k","password":"pw3","urls":[],"totpUri":"","passkeys":[{}]}},"state":1}
 ]},
 "v2":{"name":"Work","items":[
  {"data":{"metadata":{"name":"Visa","note":""},"type":"creditCard","content":{
    "cardholderName":"Alice Doe","cardType":"Visa","number":"4111111111111111",
    "verificationNumber":"123","expirationDate":"2030-05","pin":"0000"}},"state":1},
  {"data":{"metadata":{"name":"Me","note":""},"type":"identity","content":{
    "fullName":"Alice Doe","email":"alice@example.com"}},"state":1}
 ]}}}"#;

fn zipped() -> Vec<u8> {
    zip_of(&[
        ("Proton Pass/readme.txt", b"hi"),
        ("Proton Pass/data.json", DATA.as_bytes()),
    ])
}

#[test]
fn proton_pass_zip_export() {
    let import = parse(Format::ProtonPass, &zipped(), None).unwrap();
    assert_eq!(import.entries.len(), 6);
    for (i, vault) in [
        "Personal", "Personal", "Personal", "Personal", "Work", "Work",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(tags(draft(&import, u32::try_from(i).unwrap())), [*vault]);
    }
    let d = draft(&import, 0);
    assert!(d.favorite);
    assert!(d.otp.is_some());
    assert_eq!(d.urls.len(), 2);
    assert_eq!(d.text(TextField::Username), Some("alice"));
    assert_eq!(secret(d, SecretField::Password), "pw1");
    // The e-mail, the hidden field and the second TOTP are kept.
    assert_eq!(labels(d), ["Email", "pin", "2fa"]);
    assert!(draft(&import, 1).trashed);
    assert_eq!(
        draft(&import, 2).text(TextField::Username),
        Some("a.b@alias.example")
    );
    assert!(
        entry(&import, 3)
            .warnings
            .contains(&EntryWarning::PasskeysNotImported)
    );
    let d = draft(&import, 4);
    assert_eq!(d.item_type, ItemType::CreditCard);
    assert_eq!(d.text(TextField::CardExpiry), Some("05/2030"));
    assert_eq!(secret(d, SecretField::CardPin), "0000");
    let d = draft(&import, 5);
    assert_eq!(d.item_type, ItemType::Identity);
    assert_eq!(d.text(TextField::FirstName), Some("Alice"));
    assert_eq!(d.text(TextField::LastName), Some("Doe"));
}

#[test]
fn proton_pass_raw_data_json_matches_the_zip() {
    let zipped = parse(Format::ProtonPass, &zipped(), None).unwrap();
    let raw = parse(Format::ProtonPass, DATA.as_bytes(), None).unwrap();
    assert_eq!(raw.entries.len(), zipped.entries.len());
}

#[test]
fn proton_pass_encrypted_and_malformed() {
    assert_eq!(
        parse(
            Format::ProtonPass,
            br#"{"encrypted":true,"vaults":{}}"#,
            None
        )
        .unwrap_err(),
        Error::Encrypted
    );
    assert_eq!(
        parse(
            Format::ProtonPass,
            &zip_of(&[("Proton Pass/data.pgp", b"x")]),
            None
        )
        .unwrap_err(),
        Error::Encrypted
    );
    assert_eq!(
        parse(
            Format::ProtonPass,
            b"-----BEGIN PGP MESSAGE-----\n...\n-----END PGP MESSAGE-----",
            None
        )
        .unwrap_err(),
        Error::Encrypted
    );
    assert_eq!(
        parse(Format::ProtonPass, &zip_of(&[("readme.txt", b"hi")]), None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::ProtonPass, b"{}", None).unwrap_err(),
        Error::Malformed
    );
}

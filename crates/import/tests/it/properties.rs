//! Property tests over the public API: parsers never panic on arbitrary
//! input, CSV values survive RFC 4180 quoting, and every accepted entry
//! makes a valid item.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::format_push_string,
    clippy::assert_is_empty,
    reason = "tests"
)]

use mosaic_core::model::{SecretField, TextField};
use mosaic_core::{DeviceId, SecretString, Timestamp};
use mosaic_import::csv::{CsvColumn, CsvMapping};
use mosaic_import::{Format, Import, parse};
use proptest::prelude::*;

fn all_entries_make_items(import: &Import) {
    let device = DeviceId::from_bytes([4; 16]).unwrap();
    for e in &import.entries {
        let item = e
            .draft
            .to_item(device, Timestamp::from_unix_millis(0))
            .unwrap();
        item.to_json().unwrap();
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn parsers_never_panic(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        for format in [Format::Enpass, Format::Bitwarden, Format::OnePassword, Format::Csv] {
            if let Ok(import) = parse(format, &data, None) {
                all_entries_make_items(&import);
            }
        }
        let pw = SecretString::from_str_copy("x").unwrap();
        let _ = parse(Format::Keepass, &data, Some(&pw));
    }

    #[test]
    fn json_formats_accept_arbitrary_strings(
        title in ".{0,40}",
        user in ".{0,40}",
        password in ".{0,40}",
        url in ".{0,40}",
        tag in ".{0,200}",
        totp in ".{0,40}",
    ) {
        let s = |v: &str| serde_json::to_string(v).unwrap();
        let enpass = format!(
            r#"{{"items":[{{"category":"login","title":{},"tags":[{}],"fields":[
                {{"type":"username","label":"u","value":{}}},
                {{"type":"password","label":"p","value":{}}},
                {{"type":"url","label":"w","value":{}}},
                {{"type":"totp","label":"t","value":{}}}]}}]}}"#,
            s(&title), s(&tag), s(&user), s(&password), s(&url), s(&totp)
        );
        let import = parse(Format::Enpass, enpass.as_bytes(), None).unwrap();
        all_entries_make_items(&import);
        if let Some(e) = import.entries.first() {
            if !password.is_empty() {
                prop_assert_eq!(
                    e.draft.secret(SecretField::Password).unwrap().expose_secret(),
                    password.as_str()
                );
            }
            if !user.trim().is_empty() {
                prop_assert_eq!(e.draft.text(TextField::Username), Some(user.as_str()));
            }
        }
        let bitwarden = format!(
            r#"{{"items":[{{"type":1,"name":{},"login":{{"username":{},"password":{},
                "totp":{},"uris":[{{"uri":{}}}]}}}}],"folders":[]}}"#,
            s(&title), s(&user), s(&password), s(&totp), s(&url)
        );
        let import = parse(Format::Bitwarden, bitwarden.as_bytes(), None).unwrap();
        all_entries_make_items(&import);
    }

    #[test]
    fn csv_values_round_trip(
        rows in proptest::collection::vec(
            (".{0,30}", ".{0,30}", "[^\u{0}]{1,30}", ".{0,30}"),
            1..8,
        ),
        sep in prop_oneof![Just(','), Just(';')],
    ) {
        let mut text = format!("title{sep}user{sep}password{sep}extra\r\n");
        for (t, u, p, x) in &rows {
            text.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}\r\n",
                quote(t), quote(u), quote(p), quote(x)
            ));
        }
        let mapping = CsvMapping {
            columns: vec![
                CsvColumn::Title,
                CsvColumn::Username,
                CsvColumn::Password,
                CsvColumn::Text("extra".into()),
            ],
        };
        let import = mosaic_import::csv::parse(text.as_bytes(), Some(&mapping)).unwrap();
        all_entries_make_items(&import);
        prop_assert_eq!(import.entries.len() + import.skipped.len(), rows.len());
        for e in &import.entries {
            let (_, _, p, _) = &rows[e.index as usize];
            prop_assert_eq!(e.draft.secret(SecretField::Password).unwrap().expose_secret(), p.as_str());
        }
    }
}

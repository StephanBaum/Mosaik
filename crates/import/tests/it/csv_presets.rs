//! The CSV presets of other password managers (synthetic files, written from
//! public format descriptions).

#![expect(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

use mosaic_core::model::{CustomFieldValue, SecretField, TextField};
use mosaic_import::csv::{CsvMapping, CsvPreset};
use mosaic_import::{Format, parse};

use crate::support::{draft, labels, secret, tags, urls};

fn preset(header: &str) -> Option<CsvPreset> {
    let names: Vec<&str> = header.split(',').collect();
    CsvMapping::detect(&names).1
}

const ROBOFORM_HEADER: &str = "Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2";

#[test]
fn roboform_logins() {
    let csv = format!(
        "{ROBOFORM_HEADER}\nGitHub,https://github.com/login,https://github.com/,alice,pw1,n1,/Work/Dev,\"x,y\"\n"
    );
    assert_eq!(preset(ROBOFORM_HEADER), Some(CsvPreset::RoboForm));
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let d = draft(&import, 0);
    assert_eq!(urls(d), ["https://github.com/login", "https://github.com/"]);
    assert_eq!(d.text(TextField::Username), Some("alice"));
    assert_eq!(secret(d, SecretField::Password), "pw1");
    assert_eq!(d.text(TextField::Notes), Some("n1"));
    assert_eq!(tags(d), ["Work/Dev"]);
    assert_eq!(labels(d), ["RoboForm fields"]);
    assert!(matches!(d.custom_fields[0].value, CustomFieldValue::Hidden(_)));
}

#[test]
fn keepassxc_csv() {
    let csv = "\"Group\",\"Title\",\"Username\",\"Password\",\"URL\",\"Notes\",\"TOTP\",\"Icon\",\"Last Modified\",\"Created\"\n\
        \"Root/Banking\",\"Bank\",\"dave\",\"pw1\",\"https://bank.example\",\"\",\"otpauth://totp/Bank:dave?secret=GEZDGNBVGY3TQOJQ\",\"0\",\"2024-01-01T00:00:00Z\",\"2024-01-01T00:00:00Z\"\n\
        \"Root\",\"Mail\",\"erin\",\"pw2\",\"https://mail.example\",\"\",\"\",\"0\",\"\",\"\"\n";
    assert_eq!(
        preset("Group,Title,Username,Password,URL,Notes,TOTP,Icon,Last Modified,Created"),
        Some(CsvPreset::KeePassXc)
    );
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let d = draft(&import, 0);
    assert_eq!(tags(d), ["Banking"]);
    assert!(d.otp.is_some());
    assert!(d.custom_fields.is_empty());
    assert!(tags(draft(&import, 1)).is_empty());
}

#[test]
fn header_drift_keeps_unknown_columns() {
    let header = format!("{ROBOFORM_HEADER},Rf_Extra");
    let csv = format!("{header}\nGitHub,https://github.com,https://github.com/,alice,pw1,,,,kept\n");
    assert_eq!(preset(&header), Some(CsvPreset::RoboForm));
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    assert!(labels(draft(&import, 0)).contains(&"Rf_Extra"));
}

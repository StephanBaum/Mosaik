//! Dashlane CSV export: a zip of CSV files (synthetic, written from public
//! format descriptions).

use mosaic_core::model::{CustomFieldValue, ItemType, SecretField, TextField};
use mosaic_import::csv::{CsvMapping, CsvPreset};
use mosaic_import::{EntryWarning, Error, Format, parse};

use crate::support::{draft, entry, kinds, labels, secret, zip_of};

const CREDENTIALS_HEADER: &str =
    "username,username2,username3,title,password,note,url,category,otpSecret";

fn credentials() -> String {
    format!(
        "{CREDENTIALS_HEADER}\nalice,alice2,,GitHub,pw1,n,https://github.com,Work,GEZDGNBVGY3TQOJQ\n"
    )
}

fn archive() -> Vec<u8> {
    let payments = "type,account_name,account_holder,cc_number,code,expiration_month,expiration_year,routing_number,account_number,country,issuing_bank\n\
        payment_card,Visa,Alice Doe,4111111111111111,123,7,2031,,,US,Big Bank\n\
        bank,Checking,Alice Doe,,,,,021000021,12345678,US,Big Bank\n";
    let ids = "type,number,name,issue_date,expiration_date,place_of_issue,state\n\
        passport,X1234567,Alice Doe,2020-01-01,2030-01-01,Berlin,\n";
    let personal = "type,title,first_name,middle_name,last_name,login,date_of_birth,place_of_birth,email,email_type,item_name,phone_number,address,country,state,city,zip\n\
        name,Ms,Alice,,Doe,,1990-02-03,,alice@example.com,,Me,555,1 Main St,US,IL,Springfield,12345\n";
    zip_of(&[
        ("notes/other.csv", b"a,b\nx,y\n"),
        ("readme.txt", b"not a table"),
        ("personalInfo.csv", personal.as_bytes()),
        ("ids.csv", ids.as_bytes()),
        ("payments.csv", payments.as_bytes()),
        (
            "securenotes.csv",
            b"title,note,category\nMemo,body,Personal\n",
        ),
        ("credentials.csv", credentials().as_bytes()),
    ])
}

#[test]
fn dashlane_export() {
    let import = parse(Format::Dashlane, &archive(), None).unwrap();
    assert_eq!(
        kinds(&import),
        [
            ItemType::Login,
            ItemType::SecureNote,
            ItemType::CreditCard,
            ItemType::SecureNote,
            ItemType::Identity,
            ItemType::Identity,
            ItemType::SecureNote,
        ]
    );
    let d = draft(&import, 0);
    assert_eq!(labels(d), ["Username 2"]);
    assert!(d.otp.is_some());

    let d = draft(&import, 2);
    assert_eq!(d.text(TextField::CardExpiry), Some("07/2031"));
    assert_eq!(secret(d, SecretField::CardNumber), "4111111111111111");

    let d = draft(&import, 3);
    let i = labels(d)
        .iter()
        .position(|l| *l == "Account number")
        .unwrap();
    assert!(matches!(
        d.custom_fields[i].value,
        CustomFieldValue::Hidden(_)
    ));

    let d = draft(&import, 4);
    assert_eq!(secret(d, SecretField::PassportNumber), "X1234567");
    assert_eq!(d.text(TextField::LastName), Some("Doe"));

    let d = draft(&import, 5);
    assert_eq!(d.text(TextField::BirthDate), Some("1990-02-03"));
    assert_eq!(d.text(TextField::PostalCode), Some("12345"));

    assert!(
        entry(&import, 6)
            .warnings
            .contains(&EntryWarning::UnknownCategory)
    );
}

#[test]
fn dashlane_limits_and_errors() {
    let names: Vec<String> = (0..65).map(|i| format!("f{i}.csv")).collect();
    let members: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"a\nb\n"[..])).collect();
    assert_eq!(
        parse(Format::Dashlane, &zip_of(&members), None).unwrap_err(),
        Error::LimitExceeded
    );
    assert_eq!(
        parse(Format::Dashlane, b"not a zip", None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::Dashlane, &zip_of(&[("readme.txt", b"hi")]), None).unwrap_err(),
        Error::Malformed
    );
}

#[test]
fn extracted_credentials_csv_imports_as_csv() {
    let header: Vec<&str> = CREDENTIALS_HEADER.split(',').collect();
    assert_eq!(CsvMapping::detect(&header).1, Some(CsvPreset::Dashlane));
    let import = parse(Format::Csv, credentials().as_bytes(), None).unwrap();
    assert_eq!(kinds(&import), [ItemType::Login]);
    assert_eq!(labels(draft(&import, 0)), ["Username 2"]);
}

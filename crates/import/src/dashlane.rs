//! Dashlane CSV export: a zip of CSV files (`credentials.csv`,
//! `securenotes.csv`, `payments.csv`, `ids.csv`, `personalInfo.csv`, ...).
//!
//! Members are matched by their lowercase file name, in any folder, and read
//! in a fixed order; any other CSV member becomes secure notes. An encrypted
//! `.dash` backup is not a zip and is refused as malformed (the guide asks
//! for the CSV export). Written from public descriptions of the export; the
//! member names and headers are unverified against a current export (launch
//! gate).
//!
//! Every member is read through [`MAX_MEMBER_LEN`], the total inflated size
//! stays within [`crate::MAX_FILE_LEN`], and the archive has at most
//! [`MAX_MEMBERS`] members.

use mosaic_core::model::{ItemType, SecretField, TextField};

use crate::Format;
use crate::archive;
use crate::csv::{self, CsvMapping, CsvPreset, Named};
use crate::draft::{Builder, EntryWarning, Kind, is_iso_date};
use crate::entries::{Collector, Import};
use crate::error::{Error, Result};

/// Most members of the archive.
pub const MAX_MEMBERS: usize = 64;
/// Largest inflated CSV member: 64 MiB.
pub const MAX_MEMBER_LEN: usize = 64 * 1024 * 1024;

/// The members read first, in this order, by lowercase file name.
const KNOWN: [&str; 5] = [
    "credentials.csv",
    "securenotes.csv",
    "payments.csv",
    "ids.csv",
    "personalinfo.csv",
];

/// Reads a Dashlane export zip.
///
/// # Errors
/// [`Error::Malformed`] if the input is not a zip archive, has no CSV
/// member or has a CSV member that is not valid; [`Error::LimitExceeded`]
/// above [`MAX_MEMBERS`] members; [`Error::TooLarge`]; [`Error::Encrypted`];
/// [`Error::Unsupported`].
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let mut zip = archive::open(input, MAX_MEMBERS)?;
    // (member index, lowercase file name) of every CSV member.
    let mut csvs: Vec<(usize, String)> = Vec::new();
    for i in 0..zip.len() {
        if let Some(Ok(name)) = zip.name_for_index(i) {
            let file = name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .to_lowercase();
            if std::path::Path::new(&file)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
            {
                csvs.push((i, file));
            }
        }
    }
    if csvs.is_empty() {
        return Err(Error::Malformed);
    }
    let mut order: Vec<(usize, String)> = Vec::with_capacity(csvs.len());
    for known in KNOWN {
        if let Some(pos) = csvs.iter().position(|(_, f)| f == known) {
            order.push(csvs.remove(pos));
        }
    }
    order.append(&mut csvs);

    let mut c = Collector::new(Format::Dashlane);
    let mut left = crate::MAX_FILE_LEN;
    for (index, file) in order {
        let data = archive::read_member(&mut zip, index, MAX_MEMBER_LEN.min(left))?;
        left = left.saturating_sub(data.len());
        table(&mut c, &file, &data)?;
    }
    Ok(c.finish())
}

/// Reads the rows of one CSV member into entries.
fn table(c: &mut Collector, file: &str, data: &[u8]) -> Result<()> {
    let mut rows = csv::rows(data)?;
    let Some(header) = rows.next_row()? else {
        return Ok(());
    };
    let names: Vec<String> = header.iter().map(|s| csv::normalized(s)).collect();
    let labels: Vec<&str> = header.iter().map(|s| s.as_str()).collect();
    let mapping = CsvMapping::for_preset(&labels, CsvPreset::Dashlane);
    while let Some(row) = rows.next_row()? {
        if row.iter().all(|v| v.trim().is_empty()) {
            continue;
        }
        let n = Named::new(&header, &names, &row);
        let builder = match file {
            "credentials.csv" => csv::entry(&row, &mapping, Some(CsvPreset::Dashlane)),
            "securenotes.csv" => secure_note(&n),
            "payments.csv" => payment(&n),
            "ids.csv" => id(&n),
            "personalinfo.csv" => personal_info(&n),
            _ => unknown_table(&n),
        };
        c.push(builder)?;
    }
    Ok(())
}

fn secure_note(n: &Named<'_>) -> Builder {
    let mut b = Builder::new(ItemType::SecureNote);
    b.title(n.get("title"));
    b.notes(n.get("note"));
    if !n.get("category").trim().is_empty() {
        b.tag(n.get("category"));
    }
    rest(&mut b, n, &["title", "note", "category"]);
    b
}

/// Every other non-blank column as a field.
fn rest(b: &mut Builder, n: &Named<'_>, known: &[&str]) {
    for (label, value) in n.others(known) {
        b.content(label, Kind::for_key(label), value);
    }
}

fn payment(n: &Named<'_>) -> Builder {
    const CARD: [&str; 7] = [
        "type",
        "account name",
        "account holder",
        "cc number",
        "code",
        "expiration month",
        "expiration year",
    ];
    match n.get("type").trim() {
        "payment_card" => {
            let mut b = Builder::new(ItemType::CreditCard);
            b.title(n.get("account name"));
            b.text(
                TextField::CardholderName,
                n.get("account holder"),
                "Account holder",
            );
            b.secret(SecretField::CardNumber, n.get("cc number"), "Card number");
            b.secret(SecretField::CardVerificationCode, n.get("code"), "Code");
            let (month, year) = (n.get("expiration month"), n.get("expiration year"));
            if !month.trim().is_empty() || !year.trim().is_empty() {
                b.expiry(&format!("{}/{}", month.trim(), year.trim()), "Expiry date");
            }
            rest(&mut b, n, &CARD);
            b
        }
        "bank" => {
            let mut b = Builder::new(ItemType::SecureNote);
            b.title(n.get("account name"));
            for (label, column) in [
                ("Account holder", "account holder"),
                ("Country", "country"),
                ("Issuing bank", "issuing bank"),
            ] {
                b.custom(label, Kind::Text, n.get(column));
            }
            b.custom("Account number", Kind::Hidden, n.get("account number"));
            b.custom("Routing number", Kind::Hidden, n.get("routing number"));
            rest(
                &mut b,
                n,
                &[
                    "type",
                    "account name",
                    "account holder",
                    "country",
                    "issuing bank",
                    "account number",
                    "routing number",
                ],
            );
            b
        }
        _ => unknown_table(n),
    }
}

fn id(n: &Named<'_>) -> Builder {
    let mut b = Builder::new(ItemType::Identity);
    let kind = n.get("type").trim();
    let name = n.get("name").trim();
    b.title(if name.is_empty() { kind } else { name });
    b.full_name(name);
    let number = match kind {
        "passport" => SecretField::PassportNumber,
        "driver_license" => SecretField::LicenseNumber,
        _ => SecretField::NationalId,
    };
    b.secret(number, n.get("number"), "Number");
    b.content("Issue date", Kind::Date, n.get("issue date"));
    b.content("Expiration date", Kind::Date, n.get("expiration date"));
    rest(
        &mut b,
        n,
        &["type", "number", "name", "issue date", "expiration date"],
    );
    b
}

fn personal_info(n: &Named<'_>) -> Builder {
    let mut b = Builder::new(ItemType::Identity);
    let item = n.get("item name").trim();
    b.title(if item.is_empty() { n.get("type") } else { item });
    for (column, field, label) in [
        ("first name", TextField::FirstName, "First name"),
        ("middle name", TextField::MiddleName, "Middle name"),
        ("last name", TextField::LastName, "Last name"),
        ("email", TextField::Email, "Email"),
        ("phone number", TextField::Phone, "Phone"),
        ("address", TextField::AddressLine1, "Address"),
        ("city", TextField::City, "City"),
        ("state", TextField::Region, "State"),
        ("zip", TextField::PostalCode, "Zip code"),
        ("country", TextField::Country, "Country"),
    ] {
        b.text(field, n.get(column), label);
    }
    let birth = n.get("date of birth").trim();
    if is_iso_date(birth) {
        b.text(TextField::BirthDate, birth, "Date of birth");
    } else {
        b.fallback("Date of birth", Kind::Text, birth);
    }
    rest(
        &mut b,
        n,
        &[
            "type",
            "item name",
            "first name",
            "middle name",
            "last name",
            "email",
            "phone number",
            "address",
            "city",
            "state",
            "zip",
            "country",
            "date of birth",
        ],
    );
    b
}

/// A CSV member this importer has no mapping for: secure notes titled by
/// the first non-blank value, every column kept as a field.
fn unknown_table(n: &Named<'_>) -> Builder {
    let mut b = Builder::new(ItemType::SecureNote);
    b.warn(EntryWarning::UnknownCategory);
    let mut titled = false;
    for (label, value) in n.others(&[]) {
        if !titled {
            b.title(value);
            titled = true;
        }
        b.custom(label, Kind::for_key(label), value);
    }
    b
}

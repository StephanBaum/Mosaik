//! `LastPass` CSV rows: logins, plain secure notes and structured notes
//! (`NoteType:` ...) that carry cards, addresses and Wi-Fi passwords.
//!
//! Written from public descriptions of the export. The header and the note
//! field names are unverified against a current export (launch gate).

use mosaic_core::model::{ItemType, SecretField, TextField};

use crate::buf::Str;
use crate::csv::{self, CsvMapping, CsvPreset, Named};
use crate::draft::{Builder, Kind, is_iso_date};
use crate::json;

/// The URL `LastPass` gives secure notes.
const NOTE_URL: &str = "http://sn";

/// A structured note: `NoteType:<type>`, then `key:value` lines, then
/// `Notes:` with the rest of the text.
struct Structured<'a> {
    note_type: &'a str,
    fields: Vec<(&'a str, Str)>,
    notes: Option<&'a str>,
}

/// Where a structured-note key goes.
enum Target {
    Text(TextField),
    Secret(SecretField),
    Expiry,
    Phone,
    Birth,
}

/// One row of the export as an entry builder.
pub(crate) fn entry(n: &Named<'_>, row: &[Str], mapping: &CsvMapping) -> Builder {
    if n.get("url").trim() != NOTE_URL {
        return csv::entry(row, mapping, Some(CsvPreset::LastPass));
    }
    let extra = n.get("extra");
    let mut b = match structured(extra) {
        Some(s) => structured_entry(&s),
        None => {
            let mut b = Builder::new(ItemType::SecureNote);
            b.notes(extra);
            b
        }
    };
    b.title(n.get("name"));
    if !n.get("grouping").trim().is_empty() {
        b.tag(n.get("grouping"));
    }
    b.favorite(n.get("fav").trim() == "1");
    b
}

fn structured(extra: &str) -> Option<Structured<'_>> {
    let mut lines = extra.split_inclusive('\n');
    let first = lines.next()?;
    let note_type = first
        .trim_end_matches(['\r', '\n'])
        .strip_prefix("NoteType:")?;
    let mut pos = first.len();
    let mut fields: Vec<(&str, Str)> = Vec::new();
    let mut notes = None;
    for line in lines {
        let body = line.trim_end_matches(['\r', '\n']);
        if body.starts_with("Notes:") {
            notes = extra.get(pos.saturating_add("Notes:".len())..);
            break;
        }
        if let Some((key, value)) = body.split_once(':') {
            fields.push((key, Str::new(value.to_owned())));
        } else if let Some((_, last)) = fields.last_mut() {
            // A line without a colon continues the previous value.
            last.push('\n');
            last.push_str(body);
        }
        pos = pos.saturating_add(line.len());
    }
    Some(Structured {
        note_type,
        fields,
        notes,
    })
}

fn structured_entry(s: &Structured<'_>) -> Builder {
    let (item_type, table): (ItemType, &[(&str, Target)]) = match s.note_type {
        "Credit Card" => (ItemType::CreditCard, CARD),
        "Address" => (ItemType::Identity, ADDRESS),
        "Wi-Fi Password" => (ItemType::Wifi, WIFI),
        _ => (ItemType::SecureNote, &[]),
    };
    let mut b = Builder::new(item_type);
    for (key, value) in &s.fields {
        let v = value.as_str();
        // Blank values (`,` and white space only) carry nothing.
        if *key == "Language" || v.trim_matches(|c: char| c == ',' || c.is_whitespace()).is_empty()
        {
            continue;
        }
        match table.iter().find(|(k, _)| k == key).map(|(_, t)| t) {
            Some(Target::Text(f)) => b.text(*f, v, key),
            Some(Target::Secret(f)) => b.secret(*f, v, key),
            Some(Target::Expiry) => match month_year(v) {
                Some(e) => b.expiry(&e, key),
                None => b.expiry(v, key),
            },
            Some(Target::Phone) => b.text(TextField::Phone, &phone(v), key),
            Some(Target::Birth) => {
                if is_iso_date(v.trim()) {
                    b.text(TextField::BirthDate, v.trim(), key);
                } else {
                    b.fallback(key, Kind::Text, v);
                }
            }
            None => b.content(key, Kind::for_key(key), v),
        }
    }
    if let Some(notes) = s.notes {
        b.notes(notes);
    }
    b
}

const CARD: &[(&str, Target)] = &[
    ("Name on Card", Target::Text(TextField::CardholderName)),
    ("Type", Target::Text(TextField::CardBrand)),
    ("Number", Target::Secret(SecretField::CardNumber)),
    (
        "Security Code",
        Target::Secret(SecretField::CardVerificationCode),
    ),
    ("Expiration Date", Target::Expiry),
];

const ADDRESS: &[(&str, Target)] = &[
    ("First Name", Target::Text(TextField::FirstName)),
    ("Middle Name", Target::Text(TextField::MiddleName)),
    ("Last Name", Target::Text(TextField::LastName)),
    ("Company", Target::Text(TextField::Company)),
    ("Address 1", Target::Text(TextField::AddressLine1)),
    ("Address 2", Target::Text(TextField::AddressLine2)),
    ("City / Town", Target::Text(TextField::City)),
    ("State", Target::Text(TextField::Region)),
    ("Zip / Postal Code", Target::Text(TextField::PostalCode)),
    ("Country", Target::Text(TextField::Country)),
    ("Email Address", Target::Text(TextField::Email)),
    ("Phone", Target::Phone),
    ("Birthday", Target::Birth),
];

const WIFI: &[(&str, Target)] = &[
    ("SSID", Target::Text(TextField::WifiSsid)),
    ("Password", Target::Secret(SecretField::WifiPassword)),
    ("Connection Type", Target::Text(TextField::WifiSecurity)),
];

/// `MM/YYYY` from `<English month name>,<YYYY>`.
fn month_year(v: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let (month, year) = v.split_once(',')?;
    let month = month.trim().to_lowercase();
    let index = MONTHS.iter().position(|m| *m == month)?;
    let year = year.trim();
    (year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("{:02}/{year}", index.saturating_add(1)))
}

/// A phone value: the `num` of a JSON object, else the text as it is.
fn phone(v: &str) -> String {
    if v.trim_start().starts_with('{')
        && let Ok(doc) = json::parse(v.as_bytes())
        && let Some(num) = doc.str("num")
    {
        return num.to_owned();
    }
    v.to_owned()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

    use super::*;

    #[test]
    fn structured_continuation_lines_and_notes() {
        let s = structured("NoteType:X\nA:1\n2\nNotes:n\nm").unwrap();
        assert_eq!(s.note_type, "X");
        assert_eq!(s.fields.len(), 1);
        assert_eq!(s.fields[0].0, "A");
        assert_eq!(s.fields[0].1.as_str(), "1\n2");
        assert_eq!(s.notes, Some("n\nm"));
        assert!(structured("plain").is_none());
    }

    #[test]
    fn months_and_phones() {
        assert_eq!(month_year("January,2030").as_deref(), Some("01/2030"));
        assert_eq!(month_year("Smarch,2030"), None);
        assert_eq!(phone(r#"{"num":"123","ext":""}"#), "123");
        assert_eq!(phone("+49 1"), "+49 1");
    }
}

//! CSV with a header row and a column mapping (brief §5 "generic CSV with
//! column mapping").
//!
//! The reader follows RFC 4180: fields separated by `,` (or `;` or a tab,
//! detected from the header row), quoted with `"` (a doubled `""` inside
//! quotes is one quote), records separated by CRLF or LF; quoted fields may
//! span lines. A UTF-8 byte order mark is skipped. The file must be UTF-8.
//!
//! [`CsvMapping::detect`] recognizes the password exports of Chrome (and
//! other Chromium browsers), Firefox, Safari and Bitwarden by their header,
//! and otherwise maps columns by common English and German header names.
//! Every row becomes a login; columns mapped to nothing known become custom
//! fields. A row without a title is named after its URL's site.

use mosaic_core::model::{ItemType, SecretField, TextField};
use zeroize::Zeroizing;

use crate::Format;
use crate::buf::Str;
use crate::draft::{Builder, Kind};
use crate::entries::{Collector, Import};
use crate::error::{Error, Result};

/// Most columns per row.
pub const MAX_COLUMNS: usize = 256;

/// What a column holds.
#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum CsvColumn {
    /// The item title.
    Title,
    /// The user name (or email used to sign in).
    Username,
    /// The password.
    Password,
    /// A login URL.
    Url,
    /// Notes.
    Notes,
    /// An `otpauth://` URI or base32 TOTP secret.
    Otp,
    /// A folder, group or category: one tag.
    Folder,
    /// Tags separated by `,` or `;`.
    Tags,
    /// `1`/`true` marks a favorite.
    Favorite,
    /// A custom text field with this label.
    Text(String),
    /// A hidden custom field with this label.
    Hidden(String),
    /// Not imported.
    Ignore,
}

/// Known export layouts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum CsvPreset {
    /// Chrome, Edge, Brave and other Chromium browsers:
    /// `name,url,username,password,note`.
    Chrome,
    /// Firefox: `url,username,password,httpRealm,formActionOrigin,guid,…`.
    Firefox,
    /// Safari / macOS Passwords: `Title,URL,Username,Password,Notes,OTPAuth`.
    Safari,
    /// Bitwarden CSV: `folder,favorite,type,name,notes,fields,…,login_uri,…`.
    Bitwarden,
    /// `LastPass`: `url,username,password,totp,extra,name,grouping,fav`;
    /// secure notes use the URL `http://sn`.
    // unverified against a current export (launch gate)
    LastPass,
    /// `NordPass`: `name,url,additional_urls,username,password,note,...,type,custom_fields`.
    // unverified against a current export (launch gate)
    NordPass,
    /// Dashlane `credentials.csv`:
    /// `username,username2,username3,title,password,note,url,category,otpSecret`.
    // unverified against a current export (launch gate)
    Dashlane,
    /// `RoboForm`: `Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2`.
    // unverified against a current export (launch gate)
    RoboForm,
    /// `KeePassXC`: `Group,Title,Username,Password,URL,Notes,TOTP,Icon,...`.
    // unverified against a current export (launch gate)
    KeePassXc,
}

/// A mapping of every column of the header row.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CsvMapping {
    /// One entry per column, in header order. Missing columns are ignored.
    pub columns: Vec<CsvColumn>,
}

impl CsvMapping {
    /// The mapping for a header row: the preset it matches, or a mapping by
    /// header names (English and German); unknown headers become custom
    /// text fields.
    #[must_use]
    pub fn detect(header: &[&str]) -> (Self, Option<CsvPreset>) {
        let preset = detect_preset(header);
        let columns = header.iter().map(|h| column_for(h, preset)).collect();
        (Self { columns }, preset)
    }

    /// The mapping for a header row of a known `preset`, even if the header
    /// lacks some of the columns detection looks for.
    pub(crate) fn for_preset(header: &[&str], preset: CsvPreset) -> Self {
        Self {
            columns: header.iter().map(|h| column_for(h, Some(preset))).collect(),
        }
    }
}

pub(crate) fn normalized(h: &str) -> String {
    h.trim()
        .trim_start_matches('\u{feff}')
        .to_lowercase()
        .replace(['_', '-'], " ")
}

// The RoboForm and KeePassXC header tables below follow public format
// descriptions: unverified against a current export (launch gate).
fn detect_preset(header: &[&str]) -> Option<CsvPreset> {
    let h: Vec<String> = header.iter().map(|s| normalized(s)).collect();
    let has = |names: &[&str]| names.iter().all(|n| h.iter().any(|x| x == n));
    if has(&["login uri", "login username", "login password"]) {
        Some(CsvPreset::Bitwarden)
    } else if has(&["httprealm", "formactionorigin", "guid"]) {
        Some(CsvPreset::Firefox)
    } else if has(&["url", "username", "password", "extra", "name", "grouping"]) {
        Some(CsvPreset::LastPass)
    } else if has(&[
        "name",
        "url",
        "username",
        "password",
        "note",
        "folder",
        "type",
        "cardnumber",
    ]) {
        Some(CsvPreset::NordPass)
    } else if has(&["name", "url", "matchurl", "login", "pwd"]) {
        Some(CsvPreset::RoboForm)
    } else if has(&[
        "username",
        "username2",
        "username3",
        "title",
        "password",
        "note",
        "url",
        "category",
    ]) {
        Some(CsvPreset::Dashlane)
    } else if has(&["group", "title", "username", "password", "url", "notes"]) {
        Some(CsvPreset::KeePassXc)
    } else if has(&["title", "url", "username", "password", "otpauth"]) {
        Some(CsvPreset::Safari)
    } else if h == ["name", "url", "username", "password", "note"]
        || h == ["name", "url", "username", "password"]
    {
        Some(CsvPreset::Chrome)
    } else {
        None
    }
}

fn column_for(header: &str, preset: Option<CsvPreset>) -> CsvColumn {
    let h = normalized(header);
    if preset == Some(CsvPreset::Firefox)
        && matches!(
            h.as_str(),
            "httprealm"
                | "formactionorigin"
                | "guid"
                | "timecreated"
                | "timelastused"
                | "timepasswordchanged"
        )
    {
        return CsvColumn::Ignore;
    }
    if preset == Some(CsvPreset::Bitwarden) && matches!(h.as_str(), "type" | "reprompt" | "fields")
    {
        return CsvColumn::Ignore;
    }
    if preset == Some(CsvPreset::RoboForm) {
        match h.as_str() {
            "matchurl" => return CsvColumn::Url,
            "pwd" => return CsvColumn::Password,
            "rffieldsv2" => return CsvColumn::Hidden("RoboForm fields".to_owned()),
            _ => {}
        }
    }
    if preset == Some(CsvPreset::Dashlane) {
        match h.as_str() {
            "username2" => return CsvColumn::Text("Username 2".to_owned()),
            "username3" => return CsvColumn::Text("Username 3".to_owned()),
            "otpsecret" | "otpurl" => return CsvColumn::Otp,
            _ => {}
        }
    }
    if preset == Some(CsvPreset::KeePassXc)
        && matches!(h.as_str(), "icon" | "last modified" | "created")
    {
        return CsvColumn::Ignore;
    }
    match h.as_str() {
        "name" | "title" | "titel" | "account" | "item" | "entry" | "eintrag" => CsvColumn::Title,
        "username" | "user name" | "user" | "login" | "login username" | "email" | "e mail"
        | "benutzername" | "benutzer" | "login name" => CsvColumn::Username,
        "password" | "passwort" | "pass" | "login password" | "kennwort" => CsvColumn::Password,
        "url" | "uri" | "website" | "web site" | "login uri" | "webseite" | "address"
        | "adresse" | "link" => CsvColumn::Url,
        "note" | "notes" | "notiz" | "notizen" | "comment" | "comments" | "kommentar" | "extra"
        | "bemerkung" => CsvColumn::Notes,
        "otp" | "totp" | "otpauth" | "2fa" | "login totp" | "one time password" => CsvColumn::Otp,
        "folder" | "group" | "category" | "ordner" | "gruppe" | "kategorie" | "grouping" => {
            CsvColumn::Folder
        }
        "tags" | "tag" | "labels" => CsvColumn::Tags,
        "favorite" | "favourite" | "favorit" | "fav" => CsvColumn::Favorite,
        _ if h.is_empty() => CsvColumn::Ignore,
        _ => CsvColumn::Text(header.trim().trim_start_matches('\u{feff}').to_owned()),
    }
}

/// Reads a CSV export with `mapping`, or with the detected mapping
/// ([`CsvMapping::detect`]) if `None`.
///
/// # Errors
/// [`Error::Malformed`] for a file that is not UTF-8, has no header row, has
/// an unterminated quoted field or more than [`MAX_COLUMNS`] columns;
/// [`Error::TooLarge`], [`Error::LimitExceeded`].
pub fn parse(input: &[u8], mapping: Option<&CsvMapping>) -> Result<Import> {
    let mut rows = rows(input)?;
    let header = rows.next_row()?.ok_or(Error::Malformed)?;
    let names: Vec<String> = header.iter().map(|s| normalized(s)).collect();
    let detected;
    let (mapping, preset) = if let Some(m) = mapping {
        (m, None)
    } else {
        let h: Vec<&str> = header.iter().map(|s| s.as_str()).collect();
        let (m, preset) = CsvMapping::detect(&h);
        detected = m;
        (&detected, preset)
    };
    let mut c = Collector::new(Format::Csv);
    while let Some(row) = rows.next_row()? {
        if row.iter().all(|v| v.trim().is_empty()) {
            continue;
        }
        let builder = match preset {
            Some(CsvPreset::LastPass) => {
                crate::lastpass::entry(&Named::new(&header, &names, &row), &row, mapping)
            }
            Some(CsvPreset::NordPass) => {
                match crate::nordpass::entry(&Named::new(&header, &names, &row)) {
                    Some(b) => b,
                    // A folder row is not an entry.
                    None => continue,
                }
            }
            _ => entry(&row, mapping, preset),
        };
        c.push(builder)?;
    }
    Ok(c.finish())
}

/// A CSV file as records: size check, UTF-8 (anything else is
/// [`Error::Malformed`]), byte order mark and delimiter handled.
pub(crate) fn rows(input: &[u8]) -> Result<Rows<'_>> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let text = core::str::from_utf8(input).map_err(|_| Error::Malformed)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    Ok(Rows {
        rest: text,
        delimiter: detect_delimiter(text),
    })
}

/// The columns of one row by normalized header name.
pub(crate) struct Named<'a> {
    labels: &'a [Str],
    names: &'a [String],
    row: &'a [Str],
}

impl<'a> Named<'a> {
    pub(crate) const fn new(labels: &'a [Str], names: &'a [String], row: &'a [Str]) -> Self {
        Self { labels, names, row }
    }

    /// The value of the column named `name` (normalized), `""` if the
    /// header has no such column or the row is short.
    pub(crate) fn get(&self, name: &str) -> &'a str {
        self.names
            .iter()
            .position(|n| n == name)
            .and_then(|i| self.row.get(i))
            .map_or("", |v| v.as_str())
    }

    /// The original label and value of every non-blank column whose
    /// normalized name is not in `known`.
    pub(crate) fn others(&self, known: &[&str]) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.labels
            .iter()
            .zip(self.names)
            .zip(self.row)
            .filter(move |((_, name), value)| {
                !value.trim().is_empty() && !known.contains(&name.as_str())
            })
            .map(|((label, _), value)| (label.as_str(), value.as_str()))
    }
}

/// The tag for a folder value of `preset`.
fn folder_tag(preset: Option<CsvPreset>, v: &str) -> Option<&str> {
    let tag = match preset {
        Some(CsvPreset::RoboForm) => v.trim().trim_start_matches('/'),
        // The first path part is the database's root group.
        Some(CsvPreset::KeePassXc) => v.split_once('/').map(|(_, rest)| rest)?,
        _ => v,
    };
    (!tag.trim().is_empty()).then_some(tag)
}

pub(crate) fn entry(row: &[Str], mapping: &CsvMapping, preset: Option<CsvPreset>) -> Builder {
    let mut b = Builder::new(ItemType::Login);
    let mut title_set = false;
    let mut first_url: Option<&str> = None;
    for (col, value) in mapping.columns.iter().zip(row) {
        let v = value.as_str();
        match col {
            CsvColumn::Title => {
                title_set |= !v.trim().is_empty();
                b.title(v);
            }
            CsvColumn::Username => b.text(TextField::Username, v, "Username"),
            CsvColumn::Password => b.secret(SecretField::Password, v, "Password"),
            CsvColumn::Url => {
                if !v.trim().is_empty() {
                    first_url.get_or_insert(v);
                }
                b.url(v, "URL");
            }
            CsvColumn::Notes => b.notes(v),
            CsvColumn::Otp => b.otp_text(v, "One-time password"),
            CsvColumn::Folder => {
                if let Some(tag) = folder_tag(preset, v) {
                    b.tag(tag);
                }
            }
            CsvColumn::Tags => {
                for t in v.split([',', ';']).filter(|t| !t.trim().is_empty()) {
                    b.tag(t);
                }
            }
            CsvColumn::Favorite => {
                b.favorite(matches!(
                    v.trim().to_lowercase().as_str(),
                    "1" | "true" | "yes" | "ja"
                ));
            }
            CsvColumn::Text(label) => b.custom(label, Kind::Text, v),
            CsvColumn::Hidden(label) => b.custom(label, Kind::Hidden, v),
            CsvColumn::Ignore => {}
        }
    }
    if !title_set && let Some(site) = first_url.and_then(crate::url::site) {
        b.title(&site);
    }
    b
}

/// `,`, `;` or tab: whichever occurs most often outside quotes in the first
/// line (`,` on a tie or if none occurs).
fn detect_delimiter(text: &str) -> char {
    let (mut comma, mut semi, mut tab) = (0usize, 0usize, 0usize);
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' | '\r' if !quoted => break,
            ',' if !quoted => comma = comma.saturating_add(1),
            ';' if !quoted => semi = semi.saturating_add(1),
            '\t' if !quoted => tab = tab.saturating_add(1),
            _ => {}
        }
    }
    if semi > comma && semi >= tab {
        ';'
    } else if tab > comma && tab > semi {
        '\t'
    } else {
        ','
    }
}

/// An RFC 4180 record reader.
pub(crate) struct Rows<'a> {
    rest: &'a str,
    delimiter: char,
}

impl Rows<'_> {
    /// The next record, or `None` at the end of the input.
    pub(crate) fn next_row(&mut self) -> Result<Option<Vec<Str>>> {
        if self.rest.is_empty() {
            return Ok(None);
        }
        let mut row: Vec<Str> = Vec::new();
        let mut field = Str::default();
        let mut chars = self.rest.char_indices().peekable();
        let mut quoted = false;
        let mut at_start = true;
        let mut end = self.rest.len();
        while let Some((i, c)) = chars.next() {
            if quoted {
                quoted = quoted_char(c, &mut chars, &mut field);
                continue;
            }
            match c {
                '"' if at_start => {
                    quoted = true;
                    at_start = false;
                }
                c if c == self.delimiter => {
                    push_field(&mut row, &mut field)?;
                    at_start = true;
                }
                '\r' if chars.peek().is_some_and(|&(_, n)| n == '\n') => {}
                '\n' | '\r' => {
                    end = i.saturating_add(1);
                    break;
                }
                c => {
                    field.push(c);
                    at_start = false;
                }
            }
        }
        if quoted {
            return Err(Error::Malformed);
        }
        push_field(&mut row, &mut field)?;
        self.rest = self.rest.get(end..).unwrap_or_default();
        Ok(Some(row))
    }
}

/// One character inside a quoted field: a doubled quote is a literal quote,
/// a single one ends the quoting. Returns whether the field is still quoted.
fn quoted_char(
    c: char,
    chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>,
    field: &mut Str,
) -> bool {
    if c != '"' {
        field.push(c);
        return true;
    }
    if chars.peek().is_some_and(|&(_, n)| n == '"') {
        chars.next();
        field.push('"');
        true
    } else {
        false
    }
}

fn push_field(row: &mut Vec<Str>, field: &mut Str) -> Result<()> {
    if row.len() >= MAX_COLUMNS {
        return Err(Error::Malformed);
    }
    row.push(core::mem::replace(field, Zeroizing::new(String::new())));
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    fn rows(text: &str) -> Vec<Vec<String>> {
        let mut r = Rows {
            rest: text,
            delimiter: detect_delimiter(text),
        };
        let mut out = Vec::new();
        while let Some(row) = r.next_row().unwrap() {
            out.push(row.iter().map(|s| s.as_str().to_owned()).collect());
        }
        out
    }

    #[test]
    fn rfc4180_records() {
        assert_eq!(
            rows("a,b\r\n\"x,1\",\"say \"\"hi\"\"\"\n\"multi\nline\",\n"),
            [
                vec!["a", "b"],
                vec!["x,1", "say \"hi\""],
                vec!["multi\nline", ""]
            ]
        );
        assert_eq!(rows("a;b\n1;2"), [vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(rows("a\tb\n1\t2\r"), [vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(rows("a,b\n\n"), [vec!["a", "b"], vec![""]]);
        assert_eq!(rows("x\"y,z"), [vec!["x\"y", "z"]]);
        let mut r = Rows {
            rest: "\"open",
            delimiter: ',',
        };
        assert_eq!(r.next_row(), Err(Error::Malformed));
        let wide = ",".repeat(MAX_COLUMNS);
        let mut r = Rows {
            rest: &wide,
            delimiter: ',',
        };
        assert_eq!(r.next_row(), Err(Error::Malformed));
    }

    #[test]
    fn delimiters() {
        assert_eq!(detect_delimiter("a;b;c,d"), ';');
        assert_eq!(detect_delimiter("a\tb\tc"), '\t');
        assert_eq!(detect_delimiter("\"a;b;c\",d"), ',');
        assert_eq!(detect_delimiter("abc"), ',');
        assert_eq!(detect_delimiter("a,b\nc;d;e;f"), ',');
    }

    #[test]
    fn presets_and_columns() {
        let d = |h: &str| {
            let names: Vec<&str> = h.split(',').collect();
            CsvMapping::detect(&names)
        };
        let (m, p) = d("name,url,username,password,note");
        assert_eq!(p, Some(CsvPreset::Chrome));
        assert_eq!(
            m.columns,
            [
                CsvColumn::Title,
                CsvColumn::Url,
                CsvColumn::Username,
                CsvColumn::Password,
                CsvColumn::Notes
            ]
        );
        assert_eq!(d("name,url,username,password").1, Some(CsvPreset::Chrome));
        let (m, p) = d(
            "url,username,password,httpRealm,formActionOrigin,guid,timeCreated,timeLastUsed,timePasswordChanged",
        );
        assert_eq!(p, Some(CsvPreset::Firefox));
        assert!(m.columns[3..].iter().all(|c| *c == CsvColumn::Ignore));
        let (m, p) = d("Title,URL,Username,Password,Notes,OTPAuth");
        assert_eq!(p, Some(CsvPreset::Safari));
        assert_eq!(m.columns[5], CsvColumn::Otp);
        let (m, p) = d(
            "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp",
        );
        assert_eq!(p, Some(CsvPreset::Bitwarden));
        assert_eq!(
            m.columns,
            [
                CsvColumn::Folder,
                CsvColumn::Favorite,
                CsvColumn::Ignore,
                CsvColumn::Title,
                CsvColumn::Notes,
                CsvColumn::Ignore,
                CsvColumn::Ignore,
                CsvColumn::Url,
                CsvColumn::Username,
                CsvColumn::Password,
                CsvColumn::Otp
            ]
        );
        let (m, p) = d("\u{feff}Titel,Benutzername,Passwort,Webseite,Notizen,Gruppe,Tags,PIN,");
        assert_eq!(p, None);
        assert_eq!(
            m.columns,
            [
                CsvColumn::Title,
                CsvColumn::Username,
                CsvColumn::Password,
                CsvColumn::Url,
                CsvColumn::Notes,
                CsvColumn::Folder,
                CsvColumn::Tags,
                CsvColumn::Text("PIN".into()),
                CsvColumn::Ignore
            ]
        );
    }
}

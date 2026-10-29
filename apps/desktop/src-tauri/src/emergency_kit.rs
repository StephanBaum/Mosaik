//! The Emergency Kit PDF (CRYPTO_SPEC §5.2, brief §6): account email,
//! sign-in address, Secret Key and a blank line for the master password.
//!
//! - Built in Rust from the Secret Key in the keystore, so the key does not
//!   travel back from the webview. The labels come from the webview in the
//!   user's language (`packages/i18n`, keys `shell.emergencyKit.*`), since
//!   every UI string lives there.
//! - Written only to the path the user picks in the native save dialog,
//!   created new or truncated, and never cached: the bytes live in a
//!   zeroizing buffer until written. `pdf-writer`'s intermediate buffers are
//!   plain `Vec`s (residual, like THREAT_MODEL R14).
//! - One A4 page in the PDF base-14 fonts (Helvetica, Courier), so nothing
//!   is embedded and the file has no external references. Text is
//!   WinAnsi-encoded: labels fall back to `?` for characters outside it,
//!   and values (email) are written as `<U+XXXX>` instead, so the kit stays
//!   unambiguous.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use mosaic_core::SecretVec;
use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize as _, Zeroizing};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::secret::SecretIn;
use crate::persist::MAX_EMAIL_LEN;

/// Longest accepted label, in characters.
pub const MAX_LABEL_CHARS: usize = 600;
/// Capacity reserved for the PDF (`Pdf::with_capacity`), so the buffer
/// never reallocates and leaves a stale copy of the Secret Key behind.
pub const KIT_CAPACITY: usize = 64 * 1024;
/// Capacity reserved for the page's content stream, for the same reason.
pub const CONTENT_CAPACITY: usize = 32 * 1024;
/// The file name the save dialog suggests (not translated: a file name).
pub const SUGGESTED_FILE_NAME: &str = "Mosaic Emergency Kit.pdf";

/// The request of the proposed `save_emergency_kit` command (an IPC
/// proposal: no command is registered for it yet).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveKitRequest {
    /// The kit's texts in the user's language.
    pub labels: KitLabels,
    /// The master password, needed once the kit was verified in this
    /// process (UX_GUIDELINES §8: re-authentication before viewing the
    /// Secret Key). Not needed during onboarding.
    pub password: Option<SecretIn>,
}

/// The response of the proposed `save_emergency_kit` command.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum KitSaved {
    /// Written to the path the user chose.
    Saved,
    /// The user cancelled the dialog.
    Cancelled,
}

/// The kit's texts, in the user's language.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KitLabels {
    /// "Mosaic Emergency Kit".
    pub title: String,
    /// What the kit is for and where to keep it.
    pub intro: String,
    /// "Account email".
    pub email: String,
    /// "Account ID".
    pub account_id: String,
    /// "Sign-in address".
    pub sign_in_url: String,
    /// The sign-in address itself, or "this device only" before sync (M6).
    pub sign_in_url_value: String,
    /// "Secret Key".
    pub secret_key: String,
    /// "Master password".
    pub master_password: String,
    /// The hint under the blank line ("write it here by hand, or keep it in
    /// your head").
    pub master_password_hint: String,
    /// The footer (pre-audit note, date or similar).
    pub footer: String,
}

impl KitLabels {
    /// Every label, for validation.
    fn all(&self) -> [&str; 10] {
        [
            &self.title,
            &self.intro,
            &self.email,
            &self.account_id,
            &self.sign_in_url,
            &self.sign_in_url_value,
            &self.secret_key,
            &self.master_password,
            &self.master_password_hint,
            &self.footer,
        ]
    }

    /// Non-empty, bounded, no control characters except none.
    ///
    /// # Errors
    /// `invalidInput`.
    pub fn validate(&self) -> IpcResult<()> {
        let ok = self.all().iter().all(|label| {
            !label.trim().is_empty()
                && label.chars().count() <= MAX_LABEL_CHARS
                && !label.chars().any(char::is_control)
        });
        if ok {
            Ok(())
        } else {
            Err(IpcError::new(ErrorCode::InvalidInput))
        }
    }
}

/// The account values printed on the kit.
pub struct KitValues<'a> {
    /// The account email.
    pub email: &'a str,
    /// The account ID (UUID text).
    pub account_id: &'a str,
    /// The Secret Key, `M1-…`.
    pub secret_key: &'a str,
}

/// `WinAnsiEncoding` (PDF 32000-1 Annex D) of one character.
fn win_ansi(c: char) -> Option<u8> {
    let code = u32::from(c);
    if (0x20..=0x7E).contains(&code) || (0xA0..=0xFF).contains(&code) {
        return u8::try_from(code).ok();
    }
    Some(match c {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        _ => return None,
    })
}

/// A label in `WinAnsi`; other characters become `?`.
fn encode_label(text: &str) -> Vec<u8> {
    text.chars().map(|c| win_ansi(c).unwrap_or(b'?')).collect()
}

/// A value in `WinAnsi`; other characters become `<U+XXXX>`.
fn encode_value(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match win_ansi(c) {
            Some(b) => out.push(b),
            None => out.extend_from_slice(format!("<U+{:04X}>", u32::from(c)).as_bytes()),
        }
    }
    out
}

/// Splits encoded text into lines of at most `width` bytes, at spaces where
/// possible.
fn wrap(text: &[u8], width: usize) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut rest = text;
    while rest.len() > width {
        let window = rest.get(..=width).unwrap_or(rest);
        let cut = window
            .iter()
            .rposition(|&b| b == b' ')
            .filter(|&i| i > 0)
            .unwrap_or(width);
        let (line, tail) = rest.split_at(cut);
        lines.push(line);
        rest = tail.strip_prefix(b" ").unwrap_or(tail);
    }
    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

const SANS: Name<'static> = Name(b"F1");
const SANS_BOLD: Name<'static> = Name(b"F2");
const MONO: Name<'static> = Name(b"F3");
/// A4 in points.
const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const LEFT: f32 = 64.0;

/// A text line at `y`, moving down by `leading`.
struct Writer {
    content: Content,
    y: f32,
}

impl Writer {
    fn line(&mut self, font: Name<'_>, size: f32, text: &[u8], leading: f32) {
        self.content
            .begin_text()
            .set_font(font, size)
            .next_line(LEFT, self.y)
            .show(Str(text))
            .end_text();
        self.y -= leading;
    }

    fn paragraph(&mut self, text: &[u8], width: usize) {
        for line in wrap(text, width) {
            self.line(SANS, 11.0, line, 15.0);
        }
    }

    /// A label and its value. Long values (an email up to 254 bytes) wrap
    /// so nothing runs off the page: at most as many characters per line
    /// as fit at 0.6 em each (Courier's width, wider than Helvetica's
    /// average).
    fn field(&mut self, label: &str, font: Name<'_>, size_pt: u16, value: &[u8]) {
        self.line(SANS_BOLD, 10.0, &encode_label(label), 16.0);
        let size = f32::from(size_pt);
        let lines = wrap(value, chars_per_line(size_pt));
        let count = lines.len();
        for (i, line) in lines.into_iter().enumerate() {
            let leading = if i + 1 == count {
                size + 18.0
            } else {
                size + 4.0
            };
            self.line(font, size, line, leading);
        }
    }
}

/// How many 0.6 em characters fit between the margins (467 pt) at
/// `size_pt`.
fn chars_per_line(size_pt: u16) -> usize {
    usize::from(4670 / size_pt.saturating_mul(6).max(1))
}

/// Renders the kit.
///
/// # Errors
/// `invalidInput` for unusable labels; `internal` if memory runs out.
pub fn render(labels: &KitLabels, values: &KitValues<'_>) -> IpcResult<SecretVec> {
    labels.validate()?;
    if values.email.len() > MAX_EMAIL_LEN {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    // The encoded Secret Key is zeroized when it drops.
    let secret = Zeroizing::new(encode_value(values.secret_key));
    let (catalog, tree, page, content_id) = (Ref::new(1), Ref::new(2), Ref::new(3), Ref::new(4));
    let (sans, sans_bold, mono) = (Ref::new(5), Ref::new(6), Ref::new(7));

    let mut w = Writer {
        content: Content::with_capacity(CONTENT_CAPACITY),
        y: 770.0,
    };
    w.line(SANS_BOLD, 22.0, &encode_label(&labels.title), 34.0);
    w.paragraph(&encode_label(&labels.intro), 88);
    w.y -= 18.0;
    w.field(&labels.email, SANS, 13, &encode_value(values.email));
    w.field(
        &labels.account_id,
        MONO,
        11,
        &encode_value(values.account_id),
    );
    w.field(
        &labels.sign_in_url,
        SANS,
        13,
        &encode_label(&labels.sign_in_url_value),
    );
    w.field(&labels.secret_key, MONO, 16, &secret);
    w.line(
        SANS_BOLD,
        10.0,
        &encode_label(&labels.master_password),
        30.0,
    );
    // The blank line for the master password, written by hand.
    w.content
        .set_line_width(0.8)
        .move_to(LEFT, w.y + 8.0)
        .line_to(PAGE_WIDTH - LEFT, w.y + 8.0)
        .stroke();
    w.y -= 8.0;
    w.paragraph(&encode_label(&labels.master_password_hint), 88);
    w.y = 64.0;
    w.paragraph(&encode_label(&labels.footer), 96);
    // `finish` hands over the content stream's own buffer (no copy); it is
    // zeroized once the PDF holds its copy.
    let mut stream = w.content.finish().into_vec();

    let mut pdf = Pdf::with_capacity(KIT_CAPACITY);
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree).kids([page]).count(1);
    {
        let mut p = pdf.page(page);
        p.parent(tree)
            .media_box(Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT))
            .contents(content_id);
        p.resources()
            .fonts()
            .pair(SANS, sans)
            .pair(SANS_BOLD, sans_bold)
            .pair(MONO, mono);
    }
    for (id, base) in [
        (sans, Name(b"Helvetica")),
        (sans_bold, Name(b"Helvetica-Bold")),
        (mono, Name(b"Courier")),
    ] {
        pdf.type1_font(id)
            .base_font(base)
            .encoding_predefined(Name(b"WinAnsiEncoding"));
    }
    pdf.stream(content_id, &stream);
    stream.zeroize();
    let mut bytes = pdf.finish();
    let kit = SecretVec::from_slice(&bytes).map_err(|_| IpcError::INTERNAL);
    bytes.zeroize();
    kit
}

/// Writes the kit to `path` (created, or truncated if the user chose to
/// replace a file). On Unix the file is readable by the user only.
///
/// # Errors
/// `invalidInput` if the file cannot be written (the UI says to pick another
/// place). A file that was created but could not be completed is removed.
pub fn write(path: &Path, kit: &SecretVec) -> IpcResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| IpcError::new(ErrorCode::InvalidInput))?;
    if file
        .write_all(kit.expose_secret())
        .and_then(|()| file.sync_all())
        .is_err()
    {
        // No half-written kit stays behind (best effort).
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn labels() -> KitLabels {
        KitLabels {
            title: "Mosaic Notfall-Kit".into(),
            intro: "Bewahre dieses Blatt sicher auf. Mit deinem Master-Passwort und dem Geheimschlüssel kommst du an deinen Tresor, auch wenn dein Gerät verloren geht. Niemand sonst kann sie wiederherstellen.".into(),
            email: "Konto-E-Mail".into(),
            account_id: "Konto-ID".into(),
            sign_in_url: "Anmeldeadresse".into(),
            sign_in_url_value: "Nur dieses Gerät".into(),
            secret_key: "Geheimschlüssel".into(),
            master_password: "Master-Passwort".into(),
            master_password_hint: "Von Hand eintragen oder im Kopf behalten.".into(),
            footer: "Mosaic ist noch nicht extern geprüft (pre-audit).".into(),
        }
    }

    const KEY: &str = "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2";

    fn render_sample(email: &str) -> Vec<u8> {
        render(
            &labels(),
            &KitValues {
                email,
                account_id: "019a2b3c-4d5e-7000-8000-000000000001",
                secret_key: KEY,
            },
        )
        .unwrap()
        .expose_secret()
        .to_vec()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    }

    /// How `pdf-writer` writes a non-ASCII string: as a hex string.
    fn hex(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![b'<'];
        for b in bytes {
            out.extend_from_slice(format!("{b:02X}").as_bytes());
        }
        out.push(b'>');
        out
    }

    #[test]
    fn the_pdf_holds_every_value_and_no_external_reference() {
        let pdf = render_sample("alex@example.com");
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(contains(&pdf, KEY.as_bytes()));
        assert!(contains(&pdf, b"alex@example.com"));
        assert!(contains(&pdf, b"019a2b3c-4d5e-7000-8000-000000000001"));
        assert!(
            contains(&pdf, &hex(&encode_label("Geheimschlüssel"))),
            "umlaut in WinAnsi"
        );
        assert!(contains(&pdf, b"/WinAnsiEncoding"));
        for forbidden in [
            &b"/URI"[..],
            b"/JavaScript",
            b"/Launch",
            b"/EmbeddedFile",
            b"http",
        ] {
            assert!(
                !contains(&pdf, forbidden),
                "{}",
                String::from_utf8_lossy(forbidden)
            );
        }
    }

    #[test]
    fn values_outside_win_ansi_stay_unambiguous() {
        let pdf = render_sample("jürgen@例え.jp");
        assert_eq!(
            encode_value("jürgen@例え.jp"),
            b"j\xFCrgen@<U+4F8B><U+3048>.jp"
        );
        assert!(contains(&pdf, &hex(&encode_value("jürgen@例え.jp"))));
        assert_eq!(encode_label("a✓b"), b"a?b");
        assert_eq!(
            encode_label("„Zitat“ – €"),
            vec![
                0x84, b'Z', b'i', b't', b'a', b't', 0x93, b' ', 0x96, b' ', 0x80
            ]
        );
    }

    #[test]
    fn bad_labels_are_refused() {
        let mut bad = labels();
        bad.title = "  ".into();
        let values = KitValues {
            email: "a@b",
            account_id: "x",
            secret_key: KEY,
        };
        assert_eq!(
            render(&bad, &values).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        let mut bad = labels();
        bad.footer = "x".repeat(MAX_LABEL_CHARS + 1);
        assert!(render(&bad, &values).is_err());
        let mut bad = labels();
        bad.intro = "line\nbreak".into();
        assert!(render(&bad, &values).is_err());
    }

    #[test]
    fn long_values_wrap_inside_the_margins() {
        assert_eq!(chars_per_line(16), 48);
        assert_eq!(chars_per_line(13), 59);
        let email = format!("{}@example.com", "a".repeat(200));
        let pdf = render_sample(&email);
        assert!(contains(&pdf, "a".repeat(59).as_bytes()));
        assert!(!contains(&pdf, "a".repeat(60).as_bytes()), "wrapped");
    }

    #[test]
    fn wrapping_breaks_at_spaces_and_hard_breaks_long_words() {
        assert_eq!(wrap(b"aaa bbb ccc", 7), vec![&b"aaa bbb"[..], b"ccc"]);
        assert_eq!(wrap(b"abcdefghij", 4), vec![&b"abcd"[..], b"efgh", b"ij"]);
        assert_eq!(wrap(b"", 4), Vec::<&[u8]>::new());
        assert_eq!(wrap(b"ab", 4), vec![&b"ab"[..]]);
    }

    #[test]
    fn kit_fits_the_preallocated_buffers() {
        let long = "ä".repeat(MAX_LABEL_CHARS);
        let mut l = labels();
        l.title = long.clone();
        l.intro = long.clone();
        l.email = long.clone();
        l.account_id = long.clone();
        l.sign_in_url = long.clone();
        l.sign_in_url_value = long.clone();
        l.secret_key = long.clone();
        l.master_password = long.clone();
        l.master_password_hint = long.clone();
        l.footer = long;
        let email = format!("{}@b", "a".repeat(MAX_EMAIL_LEN - 2));
        assert_eq!(email.len(), MAX_EMAIL_LEN);
        let pdf = render(
            &l,
            &KitValues {
                email: &email,
                account_id: "019a2b3c-4d5e-7000-8000-000000000001",
                secret_key: KEY,
            },
        )
        .unwrap();
        assert!(pdf.len() <= KIT_CAPACITY, "{}", pdf.len());
    }

    #[test]
    fn an_overlong_email_is_invalid_input() {
        let email = "a".repeat(MAX_EMAIL_LEN + 1);
        let error = render(
            &labels(),
            &KitValues {
                email: &email,
                account_id: "x",
                secret_key: KEY,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn writing_into_a_missing_directory_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-dir").join("kit.pdf");
        let kit = SecretVec::from_slice(b"%PDF-small").unwrap();
        assert!(write(&missing, &kit).is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn writes_the_file_and_replaces_an_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SUGGESTED_FILE_NAME);
        std::fs::write(&path, vec![b'x'; 100_000]).unwrap();
        let kit = SecretVec::from_slice(b"%PDF-small").unwrap();
        write(&path, &kit).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"%PDF-small");
        let missing = dir.path().join("no-such-dir").join("kit.pdf");
        assert_eq!(
            write(&missing, &kit).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
}

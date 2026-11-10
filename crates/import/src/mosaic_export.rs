//! Mosaic's own encrypted export (`.mosaicx`, CRYPTO_SPEC §11.2, ADR-0073),
//! read into the same [`Import`] as every other format.
//!
//! The file is read through `mosaic_core::ExportReader` as a stream and the
//! JSON of [`mosaic_vault::export`] is parsed item by item, so the plaintext
//! of the whole export is never held at once (only the drafts are, as for
//! every importer). Two rules protect the account:
//!
//! - **Nothing is committed from unverified data.** STREAM authenticates
//!   each chunk as it is read, but a file cut off at a chunk boundary
//!   verifies up to the cut. [`parse_reader`] therefore returns an
//!   [`Import`] only after the reader reported the verified end of the
//!   stream; a truncated, extended, reordered or modified file is an error
//!   and no draft leaves this module.
//! - **A wrong passphrase and a damaged file look the same**:
//!   [`Error::WrongCredentials`] (the CLI reports `exportUnreadable`).
//!
//! Mapping to drafts: `fields` with a built-in key set that field, `custom.*`
//! keys become custom fields, `otp` the item's OTP, `passkey` a `passkey`
//! item, `sshKey` the SSH key material, `history` the password history,
//! `attachments` the files (an attachment without data is reported with
//! [`crate::EntryWarning::AttachmentMissing`]). `createdAt` is read and
//! ignored: an import creates new items stamped now. The vaults of the
//! export are flattened into one list; the caller chooses the target vault.
//!
//! [`Import::format`] is [`Format::Cxf`] for an import from this module: the
//! enum has no variant of its own yet (the desktop matches it exhaustively).

use core::fmt;
use std::io::{self, Read};

use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_core::{ExportPassphrase, ExportReader, SecretString, Timestamp};
use mosaic_passkey::{MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN};
use mosaic_vault::export::{
    CUSTOM_PREFIX, ExportField, ExportItem, ExportPasskey, FORMAT, SecretText, VERSION,
};
use serde::de::{DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::buf;
use crate::draft::{AttachmentBudget, Builder, EntryWarning, Kind};
use crate::entries::{Collector, Import};
use crate::error::{Error, Result};
use crate::passkey::{self, MAX_PKCS8_LEN, Parts};
use crate::{Format, MAX_FILE_LEN};

/// Reads an export held in memory. `passphrase` is the export passphrase.
///
/// # Errors
/// [`Error::TooLarge`] above [`MAX_FILE_LEN`]; [`Error::PasswordRequired`]
/// for an empty passphrase; otherwise as [`parse_reader`].
pub fn parse_bytes(input: &[u8], passphrase: &SecretString) -> Result<Import> {
    if input.len() > MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let pass = ExportPassphrase::new(passphrase).map_err(|_| Error::PasswordRequired)?;
    parse_reader(input, &pass)
}

/// Reads an export from `reader` (a file, say) and returns the entries only
/// after the whole stream verified.
///
/// # Errors
/// - [`Error::Malformed`]: not an export (no `MOSX` header, a truncated
///   header), or verified content that is not the JSON of §11.2.
/// - [`Error::UnsupportedVersion`]: a version, algorithm or Argon2id profile
///   this build does not read (nothing is derived then).
/// - [`Error::WrongCredentials`]: the passphrase is wrong, or the file is
///   damaged, truncated or extended.
/// - [`Error::TooLarge`] (plaintext above 2 GiB), [`Error::LimitExceeded`]
///   (more than [`crate::MAX_ENTRIES`] items), [`Error::Allocation`].
pub fn parse_reader<R: Read>(reader: R, passphrase: &ExportPassphrase) -> Result<Import> {
    let reader = ExportReader::open(reader, passphrase).map_err(open_error)?;
    let mut tracked = Tracked {
        inner: reader,
        failed: false,
    };
    let mut state = State {
        collector: Collector::new(Format::Cxf),
        budget: AttachmentBudget::new(),
        error: None,
    };
    let mut de = serde_json::Deserializer::from_reader(&mut tracked);
    // `end` reads to the end of the stream, which is where the reader
    // verifies its last chunk.
    let parsed = Root { state: &mut state }
        .deserialize(&mut de)
        .and_then(|()| de.end());
    match parsed {
        Ok(()) => Ok(state.collector.finish()),
        Err(_) if tracked.failed => Err(Error::WrongCredentials),
        Err(_) => Err(state.error.unwrap_or(Error::Malformed)),
    }
}

/// The error of the header and the first chunk.
fn open_error(e: mosaic_core::Error) -> Error {
    use mosaic_core::Error as C;
    match e {
        C::Decryption => Error::WrongCredentials,
        C::UnsupportedVersion | C::UnsupportedAlgorithm | C::UnknownProfile => {
            Error::UnsupportedVersion
        }
        C::SizeLimit => Error::TooLarge,
        C::Allocation => Error::Allocation,
        _ => Error::Malformed,
    }
}

/// Remembers whether the reader failed, to tell a damaged stream from
/// verified content that is not an export.
struct Tracked<R: Read> {
    inner: ExportReader<R>,
    failed: bool,
}

impl<R: Read> Read for Tracked<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(out);
        if n.is_err() {
            self.failed = true;
        }
        n
    }
}

struct State {
    collector: Collector,
    budget: AttachmentBudget,
    /// The importer's own error behind a serde error.
    error: Option<Error>,
}

impl State {
    fn add(&mut self, item: &ExportItem) -> Result<()> {
        let builder = entry(item, self.collector.passkey_room(), &mut self.budget);
        self.collector.push(builder)
    }
}

// ---------------------------------------------------------------------------
// Streaming parse: root -> vaults -> items, one item at a time.
// ---------------------------------------------------------------------------

struct Root<'a> {
    state: &'a mut State,
}

impl<'de> DeserializeSeed<'de> for Root<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> core::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for Root<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a Mosaic export")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> core::result::Result<(), A::Error> {
        let (mut format_ok, mut version_ok) = (false, false);
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "format" => {
                    format_ok = map.next_value::<String>()? == FORMAT;
                    if !format_ok {
                        return Err(A::Error::custom("format"));
                    }
                }
                "v" => {
                    version_ok = map.next_value::<u64>()? == u64::from(VERSION);
                    if !version_ok {
                        self.state.error = Some(Error::UnsupportedVersion);
                        return Err(A::Error::custom("version"));
                    }
                }
                "vaults" => {
                    // The header members come first (the writer's order); a
                    // file that does not say what it is yet is refused.
                    if !(format_ok && version_ok) {
                        return Err(A::Error::custom("order"));
                    }
                    map.next_value_seed(Vaults {
                        state: &mut *self.state,
                    })?;
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        if format_ok && version_ok {
            Ok(())
        } else {
            Err(A::Error::custom("missing members"))
        }
    }
}

struct Vaults<'a> {
    state: &'a mut State,
}

impl<'de> DeserializeSeed<'de> for Vaults<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> core::result::Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Vaults<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of vaults")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> core::result::Result<(), A::Error> {
        while seq
            .next_element_seed(Vault {
                state: &mut *self.state,
            })?
            .is_some()
        {}
        Ok(())
    }
}

struct Vault<'a> {
    state: &'a mut State,
}

impl<'de> DeserializeSeed<'de> for Vault<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> core::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for Vault<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a vault")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> core::result::Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "items" {
                map.next_value_seed(Items {
                    state: &mut *self.state,
                })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

struct Items<'a> {
    state: &'a mut State,
}

impl<'de> DeserializeSeed<'de> for Items<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> core::result::Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Items<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of items")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> core::result::Result<(), A::Error> {
        while let Some(item) = seq.next_element::<ExportItem>()? {
            if let Err(e) = self.state.add(&item) {
                self.state.error = Some(e);
                return Err(A::Error::custom("import limit"));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// One item -> one draft.
// ---------------------------------------------------------------------------

fn entry(item: &ExportItem, passkey_room: usize, budget: &mut AttachmentBudget) -> Builder {
    // A passkey item becomes a login draft: `Builder::finish` turns an entry
    // that holds only a title and passkeys into passkey items, and one that
    // holds more keeps its other content in an item of its own.
    let item_type = match ItemType::parse(&item.item_type) {
        ItemType::Passkey => ItemType::Login,
        other => other,
    };
    let mut b = Builder::new(item_type).with_passkey_room(passkey_room);
    b.title(item.title.as_str());
    b.notes(item.notes.as_str());
    for f in &item.fields {
        field(&mut b, f);
    }
    for url in &item.urls {
        b.url(url.as_str(), "URL");
    }
    if let Some(otp) = &item.otp {
        b.otp_text(otp.as_str(), "One-time password");
    }
    if let Some(p) = &item.passkey
        && b.wants_passkey()
    {
        b.passkey(passkey_of(p));
    }
    if let Some(key) = &item.ssh_key {
        b.ssh_key(
            key.as_str(),
            item.ssh_public_key.as_ref().map_or("", SecretText::as_str),
            item.ssh_fingerprint.as_ref().map_or("", SecretText::as_str),
        );
    }
    for h in &item.history {
        b.history(h.value.as_str());
    }
    for tag in &item.tags {
        b.tag(tag.as_str());
    }
    b.favorite(item.favorite);
    for a in &item.attachments {
        match &a.data {
            Some(data) => {
                match buf::base64url_decode(data.as_str(), mosaic_core::MAX_ATTACHMENT_LEN) {
                    Ok(bytes) => {
                        b.attachment(budget, a.name.as_str(), Some(a.mime.as_str()), &bytes);
                    }
                    Err(Error::TooLarge) => b.warn(EntryWarning::AttachmentTooLarge),
                    Err(_) => b.warn(EntryWarning::AttachmentMissing),
                }
            }
            None => b.warn(EntryWarning::AttachmentMissing),
        }
    }
    b
}

fn field(b: &mut Builder, f: &ExportField) {
    let (label, value) = (f.label.as_str(), f.value.as_str());
    if let Some(text) = TextField::from_key(&f.key)
        && !matches!(text, TextField::Title | TextField::Notes)
    {
        b.text(text, value, label);
    } else if let Some(secret) = SecretField::from_key(&f.key) {
        b.secret(secret, value, label);
    } else if let Some(kind) = f.key.strip_prefix(CUSTOM_PREFIX) {
        match kind {
            "text" => b.custom(label, Kind::Text, value),
            "hidden" => b.custom(label, Kind::Hidden, value),
            "url" => b.custom(label, Kind::Url, value),
            "email" => b.custom(label, Kind::Email, value),
            "phone" => b.custom(label, Kind::Phone, value),
            "date" => b.custom(label, Kind::Date, value),
            "otp" => b.custom_otp(label, value),
            _ => b.fallback(label, secret_kind(f.secret), value),
        }
    } else {
        b.fallback(label, secret_kind(f.secret), value);
    }
}

const fn secret_kind(secret: bool) -> Kind {
    if secret { Kind::Hidden } else { Kind::Text }
}

fn passkey_of(p: &ExportPasskey) -> passkey::Built {
    let unreadable = |_| passkey::Problem::Unreadable;
    let credential_id = buf::base64url_decode(p.credential_id.as_str(), MAX_CREDENTIAL_ID_LEN)
        .map_err(unreadable)?;
    let user_handle =
        buf::base64url_decode(p.user_handle.as_str(), MAX_USER_HANDLE_LEN).map_err(unreadable)?;
    let private_key = buf::base64url_decode(p.key.as_str(), MAX_PKCS8_LEN).map_err(unreadable)?;
    passkey::build(&Parts {
        credential_id,
        rp_id: p.rp_id.as_str(),
        user_handle,
        user_name: p.username.as_str(),
        user_display_name: p.user_display_name.as_str(),
        private_key,
        sign_count: p.sign_count,
        discoverable: p.discoverable,
        created_at: p.created_at.map(Timestamp::from_unix_millis),
    })
}

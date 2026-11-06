//! The decrypted KDBX 4 payload: inner header, protected-value stream and
//! the XML document.

use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use salsa20::Salsa20;
use sha2::{Digest, Sha256, Sha512};
use std::borrow::Cow;
use zeroize::{Zeroize, Zeroizing};

use aes::cipher::{KeyIvInit, StreamCipher};
use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_passkey::{MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN};

use super::Cursor;
use crate::Format;
use crate::buf::{self, Buf, Str};
use crate::draft::{AttachmentBudget, Builder, EntryWarning, Kind};
use crate::entries::{Collector, Import};
use crate::error::{Error, Result};
use crate::otp::KeepassOtp;
use crate::passkey::{self, Built, MAX_PKCS8_LEN, Parts, Problem};

/// Most attachment binaries in the inner header.
const MAX_BINARIES: usize = 100_000;
/// Deepest XML nesting.
const MAX_XML_DEPTH: usize = 256;
/// Most groups.
const MAX_GROUPS: usize = 100_000;
/// Most password history entries read per entry (more are dropped before
/// the draft keeps the newest [`mosaic_core::model::MAX_HISTORY`]).
const MAX_HISTORY_READ: usize = 1_000;

const SALSA20_NONCE: [u8; 8] = [0xE8, 0x30, 0x09, 0x4B, 0x97, 0x20, 0x5D, 0x2A];

/// The protected-value stream.
enum Stream {
    None,
    Salsa20(Box<Salsa20>),
    ChaCha20(Box<chacha20::ChaCha20>),
}

impl Stream {
    fn new(id: u32, key: &[u8]) -> Result<Self> {
        match id {
            0 => Ok(Self::None),
            2 => {
                let k = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(key)));
                let c = Salsa20::new_from_slices(k.as_slice(), &SALSA20_NONCE)
                    .map_err(|_| Error::Malformed)?;
                Ok(Self::Salsa20(Box::new(c)))
            }
            3 => {
                let h = Zeroizing::new(<[u8; 64]>::from(Sha512::digest(key)));
                let (k, rest) = h.split_at(32);
                let nonce = rest.get(..12).ok_or(Error::Malformed)?;
                let c =
                    chacha20::ChaCha20::new_from_slices(k, nonce).map_err(|_| Error::Malformed)?;
                Ok(Self::ChaCha20(Box::new(c)))
            }
            // 1 is ArcFour (KeePass 2.0 era).
            _ => Err(Error::Unsupported),
        }
    }

    fn apply(&mut self, data: &mut [u8]) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Salsa20(c) => c.try_apply_keystream(data).map_err(|_| Error::Malformed),
            Self::ChaCha20(c) => c.try_apply_keystream(data).map_err(|_| Error::Malformed),
        }
    }
}

/// The inner header.
struct Inner<'a> {
    stream: Stream,
    binaries: Vec<&'a [u8]>,
    xml: &'a [u8],
}

impl<'a> Inner<'a> {
    fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut r = Cursor::new(payload);
        let mut stream_id = None;
        let mut stream_key = None;
        let mut binaries = Vec::new();
        loop {
            let id = r.u8()?;
            let data = r.sized()?;
            match id {
                0 => break,
                1 => {
                    let v = u32::from_le_bytes(data.try_into().map_err(|_| Error::Malformed)?);
                    super::set(&mut stream_id, v)?;
                }
                2 => super::set(&mut stream_key, data)?,
                3 => {
                    if binaries.len() >= MAX_BINARIES {
                        return Err(Error::LimitExceeded);
                    }
                    // The first byte holds flags (bit 0: "protect in memory").
                    binaries.push(data.get(1..).ok_or(Error::Malformed)?);
                }
                _ => return Err(Error::Malformed),
            }
        }
        let stream = Stream::new(
            stream_id.ok_or(Error::Malformed)?,
            stream_key.unwrap_or_default(),
        )?;
        Ok(Self {
            stream,
            binaries,
            xml: r.remaining(),
        })
    }
}

/// XML elements this reader cares about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Meta,
    Root,
    Group,
    Entry,
    History,
    String,
    Binary,
    Key,
    Value,
    Name,
    Uuid,
    Tags,
    RecycleBinUuid,
    RecycleBinEnabled,
    Other,
}

impl Tag {
    fn of(e: &BytesStart<'_>) -> Self {
        match e.local_name().as_ref() {
            "Meta" => Self::Meta,
            "Root" => Self::Root,
            "Group" => Self::Group,
            "Entry" => Self::Entry,
            "History" => Self::History,
            "String" => Self::String,
            "Binary" => Self::Binary,
            "Key" => Self::Key,
            "Value" => Self::Value,
            "Name" => Self::Name,
            "UUID" => Self::Uuid,
            "Tags" => Self::Tags,
            "RecycleBinUUID" => Self::RecycleBinUuid,
            "RecycleBinEnabled" => Self::RecycleBinEnabled,
            _ => Self::Other,
        }
    }
}

/// A string value of an entry.
struct Field {
    key: Str,
    value: Str,
    protected: bool,
}

/// One entry, as read from the XML.
#[derive(Default)]
struct RawEntry {
    group: Option<usize>,
    fields: Vec<Field>,
    /// `(name, binary index)`.
    binaries: Vec<(Str, Option<usize>)>,
    tags: Str,
    /// Passwords of the history entries, in file order (oldest first).
    history: Vec<Str>,
}

struct Group {
    name: Str,
    uuid: Str,
    parent: Option<usize>,
}

/// The `String` or `Binary` element being read.
#[derive(Default)]
struct Pair {
    key: Option<Str>,
    value: Option<Str>,
    protected: bool,
    reference: Option<usize>,
}

/// What the XML holds.
#[derive(Default)]
struct Document {
    groups: Vec<Group>,
    entries: Vec<RawEntry>,
    recycle_bin: Option<Str>,
    recycle_bin_enabled: bool,
}

/// A `History` entry being read; only its password is kept.
#[derive(Default)]
struct HistoryEntry {
    password: Option<Str>,
}

/// The state of the streaming XML reader. Events are fed in document order,
/// so protected values are decrypted in document order too.
struct XmlState {
    doc: Document,
    path: Vec<Tag>,
    group_stack: Vec<usize>,
    entry: Option<RawEntry>,
    history_entry: Option<HistoryEntry>,
    pair: Option<Pair>,
    /// Text of the innermost element, while it is a leaf we read. Collected as
    /// bytes in a `Buf` that grows by wipe-and-move (`buf::append`), so no
    /// reallocation leaves an unwiped copy of a secret behind; it is checked
    /// as UTF-8 once, at the end tag.
    text: Option<Buf>,
    value_protected: bool,
    /// Longest text a leaf may hold (the XML size).
    limit: usize,
}

impl XmlState {
    fn new(limit: usize) -> Self {
        Self {
            doc: Document {
                recycle_bin_enabled: true,
                ..Document::default()
            },
            path: Vec::new(),
            group_stack: Vec::new(),
            entry: None,
            history_entry: None,
            pair: None,
            text: None,
            value_protected: false,
            limit,
        }
    }

    /// A start tag (or the start half of an empty element).
    fn start(&mut self, e: &BytesStart<'_>) -> Result<()> {
        let tag = Tag::of(e);
        let parent = self.path.last().copied();
        if self.path.len() >= MAX_XML_DEPTH {
            return Err(Error::LimitExceeded);
        }
        self.path.push(tag);
        match tag {
            Tag::Group if matches!(parent, Some(Tag::Root | Tag::Group)) => {
                if self.doc.groups.len() >= MAX_GROUPS {
                    return Err(Error::LimitExceeded);
                }
                self.doc.groups.push(Group {
                    name: Str::default(),
                    uuid: Str::default(),
                    parent: self.group_stack.last().copied(),
                });
                self.group_stack
                    .push(self.doc.groups.len().saturating_sub(1));
            }
            Tag::Entry if parent == Some(Tag::Group) && self.entry.is_none() => {
                self.entry = Some(RawEntry {
                    group: self.group_stack.last().copied(),
                    ..RawEntry::default()
                });
            }
            Tag::Entry if parent == Some(Tag::History) && self.entry.is_some() => {
                self.history_entry = Some(HistoryEntry::default());
            }
            Tag::String | Tag::Binary if parent == Some(Tag::Entry) => {
                self.pair = Some(Pair::default());
            }
            Tag::Value => {
                // Every protected value consumes the stream, wherever it
                // is, so later values stay aligned.
                self.value_protected =
                    attr(e, "Protected").is_some_and(|v| v.eq_ignore_ascii_case("true"));
                if let Some(p) = self.pair.as_mut() {
                    p.protected = self.value_protected;
                    if let Some(r) = attr(e, "Ref") {
                        p.reference = r.trim().parse().ok();
                    }
                }
                self.text = Some(Buf::default());
            }
            Tag::Key
            | Tag::Name
            | Tag::Uuid
            | Tag::Tags
            | Tag::RecycleBinUuid
            | Tag::RecycleBinEnabled => self.text = Some(Buf::default()),
            _ => {}
        }
        Ok(())
    }

    /// Text, CDATA and entity events; appended while a leaf is being read.
    fn push_text(&mut self, ev: &Event<'_>) -> Result<()> {
        let Some(b) = self.text.as_mut() else {
            return Ok(());
        };
        match ev {
            Event::Text(t) => {
                // The normalised value is allocated once at its final
                // size, so wiping the owned value leaves nothing behind.
                let c = t.xml10_content();
                let appended = buf::append(b, c.as_bytes(), self.limit);
                if let Cow::Owned(mut owned) = c {
                    owned.zeroize();
                }
                appended
            }
            Event::CData(t) => buf::append(b, t.as_bytes(), self.limit),
            Event::GeneralRef(r) => {
                let c = entity_char(r)?;
                let mut tmp = [0u8; 4];
                let appended = buf::append(b, c.encode_utf8(&mut tmp).as_bytes(), self.limit);
                tmp.zeroize();
                appended
            }
            _ => Ok(()),
        }
    }

    /// An end tag (or the end half of an empty element).
    fn end(&mut self, stream: &mut Stream) -> Result<()> {
        let tag = self.path.pop().ok_or(Error::Malformed)?;
        let parent = self.path.last().copied();
        let value: Option<Str> = self.text.take().map(buf::into_string).transpose()?;
        match (tag, parent) {
            (Tag::Value, _) => self.end_value(value, stream)?,
            (Tag::Key, Some(Tag::String | Tag::Binary)) => {
                if let Some(p) = self.pair.as_mut() {
                    p.key = value;
                }
            }
            (Tag::String, Some(Tag::Entry)) => self.end_string(),
            (Tag::Binary, Some(Tag::Entry)) => self.end_binary(),
            (Tag::Tags, Some(Tag::Entry)) => {
                if self.history_entry.is_none()
                    && let Some(e) = self.entry.as_mut()
                {
                    e.tags = value.unwrap_or_default();
                }
            }
            (Tag::Entry, Some(Tag::History)) => {
                if let (Some(HistoryEntry { password: Some(pw) }), Some(e)) =
                    (self.history_entry.take(), self.entry.as_mut())
                    && e.history.len() < MAX_HISTORY_READ
                {
                    e.history.push(pw);
                }
            }
            (Tag::Entry, Some(Tag::Group)) => {
                if let Some(e) = self.entry.take() {
                    self.doc.entries.push(e);
                }
            }
            (Tag::Name, Some(Tag::Group)) => {
                if let Some(g) = self.current_group() {
                    g.name = value.unwrap_or_default();
                }
            }
            (Tag::Uuid, Some(Tag::Group)) => {
                if let Some(g) = self.current_group() {
                    g.uuid = value.unwrap_or_default();
                }
            }
            (Tag::Group, Some(Tag::Root | Tag::Group)) => {
                self.group_stack.pop();
            }
            (Tag::RecycleBinUuid, Some(Tag::Meta)) => self.doc.recycle_bin = value,
            (Tag::RecycleBinEnabled, Some(Tag::Meta)) => {
                self.doc.recycle_bin_enabled =
                    value.is_none_or(|v| !v.trim().eq_ignore_ascii_case("false"));
            }
            _ => {}
        }
        Ok(())
    }

    /// The innermost open group.
    fn current_group(&mut self) -> Option<&mut Group> {
        let i = *self.group_stack.last()?;
        self.doc.groups.get_mut(i)
    }

    /// A `Value` element: decrypts it when protected, and hands it to the
    /// open key/value pair.
    fn end_value(&mut self, value: Option<Str>, stream: &mut Stream) -> Result<()> {
        let mut v = value.unwrap_or_default();
        if core::mem::take(&mut self.value_protected) {
            let mut bytes = buf::base64_decode(&v)?;
            stream.apply(&mut bytes)?;
            v = buf::into_string(bytes)?;
        }
        if let Some(p) = self.pair.as_mut() {
            p.value = Some(v);
        }
        Ok(())
    }

    /// A `String` element of an entry: a field, or the password of a history
    /// entry.
    fn end_string(&mut self) {
        let Some(p) = self.pair.take() else { return };
        let key = p.key.unwrap_or_default();
        let value = p.value.unwrap_or_default();
        match (self.history_entry.as_mut(), self.entry.as_mut()) {
            (Some(h), _) => {
                if key.as_str() == "Password" {
                    h.password = Some(value);
                }
            }
            (None, Some(e)) => e.fields.push(Field {
                key,
                value,
                protected: p.protected,
            }),
            (None, None) => {}
        }
    }

    /// A `Binary` element of an entry (history entries' attachments are
    /// dropped).
    fn end_binary(&mut self) {
        let Some(p) = self.pair.take() else { return };
        if self.history_entry.is_none()
            && let Some(e) = self.entry.as_mut()
        {
            e.binaries.push((p.key.unwrap_or_default(), p.reference));
        }
    }

    fn finish(self) -> Result<Document> {
        if self.path.is_empty() {
            Ok(self.doc)
        } else {
            Err(Error::Malformed)
        }
    }
}

/// The character of an entity or character reference.
fn entity_char(r: &BytesRef<'_>) -> Result<char> {
    match r.resolve_char_ref() {
        Ok(Some(c)) => Ok(c),
        Ok(None) => match &**r {
            "amp" => Ok('&'),
            "lt" => Ok('<'),
            "gt" => Ok('>'),
            "quot" => Ok('"'),
            "apos" => Ok('\''),
            _ => Err(Error::Malformed),
        },
        Err(_) => Err(Error::Malformed),
    }
}

/// Reads the XML, decrypting protected values in document order.
fn read_xml(xml: &str, stream: &mut Stream) -> Result<Document> {
    let mut reader = Reader::from_str(xml);
    let mut st = XmlState::new(xml.len());
    loop {
        match reader.read_event().map_err(|_| Error::Malformed)? {
            Event::Start(e) => st.start(&e)?,
            Event::Empty(e) => {
                st.start(&e)?;
                st.end(stream)?;
            }
            Event::End(_) => st.end(stream)?,
            Event::Eof => break,
            other => st.push_text(&other)?,
        }
    }
    st.finish()
}

/// The unescaped value of attribute `name`.
fn attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    let a = e.try_get_attribute(name).ok()??;
    a.normalized_value(XmlVersion::Implicit1_0)
        .ok()
        .map(std::borrow::Cow::into_owned)
}

/// Parses a decrypted, decompressed payload.
pub(crate) fn parse_payload(payload: &[u8]) -> Result<Import> {
    let Inner {
        mut stream,
        binaries,
        xml,
    } = Inner::parse(payload)?;
    let xml = core::str::from_utf8(xml).map_err(|_| Error::Malformed)?;
    let doc = read_xml(xml, &mut stream)?;
    let mut c = Collector::new(Format::Keepass);
    let mut budget = AttachmentBudget::new();
    for e in &doc.entries {
        c.push(entry(e, &doc, &binaries, &mut budget, c.passkey_room()))?;
    }
    Ok(c.finish())
}

/// The group path below the root group, and whether the entry is in the
/// recycle bin.
fn location(doc: &Document, group: Option<usize>) -> (Str, bool) {
    let recycle = doc.recycle_bin.as_ref().filter(|u| {
        doc.recycle_bin_enabled && !u.trim().is_empty() && u.trim() != "AAAAAAAAAAAAAAAAAAAAAA=="
    });
    let mut names: Vec<&str> = Vec::new();
    let mut in_bin = false;
    let mut cur = group;
    let mut hops = 0usize;
    while let Some(g) = cur.and_then(|i| doc.groups.get(i)) {
        if hops > MAX_XML_DEPTH {
            break;
        }
        hops = hops.saturating_add(1);
        if recycle.is_some_and(|u| u.as_str() == g.uuid.as_str()) {
            in_bin = true;
        }
        if g.parent.is_some() {
            names.push(g.name.trim());
        }
        cur = g.parent;
    }
    names.reverse();
    let mut path = Str::default();
    for n in names.into_iter().filter(|n| !n.is_empty()) {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(n);
    }
    (path, in_bin)
}

fn entry(
    e: &RawEntry,
    doc: &Document,
    binaries: &[&[u8]],
    budget: &mut AttachmentBudget,
    passkey_room: usize,
) -> Builder {
    let get = |k: &str| {
        e.fields
            .iter()
            .find(|f| f.key.as_str() == k)
            .map_or("", |f| f.value.as_str())
    };
    let login_like = ["UserName", "Password", "URL", "otp"]
        .iter()
        .any(|k| !get(k).is_empty())
        || e.fields.iter().any(|f| {
            f.key.starts_with("TimeOtp-")
                || f.key.starts_with("HmacOtp-")
                || f.key.as_str() == "TOTP Seed"
        });
    let has_notes = !get("Notes").trim().is_empty();
    let ty = if !login_like && has_notes {
        ItemType::SecureNote
    } else {
        ItemType::Login
    };
    let mut b = Builder::new(ty).with_passkey_room(passkey_room);
    let mut kp_otp = KeepassOtp::default();
    let mut otp_raw: Vec<&Field> = Vec::new();
    let mut passkey_present = false;
    for f in &e.fields {
        let (k, v) = (f.key.as_str(), f.value.as_str());
        match k {
            "Title" => b.title(v),
            "UserName" => b.text(TextField::Username, v, k),
            "Password" => b.secret(SecretField::Password, v, k),
            "URL" => b.url(v, k),
            "Notes" => b.notes(v),
            "otp" => b.otp_text(v, "One-time password"),
            _ if k.starts_with("KP2A_URL") => b.url(v, k),
            // The passkey members are gathered and read together below.
            _ if k.starts_with("KPEX_PASSKEY_") || k.starts_with("KPXC_PASSKEY") => {
                passkey_present = true;
            }
            _ if kp_otp.take(k, v) => otp_raw.push(f),
            _ => {
                let kind = if f.protected {
                    Kind::Hidden
                } else {
                    Kind::Text
                };
                b.custom(k, kind, v);
            }
        }
    }
    if kp_otp.is_present() {
        if let Some(parsed) = kp_otp.finish() {
            b.otp(parsed, "One-time password");
        } else {
            b.warn(EntryWarning::OtpUnreadable);
            for f in otp_raw {
                b.custom(&f.key, Kind::Hidden, &f.value);
            }
        }
    }
    if passkey_present && b.wants_passkey() {
        b.passkey(keepassxc_passkey(e));
    }
    for old in &e.history {
        b.history(old);
    }
    for (name, reference) in &e.binaries {
        match reference.and_then(|i| binaries.get(i)) {
            Some(data) => b.attachment(budget, name, None, data),
            None => b.warn(EntryWarning::AttachmentMissing),
        }
    }
    for t in e.tags.split([';', ',']) {
        if !t.trim().is_empty() {
            b.tag(t);
        }
    }
    let (path, in_bin) = location(doc, e.group);
    if in_bin {
        b.trashed(true);
    } else if !path.is_empty() {
        b.tag(&path);
    }
    b
}

/// The `KeePassXC` passkey of an entry (`KPEX_PASSKEY_*` attributes): credential
/// ID and user handle in base64url, the private key as a PEM `PRIVATE KEY`
/// block (PKCS#8). The PEM text is stripped into a zeroizing buffer before it
/// is decoded. A missing required attribute, or one that does not decode,
/// makes the passkey unreadable; a valid key of another algorithm is
/// unsupported.
fn keepassxc_passkey(e: &RawEntry) -> Built {
    let get = |k: &str| {
        e.fields
            .iter()
            .find(|f| f.key.as_str() == k)
            .map(|f| f.value.as_str())
            .filter(|v| !v.trim().is_empty())
            .ok_or(Problem::Unreadable)
    };
    let unreadable = |_| Problem::Unreadable;
    let credential_id = buf::base64url_decode(
        get("KPEX_PASSKEY_CREDENTIAL_ID")?.trim(),
        MAX_CREDENTIAL_ID_LEN,
    )
    .map_err(unreadable)?;
    let user_handle =
        buf::base64url_decode(get("KPEX_PASSKEY_USER_HANDLE")?.trim(), MAX_USER_HANDLE_LEN)
            .map_err(unreadable)?;
    let private_key = buf::pem_private_key_der(get("KPEX_PASSKEY_PRIVATE_KEY_PEM")?, MAX_PKCS8_LEN)
        .map_err(unreadable)?;
    let user_name = get("KPEX_PASSKEY_USERNAME").unwrap_or_default();
    passkey::build(&Parts {
        credential_id,
        rp_id: get("KPEX_PASSKEY_RELYING_PARTY")?,
        user_handle,
        user_name,
        user_display_name: "",
        private_key,
        sign_count: 0,
        discoverable: true,
        created_at: None,
    })
}

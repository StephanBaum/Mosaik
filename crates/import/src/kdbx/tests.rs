//! KDBX tests: a writer for synthetic files (the format exactly as KeePass
//! specifies it, independent of the reader's code paths where possible),
//! round trips over every cipher, KDF and stream, and the error paths.
//! Files written by another implementation (pykeepass) are checked in
//! `tests/formats.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::format_push_string,
    clippy::assert_is_empty,
    reason = "tests"
)]

use std::io::Write;

use aes::Aes256;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeEncrypt, KeyIvInit, StreamCipher};
use base64ct::{Base64, Encoding};
use flate2::Compression;
use flate2::write::GzEncoder;
use mosaic_core::SecretString;
use mosaic_core::model::{CustomFieldValue, ItemType, OtpKind, SecretField, Text, TextField};
use sha2::{Digest, Sha256, Sha512};

use super::crypto::{KDF_ARGON2D, KDF_ARGON2ID, Kdf, Keys};
use super::{CIPHER_AES256, CIPHER_CHACHA20, Cipher, Header, parse};
use crate::draft::EntryWarning;
use crate::entries::SkipReason;
use crate::error::Error;

const PASSWORD: &str = "correct horse";
const KDF_AES: [u8; 16] = [
    0xc9, 0xd9, 0xf3, 0x9a, 0x62, 0x8a, 0x44, 0x60, 0xbf, 0x74, 0x0d, 0x08, 0xc1, 0x8a, 0x4f, 0xea,
];
const TWOFISH: [u8; 16] = [
    0xad, 0x68, 0xf2, 0x9f, 0x57, 0x6f, 0x4b, 0xb9, 0xa3, 0x6a, 0xd4, 0x7a, 0xf9, 0x65, 0x34, 0x6c,
];

fn pw(s: &str) -> SecretString {
    SecretString::from_str_copy(s).unwrap()
}

/// A `VariantDictionary` writer.
#[derive(Default)]
struct Dict(Vec<u8>);

impl Dict {
    fn entry(mut self, ty: u8, name: &str, value: &[u8]) -> Self {
        self.0.push(ty);
        self.0
            .extend_from_slice(&u32::try_from(name.len()).unwrap().to_le_bytes());
        self.0.extend_from_slice(name.as_bytes());
        self.0
            .extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
        self.0.extend_from_slice(value);
        self
    }
    fn bytes(self, name: &str, v: &[u8]) -> Self {
        self.entry(0x42, name, v)
    }
    fn u32(self, name: &str, v: u32) -> Self {
        self.entry(0x04, name, &v.to_le_bytes())
    }
    fn u64(self, name: &str, v: u64) -> Self {
        self.entry(0x05, name, &v.to_le_bytes())
    }
    fn finish(self) -> Vec<u8> {
        let mut out = vec![0x00, 0x01];
        out.extend_from_slice(&self.0);
        out.push(0);
        out
    }
}

fn aes_kdf(rounds: u64) -> Vec<u8> {
    Dict::default()
        .bytes("$UUID", &KDF_AES)
        .u64("R", rounds)
        .bytes("S", &[7; 32])
        .finish()
}

fn argon2(uuid: [u8; 16], version: u32, memory: u64, passes: u64) -> Dict {
    Dict::default()
        .bytes("$UUID", &uuid)
        .u32("V", version)
        .u64("M", memory)
        .u64("I", passes)
        .u32("P", 1)
        .bytes("S", &[9; 16])
}

/// The inner protected-value stream, writer side.
enum Protector {
    Salsa(Box<salsa20::Salsa20>),
    ChaCha(Box<chacha20::ChaCha20>),
}

impl Protector {
    fn new(id: u32, key: &[u8]) -> Self {
        if id == 2 {
            Self::Salsa(Box::new(
                salsa20::Salsa20::new_from_slices(
                    &Sha256::digest(key),
                    &[0xE8, 0x30, 0x09, 0x4B, 0x97, 0x20, 0x5D, 0x2A],
                )
                .unwrap(),
            ))
        } else {
            let h = Sha512::digest(key);
            Self::ChaCha(Box::new(
                chacha20::ChaCha20::new_from_slices(&h[..32], &h[32..44]).unwrap(),
            ))
        }
    }

    fn protect(&mut self, value: &str) -> String {
        let mut b = value.as_bytes().to_vec();
        match self {
            Self::Salsa(c) => c.apply_keystream(&mut b),
            Self::ChaCha(c) => c.apply_keystream(&mut b),
        }
        let mut out = vec![0u8; b.len() * 4 / 3 + 4];
        Base64::encode(&b, &mut out).unwrap().to_owned()
    }
}

/// Builds the XML with protected values encrypted in document order.
struct Xml {
    out: String,
    stream: Protector,
}

impl Xml {
    fn raw(&mut self, s: &str) -> &mut Self {
        self.out.push_str(s);
        self
    }
    fn string(&mut self, key: &str, value: &str, protected: bool) -> &mut Self {
        let v = if protected {
            format!(
                "<Value Protected=\"True\">{}</Value>",
                self.stream.protect(value)
            )
        } else {
            format!("<Value>{value}</Value>")
        };
        self.out
            .push_str(&format!("<String><Key>{key}</Key>{v}</String>"));
        self
    }
}

struct Spec {
    cipher: [u8; 16],
    kdf: Vec<u8>,
    gzip: bool,
    stream_id: u32,
    binaries: Vec<Vec<u8>>,
    block_len: usize,
    version: u32,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            cipher: CIPHER_AES256,
            kdf: aes_kdf(10),
            gzip: true,
            stream_id: 3,
            binaries: vec![b"attachment-bytes".to_vec()],
            block_len: 64,
            version: 0x0004_0001,
        }
    }
}

const STREAM_KEY: [u8; 32] = [5; 32];

fn header_fields(spec: &Spec, iv: &[u8]) -> Vec<(u8, Vec<u8>)> {
    vec![
        (2, spec.cipher.to_vec()),
        (3, u32::from(spec.gzip).to_le_bytes().to_vec()),
        (4, vec![3; 32]),
        (7, iv.to_vec()),
        (11, spec.kdf.clone()),
        (12, b"public".to_vec()),
    ]
}

fn header_bytes(version: u32, fields: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice(&0x9AA2_D903u32.to_le_bytes());
    h.extend_from_slice(&0xB54B_FB67u32.to_le_bytes());
    h.extend_from_slice(&version.to_le_bytes());
    for (id, data) in fields {
        h.push(*id);
        h.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        h.extend_from_slice(data);
    }
    h.push(0);
    h.extend_from_slice(&4u32.to_le_bytes());
    h.extend_from_slice(b"\r\n\r\n");
    h
}

fn inner_header(spec: &Spec) -> Vec<u8> {
    let mut p = Vec::new();
    let mut field = |id: u8, data: &[u8]| {
        p.push(id);
        p.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        p.extend_from_slice(data);
    };
    field(1, &spec.stream_id.to_le_bytes());
    field(2, &STREAM_KEY);
    for b in &spec.binaries {
        let mut d = vec![1u8];
        d.extend_from_slice(b);
        field(3, &d);
    }
    field(0, &[]);
    p
}

/// Encrypts a payload into a complete KDBX 4 file.
fn seal(spec: &Spec, payload: &[u8], password: &str) -> Vec<u8> {
    let cipher = if spec.cipher == CIPHER_CHACHA20 {
        Cipher::ChaCha20
    } else {
        Cipher::Aes256Cbc
    };
    let iv: Vec<u8> = if cipher == Cipher::ChaCha20 {
        vec![4; 12]
    } else {
        vec![4; 16]
    };
    let header = header_bytes(spec.version, &header_fields(spec, &iv));
    let kdf = Kdf::parse(&spec.kdf).unwrap();
    let composite = super::crypto::composite_key(&pw(password));
    let transformed = kdf.transform(&composite).unwrap();
    let keys = Keys::derive(&[3; 32], &transformed);
    let data = if spec.gzip {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(payload).unwrap();
        e.finish().unwrap()
    } else {
        payload.to_vec()
    };
    let ct = match cipher {
        Cipher::Aes256Cbc => {
            let mut buf = vec![0u8; data.len() + 16];
            cbc::Encryptor::<Aes256>::new_from_slices(keys.cipher_key(), &iv)
                .unwrap()
                .encrypt_padded_b2b::<Pkcs7>(&data, &mut buf)
                .unwrap()
                .to_vec()
        }
        Cipher::ChaCha20 => {
            let mut buf = data;
            chacha20::ChaCha20::new_from_slices(keys.cipher_key(), &iv)
                .unwrap()
                .apply_keystream(&mut buf);
            buf
        }
    };
    let mut file = header.clone();
    file.extend_from_slice(&Sha256::digest(&header));
    file.extend_from_slice(&keys.tag(u64::MAX, &[&header]));
    let blocks = ct.chunks(spec.block_len).chain(core::iter::once(&[][..]));
    for (index, chunk) in (0u64..).zip(blocks) {
        let len = u32::try_from(chunk.len()).unwrap().to_le_bytes();
        file.extend_from_slice(&keys.tag(index, &[&index.to_le_bytes(), &len, chunk]));
        file.extend_from_slice(&len);
        file.extend_from_slice(chunk);
    }
    file
}

/// The sample database: groups, a recycle bin, and entries covering every
/// mapping.
fn sample_xml(stream_id: u32) -> String {
    let mut x = Xml {
        out: String::new(),
        stream: Protector::new(stream_id, &STREAM_KEY),
    };
    x.raw("<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta>")
        .raw("<Generator>test</Generator><RecycleBinEnabled>True</RecycleBinEnabled>")
        .raw("<RecycleBinUUID>YmluYmluYmluYmluYmluYg==</RecycleBinUUID>")
        .raw("<CustomData><Item><Key>k</Key>");
    // A protected value outside any entry still advances the stream.
    let meta_secret = x.stream.protect("meta");
    x.raw(&format!(
        "<Value Protected=\"True\">{meta_secret}</Value></Item></CustomData>"
    ))
    .raw("</Meta><Root><Group><UUID>cm9vdHJvb3Ryb290cm9vdA==</UUID><Name>Root</Name>")
    .raw("<Group><UUID>aW50ZXJuZXRpbnRlcm5ldA==</UUID><Name>Internet</Name><Group><Name>Social</Name>")
    // Entry 0: a full login.
    .raw("<Entry><UUID>ZW50cnllbnRyeWVudHJ5ZQ==</UUID><Tags>a;b,c</Tags>");
    x.string("Title", "Mail &amp; more &#65;", false)
        .string("UserName", "alice", false)
        .string("Password", "pässword", true)
        .string("URL", "https://mail.example.com", false)
        .string("Notes", "line1\r\nline2", false)
        .string(
            "otp",
            "otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ&digits=8",
            true,
        )
        .string("PIN", "1234", true)
        .string("Account", "acc-1", false)
        .string("KP2A_URL_1", "https://alt.example.com", false)
        .string("KPEX_PASSKEY_CREDENTIAL_ID", "x", false)
        .raw("<String><Key>Empty</Key><Value Protected=\"True\"/></String>")
        .raw("<String><Key>Cdata</Key><Value><![CDATA[raw <text>]]></Value></String>")
        .raw("<Binary><Key>notes.txt</Key><Value Ref=\"0\"/></Binary>")
        .raw("<Binary><Key>gone.bin</Key><Value Ref=\"9\"/></Binary>")
        .raw("<AutoType><Enabled>True</Enabled></AutoType><History><Entry>");
    x.string("Title", "Mail", false)
        .string("Password", "old-1", true)
        .raw("<Binary><Key>old.txt</Key><Value Ref=\"0\"/></Binary>")
        .raw("<Tags>old</Tags>")
        .raw("</Entry><Entry>");
    x.string("Password", "old-2", true).raw("</Entry><Entry>");
    x.string("UserName", "no password here", false)
        .raw("</Entry></History></Entry>")
        .raw("</Group></Group>")
        // Entry 1: notes only, in a group without a name.
        .raw("<Group><Name></Name><Entry>");
    x.string("Title", "Recipe", false)
        .string("Notes", "flour", false)
        .raw("</Entry></Group>")
        // Entry 2: KeePass 2.47 TimeOtp attributes.
        .raw("<Entry>");
    x.string("Title", "TOTP", false)
        .string("UserName", "bob", false)
        .string("TimeOtp-Secret-Base32", "GEZDGNBVGY3TQOJQ", true)
        .string("TimeOtp-Period", "60", false)
        .raw("</Entry>")
        // Entry 3: broken OTP attributes are kept as hidden fields.
        .raw("<Entry>");
    x.string("Title", "Broken", false)
        .string("UserName", "carol", false)
        .string("TOTP Seed", "!!!", true)
        .string("TOTP Settings", "30;6", false)
        .raw("</Entry>")
        // Entry 4: empty.
        .raw("<Entry><String><Key>Title</Key><Value></Value></String></Entry>")
        // Recycle bin with entry 5.
        .raw("<Group><UUID>YmluYmluYmluYmluYmluYg==</UUID><Name>Recycle Bin</Name><Entry>");
    x.string("Title", "Deleted", false)
        .string("Password", "gone", true)
        .raw("</Entry></Group></Group><DeletedObjects/></Root></KeePassFile>");
    x.out
}

fn sample(spec: &Spec) -> Vec<u8> {
    let mut payload = inner_header(spec);
    payload.extend_from_slice(sample_xml(spec.stream_id).as_bytes());
    seal(spec, &payload, PASSWORD)
}

fn check_sample(import: &crate::entries::Import) {
    assert_eq!(import.entries.len(), 5);
    assert_eq!(
        import.skipped,
        [crate::entries::SkippedEntry {
            index: 4,
            reason: SkipReason::Empty
        }]
    );
    let e = &import.entries[0];
    let d = &e.draft;
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.title(), Some("Mail & more A"));
    assert_eq!(d.text(TextField::Username), Some("alice"));
    assert_eq!(
        d.secret(SecretField::Password).unwrap().expose_secret(),
        "pässword"
    );
    assert_eq!(d.text(TextField::Notes), Some("line1\nline2"));
    let urls: Vec<&str> = d.urls.iter().map(Text::as_str).collect();
    assert_eq!(
        urls,
        ["https://mail.example.com", "https://alt.example.com"]
    );
    let (otp, _) = d.otp.as_ref().unwrap();
    assert_eq!(otp.digits, 8);
    let fields: Vec<(&str, bool)> = d
        .custom_fields
        .iter()
        .map(|f| {
            (
                f.label.as_str(),
                matches!(f.value, CustomFieldValue::Hidden(_)),
            )
        })
        .collect();
    assert_eq!(
        fields,
        [("PIN", true), ("Account", false), ("Cdata", false)]
    );
    let tags: Vec<&str> = d.tags.iter().map(Text::as_str).collect();
    assert_eq!(tags, ["a", "b", "c", "Internet/Social"]);
    let history: Vec<&str> = d
        .password_history
        .iter()
        .map(SecretString::expose_secret)
        .collect();
    assert_eq!(history, ["old-1", "old-2"]);
    assert_eq!(d.attachments.len(), 1);
    assert_eq!(d.attachments[0].data.expose_secret(), b"attachment-bytes");
    assert_eq!(d.attachments[0].mime.as_str(), "text/plain");
    assert_eq!(
        e.warnings,
        [
            EntryWarning::AttachmentMissing,
            EntryWarning::PasskeysNotImported
        ]
    );

    let note = &import.entries[1].draft;
    assert_eq!(note.item_type, ItemType::SecureNote);
    assert!(note.tags.is_empty());

    let totp = &import.entries[2].draft;
    let (otp, _) = totp.otp.as_ref().unwrap();
    assert_eq!((otp.kind, otp.period), (OtpKind::Totp, 60));
    assert!(totp.custom_fields.is_empty());

    let broken = &import.entries[3];
    assert!(broken.draft.otp.is_none());
    assert_eq!(broken.warnings, [EntryWarning::OtpUnreadable]);
    assert_eq!(broken.draft.custom_fields.len(), 2);

    let deleted = &import.entries[4].draft;
    assert!(deleted.trashed);
    assert!(deleted.tags.is_empty());
}

#[test]
fn aes_cbc_aes_kdf_gzip_chacha_stream() {
    let spec = Spec::default();
    let import = parse(&sample(&spec), &pw(PASSWORD)).unwrap();
    check_sample(&import);
}

#[test]
fn chacha20_argon2d_plain_salsa_stream() {
    let spec = Spec {
        cipher: CIPHER_CHACHA20,
        kdf: argon2(KDF_ARGON2D, 0x13, 64 * 1024, 2).finish(),
        gzip: false,
        stream_id: 2,
        block_len: 1 << 20,
        version: 0x0004_0000,
        ..Spec::default()
    };
    let import = parse(&sample(&spec), &pw(PASSWORD)).unwrap();
    check_sample(&import);
}

#[test]
fn argon2id_v10_with_secret_and_data() {
    let spec = Spec {
        kdf: argon2(KDF_ARGON2ID, 0x10, 32 * 1024, 1)
            .bytes("K", b"key-secret")
            .bytes("A", b"assoc")
            .finish(),
        ..Spec::default()
    };
    let import = parse(&sample(&spec), &pw(PASSWORD)).unwrap();
    check_sample(&import);
    assert_eq!(
        parse(&sample(&spec), &pw("wrong")).unwrap_err(),
        Error::WrongCredentials
    );
}

#[test]
fn credentials_and_tampering() {
    let file = sample(&Spec::default());
    assert_eq!(
        parse(&file, &pw("Correct horse")).unwrap_err(),
        Error::WrongCredentials
    );
    let header_len = Header::parse(&file).unwrap().bytes.len();
    // A flipped bit in the header fails the SHA-256 check.
    let mut h = file.clone();
    h[header_len - 2] ^= 1;
    assert_eq!(parse(&h, &pw(PASSWORD)).unwrap_err(), Error::Malformed);
    // A flipped bit in a block fails its HMAC.
    let mut b = file.clone();
    let last = b.len() - 40;
    b[last] ^= 1;
    assert_eq!(parse(&b, &pw(PASSWORD)).unwrap_err(), Error::Malformed);
    // Truncated: no end block.
    assert_eq!(
        parse(&file[..file.len() - 36], &pw(PASSWORD)).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(&file[..header_len + 10], &pw(PASSWORD)).unwrap_err(),
        Error::Malformed
    );
    // Through the format-level entry point.
    assert_eq!(
        crate::parse(crate::Format::Keepass, &file, None).unwrap_err(),
        Error::PasswordRequired
    );
    assert_eq!(
        crate::parse(crate::Format::Keepass, &file, Some(&pw(PASSWORD)))
            .unwrap()
            .entries
            .len(),
        5
    );
}

fn header_with(fields: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut f = header_bytes(0x0004_0000, fields);
    f.extend_from_slice(&[0; 64]);
    f
}

#[test]
fn header_errors() {
    let spec = Spec::default();
    let good = header_fields(&spec, &[4; 16]);
    let check = |fields: Vec<(u8, Vec<u8>)>, want: Error| {
        assert_eq!(
            parse(&header_with(&fields), &pw("x")).err(),
            Some(want),
            "{fields:?}"
        );
    };
    let replace = |id: u8, data: Vec<u8>| {
        let mut f = good.clone();
        for (i, d) in &mut f {
            if *i == id {
                *d = data.clone();
            }
        }
        f
    };
    let without =
        |id: u8| -> Vec<(u8, Vec<u8>)> { good.iter().filter(|(i, _)| *i != id).cloned().collect() };
    check(replace(2, TWOFISH.to_vec()), Error::Unsupported);
    check(replace(2, vec![1; 15]), Error::Malformed);
    check(replace(3, 2u32.to_le_bytes().to_vec()), Error::Unsupported);
    check(replace(3, vec![1]), Error::Malformed);
    check(replace(4, vec![3; 31]), Error::Malformed);
    check(replace(7, vec![4; 12]), Error::Malformed);
    for id in [2, 3, 4, 7, 11] {
        check(without(id), Error::Malformed);
    }
    let mut dup = good.clone();
    dup.push((4, vec![3; 32]));
    check(dup, Error::Malformed);
    let mut chacha = replace(2, CIPHER_CHACHA20.to_vec());
    chacha = chacha
        .into_iter()
        .map(|(i, d)| if i == 7 { (i, vec![4; 16]) } else { (i, d) })
        .collect();
    check(chacha, Error::Malformed);

    // KDF parameters.
    check(replace(11, vec![0x00, 0x02, 0]), Error::Unsupported);
    check(replace(11, vec![0x00, 0x01, 0x99]), Error::Malformed);
    check(
        replace(11, Dict::default().bytes("$UUID", &[1; 16]).finish()),
        Error::Unsupported,
    );
    check(replace(11, Dict::default().finish()), Error::Malformed);
    check(
        replace(11, Dict::default().u32("$UUID", 1).finish()),
        Error::Malformed,
    );
    check(
        replace(
            11,
            Dict::default()
                .bytes("$UUID", &KDF_AES)
                .u64("R", 1)
                .bytes("S", &[1; 31])
                .finish(),
        ),
        Error::Malformed,
    );
    check(
        replace(
            11,
            Dict::default()
                .bytes("$UUID", &KDF_AES)
                .bytes("$UUID", &KDF_AES)
                .finish(),
        ),
        Error::Malformed,
    );
    check(
        replace(
            11,
            Dict::default()
                .bytes("$UUID", &KDF_AES)
                .entry(0x04, "R", &[1])
                .finish(),
        ),
        Error::Malformed,
    );
    check(
        replace(
            11,
            Dict::default()
                .bytes("$UUID", &KDF_AES)
                .entry(0x05, "R", &[1])
                .finish(),
        ),
        Error::Malformed,
    );
    check(
        replace(11, argon2(KDF_ARGON2D, 0x12, 1024, 1).finish()),
        Error::Unsupported,
    );
    check(
        replace(11, argon2(KDF_ARGON2D, 0x13, 1 << 60, 1).finish()),
        Error::LimitExceeded,
    );
    check(
        replace(11, argon2(KDF_ARGON2D, 0x13, 1024, 1 << 40).finish()),
        Error::LimitExceeded,
    );
    check(
        replace(
            11,
            argon2(KDF_ARGON2D, 0x13, 2048 * 1024 * 1024, 1).finish(),
        ),
        Error::LimitExceeded,
    );
    check(
        replace(
            11,
            argon2(KDF_ARGON2D, 0x13, 1024 * 1024 * 1024, 100).finish(),
        ),
        Error::LimitExceeded,
    );
    check(
        replace(
            11,
            Dict::default()
                .bytes("$UUID", &KDF_ARGON2D)
                .u32("V", 0x13)
                .u64("M", 1024 * 1024)
                .u64("I", 1)
                .u32("P", 1000)
                .bytes("S", &[1; 16])
                .finish(),
        ),
        Error::LimitExceeded,
    );
    check(
        replace(
            11,
            argon2(KDF_ARGON2D, 0x13, 64 * 1024, 1)
                .bytes("A", &[1; 33])
                .finish(),
        ),
        Error::Unsupported,
    );
    check(replace(11, aes_kdf(u64::MAX)), Error::LimitExceeded);
    // Bool, i32, i64 and string variants are accepted and ignored.
    let extra = Dict::default()
        .bytes("$UUID", &KDF_AES)
        .u64("R", 1)
        .bytes("S", &[1; 32])
        .entry(0x08, "b", &[1])
        .entry(0x0C, "i", &[0; 4])
        .entry(0x0D, "l", &[0; 8])
        .entry(0x18, "s", b"x")
        .finish();
    // Header SHA-256 is zeroed, so it fails only after the KDF parsed.
    check(replace(11, extra), Error::Malformed);
    let mut many = Dict::default();
    for i in 0..65 {
        many = many.u32(&format!("x{i}"), 1);
    }
    check(replace(11, many.finish()), Error::Malformed);
}

#[test]
fn versions_and_signatures() {
    let mut v3 = header_with(&header_fields(&Spec::default(), &[4; 16]));
    v3[8..12].copy_from_slice(&0x0003_0001u32.to_le_bytes());
    assert_eq!(parse(&v3, &pw("x")).unwrap_err(), Error::UnsupportedVersion);
    let mut kdb = v3.clone();
    kdb[4..8].copy_from_slice(&0xB54B_FB65u32.to_le_bytes());
    assert_eq!(
        parse(&kdb, &pw("x")).unwrap_err(),
        Error::UnsupportedVersion
    );
    let mut bad = v3;
    bad[0] = 0;
    assert_eq!(parse(&bad, &pw("x")).unwrap_err(), Error::Malformed);
    assert_eq!(parse(b"", &pw("x")).unwrap_err(), Error::Malformed);
    assert_eq!(parse(&[0; 7], &pw("x")).unwrap_err(), Error::Malformed);
    // A header without an end field.
    let mut endless = Vec::new();
    endless.extend_from_slice(&0x9AA2_D903u32.to_le_bytes());
    endless.extend_from_slice(&0xB54B_FB67u32.to_le_bytes());
    endless.extend_from_slice(&0x0004_0000u32.to_le_bytes());
    for _ in 0..65 {
        endless.extend_from_slice(&[1, 0, 0, 0, 0]);
    }
    assert_eq!(parse(&endless, &pw("x")).unwrap_err(), Error::Malformed);
}

fn payload_of(spec: &Spec, xml: &str) -> Vec<u8> {
    let mut p = inner_header(spec);
    p.extend_from_slice(xml.as_bytes());
    p
}

#[test]
fn payload_errors() {
    let spec = Spec::default();
    let ok = "<KeePassFile><Root></Root></KeePassFile>";
    assert!(
        super::parse_payload(&payload_of(&spec, ok))
            .unwrap()
            .entries
            .is_empty()
    );
    let arc4 = Spec {
        stream_id: 1,
        ..Spec::default()
    };
    assert_eq!(
        super::parse_payload(&payload_of(&arc4, ok)).unwrap_err(),
        Error::Unsupported
    );
    let none = Spec {
        stream_id: 0,
        ..Spec::default()
    };
    let plain = "<KeePassFile><Root><Group><Entry><String><Key>Password</Key>\
                 <Value Protected=\"True\">cGxhaW4=</Value></String></Entry></Group></Root></KeePassFile>";
    let i = super::parse_payload(&payload_of(&none, plain)).unwrap();
    assert_eq!(
        i.entries[0]
            .draft
            .secret(SecretField::Password)
            .unwrap()
            .expose_secret(),
        "plain"
    );
    for (payload, want) in [
        (vec![9, 0, 0, 0, 0], Error::Malformed),
        (vec![1, 1, 0, 0, 0, 3], Error::Malformed),
        (vec![3, 0, 0, 0, 0], Error::Malformed),
        (vec![0, 0, 0, 0, 0], Error::Malformed),
        (
            vec![1, 4, 0, 0, 0, 3, 0, 0, 0, 1, 4, 0, 0, 0, 3, 0, 0, 0],
            Error::Malformed,
        ),
    ] {
        assert_eq!(super::parse_payload(&payload).unwrap_err(), want);
    }
    for bad_xml in [
        "<KeePassFile><Root></Group></KeePassFile>",
        "<KeePassFile><Root>",
        "<KeePassFile><Root><Group><Name>&unknown;</Name></Group></Root></KeePassFile>",
        "<KeePassFile><Root><Group><Name>&#xD800;</Name></Group></Root></KeePassFile>",
        "<KeePassFile><Root><Group><Entry><String><Key>P</Key>\
         <Value Protected=\"True\">*</Value></String></Entry></Group></Root></KeePassFile>",
        "<KeePassFile><Root><Group><Entry><String><Key>P</Key>\
         <Value Protected=\"True\">/w==</Value></String></Entry></Group></Root></KeePassFile>",
    ] {
        let s = Spec {
            stream_id: 0,
            ..Spec::default()
        };
        assert_eq!(
            super::parse_payload(&payload_of(&s, bad_xml)).unwrap_err(),
            Error::Malformed,
            "{bad_xml}"
        );
    }
    let mut not_utf8 = inner_header(&spec);
    not_utf8.extend_from_slice(b"\xff");
    assert_eq!(
        super::parse_payload(&not_utf8).unwrap_err(),
        Error::Malformed
    );
    let deep = "<a>".repeat(300);
    assert_eq!(
        super::parse_payload(&payload_of(&spec, &deep)).unwrap_err(),
        Error::LimitExceeded
    );
}

#[test]
fn recycle_bin_disabled_and_nil() {
    let xml = |enabled: &str, uuid: &str| {
        format!(
            "<KeePassFile><Meta><RecycleBinEnabled>{enabled}</RecycleBinEnabled>\
             <RecycleBinUUID>{uuid}</RecycleBinUUID></Meta><Root><Group><Name>R</Name>\
             <Group><UUID>YmluYmluYmluYmluYmluYg==</UUID><Name>Bin</Name><Entry>\
             <String><Key>Title</Key><Value>t</Value></String></Entry></Group></Group>\
             </Root></KeePassFile>"
        )
    };
    let spec = Spec::default();
    let run = |x: String| {
        let i = super::parse_payload(&payload_of(&spec, &x)).unwrap();
        let d = &i.entries[0].draft;
        (d.trashed, d.tags.len())
    };
    assert_eq!(run(xml("True", "YmluYmluYmluYmluYmluYg==")), (true, 0));
    assert_eq!(run(xml("False", "YmluYmluYmluYmluYmluYg==")), (false, 1));
    assert_eq!(run(xml("True", "AAAAAAAAAAAAAAAAAAAAAA==")), (false, 1));
    assert_eq!(run(xml("True", "")), (false, 1));
}

#[test]
fn hotp_and_steam_attributes() {
    let spec = Spec {
        stream_id: 0,
        ..Spec::default()
    };
    let xml = "<KeePassFile><Root><Group><Entry>\
        <String><Key>HmacOtp-Secret-Base32</Key><Value>GEZDGNBVGY3TQOJQ</Value></String>\
        <String><Key>HmacOtp-Counter</Key><Value>4</Value></String>\
        </Entry><Entry>\
        <String><Key>TOTP Seed</Key><Value>GEZDGNBVGY3TQOJQ</Value></String>\
        <String><Key>TOTP Settings</Key><Value>30;S</Value></String>\
        </Entry></Group></Root></KeePassFile>";
    let i = super::parse_payload(&payload_of(&spec, xml)).unwrap();
    let (c, n) = i.entries[0].draft.otp.as_ref().unwrap();
    assert_eq!((c.kind, *n), (OtpKind::Hotp, 4));
    let (c, _) = i.entries[1].draft.otp.as_ref().unwrap();
    assert_eq!(c.kind, OtpKind::Steam);
}

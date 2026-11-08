//! Domain separation, associated data and padding (CRYPTO_SPEC §3, §7.2).
//!
//! - [`ctx`]: `"mosaic/v1/" || label`.
//! - `Aad` (crate-private): `aad(label, f1, f2, ...) = ctx(label) || 0x00 || f1 || f2 || ...`
//!   with fixed-length fields appended as-is and variable-length fields
//!   prefixed by their length as `u32` (big-endian). Header binding is
//!   `Aad::with_header`: the AAD passed to the AEAD is
//!   `header || aad(label, ...)`.
//! - [`pad`] / [`unpad`]: ISO/IEC 7816-4 padding (`0x80` then zeros) to the
//!   next multiple of 256 bytes, at least 512 bytes. [`unpad`] is strict and
//!   accepts only the canonical padding of some message.

use subtle::{ConditionallySelectable, ConstantTimeEq, ConstantTimeLess, CtOption};

use crate::error::{Error, Result};
use crate::secret::SecretVec;

/// Prefix of every domain-separation string.
pub const CTX_PREFIX: &str = "mosaic/v1/";

/// A domain-separation label. The `0x00` after `ctx(label)` in `aad(...)`
/// is only an unambiguous separator if labels contain no NUL byte, so the
/// only constructor is a `const fn` that rejects NUL. Labels are `const`
/// items, so a bad label is a compile-time error, never a runtime panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(&'static str);

impl Label {
    /// A label; use only to define `const` items.
    pub(crate) const fn new(s: &'static str) -> Self {
        let mut rest = s.as_bytes();
        while let [first, tail @ ..] = rest {
            assert!(*first != 0, "labels must not contain NUL");
            rest = tail;
        }
        Self(s)
    }

    /// The label bytes.
    #[must_use]
    pub const fn as_bytes(self) -> &'static [u8] {
        self.0.as_bytes()
    }

    /// The label text, for [`ctx`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// `ctx(label)` as bytes.
    #[must_use]
    pub fn ctx(self) -> Vec<u8> {
        ctx(self.0)
    }
}

/// Every label used by Mosaic, in this crate or in another one that takes it
/// from here (the server's keys, the OPAQUE context, the pre-login HMACs).
/// Labels MUST NOT be reused for a different purpose (CRYPTO_SPEC §3).
pub mod label {
    use super::Label;

    /// HKDF info for `S` (§5.4).
    pub(crate) const SECRET_KEY: Label = Label::new("secret-key");
    /// HKDF info for `R` (§5.4).
    pub(crate) const ROOT: Label = Label::new("root");
    /// HKDF info for `KEK` (§5.4).
    pub(crate) const KEK: Label = Label::new("kek");
    /// HKDF info for `OPW` (§5.4).
    pub(crate) const OPAQUE_PASSWORD: Label = Label::new("opaque-password");
    /// SHA-256 prefix of the Secret Key checksum (§5.2).
    pub(crate) const SECRET_KEY_CHECKSUM: Label = Label::new("secret-key-checksum");
    /// AAD of a wrapped account key (§6).
    pub(crate) const WRAP_AK: Label = Label::new("wrap/ak");
    /// AAD of a wrapped vault key (§7.1).
    pub(crate) const WRAP_VAULT: Label = Label::new("wrap/vault");
    /// AAD of vault metadata (§7.1).
    pub(crate) const VAULT_META: Label = Label::new("vault-meta");
    /// AAD of an item record (§7.2).
    pub(crate) const ITEM: Label = Label::new("item");
    /// AAD of a tombstone (§7.2).
    pub(crate) const TOMBSTONE: Label = Label::new("tombstone");
    /// AAD of every attachment chunk (§7.3).
    pub(crate) const ATTACHMENT: Label = Label::new("attachment");
    /// HKDF info of a conflict copy's item ID (§10, ADR-0016).
    pub(crate) const CONFLICT_COPY_ID: Label = Label::new("conflict-copy-id");

    /// HKDF info of the Windows Hello quick-unlock `W` (§6.2).
    pub(crate) const HELLO_UNLOCK: Label = Label::new("hello-unlock");
    /// HKDF info of a hardware key's `W` (§6.3).
    pub(crate) const PRF_UNLOCK: Label = Label::new("prf-unlock");
    /// HKDF info of a hardware key's `W_hw` for a TOTP vault (§7.4).
    pub(crate) const PRF_HW_VAULT: Label = Label::new("prf-hw-vault");
    /// AAD of a TK wrapping (§7.4).
    pub(crate) const WRAP_HW_VAULT: Label = Label::new("wrap/hw-vault");
    /// AAD of a vault key wrapped under TK (§7.4).
    pub(crate) const WRAP_VAULT_UNDER_TK: Label = Label::new("wrap/vault-under-tk");

    /// HKDF info of the settings MAC key (§11.1).
    pub(crate) const SETTINGS_MAC: Label = Label::new("settings-mac");

    /// HKDF info of the browser channel's authentication key (§15).
    pub(crate) const BROWSER_AUTH: Label = Label::new("browser/auth");
    /// Prefix of the browser channel handshake transcript (§15).
    pub(crate) const BROWSER_HANDSHAKE: Label = Label::new("browser/handshake");
    /// HKDF info of the browser channel's session keys (§15).
    pub(crate) const BROWSER_SESSION: Label = Label::new("browser/session");
    /// AAD of a browser channel frame (§15).
    pub(crate) const BROWSER_FRAME: Label = Label::new("browser/frame");

    /// HKDF info of the checkpoint key `K_cp` (§10).
    pub(crate) const CHECKPOINT_KEY: Label = Label::new("checkpoint-key");
    /// Prefix of the checkpoint MAC input (§10).
    pub(crate) const CHECKPOINT: Label = Label::new("checkpoint");
    /// HKDF info of the sync session token (§9.2).
    pub(crate) const SESSION: Label = Label::new("session");
    /// HKDF info of the folder backend MAC key `K_fold` (§10.1).
    pub(crate) const SYNC_FOLDER_MAC_KEY: Label = Label::new("sync/folder-mac-key");
    /// Prefix of a folder segment MAC (§10.1).
    pub(crate) const SYNC_FOLDER_SEGMENT: Label = Label::new("sync/folder-segment");
    /// Prefix of a folder account file MAC (§10.1).
    pub(crate) const SYNC_FOLDER_ACCOUNT: Label = Label::new("sync/folder-account");
    /// Prefix of a folder blob delete marker MAC (§10.1).
    pub(crate) const SYNC_FOLDER_BLOB_DELETE: Label = Label::new("sync/folder-blob-delete");
    /// HKDF info of the folder device-info key `K_dev` (§10.1).
    pub(crate) const SYNC_DEVICE_INFO_KEY: Label = Label::new("sync/device-info-key");
    /// AAD of sealed device info (§10.1).
    pub(crate) const SYNC_DEVICE_INFO: Label = Label::new("sync/device-info");
    /// Prefix of the first view digest in a hosted checkpoint (§10). Retired
    /// by `sync/view2` (M6b); the registry keeps the label so it is never
    /// reused.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "retired label, kept in the registry")
    )]
    pub(crate) const SYNC_VIEW: Label = Label::new("sync/view");
    /// Prefix of the key commitment inside the view digest (§10).
    pub(crate) const SYNC_VIEW_KEY: Label = Label::new("sync/view-key");

    /// AAD of a wrapped identity private key `isk` (§8.1).
    pub(crate) const WRAP_IDENTITY: Label = Label::new("wrap/identity");
    /// HPKE `info` and AAD label of a vault-key share (§8.2).
    pub(crate) const SHARE_VAULT: Label = Label::new("share/vault");
    /// HPKE `info` and AAD label of an emergency seal (§8.4).
    pub(crate) const EMERGENCY: Label = Label::new("emergency");
    /// SHA-256 prefix of an identity-key fingerprint (§8.2).
    pub(crate) const FINGERPRINT: Label = Label::new("fingerprint");
    /// AAD of a vault roster (§8.2).
    pub(crate) const VAULT_ROSTER: Label = Label::new("vault-roster");
    /// HKDF info of the contact-book key `K_contacts` (§8.5).
    pub(crate) const CONTACTS_KEY: Label = Label::new("contacts-key");
    /// AAD of the encrypted contact book (§8.5).
    pub(crate) const CONTACTS: Label = Label::new("contacts");
    /// Prefix of the view digest of a hosted checkpoint, version 2 (§10).
    pub(crate) const SYNC_VIEW2: Label = Label::new("sync/view2");
    /// Prefix of the metadata and roster commitment inside the view digest (§10).
    pub(crate) const SYNC_VIEW_META: Label = Label::new("sync/view-meta");

    /// HKDF info of the server key of the pre-login fakes (`mosaic-server`).
    pub const SERVER_PRELOGIN_KEY: Label = Label::new("server/prelogin-key");
    /// HKDF info of the server key that seals TOTP secrets (`mosaic-server`).
    pub const SERVER_TOTP_KEY: Label = Label::new("server/totp-key");
    /// AAD label of a sealed TOTP secret (`mosaic-server`).
    pub const SERVER_TOTP_AT_REST: Label = Label::new("server/totp-at-rest");
    /// The `context` of the OPAQUE login and registration finish (§9.1);
    /// the wire value is `ctx(OPAQUE_LOGIN)`.
    pub const OPAQUE_LOGIN: Label = Label::new("opaque-login");
    /// HKDF info of the server key of the directory fakes (`mosaic-server`).
    pub const SERVER_DIRECTORY_KEY: Label = Label::new("server/directory-key");
    /// HMAC input of the directory fake identity key (`mosaic-server`).
    pub const SERVER_DIRECTORY_IPK: Label = Label::new("server/directory-ipk");
    /// Input of the pre-login fake account ID (§9.2). Used as raw HMAC
    /// input **without** the `mosaic/v1/` prefix (§3 exception).
    pub const PRELOGIN_ACCOUNT_ID: Label = Label::new("prelogin/account-id");
    /// Input of the pre-login fake salt (§9.2); raw, like the others.
    pub const PRELOGIN_SALT: Label = Label::new("prelogin/salt");
    /// Input of the pre-login fake profile (§9.2); raw, like the others.
    pub const PRELOGIN_PROFILE: Label = Label::new("prelogin/profile");

    /// Every label, for the uniqueness test.
    #[cfg(test)]
    pub(crate) const ALL: [Label; 51] = [
        SERVER_PRELOGIN_KEY,
        SERVER_TOTP_KEY,
        SERVER_TOTP_AT_REST,
        SECRET_KEY,
        ROOT,
        KEK,
        OPAQUE_PASSWORD,
        SECRET_KEY_CHECKSUM,
        WRAP_AK,
        WRAP_VAULT,
        VAULT_META,
        ITEM,
        TOMBSTONE,
        ATTACHMENT,
        CONFLICT_COPY_ID,
        HELLO_UNLOCK,
        PRF_UNLOCK,
        PRF_HW_VAULT,
        WRAP_HW_VAULT,
        WRAP_VAULT_UNDER_TK,
        SETTINGS_MAC,
        BROWSER_AUTH,
        BROWSER_HANDSHAKE,
        BROWSER_SESSION,
        BROWSER_FRAME,
        CHECKPOINT_KEY,
        CHECKPOINT,
        SESSION,
        SYNC_FOLDER_MAC_KEY,
        SYNC_FOLDER_SEGMENT,
        SYNC_FOLDER_ACCOUNT,
        SYNC_FOLDER_BLOB_DELETE,
        SYNC_DEVICE_INFO_KEY,
        SYNC_DEVICE_INFO,
        SYNC_VIEW,
        SYNC_VIEW_KEY,
        OPAQUE_LOGIN,
        PRELOGIN_ACCOUNT_ID,
        PRELOGIN_SALT,
        PRELOGIN_PROFILE,
        WRAP_IDENTITY,
        SHARE_VAULT,
        EMERGENCY,
        FINGERPRINT,
        VAULT_ROSTER,
        CONTACTS_KEY,
        CONTACTS,
        SYNC_VIEW2,
        SYNC_VIEW_META,
        SERVER_DIRECTORY_KEY,
        SERVER_DIRECTORY_IPK,
    ];
}

/// Padding granularity in bytes (§7.2).
pub const PAD_BLOCK: usize = 256;
/// Minimum padded length in bytes (§7.2).
pub const PAD_MIN: usize = 512;

/// `ctx(label) = "mosaic/v1/" || label` as bytes.
#[must_use]
pub fn ctx(label: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(CTX_PREFIX.len() + label.len());
    out.extend_from_slice(CTX_PREFIX.as_bytes());
    out.extend_from_slice(label.as_bytes());
    out
}

/// Builder for `[header ||] aad(label, f1, f2, ...)`. Crate-private: every
/// AAD is built here from a [`Label`] constant.
///
/// Callers append exactly the fields an object's definition lists, in order.
/// Fields appended with [`Aad::fixed`] and the integer helpers MUST have a
/// length fixed by that definition; anything else goes through [`Aad::var`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aad(Vec<u8>);

impl Aad {
    /// Starts `aad(label, ...)`: `ctx(label) || 0x00`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new(label: Label) -> Self {
        Self::with_header(&[], label)
    }

    /// Starts the header-bound form `header || ctx(label) || 0x00`.
    #[must_use]
    pub(crate) fn with_header(header: &[u8], label: Label) -> Self {
        let label = label.as_bytes();
        let mut out = Vec::with_capacity(header.len() + CTX_PREFIX.len() + label.len() + 64);
        out.extend_from_slice(header);
        out.extend_from_slice(CTX_PREFIX.as_bytes());
        out.extend_from_slice(label);
        out.push(0x00);
        Self(out)
    }

    /// Appends a fixed-length field as-is.
    #[must_use]
    pub(crate) fn fixed(mut self, field: &[u8]) -> Self {
        self.0.extend_from_slice(field);
        self
    }

    /// Appends a `u8`.
    #[must_use]
    pub(crate) fn u8(self, v: u8) -> Self {
        self.fixed(&[v])
    }

    /// Appends a big-endian `u16`.
    #[must_use]
    pub(crate) fn u16(self, v: u16) -> Self {
        self.fixed(&v.to_be_bytes())
    }

    /// Appends a big-endian `u32`.
    #[must_use]
    pub(crate) fn u32(self, v: u32) -> Self {
        self.fixed(&v.to_be_bytes())
    }

    /// Appends a variable-length field as `len:u32 || field`. No v1 object
    /// has a variable-length AAD field yet; later milestones will.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the field is longer than `u32::MAX` bytes.
    #[cfg(test)]
    pub(crate) fn var(self, field: &[u8]) -> Result<Self> {
        let len = u32::try_from(field.len()).map_err(|_| Error::SizeLimit)?;
        Ok(self.u32(len).fixed(field))
    }

    /// The encoded associated data.
    #[must_use]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The padded length for an `n`-byte message: the next multiple of
/// [`PAD_BLOCK`] strictly greater than `n` (room for the `0x80` byte), and at
/// least [`PAD_MIN`].
///
/// # Errors
/// [`Error::SizeLimit`] if the result would overflow `usize`.
pub fn padded_len(n: usize) -> Result<usize> {
    let blocks = (n / PAD_BLOCK).checked_add(1).ok_or(Error::SizeLimit)?;
    let len = blocks.checked_mul(PAD_BLOCK).ok_or(Error::SizeLimit)?;
    Ok(len.max(PAD_MIN))
}

/// `pad(x)` into a secret buffer that has `extra_capacity` spare bytes (for
/// the AEAD tag), so that encrypting in place never reallocates.
pub(crate) fn pad_with_capacity(message: &[u8], extra_capacity: usize) -> Result<SecretVec> {
    let len = padded_len(message.len())?;
    let cap = len.checked_add(extra_capacity).ok_or(Error::SizeLimit)?;
    let mut out = SecretVec::with_capacity(cap)?;
    let v = out.as_mut_vec();
    v.extend_from_slice(message);
    v.push(0x80);
    v.resize(len, 0x00);
    Ok(out)
}

/// `pad(x)`: ISO/IEC 7816-4 padding to [`padded_len`]`(x.len())` bytes.
///
/// # Errors
/// [`Error::SizeLimit`] on overflow, [`Error::Allocation`] if the buffer
/// cannot be reserved.
pub fn pad(message: &[u8]) -> Result<SecretVec> {
    pad_with_capacity(message, 0)
}

/// Strictly removes padding and returns the length of the message.
///
/// `padded` is decrypted plaintext, so the scan is constant-time in its
/// contents: every byte is visited, the position and value of the last
/// non-zero byte are tracked with `subtle` selects, and only the combined
/// validity flag becomes a branch. The work depends only on `padded.len()`,
/// which is public (it is the ciphertext length). Without this, the scan
/// time would reveal the exact message length that padding hides (§7.2).
///
/// # Errors
/// [`Error::Malformed`] unless `padded` is exactly the canonical
/// [`pad`] output for some message.
pub(crate) fn unpadded_len(padded: &[u8]) -> Result<usize> {
    let len = padded.len();
    if len < PAD_MIN || !len.is_multiple_of(PAD_BLOCK) {
        return Err(Error::Malformed);
    }
    // `padded_len(m) == len` holds exactly for `m` in `lowest..len`: any
    // `m < PAD_MIN` pads to `PAD_MIN`; above that, only the last block.
    let lowest = if len == PAD_MIN { 0 } else { len - PAD_BLOCK };
    let lowest = u64::try_from(lowest).map_err(|_| Error::Malformed)?;

    // Scanning forward and selecting on every non-zero byte leaves the
    // position and value of the last one. An all-zero buffer leaves
    // `marker_byte == 0`, which fails the marker check below.
    let mut marker_pos = 0u64;
    let mut marker_byte = 0u8;
    for (i, &b) in (0u64..).zip(padded) {
        let non_zero = !b.ct_eq(&0x00);
        marker_pos.conditional_assign(&i, non_zero);
        marker_byte.conditional_assign(&b, non_zero);
    }
    let canonical = marker_byte.ct_eq(&0x80) & !marker_pos.ct_lt(&lowest);
    let pos = CtOption::new(marker_pos, canonical);
    let pos: Option<u64> = pos.into();
    pos.and_then(|p| usize::try_from(p).ok())
        .ok_or(Error::Malformed)
}

/// Strict `unpad`: the inverse of [`pad`].
///
/// # Errors
/// [`Error::Malformed`] unless `padded` is exactly the canonical [`pad`]
/// output for some message (wrong length, missing `0x80` marker, non-zero
/// filler, or more padding than necessary).
pub fn unpad(padded: &[u8]) -> Result<&[u8]> {
    let len = unpadded_len(padded)?;
    padded.get(..len).ok_or(Error::Malformed)
}

/// Splits `N` bytes off the front of `input`, or fails with
/// [`Error::Malformed`].
pub(crate) fn take<const N: usize>(input: &[u8]) -> Result<(&[u8; N], &[u8])> {
    input.split_first_chunk::<N>().ok_or(Error::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_and_aad_layout() {
        assert_eq!(ctx("item"), b"mosaic/v1/item");
        let a = Aad::new(Label::new("x"))
            .fixed(&[1, 2])
            .u8(3)
            .u16(0x0405)
            .u32(0x0607_0809);
        assert_eq!(
            a.as_bytes(),
            b"mosaic/v1/x\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09"
        );
        let h = Aad::with_header(&[0xAA, 0xBB], Label::new("y"))
            .var(b"abc")
            .unwrap();
        assert_eq!(h.as_bytes(), b"\xAA\xBBmosaic/v1/y\x00\x00\x00\x00\x03abc");
    }

    #[test]
    fn var_fields_are_injective() {
        let l = Label::new("l");
        let a = Aad::new(l).var(b"ab").unwrap().var(b"c").unwrap();
        let b = Aad::new(l).var(b"a").unwrap().var(b"bc").unwrap();
        assert_ne!(a, b);
    }

    /// Labels are distinct and NUL-free, so the `ctx(label) || 0x00` start
    /// of an AAD is prefix-free and never collides across purposes.
    #[test]
    fn labels_are_distinct_and_nul_free() {
        for (i, a) in label::ALL.iter().enumerate() {
            assert!(!a.as_bytes().contains(&0));
            for b in label::ALL.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    /// I2: a label that is a prefix of another is a new pair to review. AAD
    /// labels are followed by `0x00`, so those are always prefix-free; labels
    /// used as bare HKDF info or hash input are injective on the full string,
    /// so these pairs (all older than M6b or reviewed in T1) are safe, but each
    /// new pair has to be added here on purpose.
    #[test]
    fn label_prefix_pairs_are_reviewed() {
        const REVIEWED: [(&str, &str); 8] = [
            ("secret-key", "secret-key-checksum"),
            ("wrap/vault", "wrap/vault-under-tk"),
            ("checkpoint", "checkpoint-key"),
            ("sync/device-info", "sync/device-info-key"),
            ("sync/view", "sync/view-key"),
            ("sync/view", "sync/view2"),
            ("sync/view", "sync/view-meta"),
            ("contacts", "contacts-key"),
        ];
        let mut found = Vec::new();
        for (i, a) in label::ALL.iter().enumerate() {
            for (j, b) in label::ALL.iter().enumerate() {
                if i != j && b.as_str().starts_with(a.as_str()) {
                    found.push((a.as_str(), b.as_str()));
                }
            }
        }
        found.sort_unstable();
        let mut expected = REVIEWED.to_vec();
        expected.sort_unstable();
        assert_eq!(found, expected);
    }

    /// L2: the labels other crates take from here keep their wire bytes
    /// (a change would change derived values), including the pre-login
    /// labels, which have no `mosaic/v1/` prefix (CRYPTO_SPEC §3).
    #[test]
    fn exported_labels_keep_their_bytes() {
        assert_eq!(label::OPAQUE_LOGIN.ctx(), b"mosaic/v1/opaque-login");
        assert_eq!(label::SERVER_PRELOGIN_KEY.as_str(), "server/prelogin-key");
        assert_eq!(label::SERVER_TOTP_KEY.as_str(), "server/totp-key");
        assert_eq!(label::SERVER_TOTP_AT_REST.as_str(), "server/totp-at-rest");
        assert_eq!(
            label::PRELOGIN_ACCOUNT_ID.as_bytes(),
            b"prelogin/account-id"
        );
        assert_eq!(label::PRELOGIN_SALT.as_bytes(), b"prelogin/salt");
        assert_eq!(label::PRELOGIN_PROFILE.as_bytes(), b"prelogin/profile");
        for l in [
            label::OPAQUE_LOGIN,
            label::SERVER_PRELOGIN_KEY,
            label::SERVER_TOTP_KEY,
            label::SERVER_TOTP_AT_REST,
            label::PRELOGIN_ACCOUNT_ID,
            label::PRELOGIN_SALT,
            label::PRELOGIN_PROFILE,
        ] {
            assert!(label::ALL.contains(&l));
        }
    }

    #[test]
    #[should_panic(expected = "labels must not contain NUL")]
    fn nul_label_is_rejected() {
        let _ = Label::new("bad\0label");
    }

    #[test]
    fn padded_lengths() {
        assert_eq!(padded_len(0).unwrap(), 512);
        assert_eq!(padded_len(511).unwrap(), 512);
        assert_eq!(padded_len(512).unwrap(), 768);
        assert_eq!(padded_len(767).unwrap(), 768);
        assert_eq!(padded_len(768).unwrap(), 1024);
        assert_eq!(padded_len(usize::MAX), Err(Error::SizeLimit));
    }

    #[test]
    fn pad_unpad_examples() {
        for n in [0usize, 1, 255, 256, 511, 512, 1000] {
            let msg = vec![0xA5u8; n];
            let p = pad(&msg).unwrap();
            assert_eq!(p.len(), padded_len(n).unwrap());
            assert_eq!(p.expose_secret().get(n), Some(&0x80));
            assert_eq!(unpad(p.expose_secret()).unwrap(), msg.as_slice());
        }
        // A message ending in 0x80 / 0x00 bytes is still unambiguous.
        let msg = [0x80u8, 0x00, 0x80, 0x00];
        let p = pad(&msg).unwrap();
        assert_eq!(unpad(p.expose_secret()).unwrap(), &msg);
    }

    #[test]
    fn unpad_rejects_non_canonical() {
        assert_eq!(unpad(&[]), Err(Error::Malformed));
        assert_eq!(unpad(&[0u8; 512]), Err(Error::Malformed), "no marker");
        let mut p = vec![0u8; 512];
        p[10] = 0x81;
        assert_eq!(unpad(&p), Err(Error::Malformed), "wrong marker");
        let mut p = vec![0u8; 513];
        p[0] = 0x80;
        assert_eq!(unpad(&p), Err(Error::Malformed), "not a block multiple");
        let mut p = vec![0u8; 256];
        p[0] = 0x80;
        assert_eq!(unpad(&p), Err(Error::Malformed), "below minimum");
        let mut p = vec![0u8; 768];
        p[10] = 0x80;
        assert_eq!(unpad(&p), Err(Error::Malformed), "excess padding");
        let mut p = vec![0u8; 512];
        p[10] = 0x80;
        p[11] = 0x01;
        assert_eq!(unpad(&p), Err(Error::Malformed), "non-zero filler");
    }

    /// The straightforward (early-exit, branching) unpadding that the
    /// constant-time [`unpadded_len`] replaces, as an oracle.
    fn reference_unpadded_len(padded: &[u8]) -> Result<usize> {
        if padded.len() < PAD_MIN || !padded.len().is_multiple_of(PAD_BLOCK) {
            return Err(Error::Malformed);
        }
        let marker = padded
            .iter()
            .rposition(|&b| b != 0x00)
            .ok_or(Error::Malformed)?;
        if padded[marker] != 0x80 || padded_len(marker)? != padded.len() {
            return Err(Error::Malformed);
        }
        Ok(marker)
    }

    /// A zero buffer of `len` bytes with `marker` at `pos`, `before` just
    /// ahead of it and `after` in the last byte.
    fn marked_padding(
        len: usize,
        pos: usize,
        marker: u8,
        before: u8,
        after: Option<u8>,
    ) -> Vec<u8> {
        let mut p = vec![0u8; len];
        p[pos] = marker;
        if pos > 0 {
            p[pos - 1] = before;
        }
        if let (Some(a), true) = (after, pos + 1 < len) {
            p[len - 1] = a;
        }
        p
    }

    /// Every marker position in the first two block sizes, with a zero or
    /// non-zero byte before it and a stray byte after it.
    #[test]
    fn constant_time_unpad_matches_reference_at_every_position() {
        for len in [PAD_MIN, PAD_MIN + PAD_BLOCK, PAD_MIN + 2 * PAD_BLOCK] {
            for pos in 0..len {
                for (marker, before, after) in [
                    (0x80, 0x00, None),
                    (0x80, 0xA5, None),
                    (0x81, 0x00, None),
                    (0x01, 0x80, None),
                    (0x80, 0x00, Some(0x01)),
                ] {
                    let p = marked_padding(len, pos, marker, before, after);
                    assert_eq!(
                        unpadded_len(&p),
                        reference_unpadded_len(&p),
                        "len {len}, pos {pos}, marker {marker:#x}"
                    );
                }
            }
            assert_eq!(unpadded_len(&vec![0u8; len]), Err(Error::Malformed));
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 512,
            ..proptest::prelude::ProptestConfig::default()
        })]

        /// Random buffers (mostly invalid) and canonical paddings with one
        /// corrupted byte agree with the reference.
        #[test]
        fn constant_time_unpad_matches_reference(
            data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..1300),
            msg_len in 0usize..1300,
            pos in proptest::prelude::any::<usize>(),
            val in proptest::prelude::any::<u8>(),
        ) {
            proptest::prop_assert_eq!(unpadded_len(&data), reference_unpadded_len(&data));
            let mut p = pad(&vec![0xC3u8; msg_len]).unwrap().expose_secret().to_vec();
            proptest::prop_assert_eq!(unpadded_len(&p), Ok(msg_len));
            let i = pos % p.len();
            p[i] = val;
            proptest::prop_assert_eq!(unpadded_len(&p), reference_unpadded_len(&p));
        }
    }

    #[test]
    fn take_splits() {
        let (a, rest) = take::<2>(&[1, 2, 3]).unwrap();
        assert_eq!((a, rest), (&[1, 2], &[3][..]));
        assert_eq!(take::<4>(&[1, 2, 3]), Err(Error::Malformed));
    }
}

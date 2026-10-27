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
pub(crate) struct Label(&'static str);

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
    pub(crate) const fn as_bytes(self) -> &'static [u8] {
        self.0.as_bytes()
    }
}

/// Every label used in this crate. Labels MUST NOT be reused for a
/// different purpose (CRYPTO_SPEC §3).
pub(crate) mod label {
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

    /// Every label, for the uniqueness test.
    #[cfg(test)]
    pub(crate) const ALL: [Label; 12] = [
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
                    let mut p = vec![0u8; len];
                    p[pos] = marker;
                    if pos > 0 {
                        p[pos - 1] = before;
                    }
                    if let (Some(a), true) = (after, pos + 1 < len) {
                        p[len - 1] = a;
                    }
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

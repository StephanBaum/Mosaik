//! Byte encodings: base64url, `clientDataJSON`, `authenticatorData` and the
//! `none` attestation object (CRYPTO_SPEC §16). The CBOR is written by hand
//! for exactly these fixed layouts; nothing here decodes CBOR.

use core::fmt::Write;

use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};

use crate::{AAGUID, COSE_KEY_LEN, Ceremony, Error, flags};

/// `A3 63 "fmt" 64 "none" 67 "attStmt" A0 68 "authData"`: a map of three
/// members, format `none`, an empty statement, then the authData key.
const ATTESTATION_HEAD: [u8; 28] = [
    0xA3, 0x63, 0x66, 0x6D, 0x74, 0x64, 0x6E, 0x6F, 0x6E, 0x65, 0x67, 0x61, 0x74, 0x74, 0x53, 0x74,
    0x6D, 0x74, 0xA0, 0x68, 0x61, 0x75, 0x74, 0x68, 0x44, 0x61, 0x74, 0x61,
];

/// `bytes` as base64url (RFC 4648 section 5) without padding.
#[must_use]
pub fn b64url(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

/// Decodes unpadded base64url into at most `max_len` bytes. Padding, other
/// alphabets, non-canonical trailing bits and over-long input are refused.
///
/// # Errors
/// [`Error::InvalidInput`] for any of those.
pub fn from_b64url(s: &str, max_len: usize) -> Result<Vec<u8>, Error> {
    if s.len() > max_len.div_ceil(3).saturating_mul(4) {
        return Err(Error::InvalidInput);
    }
    let bytes = Base64UrlUnpadded::decode_vec(s).map_err(|_| Error::InvalidInput)?;
    if bytes.len() > max_len {
        return Err(Error::InvalidInput);
    }
    Ok(bytes)
}

/// The string form of a value in `CollectedClientData` (WebAuthn L3
/// CCDToString): quoted, `"` and `\` escaped with a backslash, code points
/// below `0x20` written as `\u` and four lowercase hex digits, every other
/// code point as it is.
#[must_use]
pub fn ccd_to_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The `clientDataJSON` bytes in the limited verification format, in this
/// member order and without whitespace. `crossOrigin` is always `false` and
/// `topOrigin` is never emitted.
#[must_use]
pub fn client_data_json(ceremony: Ceremony, challenge: &[u8], origin: &str) -> Vec<u8> {
    let kind = match ceremony {
        Ceremony::Create => "webauthn.create",
        Ceremony::Get => "webauthn.get",
    };
    format!(
        "{{\"type\":\"{kind}\",\"challenge\":\"{}\",\"origin\":{},\"crossOrigin\":false}}",
        b64url(challenge),
        ccd_to_string(origin),
    )
    .into_bytes()
}

/// `SHA-256(rp_id)`: the first 32 bytes of the authenticator data.
#[must_use]
pub fn rp_id_hash(rp_id: &str) -> [u8; 32] {
    Sha256::digest(rp_id.as_bytes()).into()
}

/// The authenticator data: `rpIdHash || flags || signCount (u32 BE)`, followed
/// by the attested credential data (the AAGUID, a u16 BE length, the
/// credential ID and the `COSE_Key`) when `attested` is given. The caller sets
/// `flags::AT` with it.
///
/// # Errors
/// [`Error::InvalidInput`] if the credential ID is longer than 65535 bytes
/// (the callers limit it to `MAX_CREDENTIAL_ID_LEN`).
pub(crate) fn authenticator_data(
    rp_id: &str,
    flags: u8,
    sign_count: u32,
    attested: Option<(&[u8], &[u8; COSE_KEY_LEN])>,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(37 + 16 + 2 + 1023 + COSE_KEY_LEN);
    out.extend_from_slice(&rp_id_hash(rp_id));
    out.push(flags);
    out.extend_from_slice(&sign_count.to_be_bytes());
    if let Some((credential_id, cose_key)) = attested {
        let len = u16::try_from(credential_id.len()).map_err(|_| Error::InvalidInput)?;
        out.extend_from_slice(&AAGUID);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(credential_id);
        out.extend_from_slice(cose_key);
    }
    Ok(out)
}

/// The CBOR header of a byte string of `len` bytes (major type 2).
fn bstr_header(len: usize) -> Result<Vec<u8>, Error> {
    if let Ok(small) = u8::try_from(len) {
        if small < 24 {
            return Ok(vec![0x40 | small]);
        }
        return Ok(vec![0x58, small]);
    }
    let wide = u16::try_from(len).map_err(|_| Error::InvalidInput)?;
    let [hi, lo] = wide.to_be_bytes();
    Ok(vec![0x59, hi, lo])
}

/// The attestation object for format `none`: the CBOR map
/// `{"fmt": "none", "attStmt": {}, "authData": h'...'}` in that key order.
///
/// # Errors
/// [`Error::InvalidInput`] if `auth_data` is 65536 bytes or longer.
pub fn attestation_object_none(auth_data: &[u8]) -> Result<Vec<u8>, Error> {
    let header = bstr_header(auth_data.len())?;
    let mut out = Vec::with_capacity(ATTESTATION_HEAD.len() + header.len() + auth_data.len());
    out.extend_from_slice(&ATTESTATION_HEAD);
    out.extend_from_slice(&header);
    out.extend_from_slice(auth_data);
    Ok(out)
}

/// A flags byte for the given ceremony inputs: UP, BE, BS, plus `extra`.
pub(crate) const fn base_flags(user_verified: bool, extra: u8) -> u8 {
    let uv = if user_verified { flags::UV } else { 0 };
    flags::UP | flags::BE | flags::BS | uv | extra
}

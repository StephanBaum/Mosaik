//! Lowercase hex with fixed lengths, for the identifiers, nonces, proofs and
//! keys of the pre-auth messages and the pairing bodies.

use core::fmt;

use mosaic_core::browser_channel::{PAIRING_KEY_LEN, PairingKey};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::{Zeroize, Zeroizing};

const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// `bytes` as lowercase hex, in zeroizing memory.
pub(crate) fn encode(bytes: &[u8]) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for b in bytes {
        for nibble in [b >> 4, b & 15] {
            if let Some(d) = DIGITS.get(usize::from(nibble)) {
                out.push(char::from(*d));
            }
        }
    }
    out
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Exactly `2 * N` lowercase hex digits into `N` bytes.
pub(crate) fn decode<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let (pairs, rest) = s.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut out = [0u8; N];
    for (byte, [hi, lo]) in out.iter_mut().zip(pairs) {
        *byte = (nibble(*hi)? << 4) | nibble(*lo)?;
    }
    Some(out)
}

/// `N` bytes serialized as `2 * N` lowercase hex digits (non-secret values:
/// pairing IDs, nonces and proofs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hex<const N: usize>(pub [u8; N]);

impl<const N: usize> Serialize for Hex<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&encode(&self.0))
    }
}

struct HexVisitor<const N: usize>;

impl<const N: usize> Visitor<'_> for HexVisitor<N> {
    type Value = Hex<N>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} lowercase hex digits", N * 2)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Hex<N>, E> {
        decode(v)
            .map(Hex)
            .ok_or_else(|| E::invalid_value(de::Unexpected::Str("hex"), &self))
    }
}

impl<'de, const N: usize> Deserialize<'de> for Hex<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(HexVisitor::<N>)
    }
}

/// A [`PairingKey`] serialized as 64 lowercase hex digits. Not `Clone`; the
/// serialized text is built in zeroizing memory.
pub struct HexKey(pub PairingKey);

impl fmt::Debug for HexKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for HexKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&encode(self.0.expose_secret()))
    }
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = HexKey;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} lowercase hex digits", PAIRING_KEY_LEN * 2)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<HexKey, E> {
        let mut bytes: [u8; PAIRING_KEY_LEN] =
            decode(v).ok_or_else(|| E::invalid_value(de::Unexpected::Str("hex"), &self))?;
        let key = PairingKey::from_bytes(&mut bytes);
        bytes.zeroize();
        Ok(HexKey(key))
    }
}

impl<'de> Deserialize<'de> for HexKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(KeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_is_strict() {
        assert_eq!(&**encode(&[0x00, 0xab, 0xff]), "00abff");
        assert_eq!(decode::<3>("00abff"), Some([0, 0xab, 0xff]));
        assert_eq!(decode::<3>("00ABFF"), None, "uppercase");
        assert_eq!(decode::<3>("00abf"), None, "short");
        assert_eq!(decode::<3>("00abff00"), None, "long");
        assert_eq!(decode::<3>("00abfg"), None, "not hex");
        assert_eq!(decode::<3>("00abf\u{e9}"), None, "non-ascii");
        assert_eq!(decode::<0>(""), Some([]));
    }

    #[test]
    fn serde_hex_is_fixed_length_lowercase() {
        let id = Hex([0xab; 16]);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{}\"", "ab".repeat(16)));
        assert_eq!(serde_json::from_str::<Hex<16>>(&json).unwrap(), id);
        assert!(serde_json::from_str::<Hex<16>>("\"ab\"").is_err());
        assert!(serde_json::from_str::<Hex<16>>(&json.to_uppercase()).is_err());
        assert!(serde_json::from_str::<Hex<16>>("7").is_err());
    }

    #[test]
    fn hex_key_round_trips_and_is_redacted() {
        let json = format!("\"{}\"", "ab".repeat(32));
        let key: HexKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key.0.expose_secret(), &[0xab; 32]);
        assert_eq!(serde_json::to_string(&key).unwrap(), json);
        assert_eq!(format!("{key:?}"), "[REDACTED]");
        assert!(serde_json::from_str::<HexKey>("\"ab\"").is_err());
    }
}

//! A bounded JSON reader for export files whose strings are zeroized on
//! drop.
//!
//! `serde_json` parses into this tree through a visitor that copies every
//! string straight into a [`Str`] (`serde_json` itself keeps a scratch copy
//! of strings with escapes, THREAT_MODEL R14). Nesting is limited to
//! [`MAX_DEPTH`] and the number of values to [`MAX_NODES`], so a small file
//! cannot expand into a huge tree.

use core::cell::Cell;
use core::fmt;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use zeroize::Zeroizing;

use crate::buf::Str;
use crate::error::{Error, Result};

/// Deepest nesting of arrays and objects.
pub(crate) const MAX_DEPTH: usize = 64;
/// Most values (of any kind) in one document. A large real export (tens of
/// thousands of items with attachments) stays well below.
pub(crate) const MAX_NODES: usize = 8_000_000;

/// A parsed JSON value.
pub(crate) enum Json {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(Str),
    Array(Vec<Json>),
    /// Members in document order.
    Object(Vec<(Str, Json)>),
}

impl fmt::Debug for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Json([REDACTED])")
    }
}

impl Json {
    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .find(|(k, _)| k.as_str() == key)
                .map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string member `key` of an object.
    pub(crate) fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Self::as_str)
    }

    /// The members of an object (empty for anything else).
    pub(crate) fn members(&self) -> &[(Str, Self)] {
        match self {
            Self::Object(m) => m,
            _ => &[],
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// The elements of an array (empty for anything else).
    pub(crate) fn items(&self) -> &[Self] {
        match self {
            Self::Array(a) => a,
            _ => &[],
        }
    }

    /// The array member `key` (empty if missing or not an array).
    pub(crate) fn array(&self, key: &str) -> &[Self] {
        self.get(key).map_or(&[], Self::items)
    }

    pub(crate) const fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    /// An integer, also from a float without a fraction.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "checked: the float is integral and within i64"
    )]
    pub(crate) fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Int(i) => Some(i),
            Self::UInt(u) => i64::try_from(u).ok(),
            Self::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e18 => Some(f as i64),
            _ => None,
        }
    }

    /// A flag: `true`/`false`, or a number (0 is false), or the strings
    /// `"0"`/`"1"`/`"true"`/`"false"`.
    pub(crate) fn as_flag(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            Self::Str(s) => match s.as_str() {
                "1" | "true" => Some(true),
                "0" | "false" | "" => Some(false),
                _ => None,
            },
            other => other.as_i64().map(|i| i != 0),
        }
    }

    /// The flag member `key`; `false` if missing.
    pub(crate) fn flag(&self, key: &str) -> bool {
        self.get(key).and_then(Self::as_flag).unwrap_or(false)
    }

    /// A string member, or a number member written out (some exports store
    /// numbers where others store text).
    pub(crate) fn text(&self, key: &str) -> Option<Str> {
        match self.get(key)? {
            Self::Str(s) => Some(Zeroizing::new(s.as_str().to_owned())),
            Self::Int(i) => Some(Zeroizing::new(i.to_string())),
            Self::UInt(u) => Some(Zeroizing::new(u.to_string())),
            _ => None,
        }
    }
}

/// Parses `bytes` as one JSON document: strict UTF-8, unique keys, finite
/// numbers, no trailing data, within [`MAX_DEPTH`] and [`MAX_NODES`].
///
/// # Errors
/// [`Error::LimitExceeded`] above the limits, [`Error::Malformed`]
/// otherwise.
pub(crate) fn parse(bytes: &[u8]) -> Result<Json> {
    let budget = Cell::new(MAX_NODES);
    let exceeded = Cell::new(false);
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let seed = Seed {
        depth: 0,
        budget: &budget,
        exceeded: &exceeded,
    };
    let v = seed.deserialize(&mut de).map_err(|_| {
        if exceeded.get() {
            Error::LimitExceeded
        } else {
            Error::Malformed
        }
    })?;
    de.end().map_err(|_| Error::Malformed)?;
    Ok(v)
}

#[derive(Clone, Copy)]
struct Seed<'b> {
    depth: usize,
    budget: &'b Cell<usize>,
    exceeded: &'b Cell<bool>,
}

impl Seed<'_> {
    fn count<E: de::Error>(&self) -> core::result::Result<(), E> {
        if let Some(left) = self.budget.get().checked_sub(1) {
            self.budget.set(left);
            Ok(())
        } else {
            self.exceeded.set(true);
            Err(E::custom("too many values"))
        }
    }

    fn nested<E: de::Error>(&self) -> core::result::Result<Self, E> {
        let depth = self.depth.saturating_add(1);
        if depth > MAX_DEPTH {
            self.exceeded.set(true);
            return Err(E::custom("nesting too deep"));
        }
        Ok(Self { depth, ..*self })
    }
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Json;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> core::result::Result<Json, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::Int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::UInt(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> core::result::Result<Json, E> {
        self.count()?;
        if v.is_finite() {
            Ok(Json::Float(v))
        } else {
            Err(E::custom("non-finite number"))
        }
    }

    fn visit_str<E: de::Error>(self, v: &str) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::Str(Zeroizing::new(v.to_owned())))
    }

    fn visit_string<E: de::Error>(self, v: String) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::Str(Zeroizing::new(v)))
    }

    fn visit_unit<E: de::Error>(self) -> core::result::Result<Json, E> {
        self.count()?;
        Ok(Json::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> core::result::Result<Json, A::Error> {
        self.count()?;
        let inner = self.nested()?;
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(inner)? {
            out.push(v);
        }
        Ok(Json::Array(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> core::result::Result<Json, A::Error> {
        self.count()?;
        let inner = self.nested()?;
        let mut out: Vec<(Str, Json)> = Vec::new();
        while let Some(k) = map.next_key_seed(KeySeed)? {
            let v = map.next_value_seed(inner)?;
            out.push((k, v));
        }
        // Unique keys, checked in O(n log n) so a huge object cannot stall.
        let mut keys: Vec<&str> = out.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        if keys.windows(2).any(|w| w.first() == w.get(1)) {
            return Err(de::Error::custom("duplicate key"));
        }
        Ok(Json::Object(out))
    }
}

struct KeySeed;

impl<'de> DeserializeSeed<'de> for KeySeed {
    type Value = Str;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> core::result::Result<Str, D::Error> {
        d.deserialize_str(KeyVisitor)
    }
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = Str;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> core::result::Result<Str, E> {
        Ok(Zeroizing::new(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> core::result::Result<Str, E> {
        Ok(Zeroizing::new(v))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::*;

    #[test]
    fn parses_values_and_accessors() {
        let v = parse(
            br#"{"a":"x","n":-3,"u":18446744073709551615,"f":2.0,"g":2.5,"b":true,
                "z":null,"arr":[1,"2"],"o":{"k":"v"},"e":"es\"c"}"#,
        )
        .unwrap();
        assert!(v.is_object());
        assert_eq!(v.str("a"), Some("x"));
        assert_eq!(v.get("n").unwrap().as_i64(), Some(-3));
        assert_eq!(v.get("u").unwrap().as_i64(), None);
        assert_eq!(v.get("f").unwrap().as_i64(), Some(2));
        assert_eq!(v.get("g").unwrap().as_i64(), None);
        assert_eq!(v.str("e"), Some("es\"c"));
        assert!(v.flag("b"));
        assert!(!v.flag("missing"));
        assert!(v.get("z").unwrap().as_flag().is_none());
        assert_eq!(v.array("arr").len(), 2);
        assert!(v.array("a").is_empty());
        assert_eq!(v.get("o").unwrap().members().len(), 1);
        assert!(v.get("a").unwrap().members().is_empty());
        assert!(v.get("a").unwrap().get("x").is_none());
        assert_eq!(v.text("n").unwrap().as_str(), "-3");
        assert_eq!(v.text("u").unwrap().as_str(), "18446744073709551615");
        assert_eq!(v.text("a").unwrap().as_str(), "x");
        assert!(v.text("b").is_none());
        assert!(v.text("missing").is_none());
        assert_eq!(format!("{v:?}"), "Json([REDACTED])");
    }

    #[test]
    fn flags() {
        for (src, want) in [
            ("1", Some(true)),
            ("0", Some(false)),
            ("\"1\"", Some(true)),
            ("\"true\"", Some(true)),
            ("\"false\"", Some(false)),
            ("\"\"", Some(false)),
            ("\"maybe\"", None),
            ("false", Some(false)),
        ] {
            assert_eq!(parse(src.as_bytes()).unwrap().as_flag(), want, "{src}");
        }
    }

    #[test]
    fn numbers_as_flags_and_integers_at_the_edges() {
        for (src, want) in [
            ("2", Some(true)),
            ("-1", Some(true)),
            ("0.0", Some(false)),
            ("1.0", Some(true)),
            ("0.5", None),
            ("18446744073709551615", None),
            ("[]", None),
            ("{}", None),
        ] {
            assert_eq!(parse(src.as_bytes()).unwrap().as_flag(), want, "{src}");
        }
        assert_eq!(
            parse(b"9223372036854775807").unwrap().as_i64(),
            Some(i64::MAX)
        );
        assert_eq!(
            parse(b"-9223372036854775808").unwrap().as_i64(),
            Some(i64::MIN)
        );
        assert_eq!(parse(b"9223372036854775808").unwrap().as_i64(), None);
        assert_eq!(parse(b"1e19").unwrap().as_i64(), None);
        assert_eq!(parse(b"null").unwrap().as_i64(), None);
    }

    #[test]
    fn duplicate_keys_are_malformed_at_any_depth_and_whitespace_is_fine() {
        assert_eq!(
            parse(br#"{"a":{"b":1,"b":2}}"#).unwrap_err(),
            Error::Malformed
        );
        assert_eq!(parse(br#"[{"k":1,"k":1}]"#).unwrap_err(), Error::Malformed);
        assert!(parse(b" \n\t{ \"a\" : [ 1 , 2 ] } \r\n").is_ok());
        assert_eq!(parse(b"").unwrap_err(), Error::Malformed);
        assert_eq!(parse(b"   ").unwrap_err(), Error::Malformed);
    }

    #[test]
    fn rejects_bad_documents() {
        for bad in [
            &b"{\"a\":1,\"a\":2}"[..],
            b"{} x",
            b"[1,",
            b"\xff",
            b"1e400",
        ] {
            assert_eq!(parse(bad).unwrap_err(), Error::Malformed);
        }
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert_eq!(parse(deep.as_bytes()).unwrap_err(), Error::LimitExceeded);
        let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(parse(ok.as_bytes()).is_ok());
    }

    #[test]
    fn node_budget() {
        let budget = Cell::new(2);
        let exceeded = Cell::new(false);
        let seed = Seed {
            depth: 0,
            budget: &budget,
            exceeded: &exceeded,
        };
        let mut de = serde_json::Deserializer::from_slice(b"[1,2,3]");
        assert!(seed.deserialize(&mut de).is_err());
        assert!(exceeded.get());
        for src in ["true", "1", "-1", "1.5", "\"s\"", "null", "[]", "{}"] {
            let budget = Cell::new(0);
            let seed = Seed {
                depth: 0,
                budget: &budget,
                exceeded: &exceeded,
            };
            let mut de = serde_json::Deserializer::from_slice(src.as_bytes());
            assert!(seed.deserialize(&mut de).is_err(), "{src}");
        }
    }
}

//! Zeroizing JSON values: the in-memory form of every string, number and
//! nested object of item plaintext.
//!
//! `serde_json::Value` keeps strings in plain `String`s that are not wiped
//! when dropped. Item plaintext lives here instead: every [`Text`] zeroizes
//! its buffer on drop and prints as `[REDACTED]`, so dropping (or locking
//! away) a decrypted item leaves no copies of its strings behind in memory
//! this crate owns. Residual copies inside `serde_json`'s scratch buffer
//! (strings with escape sequences) are recorded in the M2 ADR.
//!
//! Every type here has a total order. The merge needs it to pick a
//! deterministic value when two replicas disagree about data that carries no
//! clock (unknown top-level members, ill-formed duplicates).

use core::cmp::Ordering;
use core::fmt;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::io;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap as _, SerializeSeq as _};
use serde::{Deserializer, Serialize, Serializer};
use subtle::ConstantTimeEq as _;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{Error, Result};
use crate::secret::{SecretString, SecretVec};

/// Deepest nesting of arrays and objects in a plaintext, counted from the
/// root object (ADR draft M2). `serde_json` itself stops at 128.
pub const MAX_JSON_DEPTH: usize = 32;

const REDACTED: &str = "[REDACTED]";

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// A string from item plaintext (a title, a URL, a password).
///
/// Zeroized on drop, `Debug` prints `[REDACTED]`, and it is deliberately not
/// `Clone`. Equality is constant-time in the contents; the order is the
/// byte order of the UTF-8 encoding and is only used for deterministic
/// tie-breaks and sorting, never to compare a secret against a guess.
pub struct Text(String);

impl Text {
    /// Copies `s` into a new zeroizing buffer of exactly its length.
    #[must_use]
    pub fn new(s: &str) -> Self {
        let mut v = String::with_capacity(s.len());
        v.push_str(s);
        Self(v)
    }

    /// Takes ownership of `s`; it is zeroized on drop from now on.
    pub(crate) fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Borrows the string. Never log it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// A second zeroizing copy, for the merge and the typed views.
    pub(crate) fn dup(&self) -> Self {
        Self::new(&self.0)
    }

    /// Copies the string into a [`SecretString`].
    pub(crate) fn to_secret(&self) -> Result<SecretString> {
        let mut s = SecretString::with_capacity(self.0.len())?;
        for c in self.0.chars() {
            s.push(c);
        }
        Ok(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

impl Eq for Text {}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Number
// ---------------------------------------------------------------------------

/// A JSON number, in the three forms `serde_json` produces.
///
/// Non-negative integers are always [`Number::PosInt`], so every number has
/// exactly one representation. Floats are finite (JSON has no NaN or
/// infinity).
#[derive(Clone, Copy, Debug)]
pub enum Number {
    /// An integer `>= 0`.
    PosInt(u64),
    /// An integer `< 0`.
    NegInt(i64),
    /// A finite float.
    Float(f64),
}

impl Number {
    /// An integer in canonical form.
    #[must_use]
    pub const fn from_i64(v: i64) -> Self {
        if v < 0 {
            Self::NegInt(v)
        } else {
            Self::PosInt(v.unsigned_abs())
        }
    }

    /// The value as `u64`, if it is a non-negative integer.
    #[must_use]
    pub const fn as_u64(self) -> Option<u64> {
        match self {
            Self::PosInt(v) => Some(v),
            Self::NegInt(_) | Self::Float(_) => None,
        }
    }

    /// The value as `i64`, if it is an integer in range.
    #[must_use]
    pub fn as_i64(self) -> Option<i64> {
        match self {
            Self::PosInt(v) => i64::try_from(v).ok(),
            Self::NegInt(v) => Some(v),
            Self::Float(_) => None,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::NegInt(_) => 0,
            Self::PosInt(_) => 1,
            Self::Float(_) => 2,
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::PosInt(a), Self::PosInt(b)) => a.cmp(b),
            (Self::NegInt(a), Self::NegInt(b)) => a.cmp(b),
            (Self::Float(a), Self::Float(b)) => a.total_cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

// ---------------------------------------------------------------------------
// Value and Object
// ---------------------------------------------------------------------------

/// A JSON value whose strings are [`Text`]. `Debug` prints `[REDACTED]`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string.
    String(Text),
    /// An array.
    Array(Vec<Value>),
    /// An object with unique keys, in key order.
    Object(Object),
}

impl Value {
    /// A string value.
    #[must_use]
    pub fn text(s: &str) -> Self {
        Self::String(Text::new(s))
    }

    /// An integer value.
    #[must_use]
    pub const fn int(v: i64) -> Self {
        Self::Number(Number::from_i64(v))
    }

    /// A non-negative integer value.
    #[must_use]
    pub const fn uint(v: u64) -> Self {
        Self::Number(Number::PosInt(v))
    }

    /// Bytes as an unpadded base64url string (RFC 4648 §5), encoded in
    /// constant time.
    #[must_use]
    pub fn bytes(bytes: &[u8]) -> Self {
        let mut buf = Zeroizing::new(vec![0u8; Base64UrlUnpadded::encoded_len(bytes)]);
        match Base64UrlUnpadded::encode(bytes, &mut buf) {
            Ok(s) => Self::text(s),
            // The buffer has exactly the encoded length, so this is
            // unreachable; an empty string decodes to nothing and is caught
            // by every typed decoder that requires bytes.
            Err(_) => Self::text(""),
        }
    }

    /// The string, if this is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(t) => Some(t.as_str()),
            _ => None,
        }
    }

    /// The boolean, if this is a boolean.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The number as `u64`, if it is a non-negative integer.
    #[must_use]
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// The number as `i64`, if it is an integer in range.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// The object, if this is an object.
    #[must_use]
    pub const fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Decodes a base64url string into secret bytes.
    pub(crate) fn to_secret_bytes(&self) -> Result<SecretVec> {
        let s = self.as_str().ok_or(Error::Malformed)?;
        let len = decoded_len(s.len());
        let mut out = SecretVec::with_capacity(len)?;
        out.as_mut_vec().resize(len, 0);
        let n = Base64UrlUnpadded::decode(s, out.as_mut_vec())
            .map_err(|_| Error::Malformed)?
            .len();
        out.as_mut_vec().truncate(n);
        Ok(out)
    }

    /// A second zeroizing copy.
    pub(crate) fn dup(&self) -> Self {
        match self {
            Self::Null => Self::Null,
            Self::Bool(b) => Self::Bool(*b),
            Self::Number(n) => Self::Number(*n),
            Self::String(t) => Self::String(t.dup()),
            Self::Array(a) => Self::Array(a.iter().map(Self::dup).collect()),
            Self::Object(o) => Self::Object(o.dup()),
        }
    }
}

/// `floor(3n / 4)` without overflow: the decoded length of `n` unpadded
/// base64 characters.
const fn decoded_len(n: usize) -> usize {
    (n / 4) * 3 + (n % 4) * 3 / 4
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// A JSON object: unique keys in byte order.
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Object(BTreeMap<Text, Value>);

impl Object {
    /// An empty object.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// The member `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Sets the member `key`.
    pub fn insert(&mut self, key: &str, value: Value) {
        self.0.insert(Text::new(key), value);
    }

    /// Removes and returns the member `key`.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    /// The members in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the object has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn insert_text(&mut self, key: Text, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    pub(crate) fn into_iter_owned(self) -> impl Iterator<Item = (Text, Value)> {
        self.0.into_iter()
    }

    /// A second zeroizing copy.
    pub(crate) fn dup(&self) -> Self {
        Self(self.0.iter().map(|(k, v)| (k.dup(), v.dup())).collect())
    }

    /// Join of two objects whose members carry no clock: every key of
    /// either side, and the larger value (in [`Value`]'s total order) where
    /// both have one. Commutative, associative and idempotent.
    pub(crate) fn join_max(&self, other: &Self) -> Self {
        let mut out = self.dup();
        for (k, v) in &other.0 {
            match out.0.get_mut(k.as_str()) {
                Some(mine) if *mine >= *v => {}
                Some(mine) => *mine = v.dup(),
                None => {
                    out.0.insert(k.dup(), v.dup());
                }
            }
        }
        out
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

impl Serialize for Text {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        match self {
            Self::Null => s.serialize_unit(),
            Self::Bool(b) => s.serialize_bool(*b),
            Self::Number(Number::PosInt(v)) => s.serialize_u64(*v),
            Self::Number(Number::NegInt(v)) => s.serialize_i64(*v),
            Self::Number(Number::Float(v)) => s.serialize_f64(*v),
            Self::String(t) => t.serialize(s),
            Self::Array(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for v in a {
                    seq.serialize_element(v)?;
                }
                seq.end()
            }
            Self::Object(o) => o.serialize(s),
        }
    }
}

impl Serialize for Object {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

/// Deserializes a [`Value`] at nesting depth `depth`.
struct ValueSeed {
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> core::result::Result<Value, D::Error> {
        d.deserialize_any(ValueVisitor { depth: self.depth })
    }
}

struct ValueVisitor {
    depth: usize,
}

impl ValueVisitor {
    fn nested<E: de::Error>(&self) -> core::result::Result<usize, E> {
        let depth = self.depth.saturating_add(1);
        if depth > MAX_JSON_DEPTH {
            return Err(E::custom("nesting too deep"));
        }
        Ok(depth)
    }
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> core::result::Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> core::result::Result<Value, E> {
        Ok(Value::int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> core::result::Result<Value, E> {
        Ok(Value::uint(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> core::result::Result<Value, E> {
        if v.is_finite() {
            Ok(Value::Number(Number::Float(v)))
        } else {
            Err(E::custom("non-finite number"))
        }
    }

    fn visit_str<E: de::Error>(self, v: &str) -> core::result::Result<Value, E> {
        Ok(Value::text(v))
    }

    fn visit_string<E: de::Error>(self, v: String) -> core::result::Result<Value, E> {
        Ok(Value::String(Text::from_string(v)))
    }

    fn visit_unit<E: de::Error>(self) -> core::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> core::result::Result<Value, A::Error> {
        let depth = self.nested()?;
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(ValueSeed { depth })? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> core::result::Result<Value, A::Error> {
        let depth = self.nested()?;
        let mut out = Object::new();
        while let Some(k) = map.next_key_seed(TextSeed)? {
            let v = map.next_value_seed(ValueSeed { depth })?;
            if out.insert_text(k, v).is_some() {
                return Err(de::Error::custom("duplicate key"));
            }
        }
        Ok(Value::Object(out))
    }
}

/// Deserializes an object key into a [`Text`].
struct TextSeed;

impl<'de> DeserializeSeed<'de> for TextSeed {
    type Value = Text;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> core::result::Result<Text, D::Error> {
        d.deserialize_str(TextVisitor)
    }
}

struct TextVisitor;

impl Visitor<'_> for TextVisitor {
    type Value = Text;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> core::result::Result<Text, E> {
        Ok(Text::new(v))
    }

    fn visit_string<E: de::Error>(self, v: String) -> core::result::Result<Text, E> {
        Ok(Text::from_string(v))
    }
}

/// Parses JSON text into a [`Value`]: strict UTF-8, unique keys, finite
/// numbers, at most [`MAX_JSON_DEPTH`] levels, no trailing data.
///
/// # Errors
/// [`Error::Malformed`] for anything else.
pub(crate) fn parse_json(bytes: &[u8]) -> Result<Value> {
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let v = ValueSeed { depth: 0 }
        .deserialize(&mut de)
        .map_err(|_| Error::Malformed)?;
    de.end().map_err(|_| Error::Malformed)?;
    Ok(v)
}

/// An `io::Write` sink for serialized plaintext that never leaves an
/// unzeroized copy behind: when it grows, it moves the bytes into a larger
/// buffer and wipes the old one, and it refuses to grow beyond `limit`.
struct ZeroizingWriter {
    buf: Vec<u8>,
    limit: usize,
    failure: Option<Error>,
}

impl io::Write for ZeroizingWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let needed = self.buf.len().saturating_add(data.len());
        if needed > self.limit {
            self.failure = Some(Error::SizeLimit);
            return Err(io::Error::other("size limit"));
        }
        if needed > self.buf.capacity() {
            let cap = needed
                .max(self.buf.capacity().saturating_mul(2))
                .min(self.limit);
            let mut grown = Vec::new();
            if grown.try_reserve_exact(cap).is_err() {
                self.failure = Some(Error::Allocation);
                return Err(io::Error::other("allocation"));
            }
            grown.extend_from_slice(&self.buf);
            self.buf.zeroize();
            self.buf = grown;
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for ZeroizingWriter {
    fn drop(&mut self) {
        self.buf.zeroize();
    }
}

/// Serializes `value` as compact JSON into a zeroizing buffer of at most
/// `limit` bytes.
///
/// # Errors
/// [`Error::SizeLimit`] above `limit`, [`Error::Allocation`] if memory
/// cannot be reserved.
pub(crate) fn to_json<T: Serialize + ?Sized>(value: &T, limit: usize) -> Result<SecretVec> {
    let mut w = ZeroizingWriter {
        buf: Vec::new(),
        limit,
        failure: None,
    };
    if w.buf.try_reserve_exact(limit.min(4096)).is_err() {
        return Err(Error::Allocation);
    }
    match serde_json::to_writer(&mut w, value) {
        Ok(()) => Ok(SecretVec::from(core::mem::take(&mut w.buf))),
        Err(_) => Err(w.failure.unwrap_or(Error::Malformed)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(json: &str) -> String {
        let v = parse_json(json.as_bytes()).unwrap();
        let out = to_json(&v, 1 << 20).unwrap();
        String::from_utf8(out.expose_secret().to_vec()).unwrap()
    }

    #[test]
    fn parse_and_serialize() {
        assert_eq!(
            roundtrip(r#"{"b":[1,-2,3.5,true,null,"x\"y"],"a":{}}"#),
            r#"{"a":{},"b":[1,-2,3.5,true,null,"x\"y"]}"#
        );
        assert_eq!(roundtrip(r#""é\n""#), "\"\u{e9}\\n\"");
    }

    #[test]
    fn rejects_bad_json() {
        for bad in [
            &b"{\"a\":1,\"a\":2}"[..],
            b"{} x",
            b"\"\xff\"",
            b"[1,]",
            b"",
            b"1e400",
            b"{\"a\":1",
        ] {
            assert_eq!(parse_json(bad).unwrap_err(), Error::Malformed, "{bad:?}");
        }
        let deep = "[".repeat(MAX_JSON_DEPTH) + &"]".repeat(MAX_JSON_DEPTH);
        assert!(parse_json(deep.as_bytes()).is_ok());
        let deeper = "[".repeat(MAX_JSON_DEPTH + 1) + &"]".repeat(MAX_JSON_DEPTH + 1);
        assert_eq!(parse_json(deeper.as_bytes()).unwrap_err(), Error::Malformed);
        let deep_obj = "{\"a\":".repeat(MAX_JSON_DEPTH + 1) + "1" + &"}".repeat(MAX_JSON_DEPTH + 1);
        assert_eq!(
            parse_json(deep_obj.as_bytes()).unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    fn numbers_are_canonical_and_ordered() {
        assert_eq!(Number::from_i64(5), Number::PosInt(5));
        assert_eq!(Number::from_i64(-5).as_i64(), Some(-5));
        assert_eq!(Number::PosInt(u64::MAX).as_i64(), None);
        assert_eq!(Number::Float(1.0).as_u64(), None);
        assert_eq!(Number::Float(1.0).as_i64(), None);
        assert_eq!(Number::NegInt(-1).as_u64(), None);
        assert!(Number::NegInt(-1) < Number::PosInt(0));
        assert!(Number::PosInt(u64::MAX) < Number::Float(-1.0));
        assert!(Number::Float(-0.0) < Number::Float(0.0));
        assert_eq!(Number::Float(2.5), Number::Float(2.5));
        let v = parse_json(b"[-0.0, 0.1, 18446744073709551615, -9223372036854775808]").unwrap();
        let back = parse_json(to_json(&v, 1024).unwrap().expose_secret()).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn value_order_is_total() {
        let vals = [
            Value::Null,
            Value::Bool(false),
            Value::Bool(true),
            Value::int(-1),
            Value::uint(1),
            Value::text("a"),
            Value::Array(vec![]),
            Value::Object(Object::new()),
        ];
        for w in vals.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn accessors() {
        let v = parse_json(br#"{"s":"x","b":true,"n":-3,"a":[1]}"#).unwrap();
        let o = v.as_object().unwrap();
        assert_eq!(o.len(), 4);
        assert!(!o.is_empty());
        assert_eq!(o.get("s").unwrap().as_str(), Some("x"));
        assert_eq!(o.get("b").unwrap().as_bool(), Some(true));
        assert_eq!(o.get("n").unwrap().as_i64(), Some(-3));
        assert_eq!(o.get("n").unwrap().as_u64(), None);
        assert_eq!(o.get("a").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(o.get("s").unwrap().as_bool(), None);
        assert_eq!(o.get("b").unwrap().as_str(), None);
        assert_eq!(o.get("b").unwrap().as_i64(), None);
        assert!(o.get("b").unwrap().as_object().is_none());
        assert!(o.get("b").unwrap().as_array().is_none());
        assert_eq!(o.iter().count(), 4);
        assert_eq!(format!("{v:?}"), "[REDACTED]");
        assert_eq!(format!("{o:?}"), "[REDACTED]");
        assert_eq!(format!("{:?}", Text::new("pw")), "[REDACTED]");
        assert_eq!(v.dup(), v);
        let t = Text::from("abc");
        assert_eq!(t.len(), 3);
        assert!(!t.is_empty());
        assert_eq!(t.to_secret().unwrap().expose_secret(), "abc");
    }

    #[test]
    fn base64url_bytes() {
        for len in 0..40 {
            let data: Vec<u8> = (0..len)
                .map(|i: u32| u8::try_from(i * 37 % 256).unwrap())
                .collect();
            let v = Value::bytes(&data);
            assert!(!v.as_str().unwrap().contains('='));
            assert_eq!(v.to_secret_bytes().unwrap().expose_secret(), &data[..]);
        }
        // Non-canonical trailing bits, padding, wrong alphabet, bad length.
        for bad in ["AB", "AA==", "A+/=", "A", "AAAAA", "a b"] {
            assert_eq!(
                Value::text(bad).to_secret_bytes().unwrap_err(),
                Error::Malformed,
                "{bad}"
            );
        }
        assert_eq!(Value::Null.to_secret_bytes().unwrap_err(), Error::Malformed);
    }

    #[test]
    fn writer_limits_and_growth() {
        let big = Value::text(&"x".repeat(10_000));
        assert_eq!(to_json(&big, 100).unwrap_err(), Error::SizeLimit);
        let out = to_json(&big, 20_000).unwrap();
        assert_eq!(out.len(), 10_002);
        // The exact limit is accepted.
        assert_eq!(to_json(&big, 10_002).unwrap().len(), 10_002);
        assert_eq!(to_json(&big, 10_001).unwrap_err(), Error::SizeLimit);
    }

    #[test]
    fn join_max_is_a_semilattice() {
        let a = parse_json(br#"{"x":1,"y":"b"}"#).unwrap();
        let b = parse_json(br#"{"x":2,"z":null}"#).unwrap();
        let (Value::Object(a), Value::Object(b)) = (a, b) else {
            unreachable!()
        };
        let ab = a.join_max(&b);
        assert_eq!(ab, b.join_max(&a));
        assert_eq!(ab.join_max(&ab), ab);
        assert_eq!(ab.get("x").unwrap().as_u64(), Some(2));
        assert_eq!(ab.len(), 3);
    }
}

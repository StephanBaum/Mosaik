//! `Item::has_otp` and `Item::has_primary_otp` answer exactly as decoding
//! the OTP configuration and the custom fields does, malformed input
//! included.

use super::*;
use crate::model::clock::Timestamp;
use crate::model::types::CustomFieldValue;
use crate::model::value::{Object, Text};

fn dev() -> DeviceId {
    DeviceId::from_bytes([7; 16]).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn login() -> Item {
    Item::new(&ItemType::Login, dev(), at(1)).unwrap()
}

/// Writes a raw register, one edit (one stamp) per call.
fn put_raw(item: &mut Item, key: &str, value: Value, ms: i64) {
    item.edit(dev(), at(ms)).put(key, value, true).unwrap();
}

fn valid_config() -> Object {
    let mut o = Object::new();
    o.insert("secret", Value::bytes(b"12345678901234567890"));
    o.insert("algorithm", Value::text("sha1"));
    o.insert("digits", Value::uint(6));
    o.insert("period", Value::uint(30));
    o.insert("kind", Value::text("totp"));
    o
}

fn broken(f: impl FnOnce(&mut Object)) -> Value {
    let mut o = valid_config();
    f(&mut o);
    Value::Object(o)
}

/// Valid and malformed configurations, as raw values.
fn config_cases() -> Vec<(&'static str, Value)> {
    let long = "A".repeat(200);
    vec![
        ("valid", Value::Object(valid_config())),
        (
            "valid with extras",
            broken(|o| {
                o.insert("issuer", Value::text("Example"));
                o.insert("label", Value::text("me"));
                o.insert("hardware_protected", Value::Bool(true));
                o.insert("future", Value::uint(1));
            }),
        ),
        (
            "valid steam",
            broken(|o| {
                o.insert("kind", Value::text("steam"));
                o.insert("digits", Value::uint(5));
            }),
        ),
        ("not an object", Value::text("x")),
        ("null", Value::Null),
        (
            "empty secret",
            broken(|o| o.insert("secret", Value::text(""))),
        ),
        (
            "bad base64",
            broken(|o| o.insert("secret", Value::text("!!!!"))),
        ),
        (
            "non-canonical base64",
            broken(|o| o.insert("secret", Value::text("AB"))),
        ),
        (
            "secret too long",
            broken(|o| o.insert("secret", Value::text(&long))),
        ),
        (
            "secret not text",
            broken(|o| o.insert("secret", Value::uint(1))),
        ),
        (
            "secret missing",
            broken(|o| {
                o.remove("secret");
            }),
        ),
        (
            "bad algorithm",
            broken(|o| o.insert("algorithm", Value::text("md5"))),
        ),
        ("digits 9", broken(|o| o.insert("digits", Value::uint(9)))),
        (
            "digits 5 totp",
            broken(|o| o.insert("digits", Value::uint(5))),
        ),
        (
            "digits huge",
            broken(|o| o.insert("digits", Value::uint(300))),
        ),
        (
            "digits negative",
            broken(|o| o.insert("digits", Value::int(-1))),
        ),
        (
            "steam 6 digits",
            broken(|o| o.insert("kind", Value::text("steam"))),
        ),
        ("period 0", broken(|o| o.insert("period", Value::uint(0)))),
        (
            "period 301",
            broken(|o| o.insert("period", Value::uint(301))),
        ),
        (
            "period over u32",
            broken(|o| o.insert("period", Value::uint(u64::from(u32::MAX) + 1))),
        ),
        ("bad kind", broken(|o| o.insert("kind", Value::text("x")))),
        (
            "kind missing",
            broken(|o| {
                o.remove("kind");
            }),
        ),
        (
            "issuer number",
            broken(|o| o.insert("issuer", Value::uint(1))),
        ),
        ("label null", broken(|o| o.insert("label", Value::Null))),
        (
            "hardware_protected text",
            broken(|o| o.insert("hardware_protected", Value::text("yes"))),
        ),
    ]
}

fn custom_value(label: Option<Value>, kind: Value, value: Option<Value>) -> Value {
    let mut o = Object::new();
    if let Some(l) = label {
        o.insert("label", l);
    }
    o.insert("kind", kind);
    if let Some(v) = value {
        o.insert("value", v);
    }
    Value::Object(o)
}

/// The answers of the callers before this change: decode the primary
/// configuration, and decode every custom field looking for an OTP one.
fn reference(item: &Item) -> (bool, bool) {
    let primary = item.otp().is_some();
    let custom = item
        .custom_fields()
        .iter()
        .any(|(_, f)| matches!(f.value, CustomFieldValue::Otp(_)));
    (primary, primary || custom)
}

fn check(item: &Item, what: &str) {
    let (primary, any) = reference(item);
    assert_eq!(item.has_primary_otp(), primary, "primary: {what}");
    assert_eq!(item.has_otp(), any, "any: {what}");
}

#[test]
fn an_item_without_otp_has_none() {
    let item = login();
    check(&item, "empty item");
    assert!(!item.has_otp());
    assert!(!item.has_primary_otp());
}

#[test]
fn a_primary_otp_matches_decoding_for_valid_and_malformed_values() {
    for (name, value) in config_cases() {
        let mut item = login();
        put_raw(&mut item, OTP, value, 10);
        check(&item, name);
    }
    let mut item = login();
    put_raw(&mut item, OTP, Value::Object(valid_config()), 10);
    assert!(item.has_otp() && item.has_primary_otp());
}

#[test]
fn a_custom_otp_field_matches_decoding_for_valid_and_malformed_values() {
    for (name, value) in config_cases() {
        let mut item = login();
        let field = custom_value(Some(Value::text("code")), Value::text("otp"), Some(value));
        put_raw(&mut item, "field:a", field, 10);
        check(&item, name);
    }
    let mut item = login();
    let field = custom_value(
        Some(Value::text("code")),
        Value::text("otp"),
        Some(Value::Object(valid_config())),
    );
    put_raw(&mut item, "field:a", field, 10);
    assert!(item.has_otp());
    assert!(!item.has_primary_otp());
}

#[test]
fn malformed_custom_fields_match_decoding() {
    let good = Value::Object(valid_config());
    let label = || Some(Value::text("l"));
    let cases = [
        (
            "missing label",
            custom_value(None, Value::text("otp"), Some(good.clone())),
        ),
        (
            "label not text",
            custom_value(Some(Value::uint(1)), Value::text("otp"), Some(good.clone())),
        ),
        (
            "kind not text",
            custom_value(label(), Value::uint(1), Some(good.clone())),
        ),
        (
            "kind text with otp value",
            custom_value(label(), Value::text("text"), Some(good.clone())),
        ),
        (
            "kind uppercase",
            custom_value(label(), Value::text("OTP"), Some(good.clone())),
        ),
        (
            "value missing",
            custom_value(label(), Value::text("otp"), None),
        ),
        (
            "value null",
            custom_value(label(), Value::text("otp"), Some(Value::Null)),
        ),
        ("not an object", Value::text("x")),
        ("null", Value::Null),
    ];
    for (name, field) in cases {
        let mut item = login();
        put_raw(&mut item, "field:a", field, 10);
        check(&item, name);
    }
}

#[test]
fn a_removed_or_overwritten_field_matches_decoding() {
    let secret = || SecretVec::from_slice(b"12345678901234567890").unwrap();
    let otp_field = || {
        CustomField::new(
            Text::new("code"),
            CustomFieldValue::Otp(OtpConfig::totp(secret())),
        )
    };

    let mut item = login();
    let id = item
        .edit(dev(), at(10))
        .add_custom_field(&otp_field())
        .unwrap();
    check(&item, "added");
    assert!(item.has_otp());

    // Overwritten by a text field: the winning value is no longer OTP.
    let text = CustomField::new(Text::new("note"), CustomFieldValue::Text(Text::new("hi")));
    item.edit(dev(), at(20))
        .set_custom_field(&id, &text)
        .unwrap();
    check(&item, "overwritten");
    assert!(!item.has_otp());

    // Overwritten back, then removed.
    item.edit(dev(), at(30))
        .set_custom_field(&id, &otp_field())
        .unwrap();
    check(&item, "overwritten back");
    assert!(item.has_otp());
    item.edit(dev(), at(40)).remove_custom_field(&id).unwrap();
    check(&item, "removed");
    assert!(!item.has_otp());

    // A primary configuration that is set, then cleared.
    let config = OtpConfig::totp(secret());
    item.edit(dev(), at(50)).set_otp(&config).unwrap();
    check(&item, "primary set");
    assert!(item.has_primary_otp());
    item.edit(dev(), at(60)).clear_otp().unwrap();
    check(&item, "primary cleared");
    assert!(!item.has_otp());

    // A deleted item.
    item.edit(dev(), at(70)).set_otp(&config).unwrap();
    item.delete(dev(), at(80)).unwrap();
    check(&item, "deleted");
}

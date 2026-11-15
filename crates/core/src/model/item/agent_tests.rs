//! Agent bindings in the item model (ADR-0159, CRYPTO_SPEC §11).
#![expect(clippy::assert_is_empty, reason = "tests")]

use super::*;
use crate::model::clock::Timestamp;
use crate::model::types::AgentInject;
use crate::model::value::Text;
use crate::model::value::parse_json;

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 16]).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn api_key() -> Item {
    let mut item = Item::new(&ItemType::ApiKey, dev(1), at(100)).unwrap();
    item.edit(dev(1), at(100)).set_title("OpenAI").unwrap();
    item
}

fn copy(item: &Item) -> Item {
    Item::from_json(item.to_json().unwrap().expose_secret()).unwrap()
}

fn record(json: &str) -> AgentBindingRecord {
    AgentBindingRecord::from_value(&parse_json(json.as_bytes()).unwrap()).unwrap()
}

fn json_of(rec: &AgentBindingRecord) -> String {
    String::from_utf8(rec.canonical_json().unwrap().expose_secret().to_vec()).unwrap()
}

const BEARER: &str = r#"{"field":"api_key.secret","origins":["https://api.openai.com"],"inject":{"kind":"bearer"},"env":"OPENAI_API_KEY","run":false,"methods":null,"paths":null}"#;

#[test]
fn agent_binding_round_trips_with_unknown_members() {
    let rec = record(
        r#"{"field":"api_key.secret","origins":["https://a.example"],"future":1,
            "inject":{"kind":"header","name":"x-api-key","prefix":"","z":2}}"#,
    );
    let mut item = api_key();
    let id = item.edit(dev(1), at(101)).add_agent_binding(&rec).unwrap();
    let back = copy(&item);
    let listed = back.agent_bindings();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, id);
    let json = json_of(&listed[0].1);
    assert!(json.contains(r#""future":1"#), "top-level member kept");
    assert!(json.contains(r#""z":2"#), "member inside inject kept");
    assert!(
        json.contains(r#""inject":{"kind":"header","name":"x-api-key","prefix":"","z":2}"#),
        "{json}"
    );
    assert!(matches!(
        &listed[0].1.inject,
        AgentInject::Header { name, prefix } if name.as_str() == "x-api-key" && prefix.is_empty()
    ));
    assert!(back.unreadable_fields().is_empty());
}

#[test]
fn canonical_json_sorts_keys_without_whitespace() {
    let rec = record(BEARER);
    assert_eq!(
        json_of(&rec),
        r#"{"env":"OPENAI_API_KEY","field":"api_key.secret","inject":{"kind":"bearer"},"methods":null,"origins":["https://api.openai.com"],"paths":null,"run":false}"#
    );
    // Unset members are written as null, so absent and null agree.
    let minimal = record(
        r#"{"inject":{"kind":"bearer"},"origins":["https://api.openai.com"],"field":"api_key.secret"}"#,
    );
    assert_eq!(
        json_of(&minimal),
        r#"{"env":null,"field":"api_key.secret","inject":{"kind":"bearer"},"methods":null,"origins":["https://api.openai.com"],"paths":null,"run":false}"#
    );
    let built = AgentBindingRecord::new(
        "api_key.secret",
        &["https://api.openai.com"],
        AgentInject::Bearer,
    );
    assert_eq!(json_of(&built), json_of(&minimal));
}

#[test]
fn canonical_json_allows_integers_only() {
    let ints = record(
        r#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"bearer"},"n":-5,"m":18446744073709551615}"#,
    );
    let j = json_of(&ints);
    assert!(
        j.contains(r#""m":18446744073709551615,"methods""#) && j.contains(r#""n":-5,"origins""#),
        "{j}"
    );
    let float = record(
        r#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"bearer"},"n":1.5}"#,
    );
    assert_eq!(float.canonical_json().err(), Some(Error::Malformed));
    let nested = record(
        r#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"bearer","k":[{"x":1e3}]}}"#,
    );
    assert_eq!(nested.canonical_json().err(), Some(Error::Malformed));
}

#[test]
fn basic_users_and_optional_members_round_trip() {
    for (user, expect) in [
        (r#"{"literal":"me"}"#, r#"{"literal":"me"}"#),
        (
            r#"{"field":"api_key.key_id"}"#,
            r#"{"field":"api_key.key_id"}"#,
        ),
        (r#""secretIsUser""#, r#""secretIsUser""#),
    ] {
        let rec = record(&format!(
            r#"{{"field":"api_key.secret","origins":["https://a.example"],"inject":{{"kind":"basic","user":{user}}},
                "env":"K","run":true,"methods":["GET","POST"],"paths":["/v1"]}}"#
        ));
        assert!(matches!(rec.inject, AgentInject::Basic { .. }));
        let json = json_of(&rec);
        assert!(json.contains(&format!(r#""inject":{{"kind":"basic","user":{expect}}}"#)));
        assert!(json.contains(r#""methods":["GET","POST"]"#));
        assert!(json.contains(r#""paths":["/v1"]"#));
        assert!(json.contains(r#""run":true"#));
        let again = record(&json);
        assert_eq!(json_of(&again), json);
        assert_eq!(json_of(&rec.dup()), json);
    }
    let q = record(
        r#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"query","name":"key"}}"#,
    );
    assert!(matches!(q.inject, AgentInject::Query { .. }));
}

#[test]
fn malformed_bindings_do_not_decode() {
    for bad in [
        r#"{"origins":["https://a.example"],"inject":{"kind":"bearer"}}"#,
        r#"{"field":"f","inject":{"kind":"bearer"}}"#,
        r#"{"field":"f","origins":[1],"inject":{"kind":"bearer"}}"#,
        r#"{"field":"f","origins":"https://a.example","inject":{"kind":"bearer"}}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"header"}}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"bearer"},"run":"yes"}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"bearer"},"env":5}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"bearer"},"methods":"GET"}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"basic","user":{"literal":"a","field":"b"}}}"#,
        r#"{"field":"f","origins":[],"inject":{"kind":"basic","user":"other"}}"#,
        r"[1]",
    ] {
        let v = parse_json(bad.as_bytes()).unwrap();
        assert!(AgentBindingRecord::from_value(&v).is_err(), "{bad}");
    }
}

#[test]
fn concurrent_edits_of_one_binding_are_a_conflict_and_of_two_bindings_are_not() {
    let mut base = api_key();
    let id = base
        .edit(dev(1), at(101))
        .add_agent_binding(&record(BEARER))
        .unwrap();
    let key = format!("agent:{}", id.as_str());

    // Two devices edit the same binding.
    let (mut a, mut b) = (copy(&base), copy(&base));
    let mut ra = record(BEARER);
    ra.origins.push(Text::new("https://a.example"));
    let mut rb = record(BEARER);
    rb.origins.push(Text::new("https://evil.example"));
    a.edit(dev(2), at(200)).set_agent_binding(&id, &ra).unwrap();
    b.edit(dev(3), at(201)).set_agent_binding(&id, &rb).unwrap();
    let merged = a.merge(&b);
    assert!(
        merged.conflicts().iter().any(|c| c.key == key),
        "a concurrent edit is a conflict like url:"
    );
    assert!(
        merged.agent_bindings().is_empty(),
        "a conflicted binding has no single value and is not usable"
    );
    // The same edit twice is no conflict.
    let (mut c, mut d) = (copy(&base), copy(&base));
    c.edit(dev(2), at(200)).set_agent_binding(&id, &ra).unwrap();
    d.edit(dev(3), at(201)).set_agent_binding(&id, &ra).unwrap();
    assert_eq!(c.merge(&d).agent_bindings().len(), 1);

    // Two devices each add a binding.
    let (mut e1, mut e2) = (copy(&base), copy(&base));
    e1.edit(dev(2), at(300))
        .add_agent_binding(&record(BEARER))
        .unwrap();
    e2.edit(dev(3), at(301))
        .add_agent_binding(&record(BEARER))
        .unwrap();
    let both = e1.merge(&e2);
    assert_eq!(both.agent_bindings().len(), 3);
    assert!(
        both.conflicts()
            .iter()
            .all(|c| !c.key.starts_with("agent:"))
    );
}

#[test]
fn set_and_remove_need_an_existing_binding_or_are_idempotent() {
    let mut item = api_key();
    let missing = ElementId::from_stored("0000000000000001.x.0").unwrap();
    let mut e = item.edit(dev(1), at(101));
    assert_eq!(
        e.set_agent_binding(&missing, &record(BEARER)).err(),
        Some(Error::Malformed)
    );
    let id = e.add_agent_binding(&record(BEARER)).unwrap();
    let mut changed = record(BEARER);
    changed.run = true;
    e.set_agent_binding(&id, &changed).unwrap();
    assert!(item.agent_bindings()[0].1.run);
    let mut e = item.edit(dev(1), at(102));
    e.remove_agent_binding(&id).unwrap();
    e.remove_agent_binding(&id).unwrap();
    assert!(item.agent_bindings().is_empty());
}

#[test]
fn an_unknown_inject_kind_is_kept_but_not_readable() {
    let unknown = parse_json(
        br#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"hmac"}}"#,
    )
    .unwrap();
    let mut item = api_key();
    item.edit(dev(1), at(101))
        .put("agent:future1", unknown, false)
        .unwrap();
    assert!(item.agent_bindings().is_empty());
    assert!(item.unreadable_fields().contains(&"agent:future1"));
    // It survives a merge and a round trip untouched.
    let merged = item.merge(&copy(&api_key()));
    assert!(merged.keys().any(|k| k == "agent:future1"));
    let again = copy(&merged);
    assert!(
        again
            .value("agent:future1")
            .is_some_and(|v| v.as_object().is_some())
    );
}

#[test]
fn older_builds_keep_agent_keys() {
    let mut item = api_key();
    item.edit(dev(1), at(101))
        .add_agent_binding(&record(
            r#"{"field":"api_key.secret","origins":["https://a.example"],"inject":{"kind":"bearer"},"future":{"a":[1,2]}}"#,
        ))
        .unwrap();
    let json = item.to_json().unwrap();
    let back = Item::from_json(json.expose_secret()).unwrap();
    assert_eq!(
        back.to_json().unwrap().expose_secret(),
        json.expose_secret()
    );
    let key = back.keys().find(|k| k.starts_with("agent:")).unwrap();
    assert!(back.value(key).is_some());
}

#[test]
fn agent_binding_debug_is_redacted() {
    let rec = record(BEARER);
    assert_eq!(format!("{rec:?}"), "AgentBindingRecord([REDACTED])");
}

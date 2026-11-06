//! Passkey import: Bitwarden `fido2Credentials`, CXF v1.0 and the commit of
//! the new `passkey` items. Every key is made here; each imported credential
//! signs an assertion that an independent ECDSA verifier accepts.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::assert_is_empty,
    reason = "tests"
)]

use mosaic_core::model::{ItemType, PasskeyCredential, SecretField, TextField};
use mosaic_core::{Argon2Profile, MasterPassword, VaultId};
use mosaic_import::{
    EntryWarning, Error, Format, Import, ImportWarning, ItemDraft, PasskeyDraft, commit, parse,
};
use mosaic_passkey::{CountPersisted, Es256Key, GetInput, assert, b64url};
use mosaic_vault::session::{ItemFilter, LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::pkcs8::DecodePublicKey;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CHALLENGE: [u8; 32] = [9; 32];
const ORIGIN: &str = "https://example.com";
const UUID: &str = "00010203-0405-0607-0809-0a0b0c0d0e0f";

/// PKCS#8 (RFC 8410) of an Ed25519 key: well formed, not ES256.
fn ed25519_pkcs8() -> Vec<u8> {
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend_from_slice(&[7u8; 32]);
    der
}

struct Made {
    key: Es256Key,
    pkcs8: String,
}

fn make_key() -> Made {
    let key = Es256Key::generate().unwrap();
    let pkcs8 = b64url(key.to_pkcs8_der().unwrap().expose_secret());
    Made { key, pkcs8 }
}

/// The credential signs a get assertion and the signature verifies against
/// the key it was made from; returns the authenticator data's sign count.
fn assert_and_verify(credential: &PasskeyCredential, sign_count: u64, key: &Es256Key) -> u32 {
    let pending = assert(
        &GetInput {
            rp_id: credential.rp_id.as_str(),
            origin: ORIGIN,
            challenge: &CHALLENGE,
            user_verified: false,
        },
        credential,
        sign_count,
    )
    .unwrap();
    let response = if pending.count_to_persist().is_some() {
        pending.into_response(CountPersisted::after_store_write())
    } else {
        pending.into_response_without_count().unwrap()
    };
    let verifying = VerifyingKey::from_public_key_der(&key.public_key_spki_der()).unwrap();
    let mut message = response.authenticator_data.clone();
    message.extend_from_slice(&Sha256::digest(&response.client_data_json));
    let signature = Signature::from_der(&response.signature).unwrap();
    verifying.verify(&message, &signature).unwrap();
    assert_eq!(response.user_handle, credential.user_handle);
    u32::from_be_bytes(response.authenticator_data[33..37].try_into().unwrap())
}

fn passkeys(import: &Import) -> Vec<&PasskeyDraft> {
    import
        .entries
        .iter()
        .flat_map(|e| e.draft.passkeys.iter())
        .collect()
}

fn warnings(import: &Import) -> Vec<Vec<EntryWarning>> {
    import.entries.iter().map(|e| e.warnings.clone()).collect()
}

fn bitwarden(creds: &[Value]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "encrypted": false,
        "folders": [{"id": "f1", "name": "Work"}],
        "items": [{
            "type": 1, "name": "Example", "folderId": "f1",
            "login": {"username": "alice", "password": "pw",
                      "uris": [{"uri": "https://example.com"}],
                      "fido2Credentials": creds}
        }]
    }))
    .unwrap()
}

fn fido2(made: &Made, id: &str) -> Value {
    json!({
        "credentialId": id, "keyType": "public-key", "keyAlgorithm": "ECDSA",
        "keyCurve": "P-256", "keyValue": made.pkcs8, "rpId": "example.com",
        "rpName": "Example", "userHandle": b64url(&[5; 24]),
        "userName": "al\u{202e}ice", "userDisplayName": "Alice A.",
        "counter": "41", "discoverable": "false",
        "creationDate": "2024-05-01T12:30:00.000Z"
    })
}

#[test]
fn bitwarden_passkeys_round_trip() {
    let (a, b) = (make_key(), make_key());
    let file = bitwarden(&[
        fido2(&a, UUID),
        fido2(&b, &format!("b64.{}", b64url(&[0xAB; 20]))),
    ]);
    let import = parse(Format::Bitwarden, &file, None).unwrap();
    assert_eq!(import.entries.len(), 1);
    assert!(import.entries[0].warnings.is_empty());
    // The login stays a login and carries no passkey member.
    assert_eq!(import.entries[0].draft.item_type, ItemType::Login);
    let p = passkeys(&import);
    assert_eq!(p.len(), 2);

    assert_eq!(p[0].credential.credential_id, (0..16).collect::<Vec<u8>>());
    assert_eq!(p[1].credential.credential_id, [0xAB; 20]);
    assert_eq!(p[0].title.as_str(), "Example");
    for (draft, made) in p.iter().zip([&a, &b]) {
        let c = &draft.credential;
        assert_eq!(c.rp_id.as_str(), "example.com");
        assert_eq!(c.user_handle, [5; 24]);
        // The name lost its direction override; the display name is kept.
        assert_eq!(c.user_name.as_str(), "alice");
        assert_eq!(c.user_display_name.as_str(), "Alice A.");
        assert!(!c.discoverable);
        assert_eq!(c.algorithm, -7);
        assert_eq!(c.created_at.as_unix_millis(), 1_714_566_600_000);
        // The counter is carried over: the next assertion signs 42.
        assert_eq!(draft.sign_count, 41);
        assert_eq!(assert_and_verify(c, draft.sign_count, &made.key), 42);
    }
}

#[test]
fn bitwarden_bad_passkeys_warn_and_are_left_out() {
    let m = make_key();
    let mut ed = fido2(&m, UUID);
    ed["keyValue"] = json!(b64url(&ed25519_pkcs8()));
    let mut other_alg = fido2(&m, UUID);
    other_alg["keyAlgorithm"] = json!("EdDSA");
    let mut other_curve = fido2(&m, UUID);
    other_curve["keyCurve"] = json!("P-384");
    let mut no_handle = fido2(&m, UUID);
    no_handle["userHandle"] = Value::Null;
    let mut bad_key = fido2(&m, UUID);
    bad_key["keyValue"] = json!(b64url(&[1, 2, 3, 4]));
    let mut standard_alphabet = fido2(&m, UUID);
    standard_alphabet["keyValue"] = json!(format!("{}+/", m.pkcs8));
    let bad_id = fido2(&m, "not-a-uuid");
    let mut bad_counter = fido2(&m, UUID);
    bad_counter["counter"] = json!("-1");
    let mut no_rp = fido2(&m, UUID);
    no_rp.as_object_mut().unwrap().remove("rpId");
    let mut long_handle = fido2(&m, UUID);
    long_handle["userHandle"] = json!(b64url(&[1; 65]));

    let unsupported = bitwarden(&[ed, other_alg, other_curve]);
    let import = parse(Format::Bitwarden, &unsupported, None).unwrap();
    assert!(passkeys(&import).is_empty());
    assert_eq!(warnings(&import), [vec![EntryWarning::PasskeyUnsupported]]);
    let p = import.preview(&[]);
    assert_eq!(p.warnings, [ImportWarning::PasskeyUnsupported]);

    let unreadable = bitwarden(&[
        no_handle,
        bad_key,
        standard_alphabet,
        bad_id,
        bad_counter,
        no_rp,
        long_handle,
        json!("nope"),
    ]);
    let import = parse(Format::Bitwarden, &unreadable, None).unwrap();
    assert!(passkeys(&import).is_empty());
    assert_eq!(warnings(&import), [vec![EntryWarning::PasskeyUnreadable]]);
    // The login itself is still imported.
    assert_eq!(import.entries[0].draft.title(), Some("Example"));
    assert_eq!(
        import.preview(&[]).warnings,
        [ImportWarning::PasskeyUnreadable]
    );
}

fn cxf_file(item_credentials: &[Value], version: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": version,
        "exporterRpId": "other.example", "exporterDisplayName": "Other",
        "timestamp": 1_714_566_600,
        "accounts": [{
            "id": "YQ", "username": "", "email": "", "collections": [],
            "items": [{
                "id": "aQ", "creationAt": 1_714_566_600, "modifiedAt": 1_714_566_600,
                "title": "Example", "favorite": true, "tags": ["Work"],
                "scope": {"urls": ["https://example.com/login"], "androidApps": []},
                "credentials": item_credentials
            }]
        }]
    }))
    .unwrap()
}

fn cxf_passkey(made: &Made, id: &[u8]) -> Value {
    json!({
        "type": "passkey", "credentialId": b64url(id), "rpId": "example.com",
        "username": "alice", "userDisplayName": "Alice", "userHandle": b64url(&[3; 32]),
        "key": made.pkcs8
    })
}

#[test]
fn cxf_passkey_and_password() {
    let m = make_key();
    let file = cxf_file(
        &[
            cxf_passkey(&m, &[8; 16]),
            json!({"type": "basic-auth",
                   "username": {"fieldType": "string", "value": "alice"},
                   "password": {"fieldType": "concealed-string", "value": "hunter2"}}),
            json!({"type": "totp", "secret": "x"}),
        ],
        &json!({"major": 1, "minor": 0}),
    );
    let import = parse(Format::Cxf, &file, None).unwrap();
    assert_eq!(import.format, Format::Cxf);
    assert_eq!(import.entries.len(), 1);
    let d: &ItemDraft = &import.entries[0].draft;
    assert_eq!(d.item_type, ItemType::Login);
    assert_eq!(d.title(), Some("Example"));
    assert_eq!(d.urls[0].as_str(), "https://example.com/login");
    assert!(d.favorite);
    assert_eq!(d.tags[0].as_str(), "Work");
    assert_eq!(
        d.secret(SecretField::Password).unwrap().expose_secret(),
        "hunter2"
    );
    assert_eq!(import.entries[0].warnings, [EntryWarning::UnmappedFields]);
    let p = &d.passkeys[0];
    assert_eq!(p.credential.credential_id, [8; 16]);
    assert_eq!(p.credential.created_at.as_unix_millis(), 1_714_566_600_000);
    assert_eq!(p.sign_count, 0);
    assert!(p.credential.discoverable);
    assert_eq!(assert_and_verify(&p.credential, 0, &m.key), 0);
}

#[test]
fn cxf_passkey_only_item_and_errors() {
    let m = make_key();
    let ok = cxf_file(
        &[cxf_passkey(&m, &[8; 16])],
        &json!({"major": 1, "minor": 3}),
    );
    let import = parse(Format::Cxf, &ok, None).unwrap();
    // An item of passkeys only makes no item of its own.
    let d = &import.entries[0].draft;
    assert_eq!(d.item_type, ItemType::Passkey);
    assert!(!d.has_item());
    let preview = import.preview(&[]);
    assert_eq!(preview.counts, [(ItemType::Passkey, 1)]);
    assert_eq!(preview.entries[0].item_type, ItemType::Passkey);

    for version in [json!({"major": 2, "minor": 0}), json!({"major": 0})] {
        assert_eq!(
            parse(Format::Cxf, &cxf_file(&[], &version), None).unwrap_err(),
            Error::UnsupportedVersion
        );
    }
    for bad in [
        &b"{}"[..],
        b"[]",
        br#"{"version":{"major":"1"},"accounts":[]}"#,
        br#"{"version":{"major":1},"accounts":{}}"#,
        b"\xff\xfe",
        b"",
    ] {
        assert_eq!(parse(Format::Cxf, bad, None).unwrap_err(), Error::Malformed);
    }

    // Bad members: unreadable; another key type: unsupported.
    let mut ed = cxf_passkey(&m, &[8; 16]);
    ed["key"] = json!(b64url(&ed25519_pkcs8()));
    let mut no_handle = cxf_passkey(&m, &[8; 16]);
    no_handle.as_object_mut().unwrap().remove("userHandle");
    let mut bad_key = cxf_passkey(&m, &[8; 16]);
    bad_key["key"] = json!("%%%");
    let mut empty_id = cxf_passkey(&m, &[1]);
    empty_id["credentialId"] = json!("");
    let mixed = cxf_file(
        &[ed, no_handle, bad_key, empty_id],
        &json!({"major": 1, "minor": 0}),
    );
    let import = parse(Format::Cxf, &mixed, None).unwrap();
    assert_eq!(
        warnings(&import),
        [vec![
            EntryWarning::PasskeyUnreadable,
            EntryWarning::PasskeyUnsupported
        ]]
    );
    assert!(passkeys(&import).is_empty());

    let note_only = cxf_file(
        &[json!({"type": "note", "content": {"fieldType": "string", "value": "remember"}})],
        &json!({"major": 1, "minor": 0}),
    );
    let import = parse(Format::Cxf, &note_only, None).unwrap();
    assert_eq!(import.entries[0].draft.item_type, ItemType::SecureNote);
    assert_eq!(
        import.entries[0].draft.text(TextField::Notes),
        Some("remember")
    );
}

fn session(store: Store) -> (UnlockedSession, VaultId) {
    let (mut s, _) = LockedSession::open(store)
        .create_account(
            &MasterPassword::new("passkey import".to_owned()),
            Argon2Profile::Baseline,
        )
        .unwrap();
    let vault = s.create_vault("Imported").unwrap();
    (s, vault)
}

#[test]
fn commit_writes_each_passkey_as_its_own_item() {
    let (a, b) = (make_key(), make_key());
    let file = bitwarden(&[
        fido2(&a, UUID),
        fido2(&b, &format!("b64.{}", b64url(&[0xAB; 20]))),
    ]);
    let (mut s, vault) = session(Store::open_in_memory().unwrap());
    let import = parse(Format::Bitwarden, &file, None).unwrap();
    let preview = import.preview(&s.items(&ItemFilter::default()));
    assert_eq!(
        preview.counts,
        [(ItemType::Login, 1), (ItemType::Passkey, 2)]
    );
    let report = commit(import, &mut s, vault, &[]).unwrap();
    assert_eq!((report.imported, report.failed), (3, 0));

    let items = s.items(&ItemFilter::default());
    assert_eq!(items.len(), 3);
    let login = items
        .iter()
        .find(|r| r.item.item_type() == Some(ItemType::Login))
        .unwrap();
    assert!(login.item.passkey().is_none());
    let stored: Vec<_> = items
        .iter()
        .filter(|r| r.item.item_type() == Some(ItemType::Passkey))
        .collect();
    assert_eq!(stored.len(), 2);
    for r in &stored {
        let credential = r.item.passkey().unwrap();
        assert_eq!(r.item.title(), Some("Example"));
        assert_eq!(r.item.passkey_sign_count(), 41);
        assert!(r.item.tags().contains(&"Work"));
        let made = if credential.credential_id.len() == 16 {
            &a
        } else {
            &b
        };
        assert_eq!(
            assert_and_verify(&credential, r.item.passkey_sign_count(), &made.key),
            42
        );
    }

    // Importing the same credential as a CXF passkey item: a duplicate.
    let again = cxf_file(
        &[cxf_passkey(&a, &(0..16).collect::<Vec<u8>>())],
        &json!({"major": 1, "minor": 0}),
    );
    let import = parse(Format::Cxf, &again, None).unwrap();
    let preview = import.preview(&s.items(&ItemFilter::default()));
    assert!(preview.entries[0].duplicate_of.is_some());
    let fresh = cxf_file(
        &[cxf_passkey(&a, &[1; 16])],
        &json!({"major": 1, "minor": 0}),
    );
    let import = parse(Format::Cxf, &fresh, None).unwrap();
    let preview = import.preview(&s.items(&ItemFilter::default()));
    assert!(preview.entries[0].duplicate_of.is_none());
    let report = commit(import, &mut s, vault, &[]).unwrap();
    assert_eq!(report.imported, 1);
}

/// The fuzz seeds with a passkey are real, importable files.
#[test]
fn passkey_fuzz_seeds_import() {
    let seed = |name: &str| {
        std::fs::read(format!("{}/fuzz/seeds/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    };
    let cxf = parse(Format::Cxf, &seed("cxf/export"), None).unwrap();
    assert_eq!(cxf.entries.len(), 2);
    assert_eq!(passkeys(&cxf).len(), 1);
    assert!(cxf.entries.iter().all(|e| e.warnings.is_empty()));
    let bw = parse(Format::Bitwarden, &seed("bitwarden_json/passkey"), None).unwrap();
    assert_eq!(passkeys(&bw)[0].sign_count, 41);
    assert!(bw.entries[0].warnings.is_empty());
}

//! Passkeys: Android Credential Manager and iOS, in both client-data modes,
//! with every response checked by an independent parser and a P-256 verifier.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mosaic_mobile::{
    AndroidCaller, AndroidPasskeyRequest, IosAssertRequest, IosRegisterRequest, ItemFilter,
    ItemKind, MobileError, PasskeyCandidate, SettingsUpdate,
};
use mosaic_passkey::{b64url, from_b64url, rp_id_hash};
use mosaic_sync::TransportError;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::pkcs8::DecodePublicKey;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::common::{PASSWORD, Phone};

const CERT: [u8; 32] = [0x11; 32];
const PACKAGE: &str = "com.example.app";
const CHALLENGE: [u8; 32] = [7; 32];
const USER_HANDLE: [u8; 8] = [4; 8];

fn caller(package: &str, cert: [u8; 32]) -> AndroidCaller {
    AndroidCaller {
        package: package.to_owned(),
        current_signers: vec![cert.to_vec()],
        web_domain: None,
        web_scheme: None,
    }
}

fn get_json(rp_id: &str, allow: &[&[u8]]) -> String {
    let allow: Vec<String> = allow
        .iter()
        .map(|id| format!("{{\"type\":\"public-key\",\"id\":\"{}\"}}", b64url(id)))
        .collect();
    format!(
        "{{\"challenge\":\"{}\",\"rpId\":\"{rp_id}\",\"allowCredentials\":[{}],\"userVerification\":\"required\"}}",
        b64url(&CHALLENGE),
        allow.join(",")
    )
}

fn create_json(rp_id: &str, exclude: &[&[u8]]) -> String {
    let exclude: Vec<String> = exclude
        .iter()
        .map(|id| format!("{{\"type\":\"public-key\",\"id\":\"{}\"}}", b64url(id)))
        .collect();
    format!(
        "{{\"challenge\":\"{}\",\"rp\":{{\"id\":\"{rp_id}\",\"name\":\"Example\"}},\
         \"user\":{{\"id\":\"{}\",\"name\":\"alice@example.com\",\"displayName\":\"Alice\"}},\
         \"pubKeyCredParams\":[{{\"type\":\"public-key\",\"alg\":-7}}],\
         \"excludeCredentials\":[{}]}}",
        b64url(&CHALLENGE),
        b64url(&USER_HANDLE),
        exclude.join(",")
    )
}

fn request(json: String, caller: AndroidCaller) -> AndroidPasskeyRequest {
    AndroidPasskeyRequest {
        request_json: json,
        caller,
        platform_origin: None,
        client_data_hash: None,
    }
}

fn assetlinks(package: &str, cert: &[u8; 32]) -> Vec<u8> {
    let fingerprint = cert
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");
    format!(
        "[{{\"relation\":[\"delegate_permission/common.get_login_creds\"],\
         \"target\":{{\"namespace\":\"android_app\",\"package_name\":\"{package}\",\
         \"sha256_cert_fingerprints\":[\"{fingerprint}\"]}}}}]"
    )
    .into_bytes()
}

/// A phone whose asset-links answer delegates `PACKAGE` for every host.
fn phone() -> Phone {
    let (phone, _) = Phone::android_with_account();
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks(PACKAGE, &CERT)));
    phone
}

/// A trusted browser from Rust's own allowlist.
fn browser() -> AndroidCaller {
    let phone = Phone::new();
    let list: Value = serde_json::from_str(&phone.core.privileged_browsers_json()).unwrap();
    let info = &list["apps"][0]["info"];
    let cert: Vec<u8> = info["signatures"][0]["cert_fingerprint_sha256"]
        .as_str()
        .unwrap()
        .split(':')
        .map(|h| u8::from_str_radix(h, 16).unwrap())
        .collect();
    AndroidCaller {
        package: info["package_name"].as_str().unwrap().to_owned(),
        current_signers: vec![cert],
        web_domain: None,
        web_scheme: None,
    }
}

fn first_vault(phone: &Phone) -> String {
    phone.vault_id()
}

/// Registers a passkey for `example.com` through the app path and returns the
/// registration response and the credential ID.
fn register(phone: &Phone) -> (Value, Vec<u8>) {
    let session = phone
        .core
        .passkey_android_create_begin(request(
            create_json("example.com", &[]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    let json = phone
        .core
        .passkey_android_create_finish(session.session_id, first_vault(phone), true)
        .unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    let id = from_b64url(value["rawId"].as_str().unwrap(), 1023).unwrap();
    (value, id)
}

fn field(value: &Value, path: &[&str]) -> Vec<u8> {
    let mut v = value;
    for key in path {
        v = &v[*key];
    }
    from_b64url(v.as_str().unwrap(), 4096).unwrap()
}

#[test]
fn passkey_android_app_requires_asset_links_delegation() {
    let (phone, _) = Phone::android_with_account();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    phone.core.test_hooks().set_asset_links_fetch(move |host| {
        assert_eq!(host, "example.com");
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(assetlinks(PACKAGE, &CERT))
    });
    let begin = |package: &str, cert: [u8; 32]| {
        phone.core.passkey_android_create_begin(request(
            create_json("example.com", &[]),
            caller(package, cert),
        ))
    };
    // The delegated app works, and its client data names the app's key hash.
    let session = begin(PACKAGE, CERT).unwrap();
    assert_eq!(session.rp_id, "example.com");
    assert_eq!(session.rp_name, "Example");
    assert_eq!(session.user_name, "alice@example.com");
    assert_eq!(session.vaults.len(), 1);
    let json = phone
        .core
        .passkey_android_create_finish(session.session_id, first_vault(&phone), true)
        .unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    let client_data: Value =
        serde_json::from_slice(&field(&value, &["response", "clientDataJSON"])).unwrap();
    assert_eq!(client_data["type"], "webauthn.create");
    assert_eq!(client_data["challenge"], b64url(&CHALLENGE));
    assert_eq!(
        client_data["origin"],
        format!("android:apk-key-hash:{}", b64url(&CERT))
    );
    // The answer is cached: a second ceremony does not fetch again.
    begin(PACKAGE, CERT).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Another app, another signer: the file does not list them.
    assert_eq!(
        begin("com.evil.app", CERT).err(),
        Some(MobileError::AppNotVerified)
    );
    assert_eq!(
        begin(PACKAGE, [0x99; 32]).err(),
        Some(MobileError::AppNotVerified)
    );
    // No file: nothing is delegated. No network: it says so, and does not
    // pretend to know.
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Err(TransportError::NotFound));
    assert_eq!(
        begin(PACKAGE, CERT).err(),
        Some(MobileError::AppNotVerified)
    );
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Err(TransportError::Unreachable));
    assert_eq!(
        begin(PACKAGE, CERT).err(),
        Some(MobileError::AssetLinksUnavailable)
    );
    // A bad rpId or a missing one.
    let mut other = request(create_json("Not A Host", &[]), caller(PACKAGE, CERT));
    assert_eq!(
        phone.core.passkey_android_create_begin(other.clone()).err(),
        Some(MobileError::RpIdInvalid)
    );
    other.request_json = format!(
        "{{\"challenge\":\"{}\",\"user\":{{\"id\":\"{}\",\"name\":\"a\"}}}}",
        b64url(&CHALLENGE),
        b64url(&USER_HANDLE)
    );
    assert_eq!(
        phone.core.passkey_android_create_begin(other).err(),
        Some(MobileError::RpIdInvalid)
    );
}

#[test]
fn passkey_android_privileged_browser_hash_mode() {
    let phone = phone();
    let hash = Sha256::digest(b"the browser's clientDataJSON").to_vec();
    let mut create = request(create_json("example.com", &[]), browser());
    create.platform_origin = Some("https://login.example.com".to_owned());
    create.client_data_hash = Some(hash.clone());
    let session = phone.core.passkey_android_create_begin(create).unwrap();
    let json = phone
        .core
        .passkey_android_create_finish(session.session_id, first_vault(&phone), true)
        .unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    // The platform built the client data, so the response has none.
    assert!(value["response"].get("clientDataJSON").is_none());
    let spki = field(&value, &["response", "publicKey"]);
    let key = VerifyingKey::from_public_key_der(&spki).unwrap();
    // An assertion in hash mode signs authenticatorData || the platform's hash.
    let mut get = request(get_json("example.com", &[]), browser());
    get.platform_origin = Some("https://login.example.com".to_owned());
    get.client_data_hash = Some(hash.clone());
    let session = phone.core.passkey_android_get_begin(get).unwrap();
    assert_eq!(session.candidates.len(), 1);
    let json = phone
        .core
        .passkey_android_get_finish(session.session_id, session.candidates[0].key.clone(), true)
        .unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();
    assert!(value["response"].get("clientDataJSON").is_none());
    let auth = field(&value, &["response", "authenticatorData"]);
    let signature = Signature::from_der(&field(&value, &["response", "signature"])).unwrap();
    let mut message = auth.clone();
    message.extend_from_slice(&hash);
    key.verify(&message, &signature).unwrap();
    assert_eq!(&auth[..32], rp_id_hash("example.com"));
    // A hash that is not 32 bytes is refused.
    let mut bad = request(get_json("example.com", &[]), browser());
    bad.platform_origin = Some("https://login.example.com".to_owned());
    bad.client_data_hash = Some(vec![1, 2, 3]);
    assert_eq!(
        phone.core.passkey_android_get_begin(bad).err(),
        Some(MobileError::InvalidInput)
    );
    // And a hash without a platform origin is not a thing an app can send.
    let mut app = request(get_json("example.com", &[]), caller(PACKAGE, CERT));
    app.client_data_hash = Some(hash);
    assert_eq!(
        phone.core.passkey_android_get_begin(app).err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn passkey_android_untrusted_platform_origin_is_refused() {
    let phone = phone();
    // An ordinary app claims a page origin: refused by Rust's own allowlist.
    let mut create = request(create_json("example.com", &[]), caller(PACKAGE, CERT));
    create.platform_origin = Some("https://example.com".to_owned());
    assert_eq!(
        phone.core.passkey_android_create_begin(create).err(),
        Some(MobileError::AppNotVerified)
    );
    let mut get = request(get_json("example.com", &[]), caller(PACKAGE, CERT));
    get.platform_origin = Some("https://example.com".to_owned());
    assert_eq!(
        phone.core.passkey_android_get_begin(get).err(),
        Some(MobileError::AppNotVerified)
    );
    // The right package of a browser with another signer is no browser.
    let mut spoof = browser();
    spoof.current_signers = vec![vec![0x42; 32]];
    let mut create = request(create_json("example.com", &[]), spoof);
    create.platform_origin = Some("https://example.com".to_owned());
    assert_eq!(
        phone.core.passkey_android_create_begin(create).err(),
        Some(MobileError::AppNotVerified)
    );
}

#[test]
fn passkey_android_rp_id_validated_against_origin() {
    let phone = phone();
    let begin = |origin: &str, rp: &str| {
        let mut r = request(get_json(rp, &[]), browser());
        r.platform_origin = Some(origin.to_owned());
        r.client_data_hash = Some(vec![0x33; 32]);
        phone.core.passkey_android_get_begin(r)
    };
    // The origin's host and a registrable suffix of it are fine.
    assert_eq!(
        begin("https://login.example.com", "example.com")
            .unwrap()
            .rp_id,
        "example.com"
    );
    assert_eq!(
        begin("https://login.example.com", "login.example.com")
            .unwrap()
            .rp_id,
        "login.example.com"
    );
    // Another site, a public suffix, an unrelated host.
    for rp in ["evil.com", "com", "example.com.evil.test", "ample.com"] {
        assert_eq!(
            begin("https://login.example.com", rp).err(),
            Some(MobileError::RpIdInvalid),
            "{rp}"
        );
    }
    // An origin that is not an origin.
    assert_eq!(
        begin("https://login.example.com/path", "example.com").err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        begin("android:apk-key-hash:abc", "example.com").err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn passkey_android_get_response_json_parses_independently() {
    let phone = phone();
    let (registered, credential_id) = register(&phone);
    let key =
        VerifyingKey::from_public_key_der(&field(&registered, &["response", "publicKey"])).unwrap();
    let session = phone
        .core
        .passkey_android_get_begin(request(
            get_json("example.com", &[&credential_id]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    assert_eq!(session.rp_id, "example.com");
    assert_eq!(session.candidates.len(), 1);
    let PasskeyCandidate {
        key: item,
        user_name,
        rp_id,
        ..
    } = session.candidates[0].clone();
    assert_eq!(
        (user_name.as_str(), rp_id.as_str()),
        ("alice@example.com", "example.com")
    );
    let json = phone
        .core
        .passkey_android_get_finish(session.session_id, item, true)
        .unwrap();

    let value: Value = serde_json::from_str(&json).unwrap();
    let id = value["id"].as_str().unwrap();
    assert_eq!(value["rawId"], value["id"]);
    assert_eq!(from_b64url(id, 1023).unwrap(), credential_id);
    assert_eq!(value["type"], "public-key");
    assert_eq!(value["authenticatorAttachment"], "platform");
    assert_eq!(value["clientExtensionResults"], serde_json::json!({}));
    assert!(!json.contains('='), "unpadded base64url");
    // Every member of the response, decoded.
    let client_data_bytes = field(&value, &["response", "clientDataJSON"]);
    let client_data: Value = serde_json::from_slice(&client_data_bytes).unwrap();
    assert_eq!(client_data["type"], "webauthn.get");
    assert_eq!(client_data["challenge"], b64url(&CHALLENGE));
    assert_eq!(
        client_data["origin"],
        format!("android:apk-key-hash:{}", b64url(&CERT))
    );
    let auth = field(&value, &["response", "authenticatorData"]);
    assert_eq!(auth.len(), 37);
    assert_eq!(&auth[..32], rp_id_hash("example.com"));
    assert_eq!(
        auth[32], 0x1d,
        "UP, UV, backup eligible and backed up (a synced passkey)"
    );
    assert_eq!(field(&value, &["response", "userHandle"]), USER_HANDLE);
    // The signature verifies over authenticatorData || SHA-256(clientDataJSON).
    let signature = Signature::from_der(&field(&value, &["response", "signature"])).unwrap();
    let mut message = auth;
    message.extend_from_slice(&Sha256::digest(&client_data_bytes));
    key.verify(&message, &signature).unwrap();
}

#[test]
fn passkey_android_create_response_json_parses_independently() {
    let phone = phone();
    let (value, credential_id) = register(&phone);
    assert_eq!(value["type"], "public-key");
    assert_eq!(value["rawId"], value["id"]);
    assert_eq!(value["authenticatorAttachment"], "platform");
    assert_eq!(value["response"]["publicKeyAlgorithm"], -7);
    assert_eq!(
        value["response"]["transports"],
        serde_json::json!(["internal", "hybrid"])
    );
    let client_data: Value =
        serde_json::from_slice(&field(&value, &["response", "clientDataJSON"])).unwrap();
    assert_eq!(client_data["type"], "webauthn.create");
    let auth = field(&value, &["response", "authenticatorData"]);
    assert_eq!(&auth[..32], rp_id_hash("example.com"));
    assert_eq!(
        auth[32], 0x5d,
        "UP, UV, BE, BS and attested credential data"
    );
    assert_eq!(&auth[33..37], [0, 0, 0, 0], "the sign count starts at 0");
    assert_eq!(&auth[37..53], [0u8; 16], "the AAGUID is zero");
    assert_eq!(
        usize::from(u16::from_be_bytes([auth[53], auth[54]])),
        credential_id.len()
    );
    assert_eq!(
        &auth[55..55 + credential_id.len()],
        credential_id.as_slice()
    );
    // The attestation object is CBOR {"fmt":"none","attStmt":{},"authData":...}.
    let attestation = field(&value, &["response", "attestationObject"]);
    let text = |needle: &[u8]| attestation.windows(needle.len()).any(|w| w == needle);
    assert!(text(b"fmt") && text(b"none") && text(b"attStmt") && text(b"authData"));
    assert!(attestation.ends_with(&auth));
    // The public key is a P-256 SPKI.
    VerifyingKey::from_public_key_der(&field(&value, &["response", "publicKey"])).unwrap();
}

#[test]
fn passkey_create_stores_item_before_response() {
    let phone = phone();
    let begin = || {
        phone
            .core
            .passkey_android_create_begin(request(
                create_json("example.com", &[]),
                caller(PACKAGE, CERT),
            ))
            .unwrap()
    };
    // A vault that does not exist: no response, and nothing stored.
    let session = begin();
    assert_eq!(
        phone
            .core
            .passkey_android_create_finish(
                session.session_id,
                "00000000-0000-7000-8000-000000000001".to_owned(),
                true
            )
            .err(),
        Some(MobileError::NotFound)
    );
    assert!(phone.core.items(ItemFilter::default()).unwrap().is_empty());
    // A response exists only with a stored item.
    let (_, credential_id) = register(&phone);
    let items = phone.core.items(ItemFilter::default()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, ItemKind::Passkey);
    assert!(items[0].has_passkey);
    assert_eq!(items[0].title, "example.com");
    assert_eq!(items[0].subtitle.as_deref(), Some("alice@example.com"));
    let identities = phone.core.passkey_identities().unwrap();
    assert_eq!(identities[0].credential_id, credential_id);
}

#[test]
fn passkey_create_excluded_credential_refused() {
    let phone = phone();
    let (_, credential_id) = register(&phone);
    assert_eq!(
        phone
            .core
            .passkey_android_create_begin(request(
                create_json("example.com", &[&credential_id]),
                caller(PACKAGE, CERT),
            ))
            .err(),
        Some(MobileError::PasskeyExcluded)
    );
    // Another site's identical credential ID is no reason to refuse, and an
    // unrelated ID is none either.
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks(PACKAGE, &CERT)));
    phone
        .core
        .passkey_android_create_begin(request(
            create_json("example.com", &[&[1, 2, 3]]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    phone
        .core
        .passkey_android_create_begin(request(
            create_json("other.example", &[&credential_id]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    // No supported algorithm.
    let rsa_only = create_json("example.com", &[]).replace("\"alg\":-7", "\"alg\":-257");
    assert_eq!(
        phone
            .core
            .passkey_android_create_begin(request(rsa_only, caller(PACKAGE, CERT)))
            .err(),
        Some(MobileError::PasskeyUnsupportedAlgorithm)
    );
}

/// The sign count of the only passkey of the store, read with the vault crate
/// after the core let go of the store.
fn stored_sign_count(phone: &Phone, secret_key: &str) -> u64 {
    use mosaic_vault::session::{ItemFilter as VaultFilter, LockedSession};
    phone.core.close_store();
    let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
    let session = LockedSession::open(store)
        .unlock(
            &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
            &mosaic_core::SecretKey::parse(secret_key).unwrap(),
        )
        .unwrap();
    let count = session
        .items(&VaultFilter::default())
        .iter()
        .find_map(|r| r.item.passkey_public().map(|_| r.item.passkey_sign_count()))
        .unwrap();
    drop(session);
    phone.core.reopen_store().unwrap();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    count
}

#[test]
fn passkey_assert_persists_sign_count_before_release() {
    let (phone, secret_key) = Phone::android_with_account();
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks(PACKAGE, &CERT)));
    let (_, credential_id) = register(&phone);
    // Give the credential a counter above 0 (a relying party that checks it).
    phone.core.close_store();
    {
        use mosaic_vault::session::{ItemFilter as VaultFilter, LockedSession};
        let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
        let mut session = LockedSession::open(store)
            .unlock(
                &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::SecretKey::parse(&secret_key).unwrap(),
            )
            .unwrap();
        let (vault, item) = session
            .items(&VaultFilter::default())
            .iter()
            .map(|r| (r.vault_id, r.item_id))
            .next()
            .unwrap();
        session
            .update_item(vault, item, |e| e.set_passkey_sign_count(41))
            .unwrap();
    }
    phone.core.reopen_store().unwrap();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let assert_once = || {
        let session = phone
            .core
            .passkey_android_get_begin(request(
                get_json("example.com", &[&credential_id]),
                caller(PACKAGE, CERT),
            ))
            .unwrap();
        let json = phone
            .core
            .passkey_android_get_finish(session.session_id, session.candidates[0].key.clone(), true)
            .unwrap();
        let value: Value = serde_json::from_str(&json).unwrap();
        let auth = field(&value, &["response", "authenticatorData"]);
        u32::from_be_bytes([auth[33], auth[34], auth[35], auth[36]])
    };
    // The counter in the response is the one that was stored first.
    assert_eq!(assert_once(), 42);
    assert_eq!(stored_sign_count(&phone, &secret_key), 42);
    assert_eq!(assert_once(), 43);
    assert_eq!(stored_sign_count(&phone, &secret_key), 43);
}

#[test]
fn passkey_refuses_without_user_verification() {
    let phone = phone();
    let session = phone
        .core
        .passkey_android_create_begin(request(
            create_json("example.com", &[]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_create_finish(session.session_id.clone(), first_vault(&phone), false)
            .err(),
        Some(MobileError::ReauthRequired)
    );
    assert!(phone.core.items(ItemFilter::default()).unwrap().is_empty());
    // The refusal spends nothing: the ceremony still finishes with verification.
    phone
        .core
        .passkey_android_create_finish(session.session_id, first_vault(&phone), true)
        .unwrap();
    let session = phone
        .core
        .passkey_android_get_begin(request(get_json("example.com", &[]), caller(PACKAGE, CERT)))
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(
                session.session_id,
                session.candidates[0].key.clone(),
                false
            )
            .err(),
        Some(MobileError::ReauthRequired)
    );
}

#[test]
fn passkey_ios_register_hash_mode() {
    let (phone, _) = Phone::with_account();
    let hash = Sha256::digest(b"ios client data").to_vec();
    let registration = |rp: &str, exclude: Vec<Vec<u8>>, algorithms: Vec<i64>| {
        phone.core.passkey_ios_register(IosRegisterRequest {
            rp_id: rp.to_owned(),
            user_name: "alice@example.com".to_owned(),
            user_handle: USER_HANDLE.to_vec(),
            client_data_hash: hash.clone(),
            algorithms,
            exclude_credential_ids: exclude,
            vault_id: phone.vault_id(),
            user_verified: true,
        })
    };
    let made = registration("example.com", Vec::new(), vec![-7, -257]).unwrap();
    assert_eq!(made.credential_id.len(), 16);
    // The attestation object carries authData with the credential in it.
    let text = |needle: &[u8]| {
        made.attestation_object
            .windows(needle.len())
            .any(|w| w == needle)
    };
    assert!(text(b"none") && text(&made.credential_id));
    assert!(text(&rp_id_hash("example.com")));
    assert_eq!(phone.core.passkey_identities().unwrap().len(), 1);
    // Excluded, unsupported, a bad rpId, a bad hash.
    assert_eq!(
        registration("example.com", vec![made.credential_id.clone()], Vec::new()).err(),
        Some(MobileError::PasskeyExcluded)
    );
    assert_eq!(
        registration("example.com", Vec::new(), vec![-257]).err(),
        Some(MobileError::PasskeyUnsupportedAlgorithm)
    );
    assert_eq!(
        registration("Not A Host", Vec::new(), Vec::new()).err(),
        Some(MobileError::RpIdInvalid)
    );
    let mut bad = IosRegisterRequest {
        rp_id: "example.com".to_owned(),
        user_name: "a".to_owned(),
        user_handle: USER_HANDLE.to_vec(),
        client_data_hash: vec![1; 31],
        algorithms: Vec::new(),
        exclude_credential_ids: Vec::new(),
        vault_id: phone.vault_id(),
        user_verified: true,
    };
    assert_eq!(
        phone.core.passkey_ios_register(bad.clone()).err(),
        Some(MobileError::InvalidInput)
    );
    bad.client_data_hash = hash;
    bad.user_handle = Vec::new();
    assert_eq!(
        phone.core.passkey_ios_register(bad).err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn passkey_ios_assert_hash_mode() {
    let (phone, _) = Phone::with_account();
    let register_hash = Sha256::digest(b"register").to_vec();
    let made = phone
        .core
        .passkey_ios_register(IosRegisterRequest {
            rp_id: "example.com".to_owned(),
            user_name: "alice@example.com".to_owned(),
            user_handle: USER_HANDLE.to_vec(),
            client_data_hash: register_hash,
            algorithms: vec![-7],
            exclude_credential_ids: Vec::new(),
            vault_id: phone.vault_id(),
            user_verified: true,
        })
        .unwrap();
    // The public key from the attestation object's authData (COSE: x at 0x21,
    // y at 0x22 after their CBOR headers).
    let auth_at = made
        .attestation_object
        .windows(32)
        .position(|w| w == rp_id_hash("example.com"))
        .unwrap();
    let cose = &made.attestation_object[auth_at + 55 + made.credential_id.len()..];
    let x = &cose[10..42];
    let y = &cose[45..77];
    let mut sec1 = vec![0x04];
    sec1.extend_from_slice(x);
    sec1.extend_from_slice(y);
    let key = VerifyingKey::from_sec1_bytes(&sec1).unwrap();

    let candidates = phone
        .core
        .passkey_ios_candidates("example.com".to_owned(), vec![made.credential_id.clone()])
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(
        phone
            .core
            .passkey_ios_candidates("example.com".to_owned(), vec![vec![9, 9]])
            .unwrap()
            .is_empty()
    );
    assert!(
        phone
            .core
            .passkey_ios_candidates("other.example".to_owned(), Vec::new())
            .unwrap()
            .is_empty()
    );
    let hash = Sha256::digest(b"the OS's clientDataJSON").to_vec();
    let assertion = phone
        .core
        .passkey_ios_assert(IosAssertRequest {
            rp_id: "example.com".to_owned(),
            client_data_hash: hash.clone(),
            credential_id: made.credential_id.clone(),
            user_verified: true,
        })
        .unwrap();
    assert_eq!(assertion.credential_id, made.credential_id);
    assert_eq!(assertion.user_handle, USER_HANDLE);
    assert_eq!(assertion.authenticator_data.len(), 37);
    assert_eq!(
        &assertion.authenticator_data[..32],
        rp_id_hash("example.com")
    );
    assert_eq!(assertion.authenticator_data[32], 0x1d);
    let signature = Signature::from_der(&assertion.signature).unwrap();
    let mut message = assertion.authenticator_data.clone();
    message.extend_from_slice(&hash);
    key.verify(&message, &signature).unwrap();
    // Not stored, not this site, bad hash.
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: hash.clone(),
                credential_id: vec![1, 2, 3],
                user_verified: true,
            })
            .err(),
        Some(MobileError::PasskeyNotFound)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "other.example".to_owned(),
                client_data_hash: hash,
                credential_id: made.credential_id.clone(),
                user_verified: true,
            })
            .err(),
        Some(MobileError::PasskeyNotFound)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: vec![0; 5],
                credential_id: made.credential_id,
                user_verified: true,
            })
            .err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn passkey_sessions_expire_and_drop_on_lock() {
    let phone = phone();
    let begin = || {
        phone
            .core
            .passkey_android_get_begin(request(get_json("example.com", &[]), caller(PACKAGE, CERT)))
            .unwrap()
    };
    let (_, _) = register(&phone);
    // Expires after five minutes.
    let session = begin();
    phone.core.test_hooks().set_clock_offset_ms(301_000);
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(
                session.session_id.clone(),
                session.candidates[0].key.clone(),
                true
            )
            .err(),
        Some(MobileError::PasskeySessionExpired)
    );
    phone.core.test_hooks().set_clock_offset_ms(0);
    // A lock drops it.
    let session = begin();
    phone.core.lock();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(
                session.session_id.clone(),
                session.candidates[0].key.clone(),
                true
            )
            .err(),
        Some(MobileError::Locked)
    );
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(session.session_id, session.candidates[0].key.clone(), true)
            .err(),
        Some(MobileError::PasskeySessionExpired)
    );
    // A session is used once; a create session does not finish an assertion.
    let session = begin();
    phone
        .core
        .passkey_android_get_finish(
            session.session_id.clone(),
            session.candidates[0].key.clone(),
            true,
        )
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(session.session_id, session.candidates[0].key.clone(), true)
            .err(),
        Some(MobileError::PasskeySessionExpired)
    );
    let create = phone
        .core
        .passkey_android_create_begin(request(
            create_json("example.com", &[]),
            caller(PACKAGE, CERT),
        ))
        .unwrap();
    let candidate = begin().candidates[0].key.clone();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(create.session_id, candidate, true)
            .err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn passkey_identities_carry_no_key_material() {
    let (phone, _) = Phone::android_with_account();
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks(PACKAGE, &CERT)));
    let (_, credential_id) = register(&phone);
    let identities = phone.core.passkey_identities().unwrap();
    assert_eq!(identities.len(), 1);
    let identity = &identities[0];
    assert_eq!(identity.rp_id, "example.com");
    assert_eq!(identity.user_name, "alice@example.com");
    assert_eq!(identity.credential_id, credential_id);
    assert_eq!(identity.user_handle, USER_HANDLE);
    assert_eq!(identity.record.vault_id, phone.vault_id());
    // The login without a passkey is not an identity, and passwords only
    // appear with the OD-M10-7 setting.
    phone
        .core
        .save_login(mosaic_mobile::LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: "Example".to_owned(),
            username: "alice".to_owned(),
            password: Some(b"pw".to_vec()),
            urls: vec![mosaic_mobile::UrlDraft {
                url: "https://example.com/login".to_owned(),
                match_etld1: false,
                allow_http: false,
            }],
            notes: None,
            otp: mosaic_mobile::OtpEdit::Keep,
            favorite: false,
        })
        .unwrap();
    assert_eq!(phone.core.passkey_identities().unwrap().len(), 1);
    assert!(phone.core.password_identities().unwrap().is_empty());
    let mut update = SettingsUpdate::default();
    update.password_identities = Some(true);
    update.reauth_password = Some(PASSWORD.as_bytes().to_vec());
    phone.core.update_settings(update).unwrap();
    let passwords = phone.core.password_identities().unwrap();
    assert_eq!(passwords.len(), 1);
    assert_eq!(passwords[0].domain, "example.com");
    assert_eq!(passwords[0].user_name, "alice");
    assert!(!format!("{passwords:?}").contains("\"pw\""));
}

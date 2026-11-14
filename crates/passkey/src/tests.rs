//! Known-answer tests V1 to V11 (CRYPTO_SPEC §16), round trips, limits and the
//! pinned fixture shared with the independent TypeScript verifier.

use hex_literal::hex;
use mosaic_core::SecretVec;
use mosaic_core::model::{PasskeyCredential, Text, Timestamp};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use proptest::prelude::*;
use sha2::{Digest, Sha256};

use super::*;
use crate::ceremony::{create_request_with, create_with};
use crate::encode::{authenticator_data, base_flags};

/// RFC 6979 A.2.5 P-256 key (the same key as the ssh-agent known-answer test).
const D: [u8; 32] = hex!("C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721");
const X: [u8; 32] = hex!("60FED4BA255A9D31C961EB74C6356D68C049B8923B61FA6CE669622E60F29FB6");
const Y: [u8; 32] = hex!("7903FE1008B8BC99A41AE9E95628BC64F2F1B20C2D7E9F5177A3C294D4462299");
const RP_ID_HASH_EXAMPLE: [u8; 32] =
    hex!("A379A6F6EEAFB9A55E378C118034E2751E682FAB9F2D30AB13D2125586CE1947");
const SPKI_PREFIX: [u8; 26] = hex!("3059301306072A8648CE3D020106082A8648CE3D030107034200");
const RP_ID: &str = "example.com";
const ORIGIN: &str = "https://example.com";
const USER_HANDLE: [u8; 8] = [9, 8, 7, 6, 5, 4, 3, 2];

fn key() -> Es256Key {
    Es256Key::from_scalar(&D)
}

fn challenge() -> Vec<u8> {
    (0u8..32).collect()
}

fn credential_id() -> Vec<u8> {
    (0u8..16).collect()
}

fn cose_k() -> Vec<u8> {
    [&hex!("A5010203262001215820")[..], &X, &hex!("225820"), &Y].concat()
}

fn created(user_verified: bool) -> Created {
    let input = CreateInput {
        rp_id: RP_ID,
        origin: ORIGIN,
        challenge: &challenge(),
        user_handle: &USER_HANDLE,
        algorithms: &[-7, -257],
        user_verified,
    };
    create_with(&input, &key(), credential_id()).unwrap()
}

fn stored_credential() -> PasskeyCredential {
    let pkcs8 = key().to_pkcs8_der().unwrap();
    let mut c = PasskeyCredential::new(
        credential_id(),
        Text::new(RP_ID),
        USER_HANDLE.to_vec(),
        pkcs8,
        COSE_ALG_ES256,
        Timestamp::from_unix_millis(0),
    );
    c.user_name = Text::new("alice");
    c
}

fn get_input(challenge: &[u8], user_verified: bool) -> GetInput<'_> {
    GetInput {
        rp_id: RP_ID,
        origin: ORIGIN,
        challenge,
        user_verified,
    }
}

fn verify(verifying_key: &VerifyingKey, message: &[u8], der_signature: &[u8]) {
    let signature = Signature::from_der(der_signature).unwrap();
    verifying_key.verify(message, &signature).unwrap();
}

fn hex_string(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02x}").unwrap();
        out
    })
}

#[test]
fn v1_cose_public_key() {
    let cose = key().cose_public_key();
    assert_eq!(cose.len(), 77);
    assert_eq!(cose.as_slice(), cose_k().as_slice());
}

#[test]
fn v2_spki() {
    let spki = key().public_key_spki_der();
    assert_eq!(spki.len(), 91);
    assert_eq!(spki, [&SPKI_PREFIX[..], &[0x04], &X, &Y].concat());
}

#[test]
fn spki_matches_the_library_encoder() {
    use p256::pkcs8::{DecodePublicKey, EncodePublicKey};

    let k = key();
    let library = k.verifying_key().to_public_key_der().unwrap();
    assert_eq!(library.as_bytes(), k.public_key_spki_der().as_slice());
    let parsed = VerifyingKey::from_public_key_der(&k.public_key_spki_der()).unwrap();
    assert_eq!(parsed, k.verifying_key());
}

#[test]
fn v3_and_v4_rfc6979_signatures() {
    let sample = key().sign(b"sample");
    assert_eq!(
        sample,
        hex!(
            "3046022100EFD48B2AACB6A8FD1140DD9CD45E81D69D2C877B56AAF991C34D0EA84EAF3716"
            "022100F7CB1C942D657C41D436C7A1B6E29F65F3E900DBB9AFF4064DC4AB2F843ACDA8"
        )
    );
    let test = key().sign(b"test");
    assert_eq!(
        test,
        hex!(
            "3045022100F1ABB023518351CD71D881567B1EA663ED3EFCF6C5132B354F28D3B0B7D38367"
            "0220019F4113742A2B14BD25926B49C649155F267E60D3814B4C0CC84250E46F0083"
        )
    );
    verify(&key().verifying_key(), b"sample", &sample);
    verify(&key().verifying_key(), b"test", &test);
}

#[test]
fn v5_rp_id_hash() {
    assert_eq!(rp_id_hash(RP_ID), RP_ID_HASH_EXAMPLE);
}

#[test]
fn v6_v7_client_data_json() {
    let get = client_data_json(Ceremony::Get, &challenge(), ORIGIN);
    assert_eq!(
        String::from_utf8(get).unwrap(),
        r#"{"type":"webauthn.get","challenge":"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","origin":"https://example.com","crossOrigin":false}"#
    );
    let create = client_data_json(Ceremony::Create, &challenge(), ORIGIN);
    assert_eq!(
        String::from_utf8(create).unwrap(),
        r#"{"type":"webauthn.create","challenge":"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","origin":"https://example.com","crossOrigin":false}"#
    );
}

#[test]
fn v8_ccd_escaping() {
    assert_eq!(
        ccd_to_string("a\"b\\c\u{7}d\u{2028}"),
        "\"a\\\"b\\\\c\\u0007d\u{2028}\""
    );
    assert_eq!(ccd_to_string(""), "\"\"");
    assert_eq!(ccd_to_string("\u{1f}\u{20}\u{7f}"), "\"\\u001f \u{7f}\"");
    assert_eq!(ccd_to_string("b\u{fc}cher"), "\"b\u{fc}cher\"");
    assert_eq!(ccd_to_string("\n\t"), "\"\\u000a\\u0009\"");
}

#[test]
fn v9_create() {
    let c = created(true);
    let expected_auth = [
        &RP_ID_HASH_EXAMPLE[..],
        &[0x5D],
        &[0, 0, 0, 0],
        &[0u8; 16],
        &[0x00, 0x10],
        &credential_id(),
        &cose_k(),
    ]
    .concat();
    assert_eq!(c.authenticator_data.len(), 148);
    assert_eq!(c.authenticator_data, expected_auth);
    let expected_object = [
        &hex!("A363666D74646E6F6E656761747453746D74A068617574684461746158")[..],
        &[0x94],
        &expected_auth,
    ]
    .concat();
    assert_eq!(c.attestation_object.len(), 178);
    assert_eq!(c.attestation_object, expected_object);
    assert_eq!(c.credential_id, credential_id());
    assert_eq!(c.algorithm, -7);
    assert_eq!(c.public_key_spki, key().public_key_spki_der());
    assert_eq!(
        c.client_data_json,
        client_data_json(Ceremony::Create, &challenge(), ORIGIN)
    );
    // The stored private key loads again and is the same key.
    let again = Es256Key::from_pkcs8_der(c.private_key_pkcs8.expose_secret()).unwrap();
    assert_eq!(again.cose_public_key().as_slice(), cose_k().as_slice());
    // Without user verification the flags are 0x59.
    assert_eq!(created(false).authenticator_data.get(32), Some(&0x59));
}

#[test]
fn v10_get() {
    let challenge = challenge();
    let pending = assert(&get_input(&challenge, true), &stored_credential(), 0).unwrap();
    assert_eq!(pending.count_to_persist(), None);
    let a = pending.into_response_without_count().unwrap();
    assert_eq!(
        a.authenticator_data,
        [&RP_ID_HASH_EXAMPLE[..], &[0x1D], &[0, 0, 0, 0]].concat()
    );
    assert_eq!(a.credential_id, credential_id());
    assert_eq!(a.user_handle, USER_HANDLE);
    assert_eq!(
        a.client_data_json,
        client_data_json(Ceremony::Get, &challenge, ORIGIN)
    );
    let message = [
        a.authenticator_data.as_slice(),
        &Sha256::digest(&a.client_data_json),
    ]
    .concat();
    verify(&key().verifying_key(), &message, &a.signature);
}

#[test]
fn v11_imported_counter() {
    let challenge = challenge();
    let pending = assert(&get_input(&challenge, false), &stored_credential(), 41).unwrap();
    assert_eq!(pending.count_to_persist(), Some(42));
    let a = pending
        .into_json_response(CountPersisted::after_store_write())
        .unwrap();
    assert_eq!(
        a.authenticator_data,
        [&RP_ID_HASH_EXAMPLE[..], &[0x19], &[0, 0, 0, 0x2A]].concat()
    );
}

#[test]
fn a_pending_count_blocks_the_release_without_a_token() {
    let challenge = challenge();
    let input = get_input(&challenge, false);
    let pending = assert(&input, &stored_credential(), 41).unwrap();
    assert_eq!(
        pending.into_response_without_count().err(),
        Some(Error::InvalidInput)
    );
    // A counter that stays 0 releases either way.
    let zero = assert(&input, &stored_credential(), 0).unwrap();
    assert!(
        zero.into_json_response(CountPersisted::after_store_write())
            .unwrap()
            .signature
            .len()
            > 8
    );
}

#[test]
fn sign_count_rule() {
    let challenge = challenge();
    let count = |stored: u64| {
        let pending = assert(&get_input(&challenge, false), &stored_credential(), stored).unwrap();
        let persist = pending.count_to_persist();
        let a = pending
            .into_json_response(CountPersisted::after_store_write())
            .unwrap();
        let bytes: [u8; 4] = a.authenticator_data[33..37].try_into().unwrap();
        (u32::from_be_bytes(bytes), persist)
    };
    assert_eq!(count(0), (0, None));
    assert_eq!(count(1), (2, Some(2)));
    assert_eq!(
        count(u64::from(u32::MAX) - 1),
        (u32::MAX, Some(u64::from(u32::MAX)))
    );
    assert_eq!(
        count(u64::from(u32::MAX)),
        (u32::MAX, Some(u64::from(u32::MAX)))
    );
    assert_eq!(count(u64::MAX), (u32::MAX, Some(u64::from(u32::MAX))));
}

#[test]
fn generate_round_trips_through_pkcs8() {
    let generated = Es256Key::generate().unwrap();
    let der = generated.to_pkcs8_der().unwrap();
    let loaded = Es256Key::from_pkcs8_der(der.expose_secret()).unwrap();
    assert_eq!(loaded.cose_public_key(), generated.cose_public_key());
    assert_eq!(
        loaded.public_key_spki_der(),
        generated.public_key_spki_der()
    );
    // Two keys differ, and so do two credential IDs.
    let other = Es256Key::generate().unwrap();
    assert_ne!(other.cose_public_key(), generated.cose_public_key());
    let (a, b) = (new_credential_id().unwrap(), new_credential_id().unwrap());
    assert_eq!((a.len(), b.len()), (CREDENTIAL_ID_LEN, CREDENTIAL_ID_LEN));
    assert_ne!(a, b);
}

#[test]
fn cose_bytes_parse_back_to_the_verifying_key() {
    let generated = Es256Key::generate().unwrap();
    let cose = generated.cose_public_key();
    assert_eq!(cose[..10], hex!("A5010203262001215820"));
    assert_eq!(cose[42..45], hex!("225820"));
    let point = [&[0x04][..], &cose[10..42], &cose[45..77]].concat();
    let parsed = VerifyingKey::from_sec1_bytes(&point).unwrap();
    assert_eq!(parsed, generated.verifying_key());
}

#[test]
fn full_ceremony_with_a_generated_key() {
    let challenge = challenge();
    let input = CreateInput {
        rp_id: RP_ID,
        origin: ORIGIN,
        challenge: &challenge,
        user_handle: &USER_HANDLE,
        algorithms: &[],
        user_verified: false,
    };
    let c = create(&input).unwrap();
    assert_eq!(c.credential_id.len(), CREDENTIAL_ID_LEN);
    assert_eq!(c.authenticator_data.len(), 148);
    let mut stored = PasskeyCredential::new(
        c.credential_id.clone(),
        Text::new(RP_ID),
        USER_HANDLE.to_vec(),
        SecretVec::from_slice(c.private_key_pkcs8.expose_secret()).unwrap(),
        c.algorithm,
        Timestamp::from_unix_millis(1),
    );
    stored.discoverable = true;
    let a = assert(&get_input(&challenge, true), &stored, 0)
        .unwrap()
        .into_response_without_count()
        .unwrap();
    let spki_key = VerifyingKey::from_sec1_bytes(&c.public_key_spki[26..]).unwrap();
    let message = [
        a.authenticator_data.as_slice(),
        &Sha256::digest(&a.client_data_json),
    ]
    .concat();
    verify(&spki_key, &message, &a.signature);
}

#[test]
fn invalid_keys_are_refused() {
    // Ed25519 and RSA PKCS#8 structures, garbage, empty, truncated, trailing data.
    let mut ed25519 = hex!("302E020100300506032B657004220420").to_vec();
    ed25519.extend_from_slice(&[7u8; 32]);
    let rsa = hex!("3016020100300D06092A864886F70D010101050004023000");
    let valid = key().to_pkcs8_der().unwrap();
    let mut trailing = valid.expose_secret().to_vec();
    trailing.push(0);
    let truncated = valid.expose_secret()[..valid.len() - 1].to_vec();
    for bad in [
        ed25519,
        rsa.to_vec(),
        b"not a key".to_vec(),
        Vec::new(),
        truncated,
        trailing,
    ] {
        assert_eq!(
            Es256Key::from_pkcs8_der(&bad).err(),
            Some(Error::InvalidKey)
        );
    }
}

#[test]
fn assert_refuses_bad_credentials() {
    let challenge = challenge();
    let input = get_input(&challenge, false);
    let mut other_alg = stored_credential();
    other_alg.algorithm = -8;
    assert_eq!(
        assert(&input, &other_alg, 0).err(),
        Some(Error::UnsupportedAlgorithm)
    );
    let mut other_rp = stored_credential();
    other_rp.rp_id = Text::new("evil.test");
    assert_eq!(
        assert(&input, &other_rp, 0).err(),
        Some(Error::InvalidInput)
    );
    let mut bad_key = stored_credential();
    bad_key.private_key = SecretVec::from_slice(b"garbage").unwrap();
    assert_eq!(assert(&input, &bad_key, 0).err(), Some(Error::InvalidKey));
    let mut no_handle = stored_credential();
    no_handle.user_handle = Vec::new();
    assert_eq!(
        assert(&input, &no_handle, 0).err(),
        Some(Error::InvalidInput)
    );
}

#[test]
fn limits_are_enforced() {
    let handle = [1u8; 64];
    let make = |challenge: &[u8], handle: &[u8], rp_id: &str, algs: &[i64]| {
        create_with(
            &CreateInput {
                rp_id,
                origin: ORIGIN,
                challenge,
                user_handle: handle,
                algorithms: algs,
                user_verified: false,
            },
            &key(),
            credential_id(),
        )
        .err()
    };
    assert_eq!(make(&[0; 16], &handle, RP_ID, &[]), None);
    assert_eq!(make(&[0; 1024], &handle[..1], RP_ID, &[-7]), None);
    assert_eq!(
        make(&[0; 15], &handle, RP_ID, &[]),
        Some(Error::InvalidInput)
    );
    assert_eq!(
        make(&[0; 1025], &handle, RP_ID, &[]),
        Some(Error::InvalidInput)
    );
    assert_eq!(make(&[0; 16], &[], RP_ID, &[]), Some(Error::InvalidInput));
    assert_eq!(
        make(&[0; 16], &[1; 65], RP_ID, &[]),
        Some(Error::InvalidInput)
    );
    assert_eq!(make(&[0; 16], &handle, "", &[]), Some(Error::InvalidInput));
    assert_eq!(
        make(&[0; 16], &handle, &"a".repeat(254), &[]),
        Some(Error::InvalidInput)
    );
    assert_eq!(
        make(&[0; 16], &handle, RP_ID, &[-8, -257]),
        Some(Error::UnsupportedAlgorithm)
    );
    // `create` checks before it draws a key.
    let input = CreateInput {
        rp_id: RP_ID,
        origin: ORIGIN,
        challenge: &[0; 15],
        user_handle: &handle,
        algorithms: &[],
        user_verified: false,
    };
    assert_eq!(create(&input).err(), Some(Error::InvalidInput));
    // The credential ID must fit the model's limits.
    let good = CreateInput {
        challenge: &[0; 16],
        ..input
    };
    for id in [Vec::new(), vec![1; MAX_CREDENTIAL_ID_LEN + 1]] {
        assert_eq!(
            create_with(&good, &key(), id).err(),
            Some(Error::InvalidInput)
        );
    }
    let long_id = vec![1; MAX_CREDENTIAL_ID_LEN];
    let c = create_with(&good, &key(), long_id.clone()).unwrap();
    assert_eq!(c.authenticator_data.len(), 37 + 16 + 2 + 1023 + 77);
    assert_eq!(c.authenticator_data[53..55], [0x03, 0xFF]);

    let challenge = [0u8; 15];
    assert_eq!(
        assert(&get_input(&challenge, false), &stored_credential(), 0).err(),
        Some(Error::InvalidInput)
    );
}

/// DER `tag || length || content` (short or 16-bit long form).
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    match u8::try_from(content.len()) {
        Ok(n) if n < 0x80 => out.push(n),
        Ok(n) => out.extend([0x81, n]),
        Err(_) => {
            out.push(0x82);
            out.extend(u16::try_from(content.len()).unwrap().to_be_bytes());
        }
    }
    out.extend_from_slice(content);
    out
}

/// A hand-assembled PKCS#8 `ecPublicKey` private key on the curve with the
/// given OID, with an optional SEC1 public point.
fn pkcs8_ec(curve_oid: &[u8], scalar: &[u8], public: Option<&[u8]>) -> Vec<u8> {
    let mut ec = der(0x02, &[1]);
    ec.extend(der(0x04, scalar));
    if let Some(point) = public {
        let bits = [&[0u8][..], point].concat();
        ec.extend(der(0xA1, &der(0x03, &bits)));
    }
    let mut algorithm = der(0x06, &hex!("2A8648CE3D0201"));
    algorithm.extend(der(0x06, curve_oid));
    let mut info = der(0x02, &[0]);
    info.extend(der(0x30, &algorithm));
    info.extend(der(0x04, &der(0x30, &ec)));
    der(0x30, &info)
}

#[test]
fn pkcs8_on_another_curve_or_with_another_public_key_is_refused() {
    const P256: [u8; 8] = hex!("2A8648CE3D030107");
    const SECP256K1: [u8; 5] = hex!("2B8104000A");
    const P384: [u8; 5] = hex!("2B81040022");
    let point = |x: &[u8], y: &[u8]| [&[0x04][..], x, y].concat();
    let own_point = point(&X, &Y);

    // Control: the hand-assembled P-256 structure loads, with and without the
    // optional public key, as the same key.
    for public in [Some(own_point.as_slice()), None] {
        let loaded = Es256Key::from_pkcs8_der(&pkcs8_ec(&P256, &D, public)).unwrap();
        assert_eq!(loaded.cose_public_key().as_slice(), cose_k().as_slice());
    }

    // The same structure on secp256k1 and P-384 (with fitting scalar and point
    // sizes) is refused.
    let k1 = pkcs8_ec(&SECP256K1, &D, Some(&own_point));
    let p384 = pkcs8_ec(&P384, &[7; 48], Some(&point(&[1; 48], &[2; 48])));
    let k1_bare = pkcs8_ec(&SECP256K1, &D, None);
    for bad in [k1, p384, k1_bare] {
        assert_eq!(
            Es256Key::from_pkcs8_der(&bad).err(),
            Some(Error::InvalidKey)
        );
    }

    // P-256 whose embedded public key belongs to another scalar: refused.
    let other = Es256Key::from_scalar(&[7; 32]);
    let other_cose = other.cose_public_key();
    let other_point = point(&other_cose[10..42], &other_cose[45..77]);
    assert_eq!(
        Es256Key::from_pkcs8_der(&pkcs8_ec(&P256, &D, Some(&other_point))).err(),
        Some(Error::InvalidKey)
    );
    // And one whose embedded point is not on the curve.
    assert_eq!(
        Es256Key::from_pkcs8_der(&pkcs8_ec(&P256, &D, Some(&point(&[1; 32], &[2; 32])))).err(),
        Some(Error::InvalidKey)
    );
}

#[test]
fn pkcs8_kind_tells_damaged_p256_from_other_algorithms() {
    const P256: [u8; 8] = hex!("2A8648CE3D030107");
    const P384: [u8; 5] = hex!("2B81040022");
    let point = |x: &[u8], y: &[u8]| [&[0x04][..], x, y].concat();
    let valid = key().to_pkcs8_der().unwrap();
    assert_eq!(pkcs8_kind(valid.expose_secret()), Pkcs8Kind::P256);

    // P-256 with the public key of another scalar: damaged, but still P-256.
    let other = Es256Key::from_scalar(&[7; 32]).cose_public_key();
    let mismatched = pkcs8_ec(&P256, &D, Some(&point(&other[10..42], &other[45..77])));
    assert!(Es256Key::from_pkcs8_der(&mismatched).is_err());
    assert_eq!(pkcs8_kind(&mismatched), Pkcs8Kind::P256);

    // Another curve is valid PKCS#8 but not ES256.
    let p384 = pkcs8_ec(&P384, &[7; 48], Some(&point(&[1; 48], &[2; 48])));
    assert_eq!(pkcs8_kind(&p384), Pkcs8Kind::Other);

    // RSA: rsaEncryption with NULL parameters and a stand-in private key.
    let mut algorithm = der(0x06, &hex!("2A864886F70D010101"));
    algorithm.extend([0x05, 0x00]);
    let mut info = der(0x02, &[0]);
    info.extend(der(0x30, &algorithm));
    info.extend(der(0x04, &der(0x30, &der(0x02, &[1]))));
    let rsa = der(0x30, &info);
    assert!(Es256Key::from_pkcs8_der(&rsa).is_err());
    assert_eq!(pkcs8_kind(&rsa), Pkcs8Kind::Other);

    for junk in [
        &[][..],
        &[0x30, 0x00],
        b"not der at all",
        &valid.expose_secret()[..40],
    ] {
        assert_eq!(pkcs8_kind(junk), Pkcs8Kind::NotPkcs8);
    }
}

#[test]
fn authenticator_data_refuses_a_credential_id_over_65535_bytes() {
    let cose = key().cose_public_key();
    let at_flags = base_flags(false, flags::AT);
    let fits = vec![1; usize::from(u16::MAX)];
    assert!(authenticator_data(RP_ID, at_flags, 0, Some((&fits, &cose))).is_ok());
    let too_long = vec![1; usize::from(u16::MAX) + 1];
    assert_eq!(
        authenticator_data(RP_ID, at_flags, 0, Some((&too_long, &cose))).err(),
        Some(Error::InvalidInput)
    );
}

#[test]
fn select_algorithm_cases() {
    assert_eq!(select_algorithm(&[]), Ok(-7));
    assert_eq!(select_algorithm(&[-8, -7, -257]), Ok(-7));
    assert_eq!(select_algorithm(&[-7]), Ok(-7));
    assert_eq!(
        select_algorithm(&[-8, -257]),
        Err(Error::UnsupportedAlgorithm)
    );
}

#[test]
fn cbor_byte_string_headers() {
    let object = |len: usize| attestation_object_none(&vec![0; len]).unwrap();
    let header = |len: usize| object(len)[28..].to_vec();
    assert_eq!(header(0)[..1], [0x40]);
    assert_eq!(header(23)[..1], [0x57]);
    assert_eq!(header(24)[..2], [0x58, 24]);
    assert_eq!(header(255)[..2], [0x58, 255]);
    assert_eq!(header(256)[..3], [0x59, 0x01, 0x00]);
    assert_eq!(header(65535)[..3], [0x59, 0xFF, 0xFF]);
    assert_eq!(
        attestation_object_none(&vec![0; 65536]).err(),
        Some(Error::InvalidInput)
    );
    assert_eq!(object(148).len(), 178);
}

#[test]
fn base64url() {
    assert_eq!(b64url(&[]), "");
    assert_eq!(b64url(&[0xFB, 0xFF, 0xFE]), "-__-");
    assert_eq!(from_b64url("-__-", 3).unwrap(), [0xFB, 0xFF, 0xFE]);
    assert_eq!(from_b64url("", 0).unwrap(), Vec::<u8>::new());
    for bad in ["AA==", "AA=", "+/+/", "AB", "A", "a b", "\u{e9}\u{e9}"] {
        assert_eq!(from_b64url(bad, 16), Err(Error::InvalidInput), "{bad:?}");
    }
    // Over the limit, whether caught by length or after decoding.
    assert_eq!(from_b64url("AAAA", 2), Err(Error::InvalidInput));
    assert_eq!(from_b64url("AAAAAAAA", 5), Err(Error::InvalidInput));
    assert_eq!(from_b64url(&"A".repeat(1000), 10), Err(Error::InvalidInput));
    assert!(from_b64url("AAAAAA", 5).is_ok());
}

#[test]
fn debug_is_redacted() {
    assert_eq!(format!("{:?}", key()), "Es256Key([REDACTED])");
    assert_eq!(format!("{:#?}", key()), "Es256Key([REDACTED])");
}

#[test]
fn errors_name_no_secrets() {
    for e in [
        Error::Rng,
        Error::UnsupportedAlgorithm,
        Error::InvalidKey,
        Error::InvalidInput,
    ] {
        let text = format!("{e} {e:?}");
        assert_ne!(text, " ");
    }
}

#[test]
fn constants() {
    assert_eq!(AAGUID, [0u8; 16]);
    assert_eq!(COSE_ALG_ES256, -7);
    assert_eq!(MAX_ALLOW_LIST, 64);
    assert_eq!(
        flags::UP | flags::UV | flags::BE | flags::BS | flags::AT,
        0x5D
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    #[test]
    fn signatures_verify_after_a_pkcs8_round_trip(msg in proptest::collection::vec(any::<u8>(), 0..256)) {
        LOADED.with(|(loaded, verifying)| {
            let signature = loaded.sign(&msg);
            let parsed = Signature::from_der(&signature).unwrap();
            prop_assert!(verifying.verify(&msg, &parsed).is_ok());
            Ok(())
        })?;
    }
}

thread_local! {
    /// The test key after a PKCS#8 round trip and its verifying key, made
    /// once per thread (key setup dominates in a debug build).
    static LOADED: (Es256Key, VerifyingKey) = {
        let der = key().to_pkcs8_der().unwrap();
        (Es256Key::from_pkcs8_der(der.expose_secret()).unwrap(), key().verifying_key())
    };
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn b64url_round_trips(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        let text = b64url(&bytes);
        prop_assert!(!text.contains('=') && !text.contains('+') && !text.contains('/'));
        prop_assert_eq!(from_b64url(&text, bytes.len()).unwrap(), bytes.clone());
        if !bytes.is_empty() {
            prop_assert!(from_b64url(&text, bytes.len() - 1).is_err());
        }
    }

    #[test]
    fn client_data_json_parses_back(origin in "[ -~\u{a0}-\u{2fff}]{0,40}", challenge in proptest::collection::vec(any::<u8>(), 16..64)) {
        let json = client_data_json(Ceremony::Get, &challenge, &origin);
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        prop_assert_eq!(value["origin"].as_str(), Some(origin.as_str()));
        prop_assert_eq!(value["type"].as_str(), Some("webauthn.get"));
        prop_assert_eq!(value["crossOrigin"].as_bool(), Some(false));
        prop_assert_eq!(
            from_b64url(value["challenge"].as_str().unwrap(), 1024).unwrap(),
            challenge
        );
    }

    #[test]
    fn ccd_never_emits_raw_controls(s in "\\PC{0,40}|[\\x00-\\x1f]{0,10}") {
        let out = ccd_to_string(&s);
        prop_assert!(out.chars().all(|c| u32::from(c) >= 0x20));
        prop_assert!(out.starts_with('"') && out.ends_with('"'));
    }
}

/// A hostile origin string for V8: a quote, a backslash, BEL, DEL, U+2028, a
/// two-byte letter and a non-BMP code point.
const CCD_HOSTILE: &str = "a\"b\\c\u{7}\u{7f}\u{2028}\u{fc}\u{1f600}";

/// The fixture the TypeScript verifier checks: everything the Rust code emits
/// for the RFC 6979 key, in hex. `MOSAIC_UPDATE_FIXTURES=1` rewrites it.
fn fixture() -> serde_json::Value {
    use serde_json::json;

    let challenge = challenge();
    let c = created(true);
    let input = get_input(&challenge, true);
    let get = assert(&input, &stored_credential(), 0)
        .unwrap()
        .into_response_without_count()
        .unwrap();
    let pending_no_uv = assert(&get_input(&challenge, false), &stored_credential(), 41).unwrap();
    let persist_count = pending_no_uv.count_to_persist();
    let get_no_uv = pending_no_uv
        .into_json_response(CountPersisted::after_store_write())
        .unwrap();
    let hostile = CCD_HOSTILE;
    let k = key();
    json!({
        "description": "Known-answer vectors V1 to V11 (CRYPTO_SPEC section 16) and a CCDToString check, regenerated by MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-passkey vectors_fixture",
        "key": {
            "d": hex_string(&D),
            "x": hex_string(&X),
            "y": hex_string(&Y),
            "cose": hex_string(&k.cose_public_key()),
            "spki": hex_string(&k.public_key_spki_der()),
        },
        "rfc6979": {
            "sample": hex_string(&k.sign(b"sample")),
            "test": hex_string(&k.sign(b"test")),
        },
        "rpId": RP_ID,
        "rpIdHash": hex_string(&rp_id_hash(RP_ID)),
        "origin": ORIGIN,
        "challenge": hex_string(&challenge),
        "credentialId": hex_string(&credential_id()),
        "create": {
            "flags": hex_string(&c.authenticator_data[32..33]),
            "authData": hex_string(&c.authenticator_data),
            "clientDataJson": hex_string(&c.client_data_json),
            "attestationObject": hex_string(&c.attestation_object),
        },
        "get": {
            "flags": hex_string(&get.authenticator_data[32..33]),
            "signCount": 0,
            "authData": hex_string(&get.authenticator_data),
            "clientDataJson": hex_string(&get.client_data_json),
            "signature": hex_string(&get.signature),
        },
        "getImportedCount": {
            "flags": hex_string(&get_no_uv.authenticator_data[32..33]),
            "storedCount": 41,
            "signCount": persist_count,
            "authData": hex_string(&get_no_uv.authenticator_data),
            "clientDataJson": hex_string(&get_no_uv.client_data_json),
            "signature": hex_string(&get_no_uv.signature),
        },
        "ccd": {
            "inputUtf8": hex_string(hostile.as_bytes()),
            "outputUtf8": hex_string(ccd_to_string(hostile).as_bytes()),
        },
    })
}

#[test]
fn vectors_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/vectors.json");
    let current = fixture();
    if std::env::var_os("MOSAIC_UPDATE_FIXTURES").is_some() {
        let mut text = serde_json::to_string_pretty(&current).unwrap();
        text.push('\n');
        std::fs::write(path, text).unwrap();
    }
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        stored, current,
        "fixtures/vectors.json is stale; run with MOSAIC_UPDATE_FIXTURES=1"
    );
}

// ---- Client-data modes (ADR-0093) ----

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn request_hash() -> [u8; 32] {
    sha256(&client_data_json(Ceremony::Get, &challenge(), ORIGIN))
}

#[test]
fn assert_hash_mode_signs_auth_data_and_given_hash() {
    let hash = [0x5Au8; 32];
    let request = GetRequest {
        rp_id: RP_ID,
        client_data: ClientData::Hash(&hash),
        user_verified: true,
    };
    let pending = assert_request(&request, &stored_credential(), 0).unwrap();
    assert_eq!(pending.count_to_persist(), None);
    let a = pending.into_client_response_without_count().unwrap();
    assert_eq!(a.client_data_json, None);
    assert_eq!(
        a.authenticator_data,
        [&RP_ID_HASH_EXAMPLE[..], &[0x1D], &[0, 0, 0, 0]].concat()
    );
    let message = [a.authenticator_data.as_slice(), &hash].concat();
    verify(&key().verifying_key(), &message, &a.signature);
    // A counter that is not 0 still goes through the persist rule.
    let pending = assert_request(&request, &stored_credential(), 41).unwrap();
    assert_eq!(pending.count_to_persist(), Some(42));
    assert_eq!(
        assert_request(&request, &stored_credential(), 41)
            .unwrap()
            .into_client_response_without_count()
            .err(),
        Some(Error::InvalidInput)
    );
}

#[test]
fn hash_mode_assertion_refuses_the_json_releases() {
    // G1-16: no empty `clientDataJSON` leaves a hash-mode assertion.
    let hash = [0x5Au8; 32];
    let request = GetRequest {
        rp_id: RP_ID,
        client_data: ClientData::Hash(&hash),
        user_verified: true,
    };
    let zero = assert_request(&request, &stored_credential(), 0).unwrap();
    assert_eq!(
        zero.into_response_without_count().err(),
        Some(Error::InvalidInput)
    );
    let counted = assert_request(&request, &stored_credential(), 41).unwrap();
    assert_eq!(
        counted
            .into_json_response(CountPersisted::after_store_write())
            .err(),
        Some(Error::InvalidInput)
    );
    // Json mode releases as before.
    let json = assert(&get_input(&challenge(), true), &stored_credential(), 41).unwrap();
    let a = json
        .into_json_response(CountPersisted::after_store_write())
        .unwrap();
    assert!(a.client_data_json.starts_with(b"{"));
}

#[test]
fn create_hash_mode_has_no_client_data_json() {
    let hash = [7u8; 32];
    let request = CreateRequest {
        rp_id: RP_ID,
        client_data: ClientData::Hash(&hash),
        user_handle: &USER_HANDLE,
        algorithms: &[-7],
        user_verified: true,
    };
    let c = create_request_with(&request, &key(), credential_id()).unwrap();
    assert_eq!(c.client_data_json, None);
    assert_eq!(c.authenticator_data, created(true).authenticator_data);
    assert_eq!(c.attestation_object, created(true).attestation_object);
    // The public entry point needs no challenge in hash mode.
    let fresh = create_request(&request).unwrap();
    assert_eq!(fresh.client_data_json, None);
    assert_eq!(fresh.algorithm, -7);
}

#[test]
fn hash_mode_equals_json_mode_for_the_same_client_data() {
    let challenge = challenge();
    let json_request = GetRequest {
        rp_id: RP_ID,
        client_data: ClientData::Json {
            origin: ORIGIN,
            challenge: &challenge,
        },
        user_verified: true,
    };
    let hash = request_hash();
    let hash_request = GetRequest {
        rp_id: RP_ID,
        client_data: ClientData::Hash(&hash),
        user_verified: true,
    };
    let a = assert_request(&json_request, &stored_credential(), 0)
        .unwrap()
        .into_client_response_without_count()
        .unwrap();
    let b = assert_request(&hash_request, &stored_credential(), 0)
        .unwrap()
        .into_client_response_without_count()
        .unwrap();
    // RFC 6979 is deterministic: the same message gives the same signature.
    assert_eq!(a.signature, b.signature);
    assert_eq!(a.authenticator_data, b.authenticator_data);
    assert!(a.client_data_json.is_some());
    assert_eq!(b.client_data_json, None);
}

#[test]
fn json_mode_requires_a_valid_challenge_and_hash_mode_still_checks_the_rp_id() {
    let short = [0u8; 15];
    let request = GetRequest {
        rp_id: RP_ID,
        client_data: ClientData::Json {
            origin: ORIGIN,
            challenge: &short,
        },
        user_verified: false,
    };
    assert_eq!(
        assert_request(&request, &stored_credential(), 0).err(),
        Some(Error::InvalidInput)
    );
    let hash = [0u8; 32];
    let request = GetRequest {
        rp_id: "",
        client_data: ClientData::Hash(&hash),
        user_verified: false,
    };
    assert_eq!(
        assert_request(&request, &stored_credential(), 0).err(),
        Some(Error::InvalidInput)
    );
}

#[test]
fn android_app_origin_is_apk_key_hash_b64url() {
    let cert = [0xFFu8; 32];
    assert_eq!(
        android_app_origin(&cert),
        format!("android:apk-key-hash:{}", b64url(&cert))
    );
    assert_eq!(
        android_app_origin(&[0u8; 32]),
        "android:apk-key-hash:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
}

#[test]
fn json_mode_vectors_unchanged() {
    // The new API in Json mode produces exactly what the existing API did
    // (V9 and V10 pin the bytes; this pins the equivalence).
    let challenge = challenge();
    let request = CreateRequest {
        rp_id: RP_ID,
        client_data: ClientData::Json {
            origin: ORIGIN,
            challenge: &challenge,
        },
        user_handle: &USER_HANDLE,
        algorithms: &[-7, -257],
        user_verified: true,
    };
    let c = create_request_with(&request, &key(), credential_id()).unwrap();
    let old = created(true);
    assert_eq!(c.client_data_json, Some(old.client_data_json));
    assert_eq!(c.authenticator_data, old.authenticator_data);
    assert_eq!(c.attestation_object, old.attestation_object);
    let old_assertion = assert(&get_input(&challenge, true), &stored_credential(), 0)
        .unwrap()
        .into_response_without_count()
        .unwrap();
    let new_assertion = assert_request(
        &GetRequest {
            rp_id: RP_ID,
            client_data: ClientData::Json {
                origin: ORIGIN,
                challenge: &challenge,
            },
            user_verified: true,
        },
        &stored_credential(),
        0,
    )
    .unwrap()
    .into_client_response_without_count()
    .unwrap();
    assert_eq!(new_assertion.signature, old_assertion.signature);
    assert_eq!(
        new_assertion.client_data_json,
        Some(old_assertion.client_data_json)
    );
}

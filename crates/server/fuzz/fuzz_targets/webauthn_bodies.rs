//! The WebAuthn input of the sync server (CRYPTO_SPEC §9.3, §14): arbitrary
//! bytes never panic the handoff request parsers or either finish path of the
//! relying party (`Rp`), which runs the library's CBOR, COSE, authenticator
//! data and signature code on attacker-controlled bytes.
//!
//! The first byte picks a mode:
//!
//! - 0: the rest is a JSON body of each handoff request type;
//! - 1: the rest is the credential JSON of a registration finish;
//! - 2: the rest is the credential JSON of an assertion finish against a
//!   fixed stored credential;
//! - 3: the rest is the raw `attestationObject` of a registration, inside a
//!   well-formed JSON envelope whose client data carries the real challenge, so
//!   the CBOR, COSE and authenticator-data parsers are reached (aliased COSE
//!   labels, duplicate map keys, trailing bytes after the authenticator data);
//! - 4: the rest is raw `authenticatorData`, with a fixed signature, inside a
//!   well-formed assertion envelope.
#![no_main]

use base64ct::{Base64UrlUnpadded, Encoding};
use libfuzzer_sys::fuzz_target;
use mosaic_server::WebauthnConfig;
use mosaic_server::webauthn_rp::{CeremonyState, Rp, StoredCredential};
use mosaic_sync::wire::json::{self, DEFAULT_MAX_REQUEST_LEN};

const ORIGIN: &str = "https://sync.example.com";
const HANDLE: [u8; 16] = [7; 16];

/// A fixed COSE key: ES256 with the P-256 base point (public constants).
fn cose_key() -> Vec<u8> {
    const GX: [u8; 32] = [
        0x6B, 0x17, 0xD1, 0xF2, 0xE1, 0x2C, 0x42, 0x47, 0xF8, 0xBC, 0xE6, 0xE5, 0x63, 0xA4, 0x40,
        0xF2, 0x77, 0x03, 0x7D, 0x81, 0x2D, 0xEB, 0x33, 0xA0, 0xF4, 0xA1, 0x39, 0x45, 0xD8, 0x98,
        0xC2, 0x96,
    ];
    const GY: [u8; 32] = [
        0x4F, 0xE3, 0x42, 0xE2, 0xFE, 0x1A, 0x7F, 0x9B, 0x8E, 0xE7, 0xEB, 0x4A, 0x7C, 0x0F, 0x9E,
        0x16, 0x2B, 0xCE, 0x33, 0x57, 0x6B, 0x31, 0x5E, 0xCE, 0xCB, 0xB6, 0x40, 0x68, 0x37, 0xBF,
        0x51, 0xF5,
    ];
    let mut k = vec![0xA5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20];
    k.extend_from_slice(&GX);
    k.extend_from_slice(&[0x22, 0x58, 0x20]);
    k.extend_from_slice(&GY);
    k
}

fn stored() -> Vec<StoredCredential> {
    vec![StoredCredential {
        id: vec![9; 16],
        public_key: cose_key(),
        sign_count: 0,
    }]
}

fn b64(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

fn client_data(kind: &str, challenge: &[u8]) -> String {
    b64(format!(
        "{{\"type\":\"{kind}\",\"challenge\":\"{}\",\"origin\":\"{ORIGIN}\",\"crossOrigin\":false}}",
        b64(challenge)
    )
    .as_bytes())
}

fn requests(body: &[u8]) {
    let _ = json::parse::<json::HandoffOptionsRequest>(body, DEFAULT_MAX_REQUEST_LEN);
    let _ = json::parse::<json::HandoffFinishRequest>(body, DEFAULT_MAX_REQUEST_LEN);
    let _ = json::parse::<json::HandoffStateRequest>(body, DEFAULT_MAX_REQUEST_LEN);
    let _ = json::parse::<json::AccountHandoffRequest>(body, DEFAULT_MAX_REQUEST_LEN);
    let _ = json::parse::<json::RemoveWebauthnRequest>(body, DEFAULT_MAX_REQUEST_LEN);
    let _ = json::parse::<json::OptionalCodeRequest>(body, DEFAULT_MAX_REQUEST_LEN);
}

fn registration(rp: &Rp, response: impl FnOnce(&[u8]) -> Vec<u8>) {
    let Ok((CeremonyState::Registration(state), _)) =
        rp.registration_options(&HANDLE, "a@b.c", &[])
    else {
        return;
    };
    let body = response(state.challenge.as_bytes());
    let _ = rp.finish_registration(&state, &body);
}

fn assertion(rp: &Rp, response: impl FnOnce(&[u8]) -> Vec<u8>) {
    let Ok((CeremonyState::Assertion(state), _)) = rp.assertion_options(&HANDLE, &stored())
    else {
        return;
    };
    let body = response(state.challenge.as_bytes());
    let _ = rp.finish_assertion(&state, &body, &stored());
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let Some(rp) = Rp::new(ORIGIN, &WebauthnConfig { enabled: true }) else {
        return;
    };
    match mode % 5 {
        0 => requests(rest),
        1 => registration(&rp, |_| rest.to_vec()),
        2 => assertion(&rp, |_| rest.to_vec()),
        3 => registration(&rp, |challenge| {
            format!(
                "{{\"id\":\"{}\",\"type\":\"public-key\",\"response\":{{\"clientDataJSON\":\"{}\",\"attestationObject\":\"{}\"}}}}",
                b64(&[9; 16]),
                client_data("webauthn.create", challenge),
                b64(rest)
            )
            .into_bytes()
        }),
        _ => assertion(&rp, |challenge| {
            format!(
                "{{\"id\":\"{}\",\"type\":\"public-key\",\"response\":{{\"clientDataJSON\":\"{}\",\"authenticatorData\":\"{}\",\"signature\":\"{}\"}}}}",
                b64(&[9; 16]),
                client_data("webauthn.get", challenge),
                b64(rest),
                b64(&[0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01])
            )
            .into_bytes()
        }),
    }
});

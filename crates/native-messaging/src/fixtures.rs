//! The protocol fixtures: `fixtures/protocol.json`, which the extension's
//! validators (T3) check in both directions.
//!
//! `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-native-messaging
//! protocol_fixtures` rewrites the file; without it the test fails if the
//! file differs from what the Rust types produce.
//!
//! ```text
//! { "version": 1,
//!   "requests":  [{ "op": "<op>", "message": <request envelope> }, ...],   every op
//!   "responses": [{ "op": "<op>", "message": <ok response> }, ...],        every op
//!   "errors":    [{ "code": "<code>", "message": <error response> }, ...], every code
//!   "events":    [{ "name": "<event>", "message": <event> }, ...] }
//! ```

use std::path::PathBuf;

use mosaic_core::browser_channel::PairingKey;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use serde_json::{Value, json};

use crate::bodies::*;
use crate::hex::{Hex, HexKey};
use crate::protocol::{
    DesktopMessage, ErrorCode, Event, LockState, Op, RequestEnvelope, Secret, error_json, ok_json,
    request_json,
};

const ORIGIN: &str = "https://example.com";
const VAULT: &str = "0190a0b0-0000-7000-8000-000000000001";
const ITEM: &str = "0190a0b0-0000-7000-8000-000000000002";
const REQUEST_ID: &str = "00112233445566778899aabbccddeeff";
const CHALLENGE: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
const CREDENTIAL_ID: &str = "AAECAwQFBgcICQoLDA0ODw";

fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

fn pairing() -> PairingBody {
    PairingBody {
        pairing_id: Hex([0x11; 16]),
        key: HexKey(PairingKey::from_bytes(&mut [0x22; 32])),
    }
}

fn pair_result() -> PairResult {
    let p = pairing();
    PairResult {
        pairing_id: p.pairing_id,
        key: p.key,
    }
}

fn save_body() -> SaveBody {
    SaveBody {
        origin: ORIGIN.into(),
        top_origin: ORIGIN.into(),
        username: Some("ada@example.com".into()),
        password: Secret::new("correct horse battery staple".into()),
    }
}

fn passkey_create_body() -> PasskeyCreateBody {
    PasskeyCreateBody {
        request_id: REQUEST_ID.into(),
        origin: ORIGIN.into(),
        top_origin: ORIGIN.into(),
        rp_id: None,
        user_id: "AQIDBA".into(),
        user_name: "ada@example.com".into(),
        user_display_name: "Ada Lovelace".into(),
        challenge: CHALLENGE.into(),
        algorithms: vec![-7, -257],
        exclude_credentials: vec![CREDENTIAL_ID.into()],
        user_verification: UserVerification::Preferred,
    }
}

fn passkey_get_body() -> PasskeyGetBody {
    PasskeyGetBody {
        request_id: REQUEST_ID.into(),
        origin: ORIGIN.into(),
        top_origin: ORIGIN.into(),
        rp_id: Some("example.com".into()),
        challenge: CHALLENGE.into(),
        allow_credentials: vec![CREDENTIAL_ID.into()],
        user_verification: UserVerification::Required,
    }
}

fn request_samples() -> Vec<(Op, Value)> {
    let mut out = Vec::new();
    let mut add = |op: Op, body: Value| {
        let bytes = request_json(7, op, &body).unwrap();
        out.push((op, serde_json::from_slice::<Value>(&bytes).unwrap()));
    };
    add(
        Op::Hello,
        to_value(&HelloBody {
            pairing: Some(pairing()),
            hint: Hint::Chrome,
        }),
    );
    add(
        Op::Pair,
        to_value(&PairBody {
            hint: Hint::Firefox,
        }),
    );
    add(Op::Status, to_value(&Empty {}));
    add(Op::Focus, to_value(&Empty {}));
    add(
        Op::ListForOrigin,
        to_value(&ListForOriginBody {
            origin: ORIGIN.into(),
            top_origin: ORIGIN.into(),
        }),
    );
    add(
        Op::Search,
        to_value(&SearchBody {
            query: "git".into(),
            origin: Some(ORIGIN.into()),
        }),
    );
    add(
        Op::Fill,
        to_value(&FillBody {
            vault_id: VAULT.into(),
            item_id: ITEM.into(),
            origin: ORIGIN.into(),
            top_origin: ORIGIN.into(),
            frame: FrameKind::Top,
            want: Want {
                login: true,
                otp: true,
            },
        }),
    );
    add(
        Op::Copy,
        to_value(&CopyBody {
            source: CopySource::Item {
                vault_id: VAULT.into(),
                item_id: ITEM.into(),
                field: CopyField::Password,
            },
        }),
    );
    let request = RawValue::from_string(
        r#"{"kind":"forSite","host":"example.com","rules":"minlength: 12; required: lower, upper; required: digit;"}"#
            .to_owned(),
    )
    .unwrap();
    add(Op::Generate, to_value(&GenerateBody { request: &request }));
    add(Op::CheckSave, to_value(&save_body()));
    add(Op::ProposeSave, to_value(&save_body()));
    add(Op::Unpair, to_value(&Empty {}));
    add(Op::PasskeyCreate, to_value(&passkey_create_body()));
    add(Op::PasskeyGet, to_value(&passkey_get_body()));
    add(
        Op::PasskeyCancel,
        to_value(&PasskeyCancelBody {
            request_id: REQUEST_ID.into(),
        }),
    );
    out
}

fn login_match() -> LoginMatch {
    LoginMatch {
        vault_id: VAULT.into(),
        item_id: ITEM.into(),
        title: Some("Example".into()),
        username: Some("ada@example.com".into()),
        host: Some("example.com".into()),
        has_otp: true,
        favorite: false,
        matched_by: MatchedBy::ExactOrigin,
        blocked: None,
    }
}

#[expect(clippy::too_many_lines, reason = "a flat list of samples")]
fn response_samples() -> Vec<(Op, Value)> {
    let mut out = Vec::new();
    let mut add = |op: Op, body: Value| {
        let bytes = ok_json(7, &body).unwrap();
        out.push((op, serde_json::from_slice::<Value>(&bytes).unwrap()));
    };
    add(
        Op::Hello,
        to_value(&HelloResult {
            desktop: DesktopState::Unlocked,
            paired: true,
        }),
    );
    add(Op::Pair, to_value(&pair_result()));
    add(
        Op::Status,
        to_value(&StatusResult {
            state: LockState::Unlocked,
        }),
    );
    add(Op::Focus, to_value(&Empty {}));
    add(
        Op::ListForOrigin,
        to_value(&ListForOriginResult {
            items: vec![
                login_match(),
                LoginMatch {
                    title: None,
                    username: None,
                    host: None,
                    has_otp: false,
                    favorite: true,
                    matched_by: MatchedBy::Etld1,
                    blocked: Some(Blocked::Insecure),
                    ..login_match()
                },
            ],
            warnings: vec![
                OriginWarning::Idn {
                    unicode_host: "bücher.example".into(),
                },
                OriginWarning::Insecure {},
                OriginWarning::Lookalike {
                    saved_host: "github.com".into(),
                },
            ],
        }),
    );
    add(
        Op::Search,
        to_value(&SearchResult {
            items: vec![SearchHit {
                vault_id: VAULT.into(),
                item_id: ITEM.into(),
                title: "Example".into(),
                username: None,
                host: Some("example.com".into()),
                has_otp: false,
                favorite: true,
                fillable: true,
            }],
        }),
    );
    add(
        Op::Fill,
        to_value(&FillResult {
            username: Some("ada@example.com".into()),
            password: Some(Secret::new("correct horse battery staple".into())),
            otp: Some(Secret::new("492039".into())),
        }),
    );
    add(
        Op::Copy,
        to_value(&CopyResult {
            clear_after_seconds: Some(30),
        }),
    );
    add(
        Op::Generate,
        to_value(&GenerateResult {
            value: Secret::new("q7#Vt2!mZp9@Lx4$Nw6&".into()),
            entropy_bits: 131.1,
            strength: "veryStrong".into(),
            rules_source: Some(RulesSource::Page),
        }),
    );
    add(
        Op::CheckSave,
        to_value(&CheckSaveResult {
            kind: SaveKind::Update,
            item_title: Some("Example".into()),
        }),
    );
    add(
        Op::ProposeSave,
        to_value(&ProposeSaveResult {
            outcome: SaveOutcome::Saved,
            vault_id: Some(VAULT.into()),
            item_id: Some(ITEM.into()),
        }),
    );
    add(Op::Unpair, to_value(&Empty {}));
    add(
        Op::PasskeyCreate,
        to_value(&PasskeyCreateResult {
            outcome: CreateOutcome::Created,
            credential: Some(CreatedCredential {
                credential_id: CREDENTIAL_ID.into(),
                client_data_json: "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIn0".into(),
                attestation_object: "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YQ".into(),
                authenticator_data: "o3mm9u6vuaVeN4wRgDTidR5oL6ufLTCrE9IVhbzhlHBd".into(),
                public_key_spki: "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE".into(),
                public_key_algorithm: -7,
                vault_id: VAULT.into(),
                item_id: ITEM.into(),
            }),
        }),
    );
    add(
        Op::PasskeyGet,
        to_value(&PasskeyGetResult {
            outcome: GetOutcome::Asserted,
            assertion: Some(AssertionBody {
                credential_id: CREDENTIAL_ID.into(),
                client_data_json: "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0In0".into(),
                authenticator_data: "o3mm9u6vuaVeN4wRgDTidR5oL6ufLTCrE9IVhbzhlHAZAAAAAA".into(),
                signature: "MEUCIQDa".into(),
                user_handle: "AQIDBA".into(),
            }),
        }),
    );
    add(Op::PasskeyCancel, to_value(&Empty {}));
    out
}

fn event_samples() -> Vec<(&'static str, Value)> {
    let events = [
        ("state", Event::state(LockState::Locked)),
        ("state", Event::state(LockState::Unlocked)),
        ("unpaired", Event::Unpaired { v: 1 }),
        ("disconnected", Event::Disconnected { v: 1 }),
        (
            "pairCode",
            Event::PairCode {
                v: 1,
                request_id: 7,
                code: "4F7K2Q".into(),
                expires_in_seconds: 120,
            },
        ),
    ];
    events.into_iter().map(|(n, e)| (n, to_value(&e))).collect()
}

fn fixtures() -> Value {
    let entry = |key: &str, name: &str, message: Value| json!({ key: name, "message": message });
    json!({
        "version": 1,
        "requests": request_samples().into_iter()
            .map(|(op, m)| entry("op", op.as_str(), m)).collect::<Vec<_>>(),
        "responses": response_samples().into_iter()
            .map(|(op, m)| entry("op", op.as_str(), m)).collect::<Vec<_>>(),
        "errors": ErrorCode::ALL.into_iter().map(|code| {
            let bytes = error_json(7, code).unwrap();
            entry("code", to_value(&code).as_str().unwrap(),
                serde_json::from_slice(&bytes).unwrap())
        }).collect::<Vec<_>>(),
        "events": event_samples().into_iter()
            .map(|(name, m)| entry("name", name, m)).collect::<Vec<_>>(),
    })
}

/// Parses `value` as `T` with the strict rules and checks it serializes back
/// to the same JSON (every key present).
fn round_trips<T: DeserializeOwned + Serialize>(value: &Value) {
    let typed: T = serde_json::from_str(&value.to_string()).unwrap();
    assert_eq!(&to_value(&typed), value);
}

fn check_request_body(op: Op, body: &Value) {
    match op {
        Op::Hello => round_trips::<HelloBody>(body),
        Op::Pair => round_trips::<PairBody>(body),
        Op::Status | Op::Focus | Op::Unpair => round_trips::<Empty>(body),
        Op::ListForOrigin => round_trips::<ListForOriginBody>(body),
        Op::Search => round_trips::<SearchBody>(body),
        Op::Fill => round_trips::<FillBody>(body),
        Op::Copy => round_trips::<CopyBody>(body),
        Op::Generate => {
            let text = body.to_string();
            let typed: GenerateBody<'_> = serde_json::from_str(&text).unwrap();
            assert_eq!(&to_value(&typed), body);
        }
        Op::CheckSave | Op::ProposeSave => round_trips::<SaveBody>(body),
        Op::PasskeyCreate => round_trips::<PasskeyCreateBody>(body),
        Op::PasskeyGet => round_trips::<PasskeyGetBody>(body),
        Op::PasskeyCancel => round_trips::<PasskeyCancelBody>(body),
    }
}

fn check_response_body(op: Op, body: &Value) {
    match op {
        Op::Hello => round_trips::<HelloResult>(body),
        Op::Pair => round_trips::<PairResult>(body),
        Op::Status => round_trips::<StatusResult>(body),
        Op::Focus | Op::Unpair | Op::PasskeyCancel => round_trips::<Empty>(body),
        Op::ListForOrigin => round_trips::<ListForOriginResult>(body),
        Op::Search => round_trips::<SearchResult>(body),
        Op::Fill => round_trips::<FillResult>(body),
        Op::Copy => round_trips::<CopyResult>(body),
        Op::Generate => round_trips::<GenerateResult>(body),
        Op::CheckSave => round_trips::<CheckSaveResult>(body),
        Op::ProposeSave => round_trips::<ProposeSaveResult>(body),
        Op::PasskeyCreate => round_trips::<PasskeyCreateResult>(body),
        Op::PasskeyGet => round_trips::<PasskeyGetResult>(body),
    }
}

#[test]
fn every_sample_is_valid_for_the_rust_types() {
    for (op, message) in request_samples() {
        let text = message.to_string();
        let request: RequestEnvelope<'_> = serde_json::from_str(&text).unwrap();
        assert_eq!(Op::from_wire(&request.op), Some(op));
        check_request_body(op, &message["body"]);
    }
    for (op, message) in response_samples() {
        assert!(matches!(
            DesktopMessage::parse(message.to_string().as_bytes()),
            Ok(DesktopMessage::Response(_))
        ));
        check_response_body(op, &message["body"]);
    }
    for code in ErrorCode::ALL {
        let bytes = error_json(7, code).unwrap();
        assert!(matches!(
            DesktopMessage::parse(&bytes),
            Ok(DesktopMessage::Response(r)) if r.error == Some(code)
        ));
    }
    for (_, message) in event_samples() {
        assert!(matches!(
            DesktopMessage::parse(message.to_string().as_bytes()),
            Ok(DesktopMessage::Event(_))
        ));
    }
    assert_eq!(request_samples().len(), Op::ALL.len());
    assert_eq!(response_samples().len(), Op::ALL.len());
}

#[test]
fn protocol_fixtures() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("protocol.json");
    let expected = fixtures();
    if std::env::var_os("MOSAIC_UPDATE_FIXTURES").is_some() {
        let mut text = serde_json::to_string_pretty(&expected).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
        return;
    }
    let text = std::fs::read_to_string(&path)
        .expect("fixtures/protocol.json is missing: run with MOSAIC_UPDATE_FIXTURES=1");
    let on_disk: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        on_disk, expected,
        "fixtures/protocol.json is stale: run with MOSAIC_UPDATE_FIXTURES=1"
    );
}

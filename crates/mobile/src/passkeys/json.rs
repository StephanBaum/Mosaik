//! The WebAuthn request and response JSON of Android Credential Manager
//! (`PublicKeyCredentialRequestOptionsJSON`, `PublicKeyCredentialCreationOptionsJSON`,
//! `AuthenticationResponseJSON`, `RegistrationResponseJSON`).
//!
//! The parser is strict where it matters and ignores what it does not use:
//! at most 64 KiB, every member it reads has its exact type, base64url is
//! unpadded and bounded, a credential descriptor or parameter of any type but
//! `public-key` is refused, and a list is capped at 64 entries. Nothing here
//! panics; the fuzz target `passkey_request_json` feeds it arbitrary bytes.

use mosaic_passkey::{
    MAX_ALLOW_LIST, MAX_CHALLENGE_LEN, MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN, b64url,
    from_b64url,
};
use serde_json::{Map, Value, json};

use crate::error::{MobileError, Result};

/// The largest request accepted, in bytes.
pub const MAX_REQUEST_LEN: usize = 64 * 1024;
/// The most `pubKeyCredParams` entries read.
const MAX_ALGORITHMS: usize = 64;
/// The longest name or relying-party name kept, in bytes.
const MAX_TEXT_LEN: usize = 512;

/// The user-verification preference of a request (the apps always verify).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserVerification {
    /// `required`.
    Required,
    /// `preferred` (the default).
    Preferred,
    /// `discouraged`.
    Discouraged,
}

/// An assertion request.
#[derive(Debug, PartialEq, Eq)]
pub struct GetOptions {
    /// The relying party's challenge.
    pub challenge: Vec<u8>,
    /// `rpId`; the origin's host when absent.
    pub rp_id: Option<String>,
    /// The allowed credential IDs (empty: any discoverable credential).
    pub allow: Vec<Vec<u8>>,
    /// The verification preference.
    pub user_verification: UserVerification,
}

/// A registration request.
#[derive(Debug, PartialEq, Eq)]
pub struct CreateOptions {
    /// The relying party's challenge.
    pub challenge: Vec<u8>,
    /// `rp.id`; the origin's host when absent.
    pub rp_id: Option<String>,
    /// `rp.name`.
    pub rp_name: String,
    /// `user.id`.
    pub user_handle: Vec<u8>,
    /// `user.name`.
    pub user_name: String,
    /// `user.displayName`.
    pub user_display_name: String,
    /// The COSE algorithms offered (empty: any).
    pub algorithms: Vec<i64>,
    /// The credential IDs already registered (`excludeCredentials`).
    pub exclude: Vec<Vec<u8>>,
    /// The verification preference.
    pub user_verification: UserVerification,
}

fn invalid() -> MobileError {
    MobileError::InvalidInput
}

fn object(json: &[u8]) -> Result<Map<String, Value>> {
    if json.len() > MAX_REQUEST_LEN {
        return Err(invalid());
    }
    match serde_json::from_slice::<Value>(json) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(invalid()),
    }
}

/// A member that is absent, `null` or a string.
fn opt_str(map: &Map<String, Value>, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(invalid()),
    }
}

fn text(map: &Map<String, Value>, key: &str) -> Result<String> {
    let value = opt_str(map, key)?.unwrap_or_default();
    if value.len() > MAX_TEXT_LEN {
        return Err(invalid());
    }
    Ok(value)
}

fn bytes(map: &Map<String, Value>, key: &str, max: usize) -> Result<Vec<u8>> {
    match map.get(key) {
        Some(Value::String(s)) => from_b64url(s, max).map_err(|_| invalid()),
        _ => Err(invalid()),
    }
}

fn object_member<'a>(
    map: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(o)) => Ok(Some(o)),
        Some(_) => Err(invalid()),
    }
}

fn user_verification(map: &Map<String, Value>) -> Result<UserVerification> {
    Ok(match opt_str(map, "userVerification")?.as_deref() {
        None | Some("preferred") => UserVerification::Preferred,
        Some("required") => UserVerification::Required,
        Some("discouraged") => UserVerification::Discouraged,
        Some(_) => return Err(invalid()),
    })
}

/// The ids of a list of credential descriptors. Every entry has the type
/// `public-key`.
fn descriptors(map: &Map<String, Value>, key: &str) -> Result<Vec<Vec<u8>>> {
    let list = match map.get(key) {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(list)) => list,
        Some(_) => return Err(invalid()),
    };
    if list.len() > MAX_ALLOW_LIST {
        return Err(invalid());
    }
    list.iter()
        .map(|entry| {
            let Value::Object(entry) = entry else {
                return Err(invalid());
            };
            if opt_str(entry, "type")?.as_deref() != Some("public-key") {
                return Err(invalid());
            }
            bytes(entry, "id", MAX_CREDENTIAL_ID_LEN)
        })
        .collect()
}

/// Parses an assertion request.
///
/// # Errors
/// `invalidInput` for anything that is not a well-formed request.
pub fn parse_get(json: &[u8]) -> Result<GetOptions> {
    let map = object(json)?;
    Ok(GetOptions {
        challenge: bytes(&map, "challenge", MAX_CHALLENGE_LEN)?,
        rp_id: opt_str(&map, "rpId")?,
        allow: descriptors(&map, "allowCredentials")?,
        user_verification: user_verification(&map)?,
    })
}

/// Parses a registration request.
///
/// # Errors
/// `invalidInput` for anything that is not a well-formed request.
pub fn parse_create(json: &[u8]) -> Result<CreateOptions> {
    let map = object(json)?;
    let rp = object_member(&map, "rp")?;
    let user = object_member(&map, "user")?.ok_or_else(invalid)?;
    let algorithms = match map.get("pubKeyCredParams") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(list)) => {
            if list.len() > MAX_ALGORITHMS {
                return Err(invalid());
            }
            list.iter()
                .map(|entry| {
                    let Value::Object(entry) = entry else {
                        return Err(invalid());
                    };
                    if opt_str(entry, "type")?.as_deref() != Some("public-key") {
                        return Err(invalid());
                    }
                    entry.get("alg").and_then(Value::as_i64).ok_or_else(invalid)
                })
                .collect::<Result<Vec<i64>>>()?
        }
        Some(_) => return Err(invalid()),
    };
    let selection = object_member(&map, "authenticatorSelection")?;
    Ok(CreateOptions {
        challenge: bytes(&map, "challenge", MAX_CHALLENGE_LEN)?,
        rp_id: rp.map(|rp| opt_str(rp, "id")).transpose()?.flatten(),
        rp_name: rp
            .map(|rp| text(rp, "name"))
            .transpose()?
            .unwrap_or_default(),
        user_handle: bytes(user, "id", MAX_USER_HANDLE_LEN)?,
        user_name: text(user, "name")?,
        user_display_name: text(user, "displayName")?,
        algorithms,
        exclude: descriptors(&map, "excludeCredentials")?,
        user_verification: match selection {
            Some(s) => user_verification(s)?,
            None => UserVerification::Preferred,
        },
    })
}

/// The `AuthenticationResponseJSON` of an assertion. `client_data_json` is
/// `None` when the platform built it (privileged browsers).
#[must_use]
pub fn authentication_response(
    credential_id: &[u8],
    client_data_json: Option<&[u8]>,
    authenticator_data: &[u8],
    signature: &[u8],
    user_handle: &[u8],
) -> String {
    let id = b64url(credential_id);
    let mut response = Map::new();
    if let Some(client_data) = client_data_json {
        response.insert("clientDataJSON".to_owned(), json!(b64url(client_data)));
    }
    response.insert(
        "authenticatorData".to_owned(),
        json!(b64url(authenticator_data)),
    );
    response.insert("signature".to_owned(), json!(b64url(signature)));
    response.insert("userHandle".to_owned(), json!(b64url(user_handle)));
    json!({
        "id": id,
        "rawId": id,
        "type": "public-key",
        "response": Value::Object(response),
        "authenticatorAttachment": "platform",
        "clientExtensionResults": {},
    })
    .to_string()
}

/// The `RegistrationResponseJSON` of a registration.
#[must_use]
pub fn registration_response(
    credential_id: &[u8],
    client_data_json: Option<&[u8]>,
    attestation_object: &[u8],
    authenticator_data: &[u8],
    public_key_spki: &[u8],
    algorithm: i64,
) -> String {
    let id = b64url(credential_id);
    let mut response = Map::new();
    if let Some(client_data) = client_data_json {
        response.insert("clientDataJSON".to_owned(), json!(b64url(client_data)));
    }
    response.insert(
        "attestationObject".to_owned(),
        json!(b64url(attestation_object)),
    );
    response.insert("transports".to_owned(), json!(["internal", "hybrid"]));
    response.insert("publicKeyAlgorithm".to_owned(), json!(algorithm));
    response.insert("publicKey".to_owned(), json!(b64url(public_key_spki)));
    response.insert(
        "authenticatorData".to_owned(),
        json!(b64url(authenticator_data)),
    );
    json!({
        "id": id,
        "rawId": id,
        "type": "public-key",
        "response": Value::Object(response),
        "authenticatorAttachment": "platform",
        "clientExtensionResults": {},
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(extra: &str) -> String {
        format!(
            "{{\"challenge\":\"{}\",\"rpId\":\"example.com\"{extra}}}",
            b64url(&[7u8; 32])
        )
    }

    #[test]
    fn parses_a_get_request() {
        let json = get(&format!(
            ",\"allowCredentials\":[{{\"type\":\"public-key\",\"id\":\"{}\"}}],\"userVerification\":\"required\"",
            b64url(&[1, 2, 3])
        ));
        let parsed = parse_get(json.as_bytes()).unwrap();
        assert_eq!(parsed.challenge, vec![7u8; 32]);
        assert_eq!(parsed.rp_id.as_deref(), Some("example.com"));
        assert_eq!(parsed.allow, vec![vec![1, 2, 3]]);
        assert_eq!(parsed.user_verification, UserVerification::Required);
    }

    #[test]
    fn parses_a_create_request() {
        let json = format!(
            "{{\"challenge\":\"{}\",\"rp\":{{\"id\":\"example.com\",\"name\":\"Example\"}},\
             \"user\":{{\"id\":\"{}\",\"name\":\"alice\",\"displayName\":\"Alice\"}},\
             \"pubKeyCredParams\":[{{\"type\":\"public-key\",\"alg\":-7}},{{\"type\":\"public-key\",\"alg\":-257}}],\
             \"excludeCredentials\":[{{\"type\":\"public-key\",\"id\":\"{}\"}}]}}",
            b64url(&[9u8; 16]),
            b64url(&[4u8; 8]),
            b64url(&[5u8; 4])
        );
        let parsed = parse_create(json.as_bytes()).unwrap();
        assert_eq!(parsed.rp_id.as_deref(), Some("example.com"));
        assert_eq!(parsed.rp_name, "Example");
        assert_eq!(parsed.user_handle, vec![4u8; 8]);
        assert_eq!(parsed.algorithms, vec![-7, -257]);
        assert_eq!(parsed.exclude, vec![vec![5u8; 4]]);
        assert_eq!(parsed.user_verification, UserVerification::Preferred);
    }

    #[test]
    fn passkey_request_json_rejects_oversized_and_unknown_types() {
        let big = format!(
            "{{\"challenge\":\"AAAA\",\"pad\":\"{}\"}}",
            "a".repeat(MAX_REQUEST_LEN)
        );
        assert!(parse_get(big.as_bytes()).is_err());
        assert!(parse_get(b"[]").is_err());
        assert!(parse_get(b"not json").is_err());
        assert!(parse_get(b"{}").is_err());
        // Padded and standard base64 are refused.
        assert!(parse_get(br#"{"challenge":"AAAA=="}"#).is_err());
        assert!(parse_get(br#"{"challenge":"+/+/"}"#).is_err());
        // A descriptor of another type, a list that is too long, a wrong member type.
        let bad_type = get(r#","allowCredentials":[{"type":"other","id":"AAAA"}]"#);
        assert!(parse_get(bad_type.as_bytes()).is_err());
        let many = (0..=MAX_ALLOW_LIST)
            .map(|_| r#"{"type":"public-key","id":"AAAA"}"#)
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_get(get(&format!(",\"allowCredentials\":[{many}]")).as_bytes()).is_err());
        assert!(parse_get(get(r#","userVerification":"maybe""#).as_bytes()).is_err());
        assert!(parse_get(get(r#","allowCredentials":{}"#).as_bytes()).is_err());
        let bad_param = r#"{"challenge":"AAAA","user":{"id":"AAAA","name":"a"},
            "pubKeyCredParams":[{"type":"secret","alg":-7}]}"#;
        assert!(parse_create(bad_param.as_bytes()).is_err());
        // A user handle over 64 bytes.
        let long = format!(
            "{{\"challenge\":\"AAAA\",\"user\":{{\"id\":\"{}\",\"name\":\"a\"}}}}",
            b64url(&[1u8; 65])
        );
        assert!(parse_create(long.as_bytes()).is_err());
    }

    #[test]
    fn responses_are_plain_json_without_padding() {
        let json = authentication_response(&[1, 2, 3], None, &[4; 37], &[5; 70], &[6; 8]);
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "public-key");
        assert_eq!(value["id"], value["rawId"]);
        assert!(value["response"].get("clientDataJSON").is_none());
        assert!(!json.contains('='));
        let json = registration_response(&[1], Some(b"{}"), &[2; 10], &[3; 37], &[4; 91], -7);
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["response"]["publicKeyAlgorithm"], -7);
        assert_eq!(
            value["response"]["transports"],
            json!(["internal", "hybrid"])
        );
        assert!(value["response"].get("clientDataJSON").is_some());
    }
}

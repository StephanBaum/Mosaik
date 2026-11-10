//! The WebAuthn relying party (CRYPTO_SPEC §9.3, ADR-0077, ADR-0078).
//!
//! This is the only module that touches WebAuthn bytes. It wraps
//! `passkey-auth` and adds the checks the library does not make: the frame
//! check (`crossOrigin`, `topOrigin`), the algorithm and attachment policy,
//! `residentKey: discouraged`, and a user handle that, when present, must
//! match. Errors of the library carry request data (the received origin, for
//! example), so they are mapped to [`RpError`] and dropped: nothing here logs
//! or returns library error text.

use base64ct::{Base64UrlUnpadded, Encoding};
use passkey_auth::{
    Attachment, AuthenticationResponse, AuthenticationState, CosePublicKey, CredentialId, Error,
    PasskeyCredential, RegistrationResponse, RegistrationState, Webauthn,
};
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::config::WebauthnConfig;

/// The relying party name shown by browsers.
const RP_NAME: &str = "Mosaic";
/// The longest COSE public key stored.
pub const MAX_PUBLIC_KEY_LEN: usize = 1024;

/// Why a ceremony step failed. Deliberately without detail: the page sees
/// `two_factor_invalid` for all of them.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RpError {
    /// The request or the response did not verify.
    #[error("the WebAuthn response is not valid")]
    Invalid,
    /// The sign count did not rise (a possible cloned authenticator).
    #[error("the signature counter did not rise")]
    CounterReplay,
}

/// A credential as stored (the database row).
#[derive(Clone)]
pub struct StoredCredential {
    /// The credential ID.
    pub id: Vec<u8>,
    /// The COSE public key.
    pub public_key: Vec<u8>,
    /// The last sign count.
    pub sign_count: u32,
}

/// The ceremony state a handoff holds between options and finish.
pub enum CeremonyState {
    /// A registration in progress.
    Registration(RegistrationState),
    /// An assertion in progress.
    Assertion(AuthenticationState),
}

/// A verified registration.
pub struct NewCredential {
    /// The credential ID.
    pub id: Vec<u8>,
    /// The COSE public key (at most [`MAX_PUBLIC_KEY_LEN`] bytes).
    pub public_key: Vec<u8>,
    /// The sign count in the authenticator data.
    pub sign_count: u32,
}

/// A verified assertion.
pub struct AssertionOutcome {
    /// The credential that signed.
    pub credential_id: Vec<u8>,
    /// The new sign count (to store with a compare-and-set).
    pub new_count: u32,
}

/// The relying party.
pub struct Rp {
    wa: Webauthn,
}

/// The browser's `PublicKeyCredential` JSON (`toJSON()`): the members the
/// page may send, and nothing else.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialJson {
    id: String,
    #[serde(rename = "rawId")]
    _raw_id: Option<IgnoredAny>,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "authenticatorAttachment")]
    _attachment: Option<IgnoredAny>,
    #[serde(rename = "clientExtensionResults")]
    _extensions: Option<IgnoredAny>,
    response: ResponseJson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseJson {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    #[serde(rename = "attestationObject")]
    attestation_object: Option<String>,
    #[serde(rename = "authenticatorData")]
    authenticator_data: Option<String>,
    signature: Option<String>,
    #[serde(rename = "userHandle")]
    user_handle: Option<String>,
    transports: Option<Vec<String>>,
    #[serde(rename = "publicKey")]
    _public_key: Option<IgnoredAny>,
    #[serde(rename = "publicKeyAlgorithm")]
    _public_key_algorithm: Option<IgnoredAny>,
}

/// The two members of `clientDataJSON` the library does not look at.
#[derive(Deserialize)]
struct FrameCheck {
    #[serde(rename = "crossOrigin")]
    cross_origin: Option<bool>,
    #[serde(rename = "topOrigin")]
    top_origin: Option<IgnoredAny>,
}

fn parse_credential(bytes: &[u8]) -> Result<CredentialJson, RpError> {
    let c: CredentialJson = serde_json::from_slice(bytes).map_err(|_| RpError::Invalid)?;
    if c.kind.as_deref().is_some_and(|k| k != "public-key") {
        return Err(RpError::Invalid);
    }
    frame_check(&c.response.client_data_json)?;
    Ok(c)
}

/// Refuses `crossOrigin: true` and any `topOrigin` (CRYPTO_SPEC §9.3).
fn frame_check(client_data_b64: &str) -> Result<(), RpError> {
    let raw = Base64UrlUnpadded::decode_vec(client_data_b64).map_err(|_| RpError::Invalid)?;
    let f: FrameCheck = serde_json::from_slice(&raw).map_err(|_| RpError::Invalid)?;
    if f.cross_origin == Some(true) || f.top_origin.is_some() {
        return Err(RpError::Invalid);
    }
    Ok(())
}

/// The host of a canonical origin (`scheme://host[:port]`), `None` for an IP
/// literal.
fn rp_host(origin: &str) -> Option<&str> {
    let rest = origin.split_once("://")?.1;
    if rest.starts_with('[') {
        return None;
    }
    let host = rest.split(':').next().unwrap_or(rest);
    if host.is_empty() || host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    Some(host)
}

impl Rp {
    /// The relying party of `canonical_origin`; `None` when the origin host is
    /// an IP literal or `[webauthn] enabled = false`.
    #[must_use]
    pub fn new(canonical_origin: &str, cfg: &WebauthnConfig) -> Option<Self> {
        if !cfg.enabled {
            return None;
        }
        let host = rp_host(canonical_origin)?;
        let wa = Webauthn::new(host, RP_NAME, canonical_origin)
            .strict_base64(true)
            .require_user_handle(false)
            .authenticator_attachment(Attachment::Any);
        Some(Self { wa })
    }

    /// The options for `navigator.credentials.create()`, and the state to
    /// keep. `residentKey` is overwritten to `discouraged`.
    ///
    /// # Errors
    /// [`RpError::Invalid`] if the options cannot be encoded.
    pub fn registration_options(
        &self,
        user_handle: &[u8; 16],
        email: &str,
        exclude: &[StoredCredential],
    ) -> Result<(CeremonyState, serde_json::Value), RpError> {
        let ids: Vec<CredentialId> = exclude.iter().map(|c| CredentialId(c.id.clone())).collect();
        let (chal, state) = self.wa.start_registration(user_handle, email, email, &ids);
        let mut value = serde_json::to_value(&chal).map_err(|_| RpError::Invalid)?;
        if let Some(sel) = value
            .get_mut("authenticatorSelection")
            .and_then(serde_json::Value::as_object_mut)
        {
            sel.insert("residentKey".to_owned(), "discouraged".into());
            sel.insert("requireResidentKey".to_owned(), false.into());
        }
        Ok((CeremonyState::Registration(state), value))
    }

    /// Verifies the page's `PublicKeyCredential` JSON against a registration
    /// state.
    ///
    /// # Errors
    /// [`RpError::Invalid`] for anything that does not verify.
    pub fn finish_registration(
        &self,
        state: &RegistrationState,
        response: &[u8],
    ) -> Result<NewCredential, RpError> {
        let c = parse_credential(response)?;
        let attestation_object = c.response.attestation_object.ok_or(RpError::Invalid)?;
        let resp = RegistrationResponse {
            id: c.id,
            transports: c.response.transports.unwrap_or_default(),
            attestation_object,
            client_data_json: c.response.client_data_json,
        };
        let cred = self
            .wa
            .finish_registration(state, &resp)
            .map_err(|_| RpError::Invalid)?;
        // The credential ID in the response must be the attested one.
        if Base64UrlUnpadded::decode_vec(&resp.id).ok().as_deref() != Some(cred.id.as_bytes()) {
            return Err(RpError::Invalid);
        }
        let public_key = cred.public_key_cose.as_bytes().to_vec();
        if public_key.len() > MAX_PUBLIC_KEY_LEN || !(16..=1023).contains(&cred.id.as_bytes().len())
        {
            return Err(RpError::Invalid);
        }
        Ok(NewCredential {
            id: cred.id.as_bytes().to_vec(),
            public_key,
            sign_count: cred.counter,
        })
    }

    /// The options for `navigator.credentials.get()`: the account's
    /// credentials are the allow list, the user handle is recorded.
    ///
    /// # Errors
    /// [`RpError::Invalid`] if the options cannot be encoded.
    pub fn assertion_options(
        &self,
        user_handle: &[u8; 16],
        allow: &[StoredCredential],
    ) -> Result<(CeremonyState, serde_json::Value), RpError> {
        let ids: Vec<CredentialId> = allow.iter().map(|c| CredentialId(c.id.clone())).collect();
        let (chal, state) = self.wa.start_authentication_for_user(user_handle, &ids);
        let value = serde_json::to_value(&chal).map_err(|_| RpError::Invalid)?;
        Ok((CeremonyState::Assertion(state), value))
    }

    /// Verifies the page's assertion JSON against an assertion state and the
    /// account's credentials.
    ///
    /// # Errors
    /// [`RpError::CounterReplay`] if the counter did not rise,
    /// [`RpError::Invalid`] for anything else.
    pub fn finish_assertion(
        &self,
        state: &AuthenticationState,
        response: &[u8],
        creds: &[StoredCredential],
    ) -> Result<AssertionOutcome, RpError> {
        let c = parse_credential(response)?;
        let asserted = Base64UrlUnpadded::decode_vec(&c.id).map_err(|_| RpError::Invalid)?;
        let stored = creds
            .iter()
            .find(|s| s.id == asserted)
            .ok_or(RpError::Invalid)?;
        let resp = AuthenticationResponse {
            id: c.id,
            authenticator_data: c.response.authenticator_data.ok_or(RpError::Invalid)?,
            signature: c.response.signature.ok_or(RpError::Invalid)?,
            client_data_json: c.response.client_data_json,
            user_handle: c.response.user_handle,
        };
        let cred = PasskeyCredential {
            id: CredentialId(stored.id.clone()),
            public_key_cose: CosePublicKey(stored.public_key.clone()),
            counter: stored.sign_count,
            transports: Vec::new(),
            aaguid: [0; 16],
        };
        match self.wa.finish_authentication(state, &resp, &cred) {
            Ok(ok) => Ok(AssertionOutcome {
                credential_id: ok.credential_id.as_bytes().to_vec(),
                new_count: ok.new_counter,
            }),
            Err(Error::CounterReplay { .. }) => Err(RpError::CounterReplay),
            Err(_) => Err(RpError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_literal_and_disabled_mean_no_relying_party() {
        let on = WebauthnConfig { enabled: true };
        assert!(Rp::new("https://sync.example.com", &on).is_some());
        assert!(Rp::new("https://sync.example.com:8443", &on).is_some());
        assert!(Rp::new("http://127.0.0.1:8743", &on).is_none());
        assert!(Rp::new("http://[::1]:1", &on).is_none());
        assert!(Rp::new("https://192.0.2.1", &on).is_none());
        assert!(
            Rp::new(
                "https://sync.example.com",
                &WebauthnConfig { enabled: false }
            )
            .is_none()
        );
    }

    #[test]
    fn frame_check_refuses_cross_origin_and_top_origin() {
        let enc = |s: &str| Base64UrlUnpadded::encode_string(s.as_bytes());
        assert!(frame_check(&enc(r#"{"type":"webauthn.get","crossOrigin":false}"#)).is_ok());
        assert!(frame_check(&enc(r#"{"type":"webauthn.get"}"#)).is_ok());
        assert!(frame_check(&enc(r#"{"crossOrigin":true}"#)).is_err());
        assert!(frame_check(&enc(r#"{"topOrigin":"https://evil.example"}"#)).is_err());
        assert!(frame_check(&enc("not json")).is_err());
        assert!(frame_check("###").is_err());
    }
}

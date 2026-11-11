//! One registration ([`create`]) or one assertion ([`assert`]).

use mosaic_core::SecretVec;
use mosaic_core::model::PasskeyCredential;
use sha2::{Digest, Sha256};

use crate::encode::{
    attestation_object_none, authenticator_data, b64url, base_flags, client_data_json,
};
use crate::key::{Es256Key, new_credential_id};
use crate::{
    COSE_ALG_ES256, Ceremony, Error, MAX_CHALLENGE_LEN, MAX_CREDENTIAL_ID_LEN, MAX_RP_ID_LEN,
    MAX_USER_HANDLE_LEN, MIN_CHALLENGE_LEN, flags,
};

/// The algorithm to use for a request that offers `offered` (the COSE
/// identifiers of `pubKeyCredParams`): ES256 if the list is empty or
/// contains -7.
///
/// # Errors
/// [`Error::UnsupportedAlgorithm`] if a non-empty list lacks -7.
pub fn select_algorithm(offered: &[i64]) -> Result<i64, Error> {
    if offered.is_empty() || offered.contains(&COSE_ALG_ES256) {
        Ok(COSE_ALG_ES256)
    } else {
        Err(Error::UnsupportedAlgorithm)
    }
}

/// Checks the rp ID and the client data both ceremonies share.
fn check_common(rp_id: &str, client_data: &ClientData<'_>) -> Result<(), Error> {
    if rp_id.is_empty() || rp_id.len() > MAX_RP_ID_LEN {
        return Err(Error::InvalidInput);
    }
    if let ClientData::Json { challenge, .. } = *client_data
        && !(MIN_CHALLENGE_LEN..=MAX_CHALLENGE_LEN).contains(&challenge.len())
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

/// `message = authenticatorData || clientDataHash`.
fn assertion_message(authenticator_data: &[u8], client_data_hash: &[u8; 32]) -> Vec<u8> {
    let mut message = Vec::with_capacity(authenticator_data.len() + 32);
    message.extend_from_slice(authenticator_data);
    message.extend_from_slice(client_data_hash);
    message
}

/// Where the client data of a ceremony comes from (CRYPTO_SPEC §16, ADR-0093).
#[derive(Clone, Copy, Debug)]
pub enum ClientData<'a> {
    /// The authenticator builds `clientDataJSON` (the extension, and Android
    /// callers that are not privileged browsers).
    Json {
        /// The origin as the platform reports it.
        origin: &'a str,
        /// The relying party's challenge, 16 to 1024 bytes.
        challenge: &'a [u8],
    },
    /// The platform built `clientDataJSON` and passes `SHA-256` of it (iOS, and
    /// Android privileged browsers). The authenticator never sees the JSON.
    Hash(&'a [u8; 32]),
}

impl ClientData<'_> {
    /// The `clientDataJSON` bytes (`None` in hash mode) and their SHA-256.
    fn resolve(&self, ceremony: Ceremony) -> (Option<Vec<u8>>, [u8; 32]) {
        match *self {
            Self::Json { origin, challenge } => {
                let json = client_data_json(ceremony, challenge, origin);
                let hash: [u8; 32] = Sha256::digest(&json).into();
                (Some(json), hash)
            }
            Self::Hash(hash) => (None, *hash),
        }
    }
}

/// The `origin` of an Android app that is not a privileged browser:
/// `"android:apk-key-hash:"` followed by the unpadded base64url of the
/// SHA-256 of the app's signing certificate.
#[must_use]
pub fn android_app_origin(cert_sha256: &[u8; 32]) -> String {
    format!("android:apk-key-hash:{}", b64url(cert_sha256))
}

/// The inputs of a registration. The caller has validated `rp_id` against
/// `origin` (`mosaic_origin::webauthn_rp_id`).
pub struct CreateInput<'a> {
    /// The validated relying party ID.
    pub rp_id: &'a str,
    /// The origin as the browser reports it, for `clientDataJSON`.
    pub origin: &'a str,
    /// The relying party's challenge, 16 to 1024 bytes.
    pub challenge: &'a [u8],
    /// The user handle, 1 to 64 bytes.
    pub user_handle: &'a [u8],
    /// The COSE algorithms the relying party offers (empty means any).
    pub algorithms: &'a [i64],
    /// Whether the user was verified (the UV flag).
    pub user_verified: bool,
}

/// The inputs of a registration with a [`ClientData`] mode. The caller has
/// validated `rp_id` against the origin (`mosaic_origin::webauthn_rp_id`).
#[derive(Debug)]
pub struct CreateRequest<'a> {
    /// The validated relying party ID.
    pub rp_id: &'a str,
    /// Where the client data comes from.
    pub client_data: ClientData<'a>,
    /// The user handle, 1 to 64 bytes.
    pub user_handle: &'a [u8],
    /// The COSE algorithms the relying party offers (empty means any).
    pub algorithms: &'a [i64],
    /// Whether the user was verified (the UV flag).
    pub user_verified: bool,
}

/// The result of a registration. The caller stores `private_key_pkcs8` and
/// `credential_id` in a `passkey` item **before** it returns the rest.
pub struct Created {
    /// The new credential ID.
    pub credential_id: Vec<u8>,
    /// The private key as PKCS#8 DER, for `PasskeyCredential::private_key`.
    pub private_key_pkcs8: SecretVec,
    /// The `clientDataJSON` bytes the relying party hashes.
    pub client_data_json: Vec<u8>,
    /// The authenticator data, with the attested credential data.
    pub authenticator_data: Vec<u8>,
    /// The CBOR attestation object (format `none`).
    pub attestation_object: Vec<u8>,
    /// The public key as an SPKI, for `getPublicKey()`.
    pub public_key_spki: Vec<u8>,
    /// The COSE algorithm of the key (-7).
    pub algorithm: i64,
}

/// The result of a registration in either client-data mode. The caller stores
/// the key and credential ID **before** it returns the rest.
pub struct ClientCreated {
    /// The new credential ID.
    pub credential_id: Vec<u8>,
    /// The private key as PKCS#8 DER, for `PasskeyCredential::private_key`.
    pub private_key_pkcs8: SecretVec,
    /// The `clientDataJSON` bytes; `None` in [`ClientData::Hash`] mode.
    pub client_data_json: Option<Vec<u8>>,
    /// The authenticator data, with the attested credential data.
    pub authenticator_data: Vec<u8>,
    /// The CBOR attestation object (format `none`).
    pub attestation_object: Vec<u8>,
    /// The public key as an SPKI, for `getPublicKey()`.
    pub public_key_spki: Vec<u8>,
    /// The COSE algorithm of the key (-7).
    pub algorithm: i64,
}

/// Registers a new credential with a fresh key and credential ID.
///
/// # Errors
/// [`Error::UnsupportedAlgorithm`], [`Error::InvalidInput`] (challenge, user
/// handle or rp ID out of range) or [`Error::Rng`].
pub fn create(input: &CreateInput<'_>) -> Result<Created, Error> {
    create_request(&json_create_request(input)).map(into_created)
}

/// Registers a new credential with a fresh key and credential ID, in either
/// client-data mode.
///
/// # Errors
/// As [`create`].
pub fn create_request(request: &CreateRequest<'_>) -> Result<ClientCreated, Error> {
    select_algorithm(request.algorithms)?;
    check_create_request(request)?;
    let key = Es256Key::generate()?;
    build_created(request, &key, new_credential_id()?)
}

fn json_create_request<'a>(input: &CreateInput<'a>) -> CreateRequest<'a> {
    CreateRequest {
        rp_id: input.rp_id,
        client_data: ClientData::Json {
            origin: input.origin,
            challenge: input.challenge,
        },
        user_handle: input.user_handle,
        algorithms: input.algorithms,
        user_verified: input.user_verified,
    }
}

fn into_created(c: ClientCreated) -> Created {
    Created {
        credential_id: c.credential_id,
        private_key_pkcs8: c.private_key_pkcs8,
        client_data_json: c.client_data_json.unwrap_or_default(),
        authenticator_data: c.authenticator_data,
        attestation_object: c.attestation_object,
        public_key_spki: c.public_key_spki,
        algorithm: c.algorithm,
    }
}

fn check_create_request(request: &CreateRequest<'_>) -> Result<(), Error> {
    check_common(request.rp_id, &request.client_data)?;
    if !(1..=MAX_USER_HANDLE_LEN).contains(&request.user_handle.len()) {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

/// [`create`] with a given key and credential ID (the known-answer tests
/// only; the public API always draws both fresh).
#[cfg(test)]
pub(crate) fn create_with(
    input: &CreateInput<'_>,
    key: &Es256Key,
    credential_id: Vec<u8>,
) -> Result<Created, Error> {
    build_created(&json_create_request(input), key, credential_id).map(into_created)
}

/// [`create_request`] with a given key and credential ID (tests only).
#[cfg(test)]
pub(crate) fn create_request_with(
    request: &CreateRequest<'_>,
    key: &Es256Key,
    credential_id: Vec<u8>,
) -> Result<ClientCreated, Error> {
    build_created(request, key, credential_id)
}

/// The registration for a given key and credential ID.
///
/// # Errors
/// As [`create`], and [`Error::InvalidInput`] for a credential ID outside
/// `1..=MAX_CREDENTIAL_ID_LEN` bytes.
fn build_created(
    request: &CreateRequest<'_>,
    key: &Es256Key,
    credential_id: Vec<u8>,
) -> Result<ClientCreated, Error> {
    let algorithm = select_algorithm(request.algorithms)?;
    check_create_request(request)?;
    if !(1..=MAX_CREDENTIAL_ID_LEN).contains(&credential_id.len()) {
        return Err(Error::InvalidInput);
    }
    let authenticator_data = authenticator_data(
        request.rp_id,
        base_flags(request.user_verified, flags::AT),
        0,
        Some((&credential_id, &key.cose_public_key())),
    )?;
    let (client_data_json, _hash) = request.client_data.resolve(Ceremony::Create);
    Ok(ClientCreated {
        attestation_object: attestation_object_none(&authenticator_data)?,
        client_data_json,
        private_key_pkcs8: key.to_pkcs8_der()?,
        public_key_spki: key.public_key_spki_der(),
        authenticator_data,
        credential_id,
        algorithm,
    })
}

/// The inputs of an assertion. The caller has validated `rp_id` against
/// `origin` and chosen the credential.
pub struct GetInput<'a> {
    /// The validated relying party ID.
    pub rp_id: &'a str,
    /// The origin as the browser reports it, for `clientDataJSON`.
    pub origin: &'a str,
    /// The relying party's challenge, 16 to 1024 bytes.
    pub challenge: &'a [u8],
    /// Whether the user was verified (the UV flag).
    pub user_verified: bool,
}

/// The inputs of an assertion with a [`ClientData`] mode. The caller has
/// validated `rp_id` and chosen the credential.
#[derive(Debug)]
pub struct GetRequest<'a> {
    /// The validated relying party ID.
    pub rp_id: &'a str,
    /// Where the client data comes from.
    pub client_data: ClientData<'a>,
    /// Whether the user was verified (the UV flag).
    pub user_verified: bool,
}

/// A signed assertion that is **not yet released**: its fields are private,
/// so the signature cannot reach a relying party before the caller has dealt
/// with the sign count (ADR-0039 D5).
///
/// 1. [`count_to_persist`](Self::count_to_persist): `Some(n)` means store
///    `passkey.sign_count = n` (`ItemEdit::set_passkey_sign_count`), wait for
///    the store write to succeed, then call
///    [`into_response`](Self::into_response) with a [`CountPersisted`].
/// 2. `None` (a counter that is, and stays, 0): call
///    [`into_response_without_count`](Self::into_response_without_count).
///
/// An assertion made in hash mode ([`assert_request`]) is released through
/// [`into_client_response`](Self::into_client_response) or
/// [`into_client_response_without_count`](Self::into_client_response_without_count).
#[must_use = "an assertion is only released through `into_response*`"]
pub struct PendingAssertion {
    credential_id: Vec<u8>,
    client_data_json: Option<Vec<u8>>,
    authenticator_data: Vec<u8>,
    signature: Vec<u8>,
    user_handle: Vec<u8>,
    persist_count: Option<u64>,
}

/// Proof that the new sign count is stored.
#[derive(Clone, Copy, Debug)]
#[must_use = "pass the token to `PendingAssertion::into_response`"]
pub struct CountPersisted(());

impl CountPersisted {
    /// The token. Call only after `set_passkey_sign_count` succeeded and the
    /// store write is done; the type cannot check that.
    pub const fn after_store_write() -> Self {
        Self(())
    }
}

/// The released result of an assertion made through [`assert`] (always
/// [`ClientData::Json`] mode).
#[derive(Debug)]
pub struct AssertionResponse {
    /// The credential ID that signed.
    pub credential_id: Vec<u8>,
    /// The `clientDataJSON` bytes the relying party hashes.
    pub client_data_json: Vec<u8>,
    /// The authenticator data (37 bytes).
    pub authenticator_data: Vec<u8>,
    /// The DER ECDSA signature over `authenticatorData || SHA-256(clientDataJSON)`.
    pub signature: Vec<u8>,
    /// The stored user handle.
    pub user_handle: Vec<u8>,
}

/// The released result of an assertion in either client-data mode.
#[derive(Debug)]
pub struct ClientAssertionResponse {
    /// The credential ID that signed.
    pub credential_id: Vec<u8>,
    /// The `clientDataJSON` bytes; `None` in [`ClientData::Hash`] mode.
    pub client_data_json: Option<Vec<u8>>,
    /// The authenticator data (37 bytes).
    pub authenticator_data: Vec<u8>,
    /// The DER ECDSA signature over `authenticatorData || clientDataHash`.
    pub signature: Vec<u8>,
    /// The stored user handle.
    pub user_handle: Vec<u8>,
}

impl PendingAssertion {
    /// The sign count to store before the response is released: `Some(n)`
    /// for `passkey.sign_count = n`, `None` for a counter that stays 0.
    #[must_use]
    pub const fn count_to_persist(&self) -> Option<u64> {
        self.persist_count
    }

    /// Releases the response after the count was stored. For an assertion
    /// made with [`assert`]; `client_data_json` is empty for one made in hash
    /// mode, so use [`into_client_response`](Self::into_client_response) there.
    #[must_use]
    pub fn into_response(self, _persisted: CountPersisted) -> AssertionResponse {
        self.release_json()
    }

    /// Releases the response of an assertion that has no count to store.
    ///
    /// # Errors
    /// [`Error::InvalidInput`] if a count is pending: it must be stored and
    /// the response released with [`into_response`](Self::into_response).
    pub fn into_response_without_count(self) -> Result<AssertionResponse, Error> {
        if self.persist_count.is_some() {
            return Err(Error::InvalidInput);
        }
        Ok(self.release_json())
    }

    /// Releases the response after the count was stored, in either mode.
    #[must_use]
    pub fn into_client_response(self, _persisted: CountPersisted) -> ClientAssertionResponse {
        self.release()
    }

    /// Releases the response of an assertion that has no count to store, in
    /// either mode.
    ///
    /// # Errors
    /// [`Error::InvalidInput`] if a count is pending.
    pub fn into_client_response_without_count(self) -> Result<ClientAssertionResponse, Error> {
        if self.persist_count.is_some() {
            return Err(Error::InvalidInput);
        }
        Ok(self.release())
    }

    fn release(self) -> ClientAssertionResponse {
        ClientAssertionResponse {
            credential_id: self.credential_id,
            client_data_json: self.client_data_json,
            authenticator_data: self.authenticator_data,
            signature: self.signature,
            user_handle: self.user_handle,
        }
    }

    fn release_json(self) -> AssertionResponse {
        let r = self.release();
        AssertionResponse {
            credential_id: r.credential_id,
            client_data_json: r.client_data_json.unwrap_or_default(),
            authenticator_data: r.authenticator_data,
            signature: r.signature,
            user_handle: r.user_handle,
        }
    }
}

/// The sign count to put in the authenticator data and the value to persist
/// (ADR-0039 D5): a counter of 0 stays 0 and is never written; any other
/// counter goes up by one, saturating at `u32::MAX`.
fn next_count(stored: u64) -> (u32, Option<u64>) {
    if stored == 0 {
        return (0, None);
    }
    let next = u32::try_from(stored.saturating_add(1)).unwrap_or(u32::MAX);
    (next, Some(u64::from(next)))
}

/// Signs an assertion with a stored credential.
///
/// `stored_count` is the item's `passkey_sign_count`.
///
/// # Errors
/// [`Error::InvalidInput`] (challenge or rp ID out of range, or the
/// credential belongs to another rp ID), [`Error::UnsupportedAlgorithm`]
/// (the credential is not ES256) or [`Error::InvalidKey`] (the stored key
/// does not load).
pub fn assert(
    input: &GetInput<'_>,
    credential: &PasskeyCredential,
    stored_count: u64,
) -> Result<PendingAssertion, Error> {
    assert_request(
        &GetRequest {
            rp_id: input.rp_id,
            client_data: ClientData::Json {
                origin: input.origin,
                challenge: input.challenge,
            },
            user_verified: input.user_verified,
        },
        credential,
        stored_count,
    )
}

/// Signs an assertion with a stored credential, in either client-data mode.
/// The signed message is `authenticatorData || clientDataHash`, where the hash
/// is the given one or `SHA-256(clientDataJSON)`.
///
/// # Errors
/// As [`assert`].
pub fn assert_request(
    input: &GetRequest<'_>,
    credential: &PasskeyCredential,
    stored_count: u64,
) -> Result<PendingAssertion, Error> {
    check_common(input.rp_id, &input.client_data)?;
    if credential.algorithm != COSE_ALG_ES256 {
        return Err(Error::UnsupportedAlgorithm);
    }
    if credential.rp_id.as_str() != input.rp_id {
        return Err(Error::InvalidInput);
    }
    credential.validate().map_err(|_| Error::InvalidInput)?;
    let key = Es256Key::from_pkcs8_der(credential.private_key.expose_secret())?;
    let (count, persist_count) = next_count(stored_count);
    let authenticator_data =
        authenticator_data(input.rp_id, base_flags(input.user_verified, 0), count, None)?;
    let (client_data_json, hash) = input.client_data.resolve(Ceremony::Get);
    let signature = key.sign(&assertion_message(&authenticator_data, &hash));
    Ok(PendingAssertion {
        credential_id: credential.credential_id.clone(),
        user_handle: credential.user_handle.clone(),
        client_data_json,
        authenticator_data,
        signature,
        persist_count,
    })
}

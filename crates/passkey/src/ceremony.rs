//! One registration ([`create`]) or one assertion ([`assert`]).

use mosaic_core::SecretVec;
use mosaic_core::model::PasskeyCredential;
use sha2::{Digest, Sha256};

use crate::encode::{attestation_object_none, authenticator_data, base_flags, client_data_json};
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

/// Checks the inputs both ceremonies share.
fn check_common(rp_id: &str, challenge: &[u8]) -> Result<(), Error> {
    if rp_id.is_empty()
        || rp_id.len() > MAX_RP_ID_LEN
        || !(MIN_CHALLENGE_LEN..=MAX_CHALLENGE_LEN).contains(&challenge.len())
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

/// `message = authenticatorData || SHA-256(clientDataJSON)`.
fn assertion_message(authenticator_data: &[u8], client_data_json: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(authenticator_data.len() + 32);
    message.extend_from_slice(authenticator_data);
    message.extend_from_slice(&Sha256::digest(client_data_json));
    message
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

/// Registers a new credential with a fresh key and credential ID.
///
/// # Errors
/// [`Error::UnsupportedAlgorithm`], [`Error::InvalidInput`] (challenge, user
/// handle or rp ID out of range) or [`Error::Rng`].
pub fn create(input: &CreateInput<'_>) -> Result<Created, Error> {
    select_algorithm(input.algorithms)?;
    check_create_input(input)?;
    let key = Es256Key::generate()?;
    build_created(input, &key, new_credential_id()?)
}

fn check_create_input(input: &CreateInput<'_>) -> Result<(), Error> {
    check_common(input.rp_id, input.challenge)?;
    if !(1..=MAX_USER_HANDLE_LEN).contains(&input.user_handle.len()) {
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
    build_created(input, key, credential_id)
}

/// The registration for a given key and credential ID.
///
/// # Errors
/// As [`create`], and [`Error::InvalidInput`] for a credential ID outside
/// `1..=MAX_CREDENTIAL_ID_LEN` bytes.
fn build_created(
    input: &CreateInput<'_>,
    key: &Es256Key,
    credential_id: Vec<u8>,
) -> Result<Created, Error> {
    let algorithm = select_algorithm(input.algorithms)?;
    check_create_input(input)?;
    if !(1..=MAX_CREDENTIAL_ID_LEN).contains(&credential_id.len()) {
        return Err(Error::InvalidInput);
    }
    let authenticator_data = authenticator_data(
        input.rp_id,
        base_flags(input.user_verified, flags::AT),
        0,
        Some((&credential_id, &key.cose_public_key())),
    )?;
    Ok(Created {
        attestation_object: attestation_object_none(&authenticator_data)?,
        client_data_json: client_data_json(Ceremony::Create, input.challenge, input.origin),
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
#[must_use = "an assertion is only released through `into_response*`"]
pub struct PendingAssertion {
    credential_id: Vec<u8>,
    client_data_json: Vec<u8>,
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

/// The released result of an assertion.
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

impl PendingAssertion {
    /// The sign count to store before the response is released: `Some(n)`
    /// for `passkey.sign_count = n`, `None` for a counter that stays 0.
    #[must_use]
    pub const fn count_to_persist(&self) -> Option<u64> {
        self.persist_count
    }

    /// Releases the response after the count was stored.
    #[must_use]
    pub fn into_response(self, _persisted: CountPersisted) -> AssertionResponse {
        self.release()
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
        Ok(self.release())
    }

    fn release(self) -> AssertionResponse {
        AssertionResponse {
            credential_id: self.credential_id,
            client_data_json: self.client_data_json,
            authenticator_data: self.authenticator_data,
            signature: self.signature,
            user_handle: self.user_handle,
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
    check_common(input.rp_id, input.challenge)?;
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
    let client_data_json = client_data_json(Ceremony::Get, input.challenge, input.origin);
    let signature = key.sign(&assertion_message(&authenticator_data, &client_data_json));
    Ok(PendingAssertion {
        credential_id: credential.credential_id.clone(),
        user_handle: credential.user_handle.clone(),
        client_data_json,
        authenticator_data,
        signature,
        persist_count,
    })
}

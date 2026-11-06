//! The ES256 key: generation, PKCS#8 storage, COSE and SPKI export, signing.

use core::fmt;

use mosaic_core::{SecretBytes, SecretVec};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, ObjectIdentifier, PrivateKeyInfoRef};

use crate::{COSE_KEY_LEN, CREDENTIAL_ID_LEN, Error};

/// How many random scalars `generate` draws before it gives up. A uniform
/// 256-bit value is invalid with probability about 2^-32.
const MAX_KEY_DRAWS: usize = 8;

/// `A5 01 02 03 26 20 01 21 58 20`: map(5), kty EC2, alg -7, crv P-256, x.
const COSE_PREFIX: [u8; 10] = [0xA5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20];
/// `22 58 20`: the y coordinate's key and byte-string header.
const COSE_Y_HEAD: [u8; 3] = [0x22, 0x58, 0x20];
/// The SPKI of a P-256 key up to the uncompressed point (91 bytes in all).
const SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01, 0x06, 0x08, 0x2A,
    0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

/// A P-256 signing key for ES256. Not `Clone`; `Debug` prints
/// `Es256Key([REDACTED])`; the scalar is zeroized on drop.
pub struct Es256Key(SigningKey);

impl Es256Key {
    /// A fresh key: 32 random bytes from the OS CSPRNG read as a scalar,
    /// redrawn if zero or not below the group order (FIPS 186-5 A.4.2).
    ///
    /// # Errors
    /// [`Error::Rng`] if the CSPRNG fails or eight draws are all invalid.
    pub fn generate() -> Result<Self, Error> {
        for _ in 0..MAX_KEY_DRAWS {
            let scalar = SecretBytes::<32>::random().map_err(|_| Error::Rng)?;
            if let Ok(key) = SigningKey::from_slice(scalar.expose_secret()) {
                return Ok(Self(key));
            }
        }
        Err(Error::Rng)
    }

    /// Loads a key from PKCS#8 DER (as stored in a passkey item).
    ///
    /// # Errors
    /// [`Error::InvalidKey`] if `der` is not a PKCS#8 P-256 private key.
    pub fn from_pkcs8_der(der: &[u8]) -> Result<Self, Error> {
        SigningKey::from_pkcs8_der(der)
            .map(Self)
            .map_err(|_| Error::InvalidKey)
    }

    /// The key as PKCS#8 DER (the scalar and the public key), in a zeroizing
    /// buffer.
    ///
    /// # Errors
    /// [`Error::InvalidKey`] if encoding or the allocation fails.
    pub fn to_pkcs8_der(&self) -> Result<SecretVec, Error> {
        let document = self.0.to_pkcs8_der().map_err(|_| Error::InvalidKey)?;
        SecretVec::from_slice(document.as_bytes()).map_err(|_| Error::InvalidKey)
    }

    /// The public key's affine coordinates, big-endian.
    fn coordinates(&self) -> ([u8; 32], [u8; 32]) {
        let point = self.0.verifying_key().to_sec1_point(false);
        let (mut x, mut y) = ([0u8; 32], [0u8; 32]);
        if let (Some(px), Some(py)) = (point.x(), point.y()) {
            x.copy_from_slice(px);
            y.copy_from_slice(py);
        }
        (x, y)
    }

    /// The public key as a `COSE_Key` (RFC 9052) in CTAP2 canonical order:
    /// `A5 01 02 03 26 20 01 21 58 20 X 22 58 20 Y`, 77 bytes.
    #[must_use]
    pub fn cose_public_key(&self) -> [u8; COSE_KEY_LEN] {
        let (x, y) = self.coordinates();
        let mut out = [0u8; COSE_KEY_LEN];
        let mut at = 0;
        let parts = [
            COSE_PREFIX.as_slice(),
            x.as_slice(),
            COSE_Y_HEAD.as_slice(),
            y.as_slice(),
        ];
        for part in parts {
            if let Some(dest) = out.get_mut(at..at + part.len()) {
                dest.copy_from_slice(part);
            }
            at += part.len();
        }
        out
    }

    /// The public key as a DER `SubjectPublicKeyInfo` (what
    /// `AuthenticatorAttestationResponse.getPublicKey()` returns), 91 bytes.
    #[must_use]
    pub fn public_key_spki_der(&self) -> Vec<u8> {
        let (x, y) = self.coordinates();
        let mut out = Vec::with_capacity(SPKI_PREFIX.len() + 65);
        out.extend_from_slice(&SPKI_PREFIX);
        out.push(0x04);
        out.extend_from_slice(&x);
        out.extend_from_slice(&y);
        out
    }

    /// ECDSA over SHA-256 with an RFC 6979 nonce, DER encoded, not
    /// normalized to low S (CRYPTO_SPEC §16).
    ///
    /// Crate-private: only [`assert`](crate::assert) signs, over the
    /// authenticator-data message, so the key is no raw signing oracle.
    #[must_use]
    pub(crate) fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.0.sign(message);
        signature.to_der().as_bytes().to_vec()
    }

    /// The matching verifying key, for the tests.
    #[cfg(test)]
    pub(crate) fn verifying_key(&self) -> p256::ecdsa::VerifyingKey {
        *self.0.verifying_key()
    }

    /// A key from a known scalar, for the known-answer tests.
    #[cfg(test)]
    pub(crate) fn from_scalar(d: &[u8; 32]) -> Self {
        Self(SigningKey::from_slice(d).expect("valid test scalar"))
    }
}

impl fmt::Debug for Es256Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Es256Key([REDACTED])")
    }
}

/// `id-ecPublicKey` (RFC 5480): the algorithm of an elliptic-curve key.
const EC_PUBLIC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
/// `secp256r1` (P-256).
const SECP256R1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");

/// What [`pkcs8_kind`] found in a PKCS#8 structure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Pkcs8Kind {
    /// A private key of the elliptic-curve algorithm on P-256. If
    /// [`Es256Key::from_pkcs8_der`] refuses it, it is damaged (a mismatched
    /// public key, a bad scalar), not unsupported.
    P256,
    /// A well-formed private key of another algorithm or curve (Ed25519,
    /// RSA, P-384): valid, but not ES256.
    Other,
    /// Not a PKCS#8 private key.
    NotPkcs8,
}

/// The algorithm and curve of a PKCS#8 private key, from its structure only
/// (no key material is decoded or copied). An importer uses it to tell a key
/// of another algorithm from a damaged P-256 key when
/// [`Es256Key::from_pkcs8_der`] refuses it.
#[must_use]
pub fn pkcs8_kind(der: &[u8]) -> Pkcs8Kind {
    let Ok(info) = PrivateKeyInfoRef::try_from(der) else {
        return Pkcs8Kind::NotPkcs8;
    };
    if info.algorithm.oid == EC_PUBLIC_KEY && info.algorithm.parameters_oid() == Ok(SECP256R1) {
        Pkcs8Kind::P256
    } else {
        Pkcs8Kind::Other
    }
}

/// A fresh credential ID: [`CREDENTIAL_ID_LEN`] random bytes. Not secret.
///
/// # Errors
/// [`Error::Rng`] if the CSPRNG fails.
pub fn new_credential_id() -> Result<Vec<u8>, Error> {
    let id = SecretBytes::<CREDENTIAL_ID_LEN>::random().map_err(|_| Error::Rng)?;
    Ok(id.expose_secret().to_vec())
}

//! The keys the agent serves: SSH key items of the unlocked vault.
//!
//! An [`AgentKey`] holds the public key (parsed when the key set is built)
//! and the OpenSSH private key text plus its passphrase, both in zeroizing
//! memory. The private key is parsed, decrypted and used only inside
//! [`AgentKey::sign`], after the user approved the request, and every parsed
//! form is dropped (and wiped) before it returns.

use std::fmt;
use std::sync::Arc;

use mosaic_core::model::{Item, ItemType, SecretField, SshKeyMaterial, TextField};
use mosaic_core::{ItemId, SecretString, VaultId};
use ssh_key::public::{EcdsaPublicKey, KeyData};
use ssh_key::{HashAlg, Kdf, PrivateKey, PublicKey};
use thiserror::Error;

use crate::wire::Identity;

/// The default upper bound for the bcrypt-pbkdf rounds of an encrypted key
/// (`ssh-keygen -a`; OpenSSH's default is 16). A higher count would let a
/// crafted key item stall the agent for hours on every sign request.
pub const DEFAULT_MAX_KDF_ROUNDS: u32 = 1024;

/// Key algorithms the agent signs with.
///
/// RSA is not supported in this build: the `rsa` crate carries the
/// unpatched timing advisory RUSTSEC-2023-0071, and adding it needs an ADR.
/// The RFC 8332 flag rule for it is [`rsa_signature_algorithm`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum KeyAlgorithm {
    /// `ssh-ed25519` (RFC 8709).
    Ed25519,
    /// `ecdsa-sha2-nistp256` (RFC 5656), deterministic nonces (RFC 6979).
    EcdsaP256,
}

impl KeyAlgorithm {
    /// The SSH algorithm name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ssh-ed25519",
            Self::EcdsaP256 => "ecdsa-sha2-nistp256",
        }
    }

    fn of(key: &KeyData) -> Result<Self, KeyError> {
        match key {
            KeyData::Ed25519(_) => Ok(Self::Ed25519),
            KeyData::Ecdsa(EcdsaPublicKey::NistP256(_)) => Ok(Self::EcdsaP256),
            _ => Err(KeyError::Unsupported),
        }
    }
}

impl fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The signature algorithm an RSA key would use for a sign request's flags
/// (draft-miller-ssh-agent §4.5.1, RFC 8332): `rsa-sha2-256` if
/// `SSH_AGENT_RSA_SHA2_256` is set (it wins, as in OpenSSH), else
/// `rsa-sha2-512` if `SSH_AGENT_RSA_SHA2_512` is set, else `None`: the
/// SHA-1 `ssh-rsa` signature is refused.
#[must_use]
pub const fn rsa_signature_algorithm(flags: u32) -> Option<&'static str> {
    if flags & crate::wire::SSH_AGENT_RSA_SHA2_256 != 0 {
        Some("rsa-sha2-256")
    } else if flags & crate::wire::SSH_AGENT_RSA_SHA2_512 != 0 {
        Some("rsa-sha2-512")
    } else {
        None
    }
}

/// Why an item cannot be served. Carries no key material.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum KeyError {
    /// The item is not an SSH key item, or has no key material.
    #[error("not an SSH key item")]
    NotAnSshKey,
    /// The item is in the trash or deleted.
    #[error("the item is in the trash or deleted")]
    Unavailable,
    /// Neither the public key nor the private key text parses.
    #[error("the SSH key does not parse")]
    Malformed,
    /// An algorithm this build does not sign with (RSA, P-384, P-521, DSA,
    /// security keys, certificates).
    #[error("unsupported SSH key algorithm")]
    Unsupported,
    /// Memory for a key copy could not be reserved.
    #[error("out of memory")]
    Allocation,
}

/// Why a sign request failed after it was approved. Carries no key
/// material.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum SignFailure {
    /// The private key text does not parse.
    #[error("the private key does not parse")]
    Malformed,
    /// The key is encrypted and the item has no passphrase.
    #[error("the key is encrypted and the item has no passphrase")]
    PassphraseRequired,
    /// The item's passphrase does not decrypt the key.
    #[error("the passphrase does not decrypt the key")]
    WrongPassphrase,
    /// The key's bcrypt-pbkdf round count is above the agent's limit.
    #[error("the key's KDF round count is above the limit")]
    KdfTooExpensive,
    /// The private key does not belong to the listed public key.
    #[error("the private key does not match the public key")]
    PublicKeyMismatch,
    /// The private key's algorithm is not supported.
    #[error("unsupported key algorithm")]
    Unsupported,
    /// Signing failed, or the vault locked while signing.
    #[error("signing failed")]
    Internal,
}

/// One SSH key the agent serves.
pub struct AgentKey {
    vault: VaultId,
    item: ItemId,
    title: Option<String>,
    algorithm: KeyAlgorithm,
    key_data: KeyData,
    blob: Vec<u8>,
    comment: String,
    fingerprint: String,
    private_key: SecretString,
    passphrase: Option<SecretString>,
}

impl fmt::Debug for AgentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentKey")
            .field("vault", &self.vault)
            .field("item", &self.item)
            .field("algorithm", &self.algorithm)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl AgentKey {
    /// A key from its parts.
    ///
    /// The public key comes from `material.public_key` (`authorized_keys`
    /// format). If that is empty or does not parse, it is read from the
    /// private key file, whose header carries it in the clear; for an
    /// unencrypted key that decodes the private key once, into memory that
    /// is wiped on drop. The fingerprint is always computed here, never
    /// taken from `material.fingerprint`. `comment` is what the identities
    /// answer shows (`None` falls back to the public key's comment).
    ///
    /// # Errors
    /// [`KeyError::Malformed`] if no public key can be read,
    /// [`KeyError::Unsupported`] for another algorithm than Ed25519 and
    /// ECDSA P-256.
    pub fn new(
        vault: VaultId,
        item: ItemId,
        title: Option<&str>,
        comment: Option<&str>,
        material: SshKeyMaterial,
        passphrase: Option<SecretString>,
    ) -> Result<Self, KeyError> {
        let public = match PublicKey::from_openssh(material.public_key.as_str()) {
            Ok(p) => p,
            Err(_) => PrivateKey::from_openssh(material.private_key.expose_secret())
                .map_err(|_| KeyError::Malformed)?
                .public_key()
                .clone(),
        };
        let algorithm = KeyAlgorithm::of(public.key_data())?;
        let blob = public.to_bytes().map_err(|_| KeyError::Malformed)?;
        let fingerprint = public.fingerprint(HashAlg::Sha256).to_string();
        let comment = comment
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| public.comment().as_str_lossy())
            .to_owned();
        let passphrase = passphrase.filter(|p| !p.expose_secret().is_empty());
        Ok(Self {
            vault,
            item,
            title: title.filter(|t| !t.is_empty()).map(str::to_owned),
            algorithm,
            key_data: public.key_data().clone(),
            blob,
            comment,
            fingerprint,
            private_key: material.private_key,
            passphrase,
        })
    }

    /// The key of an SSH key item. The passphrase of an encrypted key is the
    /// item's [`SecretField::Password`]; the identities answer shows the
    /// item's [`TextField::SshComment`], or else the public key's comment.
    /// The item title is shown in the approval request only.
    ///
    /// # Errors
    /// [`KeyError::Unavailable`] for an item in the trash or deleted,
    /// [`KeyError::NotAnSshKey`] for another item type or an item without
    /// key material, [`KeyError::Allocation`] if the passphrase copy fails,
    /// otherwise as [`AgentKey::new`].
    pub fn from_item(vault: VaultId, item_id: ItemId, item: &Item) -> Result<Self, KeyError> {
        if item.is_deleted() || item.trashed_at().is_some() {
            return Err(KeyError::Unavailable);
        }
        if item.item_type() != Some(ItemType::SshKey) {
            return Err(KeyError::NotAnSshKey);
        }
        let material = item.ssh_key().ok_or(KeyError::NotAnSshKey)?;
        let passphrase = item
            .secret(SecretField::Password)
            .map_err(|_| KeyError::Allocation)?;
        Self::new(
            vault,
            item_id,
            item.title(),
            item.text(TextField::SshComment),
            material,
            passphrase,
        )
    }

    /// The key's vault.
    #[must_use]
    pub const fn vault(&self) -> VaultId {
        self.vault
    }

    /// The key's item.
    #[must_use]
    pub const fn item(&self) -> ItemId {
        self.item
    }

    /// The item title, if any.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The key algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> KeyAlgorithm {
        self.algorithm
    }

    /// The SHA-256 fingerprint, `SHA256:` and unpadded Base64, as
    /// `ssh-keygen -l` prints it.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The public key blob (RFC 4253 §6.6), as the identities answer lists
    /// it.
    #[must_use]
    pub fn public_key_blob(&self) -> &[u8] {
        &self.blob
    }

    /// The comment the identities answer shows.
    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }

    /// Parses the private key, decrypts it with the item passphrase if it
    /// is encrypted, checks that it belongs to the listed public key, signs
    /// `data` and returns the signature blob (string algorithm name, string
    /// signature). `flags` are ignored by Ed25519 and ECDSA.
    ///
    /// Blocking: decrypting runs bcrypt-pbkdf. Call it off the async
    /// executor.
    ///
    /// # Errors
    /// See [`SignFailure`]. A key whose bcrypt-pbkdf rounds exceed
    /// `max_kdf_rounds` is refused before any derivation.
    pub fn sign(
        &self,
        data: &[u8],
        flags: u32,
        max_kdf_rounds: u32,
    ) -> Result<Vec<u8>, SignFailure> {
        let _ = flags;
        let parsed = PrivateKey::from_openssh(self.private_key.expose_secret())
            .map_err(|_| SignFailure::Malformed)?;
        if parsed.public_key().key_data() != &self.key_data {
            return Err(SignFailure::PublicKeyMismatch);
        }
        let key = if parsed.is_encrypted() {
            if let Kdf::Bcrypt { rounds, .. } = parsed.kdf()
                && *rounds > max_kdf_rounds
            {
                return Err(SignFailure::KdfTooExpensive);
            }
            let passphrase = self
                .passphrase
                .as_ref()
                .ok_or(SignFailure::PassphraseRequired)?;
            parsed
                .decrypt(passphrase.expose_secret().as_bytes())
                .map_err(|_| SignFailure::WrongPassphrase)?
        } else {
            parsed
        };
        // The private key file checks its key against the header's public
        // key (`decrypt` again after decryption), and the header was
        // compared with the listed key above.
        let signature = sign_with(&key, data).ok_or(SignFailure::Internal)?;
        drop(key);
        Vec::<u8>::try_from(signature).map_err(|_| SignFailure::Internal)
    }
}

/// Signs with ssh-key's `Signer` implementation (Ed25519: RFC 8032;
/// ECDSA P-256: SHA-256 with RFC 6979 nonces). The `SigningKey` bound
/// brings `try_sign` into scope without naming the `signature` crate.
fn sign_with<K: ssh_key::SigningKey>(key: &K, data: &[u8]) -> Option<ssh_key::Signature> {
    key.try_sign(data).ok()
}

/// The keys the agent serves, in insertion order. A public key is listed
/// once: a second item with the same key is skipped, so an approval always
/// names one item.
#[derive(Default, Debug)]
pub struct KeySet {
    keys: Vec<Arc<AgentKey>>,
}

/// An item [`KeySet::from_items`] left out, and why.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkippedKey {
    /// The item's vault.
    pub vault: VaultId,
    /// The item.
    pub item: ItemId,
    /// Why it is not served.
    pub reason: SkipReason,
}

/// Why [`KeySet::from_items`] left an SSH key item out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SkipReason {
    /// The key cannot be served.
    Key(KeyError),
    /// Another item already serves the same public key.
    Duplicate,
}

impl KeySet {
    /// An empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Adds a key. Returns `false` (and drops the key) if the set already
    /// serves the same public key.
    pub fn insert(&mut self, key: AgentKey) -> bool {
        if self.find(key.public_key_blob()).is_some() {
            return false;
        }
        self.keys.push(Arc::new(key));
        true
    }

    /// The keys of every SSH key item among `items`. Items of other types
    /// are ignored silently; SSH key items that cannot be served are
    /// returned in the second list, so the app can say why.
    pub fn from_items<'a>(
        items: impl IntoIterator<Item = (VaultId, ItemId, &'a Item)>,
    ) -> (Self, Vec<SkippedKey>) {
        let mut set = Self::new();
        let mut skipped = Vec::new();
        for (vault, item_id, item) in items {
            let reason = match AgentKey::from_item(vault, item_id, item) {
                Ok(key) => {
                    if set.insert(key) {
                        continue;
                    }
                    SkipReason::Duplicate
                }
                Err(KeyError::NotAnSshKey) if item.item_type() != Some(ItemType::SshKey) => {
                    continue;
                }
                Err(KeyError::Unavailable) => continue,
                Err(e) => SkipReason::Key(e),
            };
            skipped.push(SkippedKey {
                vault,
                item: item_id,
                reason,
            });
        }
        (set, skipped)
    }

    /// Number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The keys, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &AgentKey> {
        self.keys.iter().map(AsRef::as_ref)
    }

    pub(crate) fn find(&self, blob: &[u8]) -> Option<Arc<AgentKey>> {
        self.keys
            .iter()
            .find(|k| k.public_key_blob() == blob)
            .cloned()
    }

    /// The identities answer: every key whose entry still fits a message.
    /// An entry too large for the space left is skipped, so one oversized
    /// comment does not hide the keys after it.
    pub(crate) fn identities(&self) -> Vec<Identity<'_>> {
        // Message number and count, then two length prefixes per entry.
        let mut budget = crate::wire::MAX_MESSAGE_LEN.saturating_sub(5);
        let mut out = Vec::with_capacity(self.keys.len());
        for k in &self.keys {
            let need = k
                .blob
                .len()
                .saturating_add(k.comment.len())
                .saturating_add(8);
            if need > budget {
                continue;
            }
            budget = budget.saturating_sub(need);
            out.push(Identity {
                key_blob: &k.blob,
                comment: k.comment.as_bytes(),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkeys::{
        PASSPHRASE, fixture, ids, key, material, openssh, secret, ssh_item, strings,
    };
    use base64ct::{Base64, Encoding};
    use hex_literal::hex;
    use signature::Verifier;
    use ssh_key::private::{EcdsaKeypair, Ed25519Keypair};

    const ROUNDS: u32 = DEFAULT_MAX_KDF_ROUNDS;

    fn verify(key: &AgentKey, data: &[u8], blob: &[u8]) {
        let public = PublicKey::from_bytes(key.public_key_blob()).unwrap();
        let sig = ssh_key::Signature::try_from(blob).unwrap();
        assert_eq!(sig.algorithm().as_str(), key.algorithm().as_str());
        // The trait method; `PublicKey::verify` itself checks `SshSig`s.
        Verifier::verify(&public, data, &sig).unwrap();
    }

    #[test]
    fn fixtures_load_with_ssh_keygen_fingerprints() {
        for (name, alg, fp, comment) in [
            (
                "ed25519",
                KeyAlgorithm::Ed25519,
                "SHA256:QjbS4EZK032epnEDVJGO7K3jwMFOWJMwAflXxbJSUUM",
                "mosaic-test-ed25519",
            ),
            (
                "ed25519_encrypted",
                KeyAlgorithm::Ed25519,
                "SHA256:y3g+LJFxKc2TrMnmUC5dojjMtDnPO88o0ws6qrvpnzQ",
                "mosaic-test-ed25519-aes256-ctr",
            ),
            (
                "ed25519_encrypted_chacha",
                KeyAlgorithm::Ed25519,
                "SHA256:A/9Nb6u5mCFhGDVYyYYSdRRHWtnHuhZT2/mNKsCNDa8",
                "mosaic-test-ed25519-chacha",
            ),
            (
                "ed25519_encrypted_gcm",
                KeyAlgorithm::Ed25519,
                "SHA256:2D9R5s4V4uiDmvtk7xVDQeOf7QJS0qBcC8D2YReEFQI",
                "mosaic-test-ed25519-gcm",
            ),
            (
                "ecdsa_p256",
                KeyAlgorithm::EcdsaP256,
                "SHA256:SsXbjjyia+SHJ+TbIquaomI1H+BvhtpY3O/3UN4vaJk",
                "mosaic-test-p256",
            ),
            (
                "ecdsa_p256_encrypted",
                KeyAlgorithm::EcdsaP256,
                "SHA256:xM8lc9I1DB9vR113NTF3lPe6vpcJgUVd7UBm4oJCqGQ",
                "mosaic-test-p256-aes256-ctr",
            ),
        ] {
            let k = key(name);
            assert_eq!(k.algorithm(), alg, "{name}");
            assert_eq!(k.fingerprint(), fp, "{name}");
            assert_eq!(k.comment(), comment, "{name}");
            assert_eq!(k.title(), Some(name));
            // The blob is the Base64 field of the public key line.
            let b64 = fixture(name).1.split_whitespace().nth(1).unwrap();
            assert_eq!(k.public_key_blob(), Base64::decode_vec(b64).unwrap());
        }
        assert_eq!(KeyAlgorithm::Ed25519.to_string(), "ssh-ed25519");
        assert_eq!(KeyAlgorithm::EcdsaP256.to_string(), "ecdsa-sha2-nistp256");
    }

    #[test]
    fn every_supported_fixture_signs_and_verifies() {
        for name in [
            "ed25519",
            "ed25519_encrypted",
            "ed25519_encrypted_chacha",
            "ed25519_encrypted_gcm",
            "ecdsa_p256",
            "ecdsa_p256_encrypted",
        ] {
            let k = key(name);
            let data = b"session id and userauth request";
            let blob = k.sign(data, 0, ROUNDS).unwrap();
            verify(&k, data, &blob);
            // Flags are ignored by Ed25519 and ECDSA.
            let flagged = k.sign(data, 0x06, ROUNDS).unwrap();
            verify(&k, data, &flagged);
        }
    }

    #[test]
    fn rsa_and_other_curves_are_not_served() {
        for name in ["rsa", "ecdsa_p384"] {
            let (private, public) = fixture(name);
            let (v, i) = ids();
            let err = AgentKey::new(v, i, None, None, material(private, public), None);
            assert_eq!(err.unwrap_err(), KeyError::Unsupported, "{name}");
            // Also when the public key comes from the private key file.
            let err = AgentKey::new(v, i, None, None, material(private, ""), None);
            assert_eq!(err.unwrap_err(), KeyError::Unsupported, "{name}");
        }
    }

    #[test]
    fn rsa_flags_follow_rfc_8332_and_refuse_sha1() {
        assert_eq!(rsa_signature_algorithm(0), None);
        assert_eq!(rsa_signature_algorithm(0x01), None, "old signature flag");
        assert_eq!(rsa_signature_algorithm(0x02), Some("rsa-sha2-256"));
        assert_eq!(rsa_signature_algorithm(0x04), Some("rsa-sha2-512"));
        assert_eq!(
            rsa_signature_algorithm(0x06),
            Some("rsa-sha2-256"),
            "256 wins"
        );
        assert_eq!(rsa_signature_algorithm(0xffff_fff9), None);
    }

    #[test]
    fn public_key_falls_back_to_the_private_key_file() {
        for name in ["ed25519", "ed25519_encrypted", "ecdsa_p256_encrypted"] {
            let (private, public) = fixture(name);
            let (v, i) = ids();
            for text in ["", "not a key", "ssh-ed25519 AAAA"] {
                let k = AgentKey::new(
                    v,
                    i,
                    None,
                    Some("from the item"),
                    material(private, text),
                    Some(secret(PASSPHRASE)),
                )
                .unwrap();
                assert_eq!(k.fingerprint(), key(name).fingerprint(), "{name}");
                assert_eq!(k.comment(), "from the item");
                assert_eq!(k.title(), None);
                let blob = k.sign(b"x", 0, ROUNDS).unwrap();
                verify(&k, b"x", &blob);
            }
            let _ = public;
        }
        let (v, i) = ids();
        let err = AgentKey::new(v, i, None, None, material("garbage", ""), None);
        assert_eq!(err.unwrap_err(), KeyError::Malformed);
    }

    #[test]
    fn passphrase_failures() {
        let (private, public) = fixture("ed25519_encrypted");
        let (v, i) = ids();
        let none = AgentKey::new(v, i, None, None, material(private, public), None).unwrap();
        assert_eq!(
            none.sign(b"x", 0, ROUNDS),
            Err(SignFailure::PassphraseRequired)
        );
        let empty = AgentKey::new(
            v,
            i,
            None,
            None,
            material(private, public),
            Some(secret("")),
        )
        .unwrap();
        assert_eq!(
            empty.sign(b"x", 0, ROUNDS),
            Err(SignFailure::PassphraseRequired),
            "an empty passphrase counts as none"
        );
        for name in [
            "ed25519_encrypted",
            "ed25519_encrypted_chacha",
            "ed25519_encrypted_gcm",
        ] {
            let (private, public) = fixture(name);
            let wrong = AgentKey::new(
                v,
                i,
                None,
                None,
                material(private, public),
                Some(secret("wrong horse")),
            )
            .unwrap();
            assert_eq!(
                wrong.sign(b"x", 0, ROUNDS),
                Err(SignFailure::WrongPassphrase),
                "{name}"
            );
        }
    }

    #[test]
    fn a_private_key_of_another_public_key_is_refused() {
        let (v, i) = ids();
        let (ed_private, _) = fixture("ed25519");
        let (_, other_public) = fixture("ed25519_encrypted");
        let k = AgentKey::new(v, i, None, None, material(ed_private, other_public), None).unwrap();
        assert_eq!(k.sign(b"x", 0, ROUNDS), Err(SignFailure::PublicKeyMismatch));
        // Same for an encrypted key: refused before any key derivation.
        let (enc_private, _) = fixture("ed25519_encrypted");
        let (_, ed_public) = fixture("ed25519");
        let k = AgentKey::new(
            v,
            i,
            None,
            None,
            material(enc_private, ed_public),
            Some(secret(PASSPHRASE)),
        )
        .unwrap();
        assert_eq!(k.sign(b"x", 0, ROUNDS), Err(SignFailure::PublicKeyMismatch));
        // A private key text that stopped parsing.
        let (_, public) = fixture("ed25519");
        let k = AgentKey::new(v, i, None, None, material("garbage", public), None).unwrap();
        assert_eq!(k.sign(b"x", 0, ROUNDS), Err(SignFailure::Malformed));
    }

    /// Rewrites the bcrypt-pbkdf round count of an encrypted fixture.
    fn with_rounds(pem: &str, rounds: u32) -> String {
        let body: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        let mut raw = Base64::decode_vec(&body).unwrap();
        let at = raw.windows(6).position(|w| w == b"bcrypt").unwrap() + 6;
        // string kdfoptions { string salt (16 bytes), uint32 rounds }
        let rounds_at = at + 4 + 4 + 16;
        raw[rounds_at..rounds_at + 4].copy_from_slice(&rounds.to_be_bytes());
        let b64 = Base64::encode_string(&raw);
        let lines: Vec<&str> = b64
            .as_bytes()
            .chunks(70)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();
        format!(
            "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
            lines.join("\n")
        )
    }

    #[test]
    fn expensive_kdf_rounds_are_refused_before_deriving() {
        let (private, public) = fixture("ed25519_encrypted");
        let (v, i) = ids();
        let same = with_rounds(private, 16);
        assert_eq!(same, private, "the rewrite reproduces the fixture");
        let costly = with_rounds(private, u32::MAX);
        let k = AgentKey::new(
            v,
            i,
            None,
            None,
            material(&costly, public),
            Some(secret(PASSPHRASE)),
        )
        .unwrap();
        assert_eq!(k.sign(b"x", 0, ROUNDS), Err(SignFailure::KdfTooExpensive));
        let k = key("ed25519_encrypted");
        assert_eq!(k.sign(b"x", 0, 15), Err(SignFailure::KdfTooExpensive));
        assert!(k.sign(b"x", 0, 16).is_ok(), "the limit is inclusive");
    }

    #[test]
    fn ed25519_known_answers_rfc_8032() {
        // RFC 8032 §7.1, tests 1 and 2.
        for (seed, public, msg, sig) in [
            (
                hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"),
                hex!("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"),
                &[][..],
                hex!(
                    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
                ),
            ),
            (
                hex!("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"),
                hex!("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"),
                &[0x72][..],
                hex!(
                    "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
                ),
            ),
        ] {
            let pem = openssh(Ed25519Keypair::from_seed(&seed));
            let (v, i) = ids();
            let k = AgentKey::new(v, i, None, None, material(&pem, ""), None).unwrap();
            assert_eq!(k.public_key_blob(), strings(&[b"ssh-ed25519", &public]));
            let blob = k.sign(msg, 0, ROUNDS).unwrap();
            assert_eq!(blob, strings(&[b"ssh-ed25519", &sig]));
        }
    }

    #[test]
    fn ecdsa_p256_known_answers_rfc_6979() {
        use p256::elliptic_curve::sec1::ToSec1Point as _;
        // RFC 6979 §A.2.5, P-256 with SHA-256.
        let x = hex!("C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721");
        let secret_key = p256::SecretKey::from_slice(&x).unwrap();
        let public = secret_key.public_key().to_sec1_point(false);
        assert_eq!(
            public.as_bytes(),
            [
                &[0x04][..],
                &hex!("60FED4BA255A9D31C961EB74C6356D68C049B8923B61FA6CE669622E60F29FB6"),
                &hex!("7903FE1008B8BC99A41AE9E95628BC64F2F1B20C2D7E9F5177A3C294D4462299"),
            ]
            .concat()
        );
        let pem = openssh(EcdsaKeypair::NistP256 {
            public,
            private: secret_key.into(),
        });
        let (v, i) = ids();
        let k = AgentKey::new(v, i, None, None, material(&pem, ""), None).unwrap();
        assert_eq!(k.algorithm(), KeyAlgorithm::EcdsaP256);
        // mpint r and s: a leading zero byte when the top bit is set.
        for (msg, r, s) in [
            (
                &b"sample"[..],
                [
                    &[0u8][..],
                    &hex!("EFD48B2AACB6A8FD1140DD9CD45E81D69D2C877B56AAF991C34D0EA84EAF3716"),
                ]
                .concat(),
                [
                    &[0u8][..],
                    &hex!("F7CB1C942D657C41D436C7A1B6E29F65F3E900DBB9AFF4064DC4AB2F843ACDA8"),
                ]
                .concat(),
            ),
            (
                &b"test"[..],
                [
                    &[0u8][..],
                    &hex!("F1ABB023518351CD71D881567B1EA663ED3EFCF6C5132B354F28D3B0B7D38367"),
                ]
                .concat(),
                hex!("019F4113742A2B14BD25926B49C649155F267E60D3814B4C0CC84250E46F0083").to_vec(),
            ),
        ] {
            let blob = k.sign(msg, 0, ROUNDS).unwrap();
            let inner = strings(&[&r, &s]);
            assert_eq!(blob, strings(&[b"ecdsa-sha2-nistp256", &inner]));
            verify(&k, msg, &blob);
        }
    }

    #[test]
    fn items_become_keys() {
        let (private, public) = fixture("ed25519_encrypted");
        let item = ssh_item(private, public, Some(PASSPHRASE), Some("deploy@ci"));
        let (v, i) = ids();
        let k = AgentKey::from_item(v, i, &item).unwrap();
        assert_eq!((k.vault(), k.item()), (v, i));
        assert_eq!(k.title(), Some("Deploy key"));
        assert_eq!(k.comment(), "deploy@ci");
        let blob = k.sign(b"x", 0, ROUNDS).unwrap();
        verify(&k, b"x", &blob);
        // No comment field: the public key's comment.
        let item = ssh_item(private, public, Some(PASSPHRASE), None);
        let k = AgentKey::from_item(v, i, &item).unwrap();
        assert_eq!(k.comment(), "mosaic-test-ed25519-aes256-ctr");
    }

    #[test]
    fn unavailable_and_foreign_items_are_not_keys() {
        use mosaic_core::DeviceId;
        use mosaic_core::model::Timestamp;
        let (private, public) = fixture("ed25519");
        let (v, i) = ids();
        let device = DeviceId::new_v7().unwrap();
        let at = Timestamp::from_unix_millis(1_761_480_000_001);
        let mut trashed = ssh_item(private, public, None, None);
        trashed.edit(device, at).trash().unwrap();
        assert_eq!(
            AgentKey::from_item(v, i, &trashed).unwrap_err(),
            KeyError::Unavailable
        );
        let login = Item::new(&ItemType::Login, device, at).unwrap();
        assert_eq!(
            AgentKey::from_item(v, i, &login).unwrap_err(),
            KeyError::NotAnSshKey
        );
        let empty = Item::new(&ItemType::SshKey, device, at).unwrap();
        assert_eq!(
            AgentKey::from_item(v, i, &empty).unwrap_err(),
            KeyError::NotAnSshKey
        );
    }

    #[test]
    fn key_sets_skip_duplicates_and_report_unusable_keys() {
        use mosaic_core::DeviceId;
        use mosaic_core::model::Timestamp;
        let device = DeviceId::new_v7().unwrap();
        let at = Timestamp::from_unix_millis(1_761_480_000_001);
        let (ed, ed_pub) = fixture("ed25519");
        let (p256, p256_pub) = fixture("ecdsa_p256");
        let (rsa, rsa_pub) = fixture("rsa");
        let items = [
            ssh_item(ed, ed_pub, None, None),
            ssh_item(p256, p256_pub, None, None),
            ssh_item(ed, ed_pub, None, None),
            ssh_item(rsa, rsa_pub, None, None),
            Item::new(&ItemType::Login, device, at).unwrap(),
            Item::new(&ItemType::SshKey, device, at).unwrap(),
        ];
        let v = ids().0;
        let item_ids: Vec<ItemId> = items.iter().map(|_| ids().1).collect();
        let (set, skipped) =
            KeySet::from_items(items.iter().zip(&item_ids).map(|(it, id)| (v, *id, it)));
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
        let fps: Vec<&str> = set.iter().map(AgentKey::fingerprint).collect();
        assert_eq!(
            fps,
            [
                key("ed25519").fingerprint(),
                key("ecdsa_p256").fingerprint()
            ]
        );
        assert_eq!(
            skipped,
            vec![
                SkippedKey {
                    vault: v,
                    item: item_ids[2],
                    reason: SkipReason::Duplicate
                },
                SkippedKey {
                    vault: v,
                    item: item_ids[3],
                    reason: SkipReason::Key(KeyError::Unsupported)
                },
                SkippedKey {
                    vault: v,
                    item: item_ids[5],
                    reason: SkipReason::Key(KeyError::NotAnSshKey)
                },
            ]
        );
        assert!(KeySet::new().is_empty());
    }

    #[test]
    fn identities_stop_before_the_message_limit() {
        let mut set = KeySet::new();
        assert!(set.insert(key("ed25519")));
        assert!(!set.insert(key("ed25519")), "duplicate");
        assert_eq!(set.identities().len(), 1);
        // A comment too large for one message leaves that key out.
        let (private, public) = fixture("ecdsa_p256");
        let (v, i) = ids();
        let huge = "c".repeat(crate::wire::MAX_MESSAGE_LEN);
        let big = AgentKey::new(v, i, None, Some(&huge), material(private, public), None).unwrap();
        assert!(set.insert(big));
        // A key after the oversized one is still listed.
        assert!(set.insert(key("ed25519_encrypted")));
        assert_eq!(set.len(), 3);
        let ids = set.identities();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[1].key_blob, key("ed25519_encrypted").public_key_blob());
        crate::wire::Response::Identities(ids).encode().unwrap();
    }

    #[test]
    fn debug_output_has_no_key_material() {
        let k = key("ed25519");
        let dbg = format!("{k:?}");
        assert!(dbg.contains(k.fingerprint()));
        assert!(!dbg.contains("OPENSSH"));
        assert!(!dbg.contains("mosaic-test"), "no comment either: {dbg}");
        let enc = key("ed25519_encrypted");
        assert!(!format!("{enc:?}").contains(PASSPHRASE));
    }
}

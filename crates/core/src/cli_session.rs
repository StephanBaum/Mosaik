//! The CLI session cache (CRYPTO_SPEC §6.8, ADR-0072).
//!
//! `mosaic unlock` seals `AK` under a random key `W_cli` into a 162-byte file
//! and hands `W_cli` (with a `session_id`) to the OS keystore, so a script
//! need not prompt for the master password on every command. Neither part is
//! useful alone. No new primitive: XChaCha20-Poly1305 with the header bound
//! into the AAD (§3).
//!
//! ```text
//! header = "MCLS" || version:u8 (1) || alg:u8 (1) || account_id[16] || install_id[16]
//!          || session_id[16] || ak_epoch:u32 || account_salt[16]
//!          || created_at_ms:u64 || expires_at_ms:u64                    (90 bytes)
//! file   = header || nonce[24] || XChaCha20-Poly1305(W_cli, nonce, AK, aad)
//! aad    = header || aad("cli-session", account_id, install_id, session_id)
//! ```
//!
//! [`open_cli_session`] enforces rules 1 to 5 of §6.8; the vault layer adds
//! the check that a held vault key opens under the recovered `AK` (rule 6)
//! and that only a master-password unlock may create a session.

use core::fmt;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use zeroize::Zeroizing;

use crate::aead::{self, NONCE_LEN};
use crate::encoding::{Aad, label, take};
use crate::envelope::{ALG_XCHACHA20_POLY1305, FORMAT_VERSION};
use crate::error::{Error, Result};
use crate::ids::{AccountId, InstallId};
use crate::kdf::AccountSalt;
use crate::rng;
use crate::secret::{AccountKey, SecretBytes, SecretString};

/// Shortest TTL of a new session: 1 minute.
pub const MIN_TTL_MS: u64 = 60 * 1000;
/// Default TTL: 5 minutes.
pub const DEFAULT_TTL_MS: u64 = 5 * 60 * 1000;
/// Longest TTL, and the longest `expires_at - created_at` a file may carry:
/// 30 minutes.
pub const MAX_TTL_MS: u64 = 30 * 60 * 1000;
/// How far a file's `created_at` may lie ahead of the opener's clock: 5
/// minutes.
pub const MAX_CLOCK_SKEW_MS: u64 = 5 * 60 * 1000;
/// Length of a session file.
pub const FILE_LEN: usize = HEADER_LEN + NONCE_LEN + 32 + aead::TAG_LEN;
/// Length of the keystore entry value: `base64url(session_id || W_cli)`.
pub const KEYSTORE_VALUE_LEN: usize = 64;

const MAGIC: [u8; 4] = *b"MCLS";
const HEADER_LEN: usize = 4 + 1 + 1 + 16 + 16 + 16 + 4 + 16 + 8 + 8;

/// The store state a session file is bound to. A password change, Secret Key
/// rotation or profile upgrade (a new salt) and a re-key (a higher epoch)
/// change it, which ends every session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliSessionBinding {
    /// The account.
    pub account_id: AccountId,
    /// This install (the store's `InstallId`).
    pub install_id: InstallId,
    /// The account-key epoch (§6.7) the store holds.
    pub ak_epoch: u32,
    /// The account salt (§5.3) the store holds.
    pub salt: AccountSalt,
}

/// The part of a session that lives in the OS keystore: `W_cli` and the
/// `session_id` the file must carry. Zeroizes on drop, is not `Clone`, and
/// prints as `[REDACTED]`.
pub struct CliSessionKey {
    w: SecretBytes<32>,
    session_id: [u8; 16],
}

impl fmt::Debug for CliSessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl CliSessionKey {
    fn generate() -> Result<Self> {
        Ok(Self {
            w: SecretBytes::random()?,
            session_id: rng::array()?,
        })
    }

    /// The random `session_id` (not secret; it also sits in the file header).
    #[must_use]
    pub const fn session_id(&self) -> &[u8; 16] {
        &self.session_id
    }

    /// The keystore entry value `base64url(session_id || W_cli)` (64
    /// characters, unpadded), in a zeroizing string.
    ///
    /// # Errors
    /// [`Error::Allocation`] only.
    pub fn to_keystore_value(&self) -> Result<SecretString> {
        let mut raw = SecretBytes::<48>::zero();
        let (id, w) = raw.expose_secret_mut().split_at_mut(16);
        id.copy_from_slice(&self.session_id);
        w.copy_from_slice(self.w.expose_secret());
        let mut text = Zeroizing::new(vec![0u8; KEYSTORE_VALUE_LEN]);
        let s = Base64UrlUnpadded::encode(raw.expose_secret(), &mut text)
            .map_err(|_| Error::Malformed)?;
        SecretString::from_str_copy(s)
    }

    /// Parses a keystore entry value written by [`Self::to_keystore_value`].
    ///
    /// # Errors
    /// [`Error::Malformed`] for anything but 64 canonical base64url
    /// characters.
    pub fn from_keystore_value(value: &str) -> Result<Self> {
        if value.len() != KEYSTORE_VALUE_LEN {
            return Err(Error::Malformed);
        }
        let mut raw = SecretBytes::<48>::zero();
        Base64UrlUnpadded::decode(value, raw.expose_secret_mut()).map_err(|_| Error::Malformed)?;
        let (id, w) = raw.expose_secret().split_at(16);
        Ok(Self {
            w: SecretBytes::from_slice(w)?,
            session_id: id.try_into().map_err(|_| Error::Malformed)?,
        })
    }
}

/// The session file: [`FILE_LEN`] bytes of header, nonce and sealed `AK`.
/// Ciphertext, so it may be copied and printed; it opens only with the
/// matching [`CliSessionKey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliSessionFile([u8; FILE_LEN]);

impl CliSessionFile {
    /// Wraps stored bytes.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `bytes` is not exactly [`FILE_LEN`] bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self(bytes.try_into().map_err(|_| Error::Malformed)?))
    }

    /// The bytes to store.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; FILE_LEN] {
        &self.0
    }

    /// The `expires_at` the header claims, in Unix milliseconds. Not
    /// authenticated: use it to display a time left, never to decide that a
    /// session is valid ([`open_cli_session`] decides).
    #[must_use]
    pub fn claimed_expires_at_ms(&self) -> Option<u64> {
        let (header, _) = take::<HEADER_LEN>(&self.0).ok()?;
        Header::parse(header).ok().map(|h| h.expires_at_ms)
    }
}

/// The decoded header.
struct Header {
    account_id: [u8; 16],
    install_id: [u8; 16],
    session_id: [u8; 16],
    ak_epoch: u32,
    salt: [u8; 16],
    created_at_ms: u64,
    expires_at_ms: u64,
}

impl Header {
    fn encode(&self) -> Result<[u8; HEADER_LEN]> {
        let mut out = Vec::with_capacity(HEADER_LEN);
        out.extend_from_slice(&MAGIC);
        out.push(FORMAT_VERSION);
        out.push(ALG_XCHACHA20_POLY1305);
        out.extend_from_slice(&self.account_id);
        out.extend_from_slice(&self.install_id);
        out.extend_from_slice(&self.session_id);
        out.extend_from_slice(&self.ak_epoch.to_be_bytes());
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.created_at_ms.to_be_bytes());
        out.extend_from_slice(&self.expires_at_ms.to_be_bytes());
        out.try_into().map_err(|_| Error::Malformed)
    }

    /// Every unknown magic, version or algorithm is [`Error::Decryption`],
    /// like every other refusal of a session file.
    fn parse(header: &[u8; HEADER_LEN]) -> Result<Self> {
        let (magic, rest) = take::<4>(header)?;
        let ([version, alg], rest) = take::<2>(rest)?;
        if *magic != MAGIC || *version != FORMAT_VERSION || *alg != ALG_XCHACHA20_POLY1305 {
            return Err(Error::Decryption);
        }
        let (account_id, rest) = take::<16>(rest)?;
        let (install_id, rest) = take::<16>(rest)?;
        let (session_id, rest) = take::<16>(rest)?;
        let (ak_epoch, rest) = take::<4>(rest)?;
        let (salt, rest) = take::<16>(rest)?;
        let (created, rest) = take::<8>(rest)?;
        let (expires, _) = take::<8>(rest)?;
        Ok(Self {
            account_id: *account_id,
            install_id: *install_id,
            session_id: *session_id,
            ak_epoch: u32::from_be_bytes(*ak_epoch),
            salt: *salt,
            created_at_ms: u64::from_be_bytes(*created),
            expires_at_ms: u64::from_be_bytes(*expires),
        })
    }
}

fn aad(header: &[u8; HEADER_LEN], h: &Header) -> Aad {
    Aad::with_header(header, label::CLI_SESSION)
        .fixed(&h.account_id)
        .fixed(&h.install_id)
        .fixed(&h.session_id)
}

/// Seals `ak` into a new session valid from `now_ms` for `ttl_ms`, bound to
/// `binding`. Returns the keystore part and the file part.
///
/// The caller (the vault layer) decides that this process may create a
/// session at all (§6.8: only after a master-password unlock).
///
/// # Errors
/// [`Error::InvalidTtl`] unless [`MIN_TTL_MS`] <= `ttl_ms` <= [`MAX_TTL_MS`];
/// [`Error::Randomness`] / [`Error::Allocation`].
pub fn seal_cli_session(
    ak: &AccountKey,
    binding: &CliSessionBinding,
    now_ms: u64,
    ttl_ms: u64,
) -> Result<(CliSessionKey, CliSessionFile)> {
    let key = CliSessionKey::generate()?;
    let nonce = aead::random_nonce()?;
    let file = seal_with(ak, &key, binding, &nonce, now_ms, ttl_ms)?;
    Ok((key, file))
}

/// A session under a fixed `W_cli`, `session_id` and nonce, for the fuzz
/// targets only, so a crash reproduces from its input.
///
/// # Errors
/// As [`seal_cli_session`], without the randomness.
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn seal_cli_session_for_fuzzing(
    ak: &AccountKey,
    binding: &CliSessionBinding,
    now_ms: u64,
    ttl_ms: u64,
) -> Result<(CliSessionKey, CliSessionFile)> {
    let key = CliSessionKey {
        w: SecretBytes::from_array(&mut [0x22; 32]),
        session_id: [0x55; 16],
    };
    let file = seal_with(ak, &key, binding, &[0x66; NONCE_LEN], now_ms, ttl_ms)?;
    Ok((key, file))
}

fn seal_with(
    ak: &AccountKey,
    key: &CliSessionKey,
    binding: &CliSessionBinding,
    nonce: &[u8; NONCE_LEN],
    now_ms: u64,
    ttl_ms: u64,
) -> Result<CliSessionFile> {
    if !(MIN_TTL_MS..=MAX_TTL_MS).contains(&ttl_ms) {
        return Err(Error::InvalidTtl);
    }
    let h = Header {
        account_id: *binding.account_id.as_bytes(),
        install_id: *binding.install_id.as_bytes(),
        session_id: key.session_id,
        ak_epoch: binding.ak_epoch,
        salt: *binding.salt.as_bytes(),
        created_at_ms: now_ms,
        expires_at_ms: now_ms.checked_add(ttl_ms).ok_or(Error::InvalidTtl)?,
    };
    let header = h.encode()?;
    let aad = aad(&header, &h);
    let sealed = aead::seal_with_prefix(
        key.w.expose_secret(),
        nonce,
        aad.as_bytes(),
        &[&header, nonce],
        ak.0.expose_secret(),
    )?;
    CliSessionFile::from_bytes(&sealed)
}

/// Opens a session file and returns the account key it holds.
///
/// Refuses, with [`Error::Decryption`] (or [`Error::Expired`] for the clock
/// rule 5), every case of CRYPTO_SPEC §6.8 rules 1 to 5: an unknown magic,
/// version or algorithm; a `session_id` that differs from the key's; an
/// account, install, epoch or salt that differs from `expected`; a failed
/// AEAD; a lifetime of zero or above 30 minutes; a `created_at` more than 5
/// minutes after `now_ms`; and `now_ms` after `expires_at`. The caller deletes
/// both parts on any error and then checks rule 6 (a held vault key opens
/// under the returned key).
///
/// The time rules run after the AEAD, so they judge authenticated values.
///
/// # Errors
/// As above.
pub fn open_cli_session(
    key: &CliSessionKey,
    file: &CliSessionFile,
    expected: &CliSessionBinding,
    now_ms: u64,
) -> Result<AccountKey> {
    let (header, rest) = take::<HEADER_LEN>(&file.0)?;
    let h = Header::parse(header)?;
    if h.session_id != key.session_id
        || h.account_id != *expected.account_id.as_bytes()
        || h.install_id != *expected.install_id.as_bytes()
        || h.ak_epoch != expected.ak_epoch
        || h.salt != *expected.salt.as_bytes()
    {
        return Err(Error::Decryption);
    }
    let (nonce, body) = take::<NONCE_LEN>(rest)?;
    let aad = aad(header, &h);
    let ak = aead::open(key.w.expose_secret(), nonce, aad.as_bytes(), body)?;
    let ak = SecretBytes::<32>::from_slice(ak.expose_secret()).map_err(|_| Error::Decryption)?;

    let lifetime = h.expires_at_ms.checked_sub(h.created_at_ms);
    if !lifetime.is_some_and(|l| (1..=MAX_TTL_MS).contains(&l)) {
        return Err(Error::Decryption);
    }
    if h.created_at_ms > now_ms.saturating_add(MAX_CLOCK_SKEW_MS) {
        return Err(Error::Decryption);
    }
    if now_ms > h.expires_at_ms {
        return Err(Error::Expired);
    }
    Ok(AccountKey(ak))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    fn binding() -> CliSessionBinding {
        CliSessionBinding {
            account_id: AccountId::from_bytes([0xA1; 16]).unwrap(),
            install_id: InstallId::from_bytes([0xB2; 16]).unwrap(),
            ak_epoch: 3,
            salt: AccountSalt::from_bytes([0xC3; 16]),
        }
    }

    fn ak() -> AccountKey {
        AccountKey(SecretBytes::from_array(&mut [0x11; 32]))
    }

    fn fixed_key() -> CliSessionKey {
        CliSessionKey {
            w: SecretBytes::from_array(&mut [0x22; 32]),
            session_id: [0x55; 16],
        }
    }

    #[test]
    fn round_trip() {
        let (key, file) = seal_cli_session(&ak(), &binding(), NOW, DEFAULT_TTL_MS).unwrap();
        assert_eq!(file.as_bytes().len(), FILE_LEN);
        assert_eq!(FILE_LEN, 162);
        assert_eq!(HEADER_LEN, 90);
        assert_eq!(file.claimed_expires_at_ms(), Some(NOW + DEFAULT_TTL_MS));
        assert_eq!(&file.as_bytes()[..4], b"MCLS");
        let back = open_cli_session(&key, &file, &binding(), NOW + 1000).unwrap();
        assert_eq!(back, ak());
        // Valid up to and including the expiry instant, an absolute TTL.
        assert!(open_cli_session(&key, &file, &binding(), NOW + DEFAULT_TTL_MS).is_ok());
        // Two seals never share a key, an ID or a nonce.
        let (key2, file2) = seal_cli_session(&ak(), &binding(), NOW, DEFAULT_TTL_MS).unwrap();
        assert_ne!(key.session_id(), key2.session_id());
        assert_ne!(file.as_bytes(), file2.as_bytes());
    }

    #[test]
    fn expired_refused() {
        let (key, file) = seal_cli_session(&ak(), &binding(), NOW, MIN_TTL_MS).unwrap();
        assert_eq!(
            open_cli_session(&key, &file, &binding(), NOW + MIN_TTL_MS + 1).unwrap_err(),
            Error::Expired
        );
    }

    #[test]
    fn ttl_over_30_min_refused() {
        // Sealing refuses it, and so does opening a file that carries it.
        for ttl in [0, MIN_TTL_MS - 1, MAX_TTL_MS + 1, u64::MAX] {
            assert_eq!(
                seal_cli_session(&ak(), &binding(), NOW, ttl).unwrap_err(),
                Error::InvalidTtl,
                "ttl {ttl}"
            );
        }
        assert!(seal_cli_session(&ak(), &binding(), NOW, MAX_TTL_MS).is_ok());
        assert_eq!(
            seal_cli_session(&ak(), &binding(), u64::MAX, MIN_TTL_MS).unwrap_err(),
            Error::InvalidTtl
        );
        let key = fixed_key();
        for ttl in [MAX_TTL_MS + 1, 24 * 60 * 60 * 1000] {
            let file = forged(&key, NOW, NOW + ttl);
            assert_eq!(
                open_cli_session(&key, &file, &binding(), NOW + 1).unwrap_err(),
                Error::Decryption,
                "ttl {ttl}"
            );
        }
        // Zero and negative lifetimes.
        for expires in [NOW, NOW - 1] {
            let file = forged(&key, NOW, expires);
            assert_eq!(
                open_cli_session(&key, &file, &binding(), NOW).unwrap_err(),
                Error::Decryption
            );
        }
        assert!(
            open_cli_session(&key, &forged(&key, NOW, NOW + MAX_TTL_MS), &binding(), NOW).is_ok()
        );
    }

    /// An authentic file with arbitrary times (what a buggy or hostile
    /// writer holding `W_cli` could produce), bypassing the sealing rules.
    fn forged(key: &CliSessionKey, created: u64, expires: u64) -> CliSessionFile {
        let b = binding();
        let h = Header {
            account_id: *b.account_id.as_bytes(),
            install_id: *b.install_id.as_bytes(),
            session_id: key.session_id,
            ak_epoch: b.ak_epoch,
            salt: *b.salt.as_bytes(),
            created_at_ms: created,
            expires_at_ms: expires,
        };
        let header = h.encode().unwrap();
        let aad = aad(&header, &h);
        let nonce = [0x66; NONCE_LEN];
        let sealed = aead::seal_with_prefix(
            key.w.expose_secret(),
            &nonce,
            aad.as_bytes(),
            &[&header, &nonce],
            &[0x11; 32],
        )
        .unwrap();
        CliSessionFile::from_bytes(&sealed).unwrap()
    }

    #[test]
    fn future_created_refused() {
        let key = fixed_key();
        let file = forged(
            &key,
            NOW + MAX_CLOCK_SKEW_MS + 1,
            NOW + MAX_CLOCK_SKEW_MS + 1000,
        );
        assert_eq!(
            open_cli_session(&key, &file, &binding(), NOW).unwrap_err(),
            Error::Decryption
        );
        // Exactly 5 minutes ahead is still within the allowed skew.
        let file = forged(
            &key,
            NOW + MAX_CLOCK_SKEW_MS,
            NOW + MAX_CLOCK_SKEW_MS + 1000,
        );
        assert!(open_cli_session(&key, &file, &binding(), NOW).is_ok());
    }

    #[test]
    fn binding_mismatch_refused_each_field() {
        let (key, file) = seal_cli_session(&ak(), &binding(), NOW, DEFAULT_TTL_MS).unwrap();
        let good = binding();
        let others = [
            CliSessionBinding {
                account_id: AccountId::from_bytes([0xA2; 16]).unwrap(),
                ..good
            },
            CliSessionBinding {
                install_id: InstallId::from_bytes([0xB3; 16]).unwrap(),
                ..good
            },
            CliSessionBinding {
                ak_epoch: good.ak_epoch + 1,
                ..good
            },
            CliSessionBinding {
                ak_epoch: good.ak_epoch - 1,
                ..good
            },
            CliSessionBinding {
                salt: AccountSalt::from_bytes([0xC4; 16]),
                ..good
            },
        ];
        for (i, other) in others.iter().enumerate() {
            assert_eq!(
                open_cli_session(&key, &file, other, NOW + 1).unwrap_err(),
                Error::Decryption,
                "binding {i}"
            );
        }
        // A keystore part from another session (another `session_id` and key).
        let (other_key, _) = seal_cli_session(&ak(), &good, NOW, DEFAULT_TTL_MS).unwrap();
        assert_eq!(
            open_cli_session(&other_key, &file, &good, NOW + 1).unwrap_err(),
            Error::Decryption
        );
        // The right `W_cli` with another `session_id` is refused as well.
        let mut wrong_id =
            CliSessionKey::from_keystore_value(key.to_keystore_value().unwrap().expose_secret())
                .unwrap();
        wrong_id.session_id[0] ^= 1;
        assert_eq!(
            open_cli_session(&wrong_id, &file, &good, NOW + 1).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn tampered_header_fails() {
        let (key, file) = seal_cli_session(&ak(), &binding(), NOW, DEFAULT_TTL_MS).unwrap();
        // Every single flipped bit anywhere in the file is refused (the
        // bound fields by the comparison, the rest by the AEAD).
        for i in 0..FILE_LEN {
            for bit in [0x01u8, 0x80] {
                let mut bytes = *file.as_bytes();
                bytes[i] ^= bit;
                let bad = CliSessionFile::from_bytes(&bytes).unwrap();
                let err = open_cli_session(&key, &bad, &binding(), NOW + 1).unwrap_err();
                assert_eq!(err, Error::Decryption, "byte {i} bit {bit:#x}");
            }
        }
        // Wrong lengths are not a session file at all.
        assert_eq!(
            CliSessionFile::from_bytes(&file.as_bytes()[..FILE_LEN - 1]).unwrap_err(),
            Error::Malformed
        );
        let mut long = file.as_bytes().to_vec();
        long.push(0);
        assert_eq!(
            CliSessionFile::from_bytes(&long).unwrap_err(),
            Error::Malformed
        );
        assert_eq!(
            CliSessionFile::from_bytes(&[]).unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    fn wrong_key_fails() {
        let (key, file) = seal_cli_session(&ak(), &binding(), NOW, DEFAULT_TTL_MS).unwrap();
        // Same `session_id`, different `W_cli`.
        let mut other = CliSessionKey {
            w: SecretBytes::random().unwrap(),
            session_id: *key.session_id(),
        };
        assert_eq!(
            open_cli_session(&other, &file, &binding(), NOW + 1).unwrap_err(),
            Error::Decryption
        );
        other.w = SecretBytes::from_array(&mut [0u8; 32]);
        assert_eq!(
            open_cli_session(&other, &file, &binding(), NOW + 1).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn keystore_value_round_trip_and_strict() {
        let key = fixed_key();
        let value = key.to_keystore_value().unwrap();
        assert_eq!(value.expose_secret().len(), KEYSTORE_VALUE_LEN);
        assert!(!value.expose_secret().contains(['=', '+', '/']));
        let back = CliSessionKey::from_keystore_value(value.expose_secret()).unwrap();
        assert_eq!(back.session_id, key.session_id);
        assert_eq!(back.w, key.w);
        let s = value.expose_secret();
        for bad in [
            "",
            &s[..63],
            &format!("{s}A"),
            &s.replace(&s[..1], "!"),
            &format!("{}=", &s[..63]),
        ] {
            assert_eq!(
                CliSessionKey::from_keystore_value(bad).unwrap_err(),
                Error::Malformed,
                "{bad:?}"
            );
        }
        assert_eq!(format!("{key:?}"), "[REDACTED]");
    }

    #[test]
    fn unknown_magic_version_alg_refused() {
        let key = fixed_key();
        let good = forged(&key, NOW, NOW + 1000);
        for (i, v) in [(0usize, b'X'), (4, 2), (5, 2), (5, 0)] {
            let mut bytes = *good.as_bytes();
            bytes[i] = v;
            let bad = CliSessionFile::from_bytes(&bytes).unwrap();
            assert_eq!(
                open_cli_session(&key, &bad, &binding(), NOW).unwrap_err(),
                Error::Decryption
            );
            assert_eq!(bad.claimed_expires_at_ms(), None);
        }
    }

    /// `crates/core/tests/vectors/export_kat.txt`, written by the independent
    /// script `export_kat.py`: the whole session file for fixed inputs and a
    /// fixed nonce.
    mod kat {
        use super::*;
        use crate::export::kat::vector;

        #[test]
        fn matches_python() {
            let file = seal_with(
                &ak(),
                &fixed_key(),
                &binding(),
                &[0x66; NONCE_LEN],
                NOW,
                DEFAULT_TTL_MS,
            )
            .unwrap();
            assert_eq!(file.as_bytes().as_slice(), vector("session_file"));
            assert_eq!(
                fixed_key()
                    .to_keystore_value()
                    .unwrap()
                    .expose_secret()
                    .as_bytes(),
                vector("session_keystore_value_text")
            );
            let back = open_cli_session(&fixed_key(), &file, &binding(), NOW + 1).unwrap();
            assert_eq!(back, ak());
        }
    }
}

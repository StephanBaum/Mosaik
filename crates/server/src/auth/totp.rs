//! TOTP second factor of the sync account (CRYPTO_SPEC §9.3).
//!
//! The secret is 20 random bytes (SHA-1, 6 digits, 30 s, ADR-0008 replay
//! guard) and is stored sealed:
//!
//! ```text
//! K_totp = HKDF(server_secret, "", ctx("server/totp-key"), 32)
//! sealed = header || nonce[24] || XChaCha20-Poly1305(K_totp, nonce, secret,
//!            aad = header || ctx("server/totp-at-rest") || 0x00 || account_id)
//! header = version:u8 (1) || algorithm:u8 (1)
//! ```

use crate::auth::subkey;
use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::{Tag, XChaCha20Poly1305, XNonce};
use mosaic_core::encoding::ctx;
use mosaic_totp::{Algorithm, Digits, Otp, OtpAuth, OtpKind, OtpSecret, Period, SecretText};
use zeroize::Zeroizing;

/// The AAD label of a sealed secret.
const LABEL: &str = "server/totp-at-rest";
/// The HKDF label of `K_totp`.
const KEY_LABEL: &str = "server/totp-key";
/// Length of the TOTP secret in bytes.
pub const SECRET_LEN: usize = 20;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
/// The window of accepted time steps around now (RFC 6238 §5.2).
const WINDOW: u8 = 1;

/// The §3 header of a sealed secret: version 1, algorithm 1.
const HEADER: [u8; 2] = [1, 1];

fn aad(account_id: &[u8; 16]) -> Vec<u8> {
    let mut aad = HEADER.to_vec();
    aad.extend_from_slice(&ctx(LABEL));
    aad.push(0);
    aad.extend_from_slice(account_id);
    aad
}

/// A fresh random secret.
#[must_use]
pub fn generate() -> Option<Zeroizing<[u8; SECRET_LEN]>> {
    let mut s = Zeroizing::new([0u8; SECRET_LEN]);
    getrandom::fill(s.as_mut_slice()).ok()?;
    Some(s)
}

/// Seals a secret for storage.
#[must_use]
pub fn seal(
    server_secret: &[u8; 32],
    account_id: &[u8; 16],
    secret: &[u8; SECRET_LEN],
) -> Option<Vec<u8>> {
    let key = subkey(server_secret, KEY_LABEL)?;
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).ok()?;
    let mut buf = Zeroizing::new(secret.to_vec());
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let tag = cipher
        .encrypt_inout_detached(
            &XNonce::from(nonce),
            &aad(account_id),
            buf.as_mut_slice().into(),
        )
        .ok()?;
    let mut out = Vec::with_capacity(HEADER.len() + NONCE_LEN + buf.len() + TAG_LEN);
    out.extend_from_slice(&HEADER);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&buf);
    out.extend_from_slice(&tag);
    Some(out)
}

/// Opens a stored secret; `None` if it does not authenticate (wrong server
/// secret, other account or damaged).
#[must_use]
pub fn open(
    server_secret: &[u8; 32],
    account_id: &[u8; 16],
    sealed: &[u8],
) -> Option<Zeroizing<Vec<u8>>> {
    let key = subkey(server_secret, KEY_LABEL)?;
    let rest = sealed.strip_prefix(HEADER.as_slice())?;
    let (nonce, rest) = rest.split_at_checked(NONCE_LEN)?;
    let (ct, tag) = rest.split_at_checked(rest.len().checked_sub(TAG_LEN)?)?;
    let nonce: [u8; NONCE_LEN] = nonce.try_into().ok()?;
    let tag = Tag::try_from(tag).ok()?;
    let mut buf = Zeroizing::new(ct.to_vec());
    let cipher = XChaCha20Poly1305::new((&*key).into());
    cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce),
            &aad(account_id),
            buf.as_mut_slice().into(),
            &tag,
        )
        .ok()?;
    Some(buf)
}

fn otp(secret: &[u8]) -> Option<Otp> {
    Some(Otp {
        secret: OtpSecret::from_bytes(secret).ok()?,
        algorithm: Algorithm::Sha1,
        kind: OtpKind::Totp {
            digits: Digits::DEFAULT,
            period: Period::DEFAULT,
        },
    })
}

/// Checks a code against a secret at `now_ms`, with the replay guard: the
/// matched step must be above `last_step`. Returns the matched step.
#[must_use]
pub fn verify(secret: &[u8], code: &str, now_ms: u64, last_step: Option<u64>) -> Option<u64> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    otp(secret)?
        .verify(code, now_ms / 1000, WINDOW, last_step)
        .ok()
        .flatten()
}

/// The setup values for a secret: its base32 text and the `otpauth://` URI
/// (`otpauth://totp/Mosaic%20sync:<email>?secret=<b32>&issuer=Mosaic%20sync`).
#[must_use]
pub fn setup_values(secret: &[u8], email: &str) -> Option<(SecretText, SecretText)> {
    let otp = otp(secret)?;
    let b32 = otp.secret.to_base32();
    let uri = OtpAuth {
        otp,
        issuer: Some(SecretText::new("Mosaic sync")),
        account_name: SecretText::new(email),
    }
    .to_uri()
    .ok()?;
    Some((b32, uri))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_binds_account_and_secret() {
        let ss = [3u8; 32];
        let acc = [4u8; 16];
        let secret = generate().unwrap();
        let sealed = seal(&ss, &acc, &secret).unwrap();
        assert_eq!(sealed.len(), 2 + NONCE_LEN + SECRET_LEN + TAG_LEN);
        assert_eq!(&sealed[..2], &[1, 1]);
        // The header is part of the AAD and is checked: another version or
        // algorithm byte is refused, and so is no header at all.
        for i in 0..2 {
            let mut other = sealed.clone();
            other[i] ^= 1;
            assert!(open(&ss, &acc, &other).is_none(), "header byte {i}");
        }
        assert!(open(&ss, &acc, &sealed[2..]).is_none());
        assert_eq!(
            open(&ss, &acc, &sealed).unwrap().as_slice(),
            secret.as_slice()
        );
        assert!(open(&ss, &[5u8; 16], &sealed).is_none());
        assert!(open(&[9u8; 32], &acc, &sealed).is_none());
        let mut bad = sealed.clone();
        bad[32] ^= 1;
        assert!(open(&ss, &acc, &bad).is_none());
    }

    #[test]
    fn verify_has_replay_guard_and_window() {
        let secret = b"12345678901234567890";
        let o = otp(secret).unwrap();
        let t = 1_000_000_020u64;
        let code = o.code_at(t).unwrap();
        let step = verify(secret, code.expose_secret(), t * 1000, None).unwrap();
        assert_eq!(step, t / 30);
        assert!(verify(secret, code.expose_secret(), t * 1000, Some(step)).is_none());
        // One step of skew is accepted, three are not.
        assert!(verify(secret, code.expose_secret(), (t + 30) * 1000, None).is_some());
        assert!(verify(secret, code.expose_secret(), (t + 90) * 1000, None).is_none());
        assert!(verify(secret, "12345", t * 1000, None).is_none());
        assert!(verify(secret, "abcdef", t * 1000, None).is_none());
    }
}

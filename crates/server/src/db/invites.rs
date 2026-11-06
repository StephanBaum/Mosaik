//! The `invites` table. Only SHA-256 hashes of invite codes are stored.

use rusqlite::{Connection, Transaction, params};
use sha2::{Digest, Sha256};

use super::int;
use crate::error::ApiError;

/// Stores a new invite.
pub fn create(conn: &Connection, hash: &[u8; 32], now: u64) -> Result<(), ApiError> {
    conn.execute(
        "INSERT INTO invites (code_hash, created_at) VALUES (?1, ?2)",
        params![hash.as_slice(), int(now)],
    )?;
    Ok(())
}

/// Marks an unused invite as used; false if there is none.
pub fn redeem(tx: &Transaction<'_>, hash: &[u8; 32], now: u64) -> Result<bool, ApiError> {
    let n = tx.execute(
        "UPDATE invites SET used_at = ?1 WHERE code_hash = ?2 AND used_at IS NULL",
        params![int(now), hash.as_slice()],
    )?;
    Ok(n == 1)
}

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters in an invite code (Crockford base32, 100 bits).
pub const CODE_LEN: usize = 20;

/// A fresh invite code.
#[must_use]
pub fn generate() -> Option<String> {
    let mut raw = [0u8; CODE_LEN];
    getrandom::fill(&mut raw).ok()?;
    // 256 is a multiple of 32, so the modulo is unbiased.
    Some(
        raw.iter()
            .map(|b| char::from(ALPHABET.get(usize::from(b % 32)).copied().unwrap_or(b'0')))
            .collect(),
    )
}

/// The canonical form of a typed code (upper case, no separators, Crockford
/// look-alike letters mapped), or `None` if it cannot be a code.
#[must_use]
pub fn normalize(code: &str) -> Option<String> {
    let out: String = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    (out.len() == CODE_LEN && out.bytes().all(|b| ALPHABET.contains(&b))).then_some(out)
}

/// What the database stores for a canonical code.
#[must_use]
pub fn hash(normalized: &str) -> [u8; 32] {
    Sha256::digest(normalized.as_bytes()).into()
}

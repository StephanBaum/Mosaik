//! `random(n)` from CRYPTO_SPEC §3: bytes from the OS CSPRNG via `getrandom`.
//!
//! If the CSPRNG fails, the operation fails with [`Error::Randomness`].
//! There is no fallback generator.

use crate::error::{Error, Result};

/// Fills `dest` with bytes from the operating system's CSPRNG.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG is unavailable or fails.
pub(crate) fn fill(dest: &mut [u8]) -> Result<()> {
    getrandom::fill(dest).map_err(|_| Error::Randomness)
}

/// Returns `N` random bytes. Only for values that are not secret
/// (nonces, salts, IDs); secrets are generated in place in their container.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub(crate) fn array<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0u8; N];
    fill(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_distinct_values() {
        let a: [u8; 32] = array().unwrap();
        let b: [u8; 32] = array().unwrap();
        assert_ne!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn fill_empty_is_ok() {
        fill(&mut []).unwrap();
    }
}

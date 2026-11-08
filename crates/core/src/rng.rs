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

/// The random number generator handed to `hpke` for one seal.
///
/// A seal draws exactly one ephemeral X25519 `ikm` (32 bytes). Instead of
/// giving `hpke` an infallible adapter over the OS CSPRNG, which would have
/// to abort the process when the CSPRNG fails, the 32 bytes are drawn first,
/// with the usual error, and served from a buffer. `hpke` asks for more than
/// the buffer holds only if its construction changes; the shortfall is filled
/// with zeros and [`HpkeRng::check`] turns it into [`Error::Randomness`], so
/// the result of such a seal is discarded and never used (ADR-0049). The
/// buffer is zeroized on drop.
pub(crate) struct HpkeRng {
    buf: zeroize::Zeroizing<[u8; 32]>,
    used: usize,
    starved: bool,
}

impl HpkeRng {
    /// Draws the ephemeral `ikm` from the OS CSPRNG.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub(crate) fn from_os() -> Result<Self> {
        Ok(Self::fixed(array()?))
    }

    /// A generator that serves `bytes`. Used by [`HpkeRng::from_os`] and,
    /// in tests, to reproduce a known ephemeral key.
    pub(crate) fn fixed(bytes: [u8; 32]) -> Self {
        Self {
            buf: zeroize::Zeroizing::new(bytes),
            used: 0,
            starved: false,
        }
    }

    /// Fails if `hpke` asked for more bytes than were drawn.
    ///
    /// # Errors
    /// [`Error::Randomness`] on a shortfall.
    pub(crate) fn check(&self) -> Result<()> {
        if self.starved {
            Err(Error::Randomness)
        } else {
            Ok(())
        }
    }

    fn serve(&mut self, dst: &mut [u8]) {
        for out in dst.iter_mut() {
            if let Some(b) = self.buf.get(self.used) {
                *out = *b;
                self.used += 1;
            } else {
                *out = 0;
                self.starved = true;
            }
        }
    }
}

impl hpke::rand_core::TryRng for HpkeRng {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> core::result::Result<u32, Self::Error> {
        let mut b = [0u8; 4];
        self.serve(&mut b);
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> core::result::Result<u64, Self::Error> {
        let mut b = [0u8; 8];
        self.serve(&mut b);
        Ok(u64::from_le_bytes(b))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> core::result::Result<(), Self::Error> {
        self.serve(dst);
        Ok(())
    }
}

impl hpke::rand_core::TryCryptoRng for HpkeRng {}

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
    fn hpke_rng_serves_the_buffer_then_flags_a_shortfall() {
        use hpke::rand_core::Rng;
        let mut r = HpkeRng::fixed([7; 32]);
        let mut a = [0u8; 32];
        r.fill_bytes(&mut a);
        assert_eq!(a, [7; 32]);
        r.check().unwrap();
        let mut b = [1u8; 4];
        r.fill_bytes(&mut b);
        assert_eq!(b, [0; 4]);
        assert_eq!(r.check().unwrap_err(), Error::Randomness);
        let os = HpkeRng::from_os().unwrap();
        assert_ne!(*os.buf, [0u8; 32]);
    }

    #[test]
    fn fill_empty_is_ok() {
        fill(&mut []).unwrap();
    }
}

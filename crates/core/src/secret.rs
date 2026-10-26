//! Containers for secret material (CRYPTO_SPEC §4).
//!
//! Every type here:
//! - zeroizes its memory on drop,
//! - does **not** implement `Clone`, `Copy`, `Display` or `Serialize`,
//! - prints only `"[REDACTED]"` through `Debug`,
//! - compares in constant time (via `subtle`) where equality is offered.
//!
//! Fixed-size secrets live in a heap box so that moving the container does
//! not leave copies of the bytes behind on the stack.

use core::fmt;

use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{Error, Result};
use crate::ids::KeyGeneration;
use crate::rng;

const REDACTED: &str = "[REDACTED]";

/// A fixed-size secret of `N` bytes that is zeroized on drop.
pub struct SecretBytes<const N: usize>(Box<[u8; N]>);

impl<const N: usize> SecretBytes<N> {
    /// An all-zero secret, to be filled in place.
    pub(crate) fn zero() -> Self {
        Self(Box::new([0u8; N]))
    }

    /// `N` fresh bytes from the OS CSPRNG, generated in place.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn random() -> Result<Self> {
        let mut s = Self::zero();
        rng::fill(s.0.as_mut_slice())?;
        Ok(s)
    }

    /// Moves `bytes` into a new secret and zeroizes the source array.
    #[must_use]
    pub fn from_array(bytes: &mut [u8; N]) -> Self {
        let mut s = Self::zero();
        s.0.copy_from_slice(bytes);
        bytes.zeroize();
        s
    }

    /// Copies `bytes` into a new secret. The caller remains responsible for
    /// the source buffer.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `bytes.len() != N`.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != N {
            return Err(Error::Malformed);
        }
        let mut s = Self::zero();
        s.0.copy_from_slice(bytes);
        Ok(s)
    }

    /// Borrows the secret bytes. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; N] {
        &self.0
    }

    /// Mutable access for in-place derivation.
    pub(crate) fn expose_secret_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
    }
}

impl<const N: usize> Drop for SecretBytes<N> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<const N: usize> ZeroizeOnDrop for SecretBytes<N> {}

impl<const N: usize> fmt::Debug for SecretBytes<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl<const N: usize> ConstantTimeEq for SecretBytes<N> {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.as_slice().ct_eq(other.0.as_slice())
    }
}

impl<const N: usize> PartialEq for SecretBytes<N> {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl<const N: usize> Eq for SecretBytes<N> {}

/// A variable-length secret byte string (decrypted plaintext, prepared
/// passwords) that is zeroized on drop, including spare capacity.
pub struct SecretVec(Vec<u8>);

impl SecretVec {
    /// An empty secret buffer with room for `capacity` bytes. Reserving up
    /// front avoids reallocations that would leave unzeroized copies behind.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the memory cannot be reserved.
    pub(crate) fn with_capacity(capacity: usize) -> Result<Self> {
        let mut v = Vec::new();
        v.try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation)?;
        Ok(Self(v))
    }

    /// Copies `bytes` into a new secret buffer.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the memory cannot be reserved.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let mut s = Self::with_capacity(bytes.len())?;
        s.0.extend_from_slice(bytes);
        Ok(s)
    }

    /// Borrows the secret bytes. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }

    /// Length in bytes. Lengths are not considered secret.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Mutable access to the backing vector for in-place crypto.
    pub(crate) fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    /// The allocated capacity, so tests can check that a buffer reserved up
    /// front was never reallocated (a reallocation leaves an unzeroized
    /// copy behind).
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl From<Vec<u8>> for SecretVec {
    /// Takes ownership of `v`; it will be zeroized on drop.
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl Drop for SecretVec {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for SecretVec {}

impl fmt::Debug for SecretVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl ConstantTimeEq for SecretVec {
    /// Constant time in the contents; the lengths are compared first and are
    /// not secret.
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.as_slice().ct_eq(other.0.as_slice())
    }
}

impl PartialEq for SecretVec {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for SecretVec {}

/// A secret UTF-8 string (a Secret Key display string, the last group shown
/// during onboarding) that is zeroized on drop.
pub struct SecretString(String);

impl SecretString {
    /// An empty secret string with room for `capacity` bytes.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the memory cannot be reserved.
    pub(crate) fn with_capacity(capacity: usize) -> Result<Self> {
        let mut s = String::new();
        s.try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation)?;
        Ok(Self(s))
    }

    /// Borrows the secret string. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub(crate) fn push(&mut self, c: char) {
        self.0.push(c);
    }

    /// The allocated capacity (see [`SecretVec::capacity`]).
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for SecretString {}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// The user's master password `MP` exactly as entered (CRYPTO_SPEC §5.1).
///
/// It is normalized (NFC) only when it is stretched; see
/// [`crate::kdf::prepare_password`].
pub struct MasterPassword(SecretString);

impl MasterPassword {
    /// Takes ownership of `password`; it is zeroized on drop.
    #[must_use]
    pub fn new(password: String) -> Self {
        Self(SecretString(password))
    }

    /// Borrows the password as entered.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl From<String> for MasterPassword {
    fn from(password: String) -> Self {
        Self::new(password)
    }
}

impl fmt::Debug for MasterPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// Defines a 256-bit key newtype with redacted `Debug` and constant-time
/// equality.
macro_rules! key_newtype {
    ($(#[$meta:meta])* $name:ident, $len:expr) => {
        $(#[$meta])*
        pub struct $name(pub(crate) SecretBytes<$len>);

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(REDACTED)
            }
        }

        impl ConstantTimeEq for $name {
            fn ct_eq(&self, other: &Self) -> Choice {
                self.0.ct_eq(&other.0)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.ct_eq(other).into()
            }
        }

        impl Eq for $name {}

        impl ZeroizeOnDrop for $name {}
    };
}

key_newtype!(
    /// The account key `AK` (CRYPTO_SPEC §6): the single key every unlock
    /// method unlocks. It wraps the vault keys.
    AccountKey,
    32
);

key_newtype!(
    /// A key-encryption key `W` (CRYPTO_SPEC §6) that wraps the account key
    /// for one unlock method. For password unlock this is `KEK` (§5.4).
    Kek,
    32
);

key_newtype!(
    /// The OPAQUE password input `OPW` (CRYPTO_SPEC §5.4, §9.1).
    OpaquePassword,
    32
);

key_newtype!(
    /// A per-attachment file key `FK` (CRYPTO_SPEC §7.3). It is stored inside
    /// the owning item's encrypted plaintext.
    AttachmentKey,
    32
);

key_newtype!(
    /// The 128-bit Secret Key `SK` (CRYPTO_SPEC §5.2).
    SecretKey,
    16
);

impl Kek {
    /// Uses an externally derived 32-byte secret as a wrapping key `W`, for
    /// unlock methods other than the password (quick unlock, hardware key).
    #[must_use]
    pub fn from_secret(secret: SecretBytes<32>) -> Self {
        Self(secret)
    }
}

impl OpaquePassword {
    /// Borrows `OPW` for the OPAQUE client. Never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }
}

impl AttachmentKey {
    /// A fresh random `FK`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn generate() -> Result<Self> {
        Ok(Self(SecretBytes::random()?))
    }

    /// Restores an `FK` read from an item's decrypted plaintext.
    #[must_use]
    pub fn from_secret(secret: SecretBytes<32>) -> Self {
        Self(secret)
    }

    /// Borrows the key bytes, to store them in the item plaintext.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }
}

impl SecretKey {
    /// Restores a Secret Key from the device keystore.
    #[must_use]
    pub fn from_secret(secret: SecretBytes<16>) -> Self {
        Self(secret)
    }

    /// Borrows the raw 16 bytes, to put them into the device keystore.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; 16] {
        self.0.expose_secret()
    }
}

/// A vault key `VK` together with its `key_generation` (CRYPTO_SPEC §7.1).
pub struct VaultKey {
    pub(crate) key: SecretBytes<32>,
    pub(crate) generation: KeyGeneration,
}

impl VaultKey {
    /// The key generation this key belongs to.
    #[must_use]
    pub fn generation(&self) -> KeyGeneration {
        self.generation
    }

    /// A vault key from fixed bytes, so fuzz targets can open the golden
    /// objects and reproduce crashes from saved inputs. Only compiled under
    /// `cfg(fuzzing)` (set by cargo-fuzz); real vault keys come from
    /// [`VaultKey::generate`] or an unwrap.
    #[cfg(fuzzing)]
    #[doc(hidden)]
    #[must_use]
    pub fn from_secret_for_fuzzing(key: SecretBytes<32>, generation: KeyGeneration) -> Self {
        Self { key, generation }
    }
}

impl fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultKey")
            .field("key", &REDACTED)
            .field("generation", &self.generation)
            .finish()
    }
}

impl ConstantTimeEq for VaultKey {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.key.ct_eq(&other.key) & self.generation.get().ct_eq(&other.generation.get())
    }
}

impl PartialEq for VaultKey {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for VaultKey {}

impl ZeroizeOnDrop for VaultKey {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let mut raw = [7u8; 32];
        let s = SecretBytes::from_array(&mut raw);
        assert_eq!(raw, [0u8; 32], "source must be zeroized");
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        assert_eq!(
            format!("{:?}", SecretVec::from(vec![1, 2, 3])),
            "[REDACTED]"
        );
        assert_eq!(
            format!("{:?}", MasterPassword::new("hunter2".into())),
            "[REDACTED]"
        );
        let k = AccountKey(SecretBytes::from_array(&mut [1u8; 32]));
        assert_eq!(format!("{k:?}"), "[REDACTED]");
        let vk = VaultKey {
            key: SecretBytes::from_array(&mut [1u8; 32]),
            generation: KeyGeneration::FIRST,
        };
        let dbg = format!("{vk:?}");
        assert!(dbg.contains("[REDACTED]") && !dbg.contains("1, 1"));
        let mut ss = SecretString::with_capacity(4).unwrap();
        ss.push('x');
        assert_eq!(format!("{ss:?}"), "[REDACTED]");
        assert_eq!(ss.expose_secret(), "x");
    }

    #[test]
    fn equality_is_by_value() {
        let a = SecretBytes::<16>::from_slice(&[1; 16]).unwrap();
        let b = SecretBytes::<16>::from_slice(&[1; 16]).unwrap();
        let c = SecretBytes::<16>::from_slice(&[2; 16]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(
            SecretBytes::<16>::from_slice(&[1; 15]).unwrap_err(),
            Error::Malformed
        );
        let v1 = SecretVec::from_slice(b"abc").unwrap();
        assert_eq!(v1, SecretVec::from(b"abc".to_vec()));
        assert_ne!(v1, SecretVec::from(b"abd".to_vec()));
        assert_ne!(v1, SecretVec::from(b"ab".to_vec()));
        assert_eq!(v1.len(), 3);
        assert!(!v1.is_empty());
        let r1 = SecretBytes::<32>::random().unwrap();
        let r2 = SecretBytes::<32>::random().unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn key_newtypes() {
        let fk = AttachmentKey::generate().unwrap();
        let copy = AttachmentKey::from_secret(SecretBytes::from_slice(fk.expose_secret()).unwrap());
        assert_eq!(fk, copy);
        assert_ne!(fk, AttachmentKey::generate().unwrap());
        let sk = SecretKey::from_secret(SecretBytes::from_array(&mut [9u8; 16]));
        assert_eq!(sk.expose_secret(), &[9u8; 16]);
        let opw = OpaquePassword(SecretBytes::from_array(&mut [3u8; 32]));
        assert_eq!(opw.expose_secret(), &[3u8; 32]);
        let w = Kek::from_secret(SecretBytes::from_array(&mut [4u8; 32]));
        assert_eq!(w, Kek(SecretBytes::from_array(&mut [4u8; 32])));
        let mp = MasterPassword::from(String::from("pw"));
        assert_eq!(mp.expose_secret(), "pw");
        let vk1 = VaultKey {
            key: SecretBytes::from_array(&mut [1u8; 32]),
            generation: KeyGeneration::FIRST,
        };
        let vk2 = VaultKey {
            key: SecretBytes::from_array(&mut [1u8; 32]),
            generation: KeyGeneration::new(2).unwrap(),
        };
        assert_ne!(vk1, vk2);
        assert_eq!(vk2.generation().get(), 2);
    }
}

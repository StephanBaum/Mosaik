//! Identifiers: 16-byte UUIDs (RFC 9562) and vault key generations.
//!
//! New IDs are UUIDv7. The nil UUID is reserved and rejected by every
//! constructor (CRYPTO_SPEC §3).

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::rng;

/// Builds a UUIDv7 (RFC 9562 §5.7) from a Unix timestamp in milliseconds and
/// 10 random bytes: `unix_ts_ms:48 || ver:4 || rand_a:12 || var:2 || rand_b:62`.
fn uuid_v7_from(unix_ms: u64, random: [u8; 10]) -> [u8; 16] {
    let ts = (unix_ms & 0xFFFF_FFFF_FFFF).to_be_bytes();
    let [_, _, t0, t1, t2, t3, t4, t5] = ts;
    let [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9] = random;
    [
        t0,
        t1,
        t2,
        t3,
        t4,
        t5,
        0x70 | (r0 & 0x0F),
        r1,
        0x80 | (r2 & 0x3F),
        r3,
        r4,
        r5,
        r6,
        r7,
        r8,
        r9,
    ]
}

fn new_v7_bytes() -> Result<[u8; 16]> {
    // A clock before 1970 or beyond u64 milliseconds is clamped; uniqueness
    // comes from the 74 random bits either way.
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    Ok(uuid_v7_from(unix_ms, rng::array()?))
}

fn fmt_uuid(bytes: &[u8; 16], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            f.write_str("-")?;
        }
        write!(f, "{b:02x}")?;
    }
    Ok(())
}

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);

        impl $name {
            /// Wraps 16 raw UUID bytes.
            ///
            /// # Errors
            /// [`Error::InvalidId`] for the reserved nil UUID.
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
                if bytes == [0u8; 16] {
                    return Err(Error::InvalidId);
                }
                Ok(Self(bytes))
            }

            /// Parses 16 raw UUID bytes from a slice.
            ///
            /// # Errors
            /// [`Error::Malformed`] if the slice is not 16 bytes long,
            /// [`Error::InvalidId`] for the nil UUID.
            pub fn from_slice(bytes: &[u8]) -> Result<Self> {
                let arr: [u8; 16] = bytes.try_into().map_err(|_| Error::Malformed)?;
                Self::from_bytes(arr)
            }

            /// A new UUIDv7 from the system clock and the OS CSPRNG.
            ///
            /// # Errors
            /// [`Error::Randomness`] if the CSPRNG fails.
            pub fn new_v7() -> Result<Self> {
                Self::from_bytes(new_v7_bytes()?)
            }

            /// The raw 16 bytes, as used in AAD and HKDF salts.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            /// Canonical lowercase hyphenated form.
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_uuid(&self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_uuid(&self.0, f)?;
                f.write_str(")")
            }
        }
    };
}

uuid_id!(
    /// An account ID (16-byte UUID). Also the HKDF salt in CRYPTO_SPEC §5.4.
    AccountId
);
uuid_id!(
    /// A vault ID (16-byte UUID).
    VaultId
);
uuid_id!(
    /// An item ID (16-byte UUID). It is also the sync `record_id` (§7.2).
    ItemId
);
uuid_id!(
    /// An attachment ID (16-byte UUID).
    AttachmentId
);
uuid_id!(
    /// A device ID (16-byte UUID). It is the `method_id` of quick unlock.
    DeviceId
);

/// A vault key generation (CRYPTO_SPEC §7.1). Starts at 1 and increases on
/// every rotation; 0 is invalid.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KeyGeneration(u32);

impl KeyGeneration {
    /// The generation of a newly created vault.
    pub const FIRST: Self = Self(1);

    /// Wraps a generation number.
    ///
    /// # Errors
    /// [`Error::InvalidKeyGeneration`] for 0.
    pub const fn new(generation: u32) -> Result<Self> {
        if generation == 0 {
            return Err(Error::InvalidKeyGeneration);
        }
        Ok(Self(generation))
    }

    /// The generation number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The next generation, for a rotation.
    ///
    /// # Errors
    /// [`Error::InvalidKeyGeneration`] if the counter would overflow.
    pub const fn next(self) -> Result<Self> {
        match self.0.checked_add(1) {
            Some(n) => Ok(Self(n)),
            None => Err(Error::InvalidKeyGeneration),
        }
    }
}

impl fmt::Display for KeyGeneration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_is_rejected() {
        assert_eq!(VaultId::from_bytes([0; 16]), Err(Error::InvalidId));
        assert_eq!(ItemId::from_slice(&[0; 16]), Err(Error::InvalidId));
        assert_eq!(AccountId::from_slice(&[1; 15]), Err(Error::Malformed));
        assert!(AttachmentId::from_bytes([1; 16]).is_ok());
        assert_eq!(DeviceId::from_bytes([0; 16]), Err(Error::InvalidId));
        assert_eq!(AccountId::from_bytes([0; 16]), Err(Error::InvalidId));
        assert_eq!(AttachmentId::from_slice(&[0; 17]), Err(Error::Malformed));
    }

    #[test]
    fn v7_layout() {
        let id = ItemId::new_v7().unwrap();
        let b = id.as_bytes();
        assert_eq!(b[6] >> 4, 7, "version nibble");
        assert_eq!(b[8] >> 6, 0b10, "RFC 9562 variant");
        assert_ne!(id, ItemId::new_v7().unwrap());

        let fixed = uuid_v7_from(0x0189_1234_5678, [0xFF; 10]);
        assert_eq!(
            fixed,
            [
                0x01, 0x89, 0x12, 0x34, 0x56, 0x78, 0x7F, 0xFF, 0xBF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF
            ]
        );
        // Timestamps are big-endian, so later IDs sort later.
        assert!(uuid_v7_from(1, [0; 10]) < uuid_v7_from(2, [0; 10]));
    }

    fn now_ms() -> u64 {
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    }

    /// `new_v7` embeds the current Unix time in milliseconds (the first 48
    /// bits, big-endian), not a constant or a truncated value.
    #[test]
    fn v7_embeds_current_unix_ms() {
        for _ in 0..8 {
            let before = now_ms();
            let id = AccountId::new_v7().unwrap();
            let after = now_ms();
            let b = id.as_bytes();
            let mut ts = [0u8; 8];
            ts[2..].copy_from_slice(&b[..6]);
            let ts = u64::from_be_bytes(ts);
            assert!(before <= ts && ts <= after, "{before} <= {ts} <= {after}");
        }
    }

    #[test]
    fn display_and_debug() {
        let id = VaultId::from_bytes([
            0x01, 0x89, 0x12, 0x34, 0x56, 0x78, 0x7a, 0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab,
        ])
        .unwrap();
        assert_eq!(id.to_string(), "01891234-5678-7abc-8def-0123456789ab");
        assert_eq!(
            format!("{id:?}"),
            "VaultId(01891234-5678-7abc-8def-0123456789ab)"
        );
    }

    #[test]
    fn key_generation() {
        assert_eq!(KeyGeneration::new(0), Err(Error::InvalidKeyGeneration));
        assert_eq!(KeyGeneration::FIRST.get(), 1);
        assert_eq!(KeyGeneration::FIRST.next().unwrap().get(), 2);
        assert_eq!(
            KeyGeneration::new(u32::MAX).unwrap().next(),
            Err(Error::InvalidKeyGeneration)
        );
        assert_eq!(KeyGeneration::new(7).unwrap().to_string(), "7");
    }
}

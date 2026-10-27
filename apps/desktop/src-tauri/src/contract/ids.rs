//! IDs on the wire: the canonical lowercase hyphenated UUID form that
//! `mosaic-core` prints. Parsing is strict (exactly that form, never the
//! nil UUID), so one ID has one spelling.

use core::fmt;

use mosaic_core::{AccountId, AttachmentId, ItemId, VaultId};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! wire_id {
    ($(#[$meta:meta])* $name:ident => $core:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub $core);

        impl $name {
            /// The core ID.
            #[must_use]
            pub const fn get(self) -> $core {
                self.0
            }
        }

        impl From<$core> for $name {
            fn from(id: $core) -> Self {
                Self(id)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                // Owned: Tauri hands commands a parsed `serde_json::Value`,
                // which cannot lend a `&str`.
                let text = String::deserialize(deserializer)?;
                parse_uuid(&text)
                    .and_then(|bytes| $core::from_bytes(bytes).ok())
                    .map(Self)
                    .ok_or_else(|| D::Error::custom("expected a canonical non-nil UUID"))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }
    };
}

wire_id!(
    /// An account ID on the wire.
    WireAccountId => AccountId
);
wire_id!(
    /// A vault ID on the wire.
    WireVaultId => VaultId
);
wire_id!(
    /// An item ID on the wire.
    WireItemId => ItemId
);
wire_id!(
    /// An attachment ID on the wire.
    WireAttachmentId => AttachmentId
);

/// Parses `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` in lowercase hex.
fn parse_uuid(text: &str) -> Option<[u8; 16]> {
    if text.len() != 36 {
        return None;
    }
    let mut bytes = [0u8; 16];
    let mut nibbles = 0usize;
    for (position, ch) in text.bytes().enumerate() {
        if matches!(position, 8 | 13 | 18 | 23) {
            if ch != b'-' {
                return None;
            }
            continue;
        }
        let value = match ch {
            b'0'..=b'9' => ch - b'0',
            b'a'..=b'f' => ch - b'a' + 10,
            _ => return None,
        };
        let byte = bytes.get_mut(nibbles / 2)?;
        *byte = (*byte << 4) | value;
        nibbles += 1;
    }
    (nibbles == 32).then_some(bytes)
}

/// Milliseconds since the Unix epoch, as JavaScript's `Date` counts them.
pub type UnixMillis = i64;

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    /// Deterministic IDs for fixtures and tests.
    pub(crate) fn vault(n: u8) -> WireVaultId {
        WireVaultId(VaultId::from_bytes(bytes(0x10, n)).unwrap())
    }

    pub(crate) fn item(n: u8) -> WireItemId {
        WireItemId(ItemId::from_bytes(bytes(0x20, n)).unwrap())
    }

    pub(crate) fn account() -> WireAccountId {
        WireAccountId(AccountId::from_bytes(bytes(0x30, 1)).unwrap())
    }

    pub(crate) fn attachment(n: u8) -> WireAttachmentId {
        WireAttachmentId(AttachmentId::from_bytes(bytes(0x40, n)).unwrap())
    }

    fn bytes(kind: u8, n: u8) -> [u8; 16] {
        // UUIDv7 layout (version 7, RFC 9562 variant), fixed time.
        [
            0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, kind, 0x80, 0, 0, 0, 0, 0, 0, n,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_in_canonical_form() {
        let id = samples::vault(7);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"019a2b3c-4d5e-7010-8000-000000000007\"");
        let back: WireVaultId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn non_canonical_ids_are_rejected() {
        for bad in [
            "\"019A2B3C-4D5E-7010-8000-000000000007\"",
            "\"019a2b3c4d5e701080000000000000007\"",
            "\"019a2b3c-4d5e-7010-8000-00000000000\"",
            "\"019a2b3c-4d5e-7010-8000-0000000000007\"",
            "\"019a2b3c-4d5e-7010-8000_000000000007\"",
            "\"00000000-0000-0000-0000-000000000000\"",
            "\"019a2b3c-4d5e-7010-8000-00000000000g\"",
            "7",
        ] {
            assert!(serde_json::from_str::<WireVaultId>(bad).is_err(), "{bad}");
        }
    }
}

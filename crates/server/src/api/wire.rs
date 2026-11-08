//! The JSON bodies of the sharing and emergency-access routes (M6b spec §5.2).
//! Same conventions as `mosaic_sync::wire::json`: camelCase keys, unknown
//! keys refused, IDs as lowercase hyphenated UUIDs, bytes as base64url
//! without padding, roles and states as small numbers.

use core::fmt;

use mosaic_sync::wire::json::{B64, B64Fixed};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// A 16-byte ID as a UUID string.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id(pub [u8; 16]);

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Id(..)")
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&crate::db::uuid_text(&self.0))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Id;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a lowercase hyphenated UUID")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Id, E> {
                super::parse_uuid(v)
                    .map(Id)
                    .map_err(|_| E::custom("invalid id"))
            }
        }
        d.deserialize_str(V)
    }
}

/// A sealed key (82 bytes; the header is checked by `ShareSeal::from_bytes`).
pub type SealBytes = B64Fixed<{ mosaic_core::share::ShareSeal::LEN }>;

macro_rules! body {
    ($(#[$m:meta])* $name:ident { $($(#[$fm:meta])* $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name { $($(#[$fm])* pub $f: $t),* }
    };
}

macro_rules! reply {
    ($(#[$m:meta])* $name:ident { $($(#[$fm:meta])* $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[$fm])* pub $f: $t),* }
    };
}

body!(IdentityRequest { ipk: B64Fixed<32>, wrapped_isk: B64Fixed<74> });
body!(LookupRequest { email: String });
reply!(DirectoryEntry { account_id: Id, ipk: B64Fixed<32> });
body!(PutContactsRequest { ciphertext: B64<262_200>, base_seq: u64 });
reply!(ContactsResponse { ciphertext: Option<B64<262_200>>, seq: u64 });
reply!(SeqResponse { seq: u64 });
body!(InviteRequest {
    invite_id: Id,
    recipient: Id,
    role: u8,
    generation: u32,
    seal: SealBytes
});
reply!(InviteSent { expires_at: u64 });
reply!(IncomingInvite {
    invite_id: Id,
    vault_id: Id,
    sender: Id,
    sender_email: String,
    role: u8,
    generation: u32,
    seal: SealBytes,
    expires_at: u64,
});
reply!(OutgoingInvite {
    invite_id: Id,
    vault_id: Id,
    recipient: Id,
    role: u8,
    state: u8,
    delivered: bool,
    expires_at: u64,
});
reply!(InvitesResponse { incoming: Vec<IncomingInvite>, outgoing: Vec<OutgoingInvite> });
reply!(InviteVault { meta: B64<65_600>, roster: Option<B64<65_600>> });
body!(AcceptRequest { wrapped: B64Fixed<74>, device: Id, generation: u32 });
reply!(AcceptResponse {
    vault_id: Id,
    generation: u32,
    role: u8
});
body!(DeclineRequest { block: bool });
reply!(MemberEntry {
    account_id: Id,
    email: String,
    role: u8,
    origin: u8,
    added_at: u64
});
reply!(InviteEntry {
    invite_id: Id,
    recipient: Id,
    role: u8,
    delivered: bool
});
reply!(MembersResponse { members: Vec<MemberEntry>, invites: Vec<InviteEntry> });
body!(RoleRequest { role: u8 });
body!(PutRosterRequest { generation: u32, ciphertext: B64<65_600>, base_seq: Option<u64> });
body!(RecipientSeal {
    recipient: Id,
    seal: SealBytes
});
body!(InviteSeal {
    invite_id: Id,
    seal: SealBytes
});
body!(RotateRequest {
    generation: u32, wrapped: B64Fixed<74>, device: Id, expected_head: u64,
    roster: B64<65_600>, seals: Vec<RecipientSeal>, invite_seals: Vec<InviteSeal>,
    revoke: Vec<Id>,
});
body!(ClaimRequest { wrapped: B64Fixed<74>, device: Id });

body!(VaultSeal {
    vault_id: Id,
    generation: u32,
    seal: SealBytes
});
body!(GrantRequest { wait_days: u8, scope: u8, seals: Vec<VaultSeal> });
body!(SealsRequest { seals: Vec<VaultSeal> });
reply!(VaultSealOut {
    vault_id: Id,
    generation: u32,
    seal: SealBytes
});
reply!(ReleaseResponse { seals: Vec<VaultSealOut> });
reply!(ReleaseAt { release_at: u64 });
reply!(VaultGeneration {
    vault_id: Id,
    generation: u32
});
reply!(GrantedEntry {
    grantee: Id, email: String, wait_days: u8, scope: u8, vaults: Vec<VaultGeneration>,
    state: u8, requested_at: Option<u64>, release_at: Option<u64>,
});
reply!(ReceivedEntry {
    grantor: Id, email: String, wait_days: u8, state: u8, requested_at: Option<u64>,
    release_at: Option<u64>, cooldown_until: Option<u64>,
});
reply!(EmergencyResponse { granted: Vec<GrantedEntry>, received: Vec<ReceivedEntry> });

//! The roster of a shared vault (CRYPTO_SPEC §8.2, ADR-0050).
//!
//! A replicated document in the layout of [`super::VaultMeta`] (§7.5), sealed
//! under the vault key as [`VaultRosterCiphertext`]. Its keys are
//! `member:<account_id as 32 hex digits>` and its values are objects
//! `{"email":E,"fpr":HEX64,"role":R,"since":G}` with `R` one of `owner`,
//! `manage`, `write`, `read`, `removed` and `G` a key generation.
//!
//! # Merge rule
//!
//! The merge of two replicas is the document merge, so it is commutative,
//! associative and idempotent. Member registers can hold several concurrent
//! values; the *effective* value is not the largest stamp but:
//!
//! 1. the highest `since`;
//! 2. on a tie, the most restrictive role (`removed` > `read` > `write` >
//!    `manage` > `owner`);
//! 3. then the largest stamp.
//!
//! A removal is a write of `removed` with `since = g + 1`, the generation of
//! the rotation that locks the member out, never a deletion of the key. So a
//! concurrent role change made at `g` cannot bring a removed member back,
//! and a re-invite, which writes `since` = the then-current generation, beats
//! the removal. Member registers are never shown as conflicts.
//!
//! # Validity
//!
//! [`VaultRoster::validate`] is run on every open: every member value parses,
//! `since` is at most the roster's own generation, and exactly one member is
//! `owner`, the vault's owner as the server reports it.

use core::fmt;

use super::clock::{Stamp, Timestamp};
use super::doc::{Doc, DocJson, Format, decode};
use super::value::{Object, Value, to_json};
use crate::envelope::{MAX_ROSTER_PLAINTEXT_LEN, VaultRosterCiphertext};
use crate::error::{Error, Result};
use crate::identity::Fingerprint;
use crate::ids::{AccountId, DeviceId, KeyGeneration, VaultId};
use crate::secret::{SecretVec, VaultKey};

const MEMBER_PREFIX: &str = "member:";

/// A member's role in a shared vault (§8.2). `Removed` is the tombstone of a
/// removed or departed member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Created the vault; may do everything and can't be removed.
    Owner,
    /// Invites, changes roles below `Owner`, removes, rotates, writes the roster.
    Manage,
    /// Pushes records, metadata and attachments.
    Write,
    /// Pulls only.
    Read,
    /// No longer a member.
    Removed,
}

impl Role {
    /// The wire name used in the roster document.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Manage => "manage",
            Self::Write => "write",
            Self::Read => "read",
            Self::Removed => "removed",
        }
    }

    /// Parses a wire name.
    ///
    /// # Errors
    /// [`Error::Malformed`] for any other string.
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "owner" => Self::Owner,
            "manage" => Self::Manage,
            "write" => Self::Write,
            "read" => Self::Read,
            "removed" => Self::Removed,
            _ => return Err(Error::Malformed),
        })
    }

    /// The server's role number: `owner` 1, `manage` 2, `write` 3, `read` 4.
    /// `Removed` has none.
    #[must_use]
    pub const fn number(self) -> Option<u8> {
        match self {
            Self::Owner => Some(1),
            Self::Manage => Some(2),
            Self::Write => Some(3),
            Self::Read => Some(4),
            Self::Removed => None,
        }
    }

    /// The role of a server role number.
    ///
    /// # Errors
    /// [`Error::Malformed`] outside 1 to 4.
    pub const fn from_number(n: u8) -> Result<Self> {
        match n {
            1 => Ok(Self::Owner),
            2 => Ok(Self::Manage),
            3 => Ok(Self::Write),
            4 => Ok(Self::Read),
            _ => Err(Error::Malformed),
        }
    }

    /// How restrictive the role is: higher wins a `since` tie.
    const fn restriction(self) -> u8 {
        match self {
            Self::Owner => 0,
            Self::Manage => 1,
            Self::Write => 2,
            Self::Read => 3,
            Self::Removed => 4,
        }
    }

    /// Whether this role may write the roster, invite and rotate.
    #[must_use]
    pub const fn can_manage(self) -> bool {
        matches!(self, Self::Owner | Self::Manage)
    }
}

/// One member as the roster shows it.
#[derive(Clone, PartialEq, Eq)]
pub struct RosterMember {
    /// The member's account.
    pub account_id: AccountId,
    /// Their effective role.
    pub role: Role,
    /// The key generation the entry was written at (§8.2).
    pub since: KeyGeneration,
    /// `fpr` of their identity key as the inviting manager saw it.
    pub fingerprint: Fingerprint,
    /// Their account email, for display.
    pub email: String,
}

impl fmt::Debug for RosterMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RosterMember")
            .field("account_id", &self.account_id)
            .field("role", &self.role)
            .field("since", &self.since)
            .finish_non_exhaustive()
    }
}

/// Lowercase hex of `bytes`.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(
            DIGITS.get(usize::from(b >> 4)).copied().unwrap_or(b'0'),
        ));
        out.push(char::from(
            DIGITS.get(usize::from(b & 15)).copied().unwrap_or(b'0'),
        ));
    }
    out
}

/// Strict lowercase-hex decode of exactly `N` bytes.
pub(crate) fn from_hex<const N: usize>(s: &str) -> Result<[u8; N]> {
    let b = s.as_bytes();
    if b.len() != N * 2 {
        return Err(Error::Malformed);
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(Error::Malformed),
    };
    let mut out = [0u8; N];
    let (pairs, _) = b.as_chunks::<2>();
    for (slot, [hi, lo]) in out.iter_mut().zip(pairs) {
        *slot = (nib(*hi)? << 4) | nib(*lo)?;
    }
    Ok(out)
}

fn member_key(account: &AccountId) -> String {
    format!("{MEMBER_PREFIX}{}", to_hex(account.as_bytes()))
}

fn parse_member_key(key: &str) -> Option<AccountId> {
    let hex = key.strip_prefix(MEMBER_PREFIX)?;
    AccountId::from_bytes(from_hex::<16>(hex).ok()?).ok()
}

/// One decoded value of a member register.
struct Parsed {
    role: Role,
    since: KeyGeneration,
    fingerprint: Fingerprint,
    email: String,
}

fn parse_value(v: &Value) -> Result<Parsed> {
    let o = v.as_object().ok_or(Error::Malformed)?;
    let email = o
        .get("email")
        .and_then(Value::as_str)
        .ok_or(Error::Malformed)?
        .to_owned();
    let fpr = o
        .get("fpr")
        .and_then(Value::as_str)
        .ok_or(Error::Malformed)?;
    let role = Role::parse(
        o.get("role")
            .and_then(Value::as_str)
            .ok_or(Error::Malformed)?,
    )?;
    let since = o
        .get("since")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(Error::Malformed)?;
    Ok(Parsed {
        role,
        since: KeyGeneration::new(since)?,
        fingerprint: Fingerprint::from_bytes(&from_hex::<32>(fpr)?)?,
        email,
    })
}

fn build_value(role: Role, since: KeyGeneration, fingerprint: &Fingerprint, email: &str) -> Value {
    let mut o = Object::new();
    o.insert("email", Value::text(email));
    o.insert("fpr", Value::text(&to_hex(fingerprint.as_bytes())));
    o.insert("role", Value::text(role.as_str()));
    o.insert("since", Value::uint(u64::from(since.get())));
    Value::Object(o)
}

/// The effective entry of a register: highest `since`, then most
/// restrictive role, then largest stamp (module documentation).
fn effective(reg: &super::doc::Register) -> Option<Parsed> {
    reg.entries()
        .filter_map(|e| parse_value(e.value()).ok().map(|p| (e.stamp(), p)))
        .max_by_key(|(stamp, p)| (p.since, p.role.restriction(), *stamp))
        .map(|(_, p)| p)
}

/// A decrypted vault roster.
#[derive(PartialEq, Eq)]
pub struct VaultRoster {
    doc: Doc,
}

impl fmt::Debug for VaultRoster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultRoster")
            .field("version", self.doc.vv())
            .finish_non_exhaustive()
    }
}

impl Default for VaultRoster {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultRoster {
    /// An empty roster. A vault that was never shared has none; the owner's
    /// first invite creates it with the owner's entry and the invitee's.
    #[must_use]
    pub fn new() -> Self {
        Self {
            doc: Doc::default(),
        }
    }

    /// Starts an edit; all changes through it are one sync event.
    pub fn edit(&mut self, device: DeviceId, at: Timestamp) -> RosterEdit<'_> {
        RosterEdit {
            doc: &mut self.doc,
            device,
            at,
            stamp: None,
        }
    }

    /// The effective members, by account ID. Entries whose value does not
    /// parse are skipped here and rejected by [`VaultRoster::validate`].
    #[must_use]
    pub fn members(&self) -> Vec<RosterMember> {
        let mut out: Vec<RosterMember> = self
            .doc
            .registers()
            .filter_map(|(key, reg)| {
                let account_id = parse_member_key(key)?;
                let p = effective(reg)?;
                Some(RosterMember {
                    account_id,
                    role: p.role,
                    since: p.since,
                    fingerprint: p.fingerprint,
                    email: p.email,
                })
            })
            .collect();
        out.sort_by_key(|m| *m.account_id.as_bytes());
        out
    }

    /// The effective entry of one account.
    #[must_use]
    pub fn member(&self, account: &AccountId) -> Option<RosterMember> {
        self.members()
            .into_iter()
            .find(|m| m.account_id == *account)
    }

    /// Checks the §8.2 validity rules for the roster of generation
    /// `generation` of a vault owned by `owner`.
    ///
    /// # Errors
    /// [`Error::RosterInvalid`] if a member key or value is malformed, a
    /// `since` is above `generation`, or the entries do not hold exactly one
    /// `owner`, equal to `owner`.
    pub fn validate(&self, owner: &AccountId, generation: KeyGeneration) -> Result<()> {
        let mut owners = 0usize;
        for (key, reg) in self.doc.registers() {
            let Some(account) = parse_member_key(key) else {
                return Err(Error::RosterInvalid);
            };
            let p = effective(reg).ok_or(Error::RosterInvalid)?;
            if p.since > generation {
                return Err(Error::RosterInvalid);
            }
            if p.role == Role::Owner {
                owners += 1;
                if account != *owner {
                    return Err(Error::RosterInvalid);
                }
            }
        }
        if owners == 1 {
            Ok(())
        } else {
            Err(Error::RosterInvalid)
        }
    }

    /// Merges two replicas (a pure join).
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            doc: self.doc.merge(&other.doc),
        }
    }

    /// Merges `received` into this local replica as
    /// [`super::VaultMeta::merge_and_repair`]; the roster only needs the
    /// device-ID-reuse checks, because member registers never surface
    /// conflicts.
    ///
    /// # Errors
    /// [`Error::DeviceIdReused`], [`Error::ClockOverflow`].
    pub fn merge_and_repair(&self, received: &Self, device: DeviceId) -> Result<(Self, bool)> {
        if received.doc.vv().get(&device) > self.doc.vv().get(&device) {
            return Err(Error::DeviceIdReused);
        }
        let (doc, repair) = self.doc.merge_and_repair(&received.doc, device, &[])?;
        if repair.own_stamp_collision {
            return Err(Error::DeviceIdReused);
        }
        Ok((Self { doc }, repair.stamp_collision))
    }

    /// The canonical plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_ROSTER_PLAINTEXT_LEN`],
    /// [`Error::Allocation`].
    pub fn to_json(&self) -> Result<SecretVec> {
        to_json(
            &DocJson {
                doc: &self.doc,
                format: Format::Document,
            },
            MAX_ROSTER_PLAINTEXT_LEN,
        )
    }

    /// Strictly decodes roster plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::UnsupportedVersion`],
    /// [`Error::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ROSTER_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Ok(Self {
            doc: decode(bytes, Format::Document)?,
        })
    }

    /// Encrypts the roster of `vault_id` under `vk`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::Allocation`], [`Error::Randomness`].
    pub fn seal(&self, vk: &VaultKey, vault_id: &VaultId) -> Result<VaultRosterCiphertext> {
        let pt = self.to_json()?;
        VaultRosterCiphertext::seal(vk, vault_id, pt.expose_secret())
    }

    /// Decrypts and decodes the roster of `vault_id`. The caller still runs
    /// [`VaultRoster::validate`].
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`], [`Error::Decryption`], and the
    /// errors of [`VaultRoster::from_json`].
    pub fn open(ct: &VaultRosterCiphertext, vk: &VaultKey, vault_id: &VaultId) -> Result<Self> {
        let pt = ct.open(vk, vault_id)?;
        Self::from_json(pt.expose_secret())
    }
}

/// An edit of a roster by one device.
pub struct RosterEdit<'a> {
    doc: &'a mut Doc,
    device: DeviceId,
    at: Timestamp,
    stamp: Option<Stamp>,
}

impl fmt::Debug for RosterEdit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RosterEdit")
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}

impl RosterEdit<'_> {
    fn stamp(&mut self) -> Result<Stamp> {
        if let Some(s) = self.stamp {
            return Ok(s);
        }
        let s = self.doc.tick(self.device)?;
        self.stamp = Some(s);
        Ok(s)
    }

    /// Writes `account`'s entry. It supersedes every value of the register
    /// this replica holds (the editor saw them all); values written
    /// concurrently elsewhere still compete under the merge rule.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_member(
        &mut self,
        account: &AccountId,
        role: Role,
        since: KeyGeneration,
        fingerprint: &Fingerprint,
        email: &str,
    ) -> Result<()> {
        let stamp = self.stamp()?;
        self.doc.write(
            &member_key(account),
            stamp,
            self.at,
            build_value(role, since, fingerprint, email),
            false,
        )
    }

    /// Marks `account` as removed at `since`, the generation of the rotation
    /// that locks them out (`g + 1`). The key stays, with the member's last
    /// fingerprint and email.
    ///
    /// # Errors
    /// [`Error::RosterInvalid`] if the account has no entry,
    /// [`Error::ClockOverflow`].
    pub fn remove_member(&mut self, account: &AccountId, since: KeyGeneration) -> Result<()> {
        let key = member_key(account);
        let current = self
            .doc
            .register(&key)
            .and_then(effective)
            .ok_or(Error::RosterInvalid)?;
        let stamp = self.stamp()?;
        self.doc.write(
            &key,
            stamp,
            self.at,
            build_value(Role::Removed, since, &current.fingerprint, &current.email),
            false,
        )
    }
}

#[cfg(test)]
#[expect(
    clippy::many_single_char_names,
    reason = "short replica names in merge tests"
)]
mod tests {
    use super::*;
    use crate::identity::IdentityPublicKey;
    use proptest::prelude::*;

    fn dev(b: u8) -> DeviceId {
        DeviceId::from_bytes([b; 16]).unwrap()
    }

    fn acct(b: u8) -> AccountId {
        AccountId::from_bytes([b; 16]).unwrap()
    }

    fn kg(n: u32) -> KeyGeneration {
        KeyGeneration::new(n).unwrap()
    }

    fn fpr(b: u8) -> Fingerprint {
        Fingerprint::of(&acct(b), &IdentityPublicKey([b; 32]))
    }

    fn at() -> Timestamp {
        Timestamp::from_unix_millis(1)
    }

    fn copy(r: &VaultRoster) -> VaultRoster {
        VaultRoster::from_json(r.to_json().unwrap().expose_secret()).unwrap()
    }

    /// Owner 1 and member 2 (role `Write`) at generation 1.
    fn base() -> VaultRoster {
        let mut r = VaultRoster::new();
        let mut e = r.edit(dev(1), at());
        e.set_member(&acct(1), Role::Owner, kg(1), &fpr(1), "o@x")
            .unwrap();
        e.set_member(&acct(2), Role::Write, kg(1), &fpr(2), "m@x")
            .unwrap();
        r
    }

    #[test]
    fn removed_beats_concurrent_role_change() {
        let b = base();
        let (mut a, mut c) = (copy(&b), copy(&b));
        // The manager removes member 2 with the rotation to generation 2 ...
        a.edit(dev(1), at()).remove_member(&acct(2), kg(2)).unwrap();
        // ... while another manager, still at generation 1, promotes them.
        c.edit(dev(3), at())
            .set_member(&acct(2), Role::Manage, kg(1), &fpr(2), "m@x")
            .unwrap();
        let ab = a.merge(&c);
        assert_eq!(ab, c.merge(&a));
        let m = ab.member(&acct(2)).unwrap();
        assert_eq!(m.role, Role::Removed);
        assert_eq!(m.since, kg(2));
        // The loser stays in the register but is not shown.
        assert_eq!(ab.members().len(), 2);
    }

    #[test]
    fn reinvite_after_removal_wins() {
        let b = base();
        let mut a = copy(&b);
        a.edit(dev(1), at()).remove_member(&acct(2), kg(2)).unwrap();
        assert_eq!(a.member(&acct(2)).unwrap().role, Role::Removed);
        // Re-invited at generation 3.
        let mut again = copy(&a);
        again
            .edit(dev(1), at())
            .set_member(&acct(2), Role::Read, kg(3), &fpr(2), "m@x")
            .unwrap();
        // A replica that only knows the removal merges to the re-invite.
        let merged = a.merge(&again);
        assert_eq!(merged, again.merge(&a));
        let m = merged.member(&acct(2)).unwrap();
        assert_eq!((m.role, m.since), (Role::Read, kg(3)));
        // Equal `since`: the most restrictive role wins.
        let (mut x, mut y) = (copy(&b), copy(&b));
        x.edit(dev(4), at())
            .set_member(&acct(2), Role::Manage, kg(2), &fpr(2), "m@x")
            .unwrap();
        y.edit(dev(5), at())
            .set_member(&acct(2), Role::Read, kg(2), &fpr(2), "m@x")
            .unwrap();
        assert_eq!(x.merge(&y).member(&acct(2)).unwrap().role, Role::Read);
        assert_eq!(y.merge(&x).member(&acct(2)).unwrap().role, Role::Read);
        // Removing an unknown account is refused.
        assert_eq!(
            a.edit(dev(1), at())
                .remove_member(&acct(9), kg(2))
                .unwrap_err(),
            Error::RosterInvalid
        );
    }

    #[test]
    fn single_owner_enforced() {
        let r = base();
        r.validate(&acct(1), kg(1)).unwrap();
        let mut two = copy(&r);
        two.edit(dev(1), at())
            .set_member(&acct(2), Role::Owner, kg(1), &fpr(2), "m@x")
            .unwrap();
        assert_eq!(
            two.validate(&acct(1), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
        assert_eq!(
            two.validate(&acct(2), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
        // No owner at all, and an empty roster.
        let mut none = copy(&r);
        none.edit(dev(1), at())
            .set_member(&acct(1), Role::Removed, kg(1), &fpr(1), "o@x")
            .unwrap();
        assert_eq!(
            none.validate(&acct(1), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
        assert_eq!(
            VaultRoster::new().validate(&acct(1), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
        // A `since` from the future.
        let mut future = copy(&r);
        future
            .edit(dev(1), at())
            .set_member(&acct(2), Role::Read, kg(5), &fpr(2), "m@x")
            .unwrap();
        assert_eq!(
            future.validate(&acct(1), kg(2)).unwrap_err(),
            Error::RosterInvalid
        );
        future.validate(&acct(1), kg(5)).unwrap();
    }

    #[test]
    fn owner_change_invalid() {
        // The server's owner is fixed at creation: a roster that names another
        // account as the owner is invalid, however it got there.
        let r = base();
        assert_eq!(
            r.validate(&acct(2), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
        let mut swapped = VaultRoster::new();
        let mut e = swapped.edit(dev(1), at());
        e.set_member(&acct(2), Role::Owner, kg(1), &fpr(2), "m@x")
            .unwrap();
        e.set_member(&acct(1), Role::Write, kg(1), &fpr(1), "o@x")
            .unwrap();
        assert_eq!(
            swapped.validate(&acct(1), kg(1)).unwrap_err(),
            Error::RosterInvalid
        );
    }

    #[test]
    fn round_trip_seal_open_and_roles() {
        let r = base();
        let vault = VaultId::from_bytes([0x0B; 16]).unwrap();
        let vk = VaultKey::generate(kg(1)).unwrap();
        let ct = r.seal(&vk, &vault).unwrap();
        assert_eq!(VaultRoster::open(&ct, &vk, &vault).unwrap(), r);
        let other = VaultId::from_bytes([0x0A; 16]).unwrap();
        assert_eq!(
            VaultRoster::open(&ct, &vk, &other).unwrap_err(),
            Error::Decryption
        );
        // A roster ciphertext is not vault metadata, and vice versa.
        let meta = crate::VaultMetaCiphertext::from_bytes(ct.as_bytes()).unwrap();
        assert_eq!(meta.open(&vk, &vault).unwrap_err(), Error::Decryption);
        for role in [
            Role::Owner,
            Role::Manage,
            Role::Write,
            Role::Read,
            Role::Removed,
        ] {
            assert_eq!(Role::parse(role.as_str()).unwrap(), role);
            if let Some(n) = role.number() {
                assert_eq!(Role::from_number(n).unwrap(), role);
            }
        }
        assert_eq!(Role::from_number(0).unwrap_err(), Error::Malformed);
        assert!(Role::Manage.can_manage() && !Role::Write.can_manage());
        assert!(format!("{:?}", r.members()[0]).starts_with("RosterMember"));
    }

    #[test]
    fn hex_helpers_are_strict() {
        assert_eq!(to_hex(&[0x0a, 0xff]), "0aff");
        assert_eq!(from_hex::<2>("0aff").unwrap(), [0x0a, 0xff]);
        assert!(from_hex::<2>("0AFF").is_err());
        assert!(from_hex::<2>("0af").is_err());
        assert!(from_hex::<2>("0afg").is_err());
    }

    /// One replica's edits: (account 1..=3, role 0..=4, since 1..=4).
    type Ops = Vec<(u8, u8, u32)>;

    fn apply(r: &mut VaultRoster, device: u8, ops: &Ops) {
        let roles = [
            Role::Owner,
            Role::Manage,
            Role::Write,
            Role::Read,
            Role::Removed,
        ];
        for (a, role, since) in ops {
            r.edit(dev(device), at())
                .set_member(
                    &acct(*a),
                    roles[usize::from(*role) % roles.len()],
                    kg(*since),
                    &fpr(*a),
                    "e@x",
                )
                .unwrap();
        }
    }

    fn replica(device: u8, ops: &Ops) -> VaultRoster {
        let mut r = VaultRoster::new();
        apply(&mut r, device, ops);
        r
    }

    fn ops() -> impl Strategy<Value = Ops> {
        proptest::collection::vec((1u8..=3, 0u8..=4, 1u32..=4), 0..6)
    }

    proptest! {
        #[test]
        fn merge_commutative_associative_idempotent(a in ops(), b in ops(), c in ops()) {
            let (ra, rb, rc) = (replica(1, &a), replica(2, &b), replica(3, &c));
            prop_assert_eq!(ra.merge(&rb), rb.merge(&ra));
            prop_assert_eq!(ra.merge(&rb).merge(&rc), ra.merge(&rb.merge(&rc)));
            prop_assert_eq!(ra.merge(&ra), copy(&ra));
            // The effective view is a function of the merged document.
            prop_assert_eq!(ra.merge(&rb).members(), rb.merge(&ra).members());
            prop_assert_eq!(ra.merge(&rb).merge(&rc).members(), rc.merge(&rb).merge(&ra).members());
        }
    }
}

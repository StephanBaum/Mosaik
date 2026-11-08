//! Sharing and emergency access in the session (M6b, CRYPTO_SPEC §8): the
//! account's identity key, contacts and pins, invites, the roster,
//! revocation rotations, the claim rule for key shares, and emergency seals.
//!
//! The network protocol is the sync layer's. Everything here is a pure
//! session operation in the two-phase style of the M6 rotation: `prepare_*`
//! changes nothing and returns a value that holds its keys in zeroizing
//! types, is not `Clone` and belongs to the unlocked session that made it
//! ([`Error::PreparedStale`] across a lock); the sync layer offers it to the
//! server and calls `commit_*` only when the server accepted.
//!
//! Who may do what (ADR-0050): a manager may invite, with the current key;
//! **only the vault owner rotates** (including on a revocation), and a member
//! accepts a rotation share only from the owner whose fingerprint is the
//! contact pin ([`mosaic_core::model::VaultRoster::authorizes_rotation`]).
//! No roster entry grants authority.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use mosaic_core::model::{
    AttachmentRef, Contact, ContactBook, ContactState, Role, RosterMember, Text, VaultRoster,
};
use mosaic_core::{
    AccountId, AttachmentId, AttachmentKey, ContactsCiphertext, Fingerprint, IdentityPublicKey,
    IdentitySecret, ItemId, KeyGeneration, ShareSeal, VaultId, VaultKey, VaultMetaCiphertext,
    VaultRosterCiphertext, WrappedKey, open_attachment, open_emergency_vault_key, open_vault_key,
    seal_attachment, seal_emergency_vault_key, seal_vault_key,
};

use super::{Error, Result, UnlockedSession, dup_item, now, seal_record, sync::check_generation};
use crate::storage::{SharedRotationWrite, Store, VaultKeyWrap};

/// How long an opened invite waits for the user's accept.
const PENDING_INVITE_TTL: Duration = Duration::from_secs(300);
/// The longest account email stored in a roster or the contact book.
const MAX_EMAIL_LEN: usize = 320;

/// The identity key pair of an unlocked session.
pub(super) struct Identity {
    pub(super) isk: IdentitySecret,
    pub(super) ipk: IdentityPublicKey,
}

/// A vault key opened from an invite, waiting for the user's accept.
pub(super) struct PendingInvite {
    vk: VaultKey,
    vault: VaultId,
    generation: KeyGeneration,
    sender: AccountId,
    sender_fpr: Fingerprint,
    role: Role,
    owner: AccountId,
    owner_fpr: Fingerprint,
    opened: Instant,
}

/// Reads and checks the stored identity key; `None` if there is none or it
/// is not readable (then [`UnlockedSession::ensure_identity_key`] refuses to
/// make another).
pub(super) fn load_identity(
    store: &Store,
    ak: &mosaic_core::AccountKey,
    account: &AccountId,
) -> Option<Identity> {
    let row = store.identity_key().ok().flatten()?;
    let isk = row.wrapped_isk.unwrap_identity(ak, account).ok()?;
    (isk.public_key().ok()? == row.ipk).then_some(Identity { isk, ipk: row.ipk })
}

/// An invite ID (a UUIDv7 the inviting device makes).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InviteId([u8; 16]);

impl InviteId {
    /// A new invite ID.
    ///
    /// # Errors
    /// [`Error::Core`] if the CSPRNG fails.
    pub fn new() -> Result<Self> {
        Ok(Self(*mosaic_core::DeviceId::new_v7()?.as_bytes()))
    }

    /// Wraps 16 raw bytes.
    ///
    /// # Errors
    /// [`Error::Core`] (`InvalidId`) for the nil ID.
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        Ok(Self(*mosaic_core::DeviceId::from_bytes(bytes)?.as_bytes()))
    }

    /// The 16 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Debug for InviteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InviteId(")?;
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

/// Why a member is to be locked out (stored as a code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationReason {
    /// An owner or manager removed the member.
    Removed,
    /// The member left.
    Left,
    /// An invite whose seal was delivered was declined.
    InviteDeclined,
    /// An invite whose seal was delivered was cancelled.
    InviteCancelled,
    /// The server listed a member the roster does not (after a user prompt).
    Unconfirmed,
}

impl RevocationReason {
    const fn code(self) -> u8 {
        match self {
            Self::Removed => 1,
            Self::Left => 2,
            Self::InviteDeclined => 3,
            Self::InviteCancelled => 4,
            Self::Unconfirmed => 5,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Removed),
            2 => Some(Self::Left),
            3 => Some(Self::InviteDeclined),
            4 => Some(Self::InviteCancelled),
            5 => Some(Self::Unconfirmed),
            _ => None,
        }
    }
}

/// A finding about sharing, kept until the user acknowledges it
/// ([`UnlockedSession::clear_sharing_findings`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharingFinding {
    /// The server serves this account another identity key than its own.
    IdentityMismatch,
    /// A vault's roster is invalid.
    RosterInvalid {
        /// The vault.
        vault: VaultId,
    },
    /// A key share failed the claim rule.
    ShareUntrusted {
        /// The vault.
        vault: VaultId,
        /// The generation of the refused share.
        generation: KeyGeneration,
        /// Who sent it.
        sender: AccountId,
    },
    /// A roster member this account has not verified: no key is sealed to
    /// them and a rotation locks them out (M6b review H2).
    MemberUnverified {
        /// The vault.
        vault: VaultId,
        /// The member.
        account: AccountId,
    },
    /// A roster member the server no longer lists.
    MemberDropped {
        /// The vault.
        vault: VaultId,
        /// The member.
        account: AccountId,
    },
}

/// An invite as the server lists it for this account.
#[derive(Clone, Debug)]
pub struct IncomingInvite {
    /// The invite.
    pub invite_id: InviteId,
    /// The vault it is for.
    pub vault_id: VaultId,
    /// Who sent it.
    pub sender: AccountId,
    /// The sender's email as the server shows it.
    pub sender_email: String,
    /// The role offered.
    pub role: Role,
    /// The vault key generation of the seal.
    pub generation: KeyGeneration,
    /// The vault key sealed to this account.
    pub seal: ShareSeal,
}

/// What [`UnlockedSession::open_invite`] found out.
pub struct InvitePreview {
    /// The invite.
    pub invite_id: InviteId,
    /// The vault.
    pub vault_id: VaultId,
    /// The vault's name (user content).
    pub vault_name: Option<String>,
    /// The role offered.
    pub role: Role,
    /// The vault key generation.
    pub generation: KeyGeneration,
    /// Who sent it.
    pub sender: AccountId,
    /// The sender's email.
    pub sender_email: String,
    /// The sender's safety code, for the user to compare.
    pub sender_code: String,
    /// The state of the sender's pin.
    pub sender_state: ContactState,
    /// The vault's owner as the roster names it.
    pub owner: AccountId,
    /// The owner's safety code, for the user to compare when the owner is
    /// not the sender (M6b review M4).
    pub owner_code: String,
    /// The state of the owner's pin: it must be verified before the invite
    /// is accepted.
    pub owner_state: ContactState,
}

impl fmt::Debug for InvitePreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InvitePreview")
            .field("invite_id", &self.invite_id)
            .field("vault_id", &self.vault_id)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

/// An invite prepared but not sent ([`UnlockedSession::prepare_invite`]):
/// nothing in the store has changed. Push the roster first, then store the
/// invite on the server, then [`UnlockedSession::commit_invite_sent`].
#[derive(Debug)]
pub struct PreparedInvite {
    session_tag: [u8; 16],
    invite_id: InviteId,
    vault_id: VaultId,
    recipient: AccountId,
    role: Role,
    generation: KeyGeneration,
    seal: ShareSeal,
    roster: VaultRosterCiphertext,
}

impl PreparedInvite {
    /// The invite ID.
    #[must_use]
    pub const fn invite_id(&self) -> InviteId {
        self.invite_id
    }

    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The invited account.
    #[must_use]
    pub const fn recipient(&self) -> AccountId {
        self.recipient
    }

    /// The role offered.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// The vault key generation the seal carries.
    #[must_use]
    pub const fn generation(&self) -> KeyGeneration {
        self.generation
    }

    /// The vault key sealed to the invited account.
    #[must_use]
    pub const fn seal(&self) -> &ShareSeal {
        &self.seal
    }

    /// The roster to push first (sealed under the current key).
    #[must_use]
    pub const fn roster(&self) -> &VaultRosterCiphertext {
        &self.roster
    }
}

/// An accepted invite or an opened emergency seal: the vault key wrapped
/// under the account key, ready to be stored
/// ([`UnlockedSession::commit_invite`]).
#[derive(Debug)]
pub struct AcceptedInvite {
    session_tag: [u8; 16],
    invite_id: Option<InviteId>,
    vault_id: VaultId,
    generation: KeyGeneration,
    wrapped: WrappedKey,
    owner: AccountId,
    role: Role,
}

impl AcceptedInvite {
    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The generation of the key.
    #[must_use]
    pub const fn generation(&self) -> KeyGeneration {
        self.generation
    }

    /// The key wrapped under the account key, as sent to the server.
    #[must_use]
    pub const fn wrapped(&self) -> &WrappedKey {
        &self.wrapped
    }

    /// The vault's owner as the roster (or the emergency grantor) names it.
    #[must_use]
    pub const fn owner(&self) -> AccountId {
        self.owner
    }

    /// The role the invite offered (`Read` for an emergency release).
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }
}

/// What the owner's device decided for a rotation.
#[derive(Default)]
pub struct RotationPlan {
    /// Every member that stays, with the identity key the server serves.
    pub remaining: Vec<(AccountId, IdentityPublicKey)>,
    /// The accounts to lock out.
    pub revoked: Vec<AccountId>,
    /// Pending invitees, with their identity keys (their seals are
    /// refreshed).
    pub invites: Vec<(InviteId, AccountId, IdentityPublicKey)>,
    /// Emergency grantees whose grants cover the vault.
    pub grants: Vec<(AccountId, IdentityPublicKey)>,
}

impl fmt::Debug for RotationPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RotationPlan")
            .field("remaining", &self.remaining.len())
            .field("revoked", &self.revoked.len())
            .field("invites", &self.invites.len())
            .field("grants", &self.grants.len())
            .finish()
    }
}

/// A shared-vault rotation prepared but not applied
/// ([`UnlockedSession::prepare_shared_rotation`]). The key is zeroized on
/// drop; it cannot be cloned and belongs to the unlocked session that made it.
#[derive(Debug)]
pub struct PreparedSharedRotation {
    session_tag: [u8; 16],
    vault_id: VaultId,
    base: KeyGeneration,
    generation: KeyGeneration,
    key: VaultKey,
    wrapped: WrappedKey,
    roster: VaultRosterCiphertext,
    seals: Vec<(AccountId, ShareSeal)>,
    invite_seals: Vec<(InviteId, ShareSeal)>,
    emergency_seals: Vec<(AccountId, ShareSeal)>,
    revoke: Vec<AccountId>,
    unreachable: Vec<(ItemId, AttachmentId)>,
}

impl PreparedSharedRotation {
    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The new generation.
    #[must_use]
    pub const fn generation(&self) -> KeyGeneration {
        self.generation
    }

    /// The generation the rotation starts from (the server's `expected`
    /// generation).
    #[must_use]
    pub const fn base_generation(&self) -> KeyGeneration {
        self.base
    }

    /// The new key wrapped under the account key.
    #[must_use]
    pub const fn wrapped(&self) -> &WrappedKey {
        &self.wrapped
    }

    /// The roster at the new generation.
    #[must_use]
    pub const fn roster(&self) -> &VaultRosterCiphertext {
        &self.roster
    }

    /// One share per remaining member.
    #[must_use]
    pub fn seals(&self) -> &[(AccountId, ShareSeal)] {
        &self.seals
    }

    /// One refreshed seal per pending invite.
    #[must_use]
    pub fn invite_seals(&self) -> &[(InviteId, ShareSeal)] {
        &self.invite_seals
    }

    /// One emergency seal per covering grant.
    #[must_use]
    pub fn emergency_seals(&self) -> &[(AccountId, ShareSeal)] {
        &self.emergency_seals
    }

    /// The accounts the rotation locks out.
    #[must_use]
    pub fn revoke(&self) -> &[AccountId] {
        &self.revoke
    }

    /// The attachments this device does not hold: they keep their old ID and
    /// key, so the removed member can still read them.
    #[must_use]
    pub fn unreachable_attachments(&self) -> &[(ItemId, AttachmentId)] {
        &self.unreachable
    }
}

/// What [`UnlockedSession::commit_shared_rotation`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedRotationOutcome {
    /// The new generation.
    pub generation: KeyGeneration,
    /// The attachments that could not be re-sealed (not held here).
    pub unreachable_attachments: Vec<(ItemId, AttachmentId)>,
}

/// What a rotation re-sealed besides the records: attachment files already
/// staged in the store (new ID, owning record, ciphertext length), the old IDs
/// they replace and the items with their new references.
#[derive(Default)]
struct Resealed {
    staged: Vec<(AttachmentId, ItemId, u64)>,
    old: Vec<AttachmentId>,
    changed: BTreeMap<ItemId, mosaic_core::Item>,
}

/// A key share from a pull page.
#[derive(Clone, Debug)]
pub struct PulledShareView {
    /// The generation of the key it carries.
    pub generation: KeyGeneration,
    /// Who sealed it.
    pub sender: AccountId,
    /// The seal.
    pub seal: ShareSeal,
}

/// A chain of key shares that passed the claim rule
/// ([`UnlockedSession::open_share_chain`]). Holds the final key; zeroizing,
/// not `Clone`, bound to the unlocked session.
#[derive(Debug)]
pub struct OpenedChain {
    session_tag: [u8; 16],
    vault_id: VaultId,
    generation: KeyGeneration,
    key: VaultKey,
    roster: VaultRosterCiphertext,
}

impl OpenedChain {
    /// The vault.
    #[must_use]
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The generation of the key to claim.
    #[must_use]
    pub const fn generation(&self) -> KeyGeneration {
        self.generation
    }
}

fn check_email(email: &str) -> Result<()> {
    if email.len() > MAX_EMAIL_LEN {
        return Err(Error::Core(mosaic_core::Error::Malformed));
    }
    Ok(())
}

fn role_number(role: Role) -> Result<u8> {
    role.number().ok_or(Error::RoleForbidden)
}

/// Whether `role` may be handed out by someone who is `own`.
fn may_grant(own: Role, role: Role) -> bool {
    match (own.number(), role.number()) {
        (Some(o), Some(r)) => r >= o && role != Role::Owner,
        _ => false,
    }
}

/// A roster's members that count (not removed).
fn active(members: Vec<RosterMember>) -> Vec<RosterMember> {
    members
        .into_iter()
        .filter(|m| m.role != Role::Removed)
        .collect()
}

impl UnlockedSession {
    // ---- identity key ------------------------------------------------------

    fn own_identity(&self) -> Result<&Identity> {
        self.identity_key.as_ref().ok_or(Error::NoIdentity)
    }

    fn own_email(&self) -> Result<String> {
        Ok(self
            .store
            .sync_config()?
            .and_then(|c| c.email)
            .unwrap_or_default())
    }

    /// Makes this account's identity key pair if there is none (CRYPTO_SPEC
    /// §8.1): `isk` is wrapped under the account key and stored, unpublished.
    /// Returns `ipk`. The sync layer calls it at the first hosted run.
    ///
    /// # Errors
    /// [`Error::IdentityMismatch`] if a stored key cannot be read (another
    /// is never made over it); [`Error::Core`], [`Error::Store`].
    pub fn ensure_identity_key(&mut self) -> Result<IdentityPublicKey> {
        if let Some(id) = &self.identity_key {
            return Ok(id.ipk);
        }
        if self.store.identity_key()?.is_some() {
            return Err(Error::IdentityMismatch);
        }
        let (isk, ipk) = IdentitySecret::generate()?;
        let wrapped = isk.wrap(&self.account_key, &self.account_id)?;
        self.store.insert_identity_key(&wrapped, &ipk, false)?;
        self.identity_key = Some(Identity { isk, ipk });
        Ok(ipk)
    }

    /// The stored identity key: `ipk`, the wrapped `isk` and whether the
    /// server holds it.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn identity(&self) -> Result<Option<(IdentityPublicKey, WrappedKey, bool)>> {
        Ok(self
            .store
            .identity_key()?
            .map(|r| (r.ipk, r.wrapped_isk, r.published)))
    }

    /// Notes that the server holds the identity key.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_identity_published(&mut self) -> Result<()> {
        Ok(self.store.mark_identity_published()?)
    }

    /// Adopts the identity key the server holds for this account (another
    /// device published first). Only a key this device never published can
    /// be replaced, and only if `wrapped` opens under the account key to an
    /// `isk` whose public key is `ipk`; the key this device made was never
    /// used for any seal, so nothing is lost (CRYPTO_SPEC §8.1). Without a
    /// local key it is stored as published.
    ///
    /// # Errors
    /// [`Error::IdentityMismatch`] for a published key that differs, a
    /// wrapping that does not open, or a key that does not match `ipk`.
    pub fn adopt_remote_identity(
        &mut self,
        wrapped: &WrappedKey,
        ipk: &IdentityPublicKey,
    ) -> Result<()> {
        let stored = self.store.identity_key()?;
        if let Some(s) = &stored
            && s.published
        {
            return if s.ipk == *ipk {
                Ok(())
            } else {
                Err(Error::IdentityMismatch)
            };
        }
        let isk = wrapped
            .unwrap_identity(&self.account_key, &self.account_id)
            .map_err(|_| Error::IdentityMismatch)?;
        if isk.public_key()? != *ipk {
            return Err(Error::IdentityMismatch);
        }
        if stored.is_some() {
            self.store.adopt_identity_key(wrapped, ipk)?;
        } else {
            self.store.insert_identity_key(wrapped, ipk, true)?;
        }
        self.identity_key = Some(Identity { isk, ipk: *ipk });
        Ok(())
    }

    /// The fingerprint of this account's identity key.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn own_fingerprint(&self) -> Result<Option<Fingerprint>> {
        Ok(self
            .store
            .identity_key()?
            .map(|r| Fingerprint::of(&self.account_id, &r.ipk)))
    }

    // ---- findings ----------------------------------------------------------

    /// Records a finding about sharing.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn record_sharing_finding(&mut self, finding: SharingFinding) -> Result<()> {
        let (kind, vault, generation, account) = match &finding {
            SharingFinding::IdentityMismatch => (1, None, None, None),
            SharingFinding::RosterInvalid { vault } => (2, Some(*vault), None, None),
            SharingFinding::ShareUntrusted {
                vault,
                generation,
                sender,
            } => (3, Some(*vault), Some(generation.get()), Some(*sender)),
            SharingFinding::MemberDropped { vault, account } => {
                (4, Some(*vault), None, Some(*account))
            }
            SharingFinding::MemberUnverified { vault, account } => {
                (5, Some(*vault), None, Some(*account))
            }
        };
        Ok(self
            .store
            .add_sharing_finding(kind, vault.as_ref(), generation, account.as_ref())?)
    }

    /// Every sharing finding, oldest first.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn sharing_findings(&self) -> Result<Vec<SharingFinding>> {
        let mut out = Vec::new();
        for row in self.store.sharing_findings()? {
            let finding = match (row.kind, row.vault_id, row.generation, row.account_id) {
                (1, ..) => SharingFinding::IdentityMismatch,
                (2, Some(vault), ..) => SharingFinding::RosterInvalid { vault },
                (3, Some(vault), Some(g), Some(sender)) => SharingFinding::ShareUntrusted {
                    vault,
                    generation: KeyGeneration::new(g)?,
                    sender,
                },
                (4, Some(vault), _, Some(account)) => {
                    SharingFinding::MemberDropped { vault, account }
                }
                (5, Some(vault), _, Some(account)) => {
                    SharingFinding::MemberUnverified { vault, account }
                }
                _ => return Err(Error::Store(crate::storage::Error::Corrupt)),
            };
            out.push(finding);
        }
        Ok(out)
    }

    /// Clears every sharing finding (the user acknowledged them).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn clear_sharing_findings(&mut self) -> Result<()> {
        Ok(self.store.clear_sharing_findings()?)
    }

    // ---- contacts ----------------------------------------------------------

    fn contact_book(&self) -> Result<ContactBook> {
        match self.store.contacts_doc()? {
            None => Ok(ContactBook::new()),
            Some(doc) => Ok(ContactBook::open(
                &doc.ciphertext,
                &self.account_key.contacts_key()?,
                &self.account_id,
            )?),
        }
    }

    fn save_contact_book(&mut self, book: &ContactBook) -> Result<()> {
        let ct = book.seal(&self.account_key.contacts_key()?, &self.account_id)?;
        let seq = self.store.contacts_doc()?.and_then(|d| d.seq);
        self.store.put_contacts_doc(&ct, seq, true)?;
        self.contacts_handed = None;
        Ok(())
    }

    /// What the contact book says about `account` when the server serves
    /// `ipk` for it.
    ///
    /// # Errors
    /// [`Error::Core`] if the book cannot be opened, [`Error::Store`].
    pub fn contact_state(
        &self,
        account: &AccountId,
        ipk: &IdentityPublicKey,
    ) -> Result<ContactState> {
        Ok(self
            .contact_book()?
            .check(account, &Fingerprint::of(account, ipk)))
    }

    /// Pins the served key of `account` (unverified) if there is no pin yet
    /// (trust on first use). A pin that exists is never overwritten; the
    /// state after the call is returned.
    ///
    /// # Errors
    /// [`Error::Core`] (`Malformed` for an email over 320 bytes),
    /// [`Error::Store`].
    pub fn pin_contact(
        &mut self,
        account: &AccountId,
        ipk: &IdentityPublicKey,
        email: &str,
    ) -> Result<ContactState> {
        check_email(email)?;
        let fpr = Fingerprint::of(account, ipk);
        let mut book = self.contact_book()?;
        let before = book.check(account, &fpr);
        let state = book.pin(self.device, now(), account, &fpr, email)?;
        if before == ContactState::Unknown {
            self.save_contact_book(&book)?;
        }
        Ok(state)
    }

    /// Records that the user compared safety codes for `expected`.
    ///
    /// # Errors
    /// [`Error::Core`] with `ContactChanged` if the pin is not `expected`
    /// (it moved, or devices disagree) and `Malformed` if there is no pin;
    /// [`Error::Store`].
    pub fn verify_contact(&mut self, account: &AccountId, expected: &Fingerprint) -> Result<()> {
        let mut book = self.contact_book()?;
        book.verify(self.device, now(), account, expected)?;
        self.save_contact_book(&book)
    }

    /// The user saw the key-change warning and accepts the new key (or
    /// settles a conflict): the pin is replaced, unverified.
    ///
    /// # Errors
    /// [`Error::Core`], [`Error::Store`].
    pub fn accept_key_change(
        &mut self,
        account: &AccountId,
        ipk: &IdentityPublicKey,
        email: &str,
    ) -> Result<()> {
        check_email(email)?;
        let mut book = self.contact_book()?;
        book.replace_pin(
            self.device,
            now(),
            account,
            &Fingerprint::of(account, ipk),
            email,
        )?;
        self.save_contact_book(&book)
    }

    /// Every contact.
    ///
    /// # Errors
    /// [`Error::Core`], [`Error::Store`].
    pub fn contacts(&self) -> Result<Vec<Contact>> {
        Ok(self.contact_book()?.contacts())
    }

    /// Applies the contact book the server holds: merged with the local one;
    /// the merge is queued for upload if the server's copy lacks something.
    ///
    /// # Errors
    /// [`Error::Core`] if it does not authenticate (or `DeviceIdReused`),
    /// [`Error::Store`].
    pub fn apply_remote_contacts(&mut self, ct: &ContactsCiphertext, seq: u64) -> Result<()> {
        let key = self.account_key.contacts_key()?;
        let received = ContactBook::open(ct, &key, &self.account_id)?;
        let Some(local) = self.store.contacts_doc()? else {
            return Ok(self.store.put_contacts_doc(ct, Some(seq), false)?);
        };
        let local_book = ContactBook::open(&local.ciphertext, &key, &self.account_id)?;
        let (merged, _) = local_book.merge_and_repair(&received, self.device)?;
        if merged == received {
            self.store.put_contacts_doc(ct, Some(seq), false)?;
        } else {
            let sealed = merged.seal(&key, &self.account_id)?;
            self.store.put_contacts_doc(&sealed, Some(seq), true)?;
        }
        self.contacts_handed = None;
        Ok(())
    }

    /// The contact book to upload with the server's sequence number it is
    /// based on, if an upload is pending.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_contacts_upload(&mut self) -> Result<Option<(ContactsCiphertext, Option<u64>)>> {
        let Some(doc) = self.store.contacts_doc()? else {
            return Ok(None);
        };
        if !doc.pending {
            return Ok(None);
        }
        self.contacts_handed = Some(doc.ciphertext.as_bytes().to_vec());
        Ok(Some((doc.ciphertext, doc.seq)))
    }

    /// Records that the server stored the contact book handed out by
    /// [`Self::pending_contacts_upload`] as `seq`. The upload flag clears
    /// only if the stored book is still exactly that one.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_contacts_uploaded(&mut self, seq: u64) -> Result<()> {
        let Some(doc) = self.store.contacts_doc()? else {
            return Ok(());
        };
        let same = self
            .contacts_handed
            .as_deref()
            .is_some_and(|h| h == doc.ciphertext.as_bytes());
        self.store.settle_contacts_doc(seq, doc.pending && !same)?;
        if same {
            self.contacts_handed = None;
        }
        Ok(())
    }

    /// The fingerprint a contact must have for a seal: the pin must be there,
    /// not conflicting, equal to the served key's fingerprint, and (if
    /// `need_verified`) verified.
    fn check_contact(
        book: &ContactBook,
        account: &AccountId,
        ipk: &IdentityPublicKey,
        need_verified: bool,
    ) -> Result<Fingerprint> {
        let fpr = Fingerprint::of(account, ipk);
        match book.check(account, &fpr) {
            ContactState::Pinned { verified } if verified || !need_verified => Ok(fpr),
            ContactState::Pinned { .. } | ContactState::Unknown => Err(Error::ContactUnverified),
            ContactState::Changed | ContactState::Conflicting => Err(Error::ContactKeyChanged),
        }
    }

    // ---- shared flag, roles, roster ---------------------------------------

    /// Whether the vault is shared. The flag is monotone (CRYPTO_SPEC §10.2).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_shared(&self, vault: VaultId) -> Result<bool> {
        Ok(self.store.vault_sharing(&vault)?.is_some())
    }

    /// This account's role in the vault: `Owner` for a vault that is not
    /// shared.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Store`].
    pub fn vault_own_role(&self, vault: VaultId) -> Result<Role> {
        self.vault(vault)?;
        match self.store.vault_sharing(&vault)? {
            None => Ok(Role::Owner),
            Some(s) => Ok(Role::from_number(s.own_role)?),
        }
    }

    /// The vault's owner as this device knows it (this account for a vault
    /// that is not shared). The sync layer checks it against the owner the
    /// server names.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_owner(&self, vault: VaultId) -> Result<AccountId> {
        Ok(self
            .store
            .vault_sharing(&vault)?
            .map_or(self.account_id, |s| s.owner_account))
    }

    /// Whether the server refused this device the vault.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn vault_access_lost(&self, vault: VaultId) -> Result<bool> {
        Ok(self
            .store
            .vault_sharing(&vault)?
            .is_some_and(|s| s.access_lost_at_ms.is_some()))
    }

    /// The server's sequence number of the stored roster (the base of the
    /// next roster upload).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn roster_seq(&self, vault: VaultId) -> Result<Option<u64>> {
        Ok(self.store.vault_roster(&vault)?.and_then(|r| r.seq))
    }

    fn load_roster(&self, vault: VaultId) -> Result<Option<VaultRoster>> {
        let Some(stored) = self.store.vault_roster(&vault)? else {
            return Ok(None);
        };
        let vk = self
            .vault(vault)?
            .keys
            .get(&stored.generation)
            .ok_or(Error::VaultUnavailable)?;
        let owner = self.vault_owner(vault)?;
        Ok(Some(VaultRoster::open(
            &stored.ciphertext,
            vk,
            &vault,
            owner,
        )?))
    }

    /// The members of the vault's roster (every role, removed ones too), or
    /// `None` if the vault has none.
    ///
    /// # Errors
    /// [`Error::Core`] if the stored roster cannot be opened,
    /// [`Error::Store`].
    pub fn vault_roster(&self, vault: VaultId) -> Result<Option<Vec<RosterMember>>> {
        Ok(self.load_roster(vault)?.map(|r| r.members()))
    }

    /// Applies the roster the server holds for the vault's current
    /// generation. `owner` is the vault owner the server names; it must be
    /// the owner this device knows (the first roster fixes it). A roster of
    /// the same generation is merged with the local one (and queued for
    /// upload if the server's copy lacks something); an older local roster
    /// is replaced. Updates this account's role from the roster entry.
    ///
    /// # Errors
    /// [`Error::KeyGenerationRollback`] / [`Error::KeyGenerationNotAccepted`]
    /// for another generation than the vault's; [`Error::OwnerMismatch`];
    /// [`Error::Core`] with `RosterInvalid` (a `RosterInvalid` finding is
    /// recorded), `Decryption`, `DeviceIdReused`; [`Error::NotInRoster`];
    /// [`Error::Store`].
    pub fn apply_remote_roster(
        &mut self,
        vault: VaultId,
        g: KeyGeneration,
        ct: &VaultRosterCiphertext,
        seq: u64,
        owner: &AccountId,
    ) -> Result<()> {
        let ov = self.vault(vault)?;
        let vk = ov.current_key()?;
        check_generation(ov.generation, g)?;
        if ct.key_generation() != g {
            return Err(Error::Core(mosaic_core::Error::KeyGenerationMismatch));
        }
        let row = self.store.vault_sharing(&vault)?;
        if let Some(row) = &row
            && row.owner_account != *owner
        {
            return Err(Error::OwnerMismatch);
        }
        let received = VaultRoster::open(ct, vk, &vault, *owner)?;
        if let Err(e) = received.validate() {
            self.record_sharing_finding(SharingFinding::RosterInvalid { vault })?;
            return Err(e.into());
        }
        // No sharing row: this device has never seen the vault shared. The
        // first roster fixes the owner, so it may name another account than
        // this one only if that account is a contact this account verified,
        // with the fingerprint the roster holds (H3): a vault that was never
        // shared must not be handed to whoever holds its key and forges a
        // roster.
        if row.is_none() && *owner != self.account_id {
            let pinned = received.member(owner).is_some_and(|m| {
                self.contact_book().is_ok_and(|b| {
                    b.check(owner, &m.fingerprint) == ContactState::Pinned { verified: true }
                })
            });
            if !pinned {
                return Err(Error::OwnerMismatch);
            }
        }
        // An older roster is a read-only input: only the one of this
        // generation merges.
        let local = match self.load_roster(vault) {
            Ok(Some(r)) if r.generation() == g => Some(r),
            _ => None,
        };
        let (final_roster, store_ct, pending) = match local {
            None => (received, ct.clone(), false),
            Some(local) => {
                let (merged, _) = local.merge_and_repair(&received, self.device)?;
                if merged == received {
                    (received, ct.clone(), false)
                } else {
                    let sealed = merged.seal(self.vault(vault)?.current_key()?, &vault)?;
                    (merged, sealed, true)
                }
            }
        };
        let own = final_roster
            .member(&self.account_id)
            .ok_or(Error::NotInRoster)?;
        self.store
            .put_vault_roster(&vault, &store_ct, Some(seq), pending)?;
        let number = own.role.number().or_else(|| row.map(|r| r.own_role));
        self.store
            .set_vault_sharing(&vault, number.unwrap_or(4), owner, false)?;
        if own.role == Role::Removed {
            self.store.mark_access_lost(&vault)?;
        }
        Ok(())
    }

    /// The roster to upload (a role change, or a merge the server lacks) with
    /// the server's sequence number it is based on.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_roster_upload(
        &self,
        vault: VaultId,
    ) -> Result<Option<(VaultRosterCiphertext, Option<u64>)>> {
        Ok(self
            .store
            .vault_roster(&vault)?
            .filter(|r| r.pending)
            .map(|r| (r.ciphertext, r.seq)))
    }

    /// Records that the server stored `uploaded` as the roster at `seq`; the
    /// upload flag clears only if the stored roster is still exactly that.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_roster_uploaded(
        &mut self,
        vault: VaultId,
        uploaded: &VaultRosterCiphertext,
        seq: u64,
    ) -> Result<()> {
        let Some(stored) = self.store.vault_roster(&vault)? else {
            return Ok(());
        };
        let same = stored.ciphertext.as_bytes() == uploaded.as_bytes();
        Ok(self
            .store
            .settle_vault_roster(&vault, seq, stored.pending && !same)?)
    }

    /// Changes a member's role in the roster (queued for upload; the server
    /// enforces the role on its side). The owner and a manager may do it
    /// for members at or below their own role, never to `Owner` or
    /// `Removed` (removal is a rotation).
    ///
    /// # Errors
    /// [`Error::RoleForbidden`], [`Error::NoRoster`], [`Error::NotInRoster`],
    /// [`Error::Core`], [`Error::Store`].
    pub fn set_member_role(
        &mut self,
        vault: VaultId,
        member: &AccountId,
        role: Role,
    ) -> Result<()> {
        let own = self.vault_own_role(vault)?;
        if !own.can_manage() || !may_grant(own, role) || role == Role::Removed {
            return Err(Error::RoleForbidden);
        }
        let mut roster = self.load_roster(vault)?.ok_or(Error::NoRoster)?;
        let entry = roster.member(member).ok_or(Error::NotInRoster)?;
        if entry.role == Role::Removed
            || entry.role == Role::Owner
            || !may_grant(own, entry.role)
            || *member == self.vault_owner(vault)?
        {
            return Err(Error::RoleForbidden);
        }
        let ov = self.vault(vault)?;
        let g = ov.generation;
        let vk = ov.current_key()?;
        if roster.generation() < g {
            roster = roster.advance(g)?;
        }
        roster.edit(self.device, now()).set_member(
            member,
            role,
            g,
            &entry.fingerprint,
            &entry.email,
        )?;
        let ct = roster.seal(vk, &vault)?;
        let seq = self.store.vault_roster(&vault)?.and_then(|r| r.seq);
        self.store.put_vault_roster(&vault, &ct, seq, true)?;
        Ok(())
    }

    /// Gives a vault this account owns the owner-only roster and the sharing
    /// row (M6b review H3): an emergency release makes the grantee a member
    /// without any invite, and the revocation rotation and the claim rule
    /// run on the shared path. Called when a grant covers the vault. The
    /// roster is queued for upload. Keeps a roster that exists.
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::WrongVaultProtection`],
    /// [`Error::NotOwner`], [`Error::VaultUnavailable`], [`Error::Core`],
    /// [`Error::Store`].
    pub fn ensure_owner_roster(&mut self, vault: VaultId) -> Result<()> {
        let own_fpr = {
            let id = self.own_identity()?;
            Fingerprint::of(&self.account_id, &id.ipk)
        };
        let ov = self.vault(vault)?;
        if ov.protection != VaultKeyWrap::AccountKey {
            return Err(Error::WrongVaultProtection);
        }
        if self.vault_own_role(vault)? != Role::Owner || self.vault_owner(vault)? != self.account_id
        {
            return Err(Error::NotOwner);
        }
        if self.load_roster(vault)?.is_none() {
            let g = ov.generation;
            let vk = ov.current_key()?;
            let mut roster = VaultRoster::new(g, self.account_id);
            roster.edit(self.device, now()).set_member(
                &self.account_id,
                Role::Owner,
                KeyGeneration::FIRST,
                &own_fpr,
                &self.own_email()?,
            )?;
            let ct = roster.seal(vk, &vault)?;
            self.store.put_vault_roster(&vault, &ct, None, true)?;
        }
        let account = self.account_id;
        self.store.set_vault_sharing(&vault, 1, &account, false)?;
        Ok(())
    }

    /// Records that the server refuses this device the vault (removed, or it
    /// left): sync stops for it and the local copy stays readable.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn mark_access_lost(&mut self, vault: VaultId) -> Result<()> {
        self.store.mark_access_lost(&vault)?;
        Ok(())
    }

    // ---- invites -----------------------------------------------------------

    /// Prepares an invite of `recipient` to the vault (CRYPTO_SPEC §8.2):
    /// the roster with the recipient's entry (created, with the owner's
    /// entry, if the vault has none) and the current vault key sealed to
    /// them. Nothing is stored. The recipient must be a verified contact
    /// whose pinned fingerprint is the served key's.
    ///
    /// An owner or manager may invite, at a role no higher than their own.
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::WrongVaultProtection`] (a hardware
    /// vault cannot be shared), [`Error::VaultUnavailable`],
    /// [`Error::RoleForbidden`], [`Error::ContactUnverified`],
    /// [`Error::ContactKeyChanged`], [`Error::AlreadyMember`],
    /// [`Error::ReinviteBlocked`], [`Error::NoRoster`],
    /// [`Error::Core`], [`Error::Store`].
    pub fn prepare_invite(
        &self,
        vault: VaultId,
        recipient: &AccountId,
        ipk: &IdentityPublicKey,
        email: &str,
        role: Role,
    ) -> Result<PreparedInvite> {
        check_email(email)?;
        let id = self.own_identity()?;
        let ov = self.vault(vault)?;
        if ov.protection != VaultKeyWrap::AccountKey {
            return Err(Error::WrongVaultProtection);
        }
        let vk = ov.current_key()?;
        let g = ov.generation;
        let own = self.vault_own_role(vault)?;
        if !own.can_manage() || !may_grant(own, role) || *recipient == self.account_id {
            return Err(Error::RoleForbidden);
        }
        let book = self.contact_book()?;
        let fpr_b = Self::check_contact(&book, recipient, ipk, true)?;
        let own_fpr = Fingerprint::of(&self.account_id, &id.ipk);
        let mut roster = match self.load_roster(vault)? {
            Some(r) if r.generation() < g => r.advance(g)?,
            Some(r) => r,
            None if own == Role::Owner => {
                let mut r = VaultRoster::new(g, self.account_id);
                r.edit(self.device, now()).set_member(
                    &self.account_id,
                    Role::Owner,
                    KeyGeneration::FIRST,
                    &own_fpr,
                    &self.own_email()?,
                )?;
                r
            }
            None => return Err(Error::NoRoster),
        };
        if let Some(existing) = roster.member(recipient) {
            if existing.role != Role::Removed {
                return Err(Error::AlreadyMember);
            }
            if existing.since >= g {
                return Err(Error::ReinviteBlocked);
            }
        }
        roster
            .edit(self.device, now())
            .set_member(recipient, role, g, &fpr_b, email)?;
        let roster_ct = roster.seal(vk, &vault)?;
        let seal = seal_vault_key(vk, &vault, &self.account_id, &id.isk, recipient, ipk)?;
        Ok(PreparedInvite {
            session_tag: self.session_tag,
            invite_id: InviteId::new()?,
            vault_id: vault,
            recipient: *recipient,
            role,
            generation: g,
            seal,
            roster: roster_ct,
        })
    }

    /// Records a sent invite: the roster the server accepted at
    /// `roster_seq` is stored and the vault is marked shared (monotone).
    ///
    /// # Errors
    /// [`Error::PreparedStale`] after a lock or if the vault rotated since;
    /// [`Error::Store`].
    pub fn commit_invite_sent(&mut self, p: PreparedInvite, roster_seq: u64) -> Result<()> {
        let PreparedInvite {
            session_tag,
            vault_id,
            generation,
            roster,
            ..
        } = p;
        if session_tag != self.session_tag || self.vault(vault_id)?.generation != generation {
            return Err(Error::PreparedStale);
        }
        let row = self.store.vault_sharing(&vault_id)?;
        let (role, owner) = row.map_or((1, self.account_id), |r| (r.own_role, r.owner_account));
        self.store
            .put_vault_roster(&vault_id, &roster, Some(roster_seq), false)?;
        self.store
            .set_vault_sharing(&vault_id, role, &owner, false)?;
        Ok(())
    }

    /// Previews an invite (CRYPTO_SPEC §8.2): checks the sender's pin
    /// (pinning an unknown sender, unverified), opens the seal, then the
    /// vault metadata and the roster with the key, which must list this
    /// account with the invited role and fingerprint, and the sender as
    /// `owner` or `manage`. The opened key waits in a pending slot of this
    /// session for five minutes. The vault's owner is the one the roster
    /// names; an owner other than the sender is pinned (unverified) if
    /// unknown, because later key changes are accepted only from the owner.
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::ContactKeyChanged`],
    /// [`Error::InviteUnreadable`], [`Error::InviteNotInRoster`],
    /// [`Error::Core`], [`Error::Store`].
    #[expect(
        clippy::too_many_lines,
        reason = "the checks of one preview belong together, in the order of CRYPTO_SPEC 8.2"
    )]
    pub fn open_invite(
        &mut self,
        inv: &IncomingInvite,
        sender_ipk: &IdentityPublicKey,
        meta: &VaultMetaCiphertext,
        roster: &VaultRosterCiphertext,
    ) -> Result<InvitePreview> {
        check_email(&inv.sender_email)?;
        let (isk, own_ipk) = {
            let id = self.own_identity()?;
            (&id.isk, id.ipk)
        };
        let sender_fpr = Fingerprint::of(&inv.sender, sender_ipk);
        let mut book = self.contact_book()?;
        let mut book_changed = false;
        match book.check(&inv.sender, &sender_fpr) {
            ContactState::Changed | ContactState::Conflicting => {
                return Err(Error::ContactKeyChanged);
            }
            ContactState::Unknown => {
                book.pin(
                    self.device,
                    now(),
                    &inv.sender,
                    &sender_fpr,
                    &inv.sender_email,
                )?;
                book_changed = true;
            }
            ContactState::Pinned { .. } => {}
        }
        let vk = open_vault_key(
            &inv.seal,
            &inv.vault_id,
            inv.generation,
            &inv.sender,
            sender_ipk,
            &self.account_id,
            isk,
        )
        .map_err(|_| Error::InviteUnreadable)?;
        let meta = mosaic_core::model::VaultMeta::open(meta, &vk, &inv.vault_id)
            .map_err(|_| Error::InviteUnreadable)?;
        let (roster, owner) = open_roster_unowned(roster, &vk, &inv.vault_id)?;
        roster.validate().map_err(|_| Error::InviteNotInRoster)?;
        // A vault this device knows gets no key from an invite (H1).
        self.check_vault_unknown(&inv.vault_id, &owner)?;
        let own_fpr = Fingerprint::of(&self.account_id, &own_ipk);
        let me = roster
            .member(&self.account_id)
            .ok_or(Error::InviteNotInRoster)?;
        if me.role != inv.role || me.since > inv.generation || me.fingerprint != own_fpr {
            return Err(Error::InviteNotInRoster);
        }
        let from = roster.member(&inv.sender).ok_or(Error::InviteNotInRoster)?;
        if !from.role.can_manage() || from.fingerprint != sender_fpr {
            return Err(Error::InviteNotInRoster);
        }
        let owner_fpr = if owner == inv.sender {
            sender_fpr
        } else {
            roster
                .member(&owner)
                .ok_or(Error::InviteNotInRoster)?
                .fingerprint
        };
        if owner != inv.sender {
            let owner_entry = roster.member(&owner).ok_or(Error::InviteNotInRoster)?;
            match book.check(&owner, &owner_entry.fingerprint) {
                ContactState::Changed | ContactState::Conflicting => {
                    return Err(Error::ContactKeyChanged);
                }
                ContactState::Unknown => {
                    book.pin(
                        self.device,
                        now(),
                        &owner,
                        &owner_entry.fingerprint,
                        &owner_entry.email,
                    )?;
                    book_changed = true;
                }
                ContactState::Pinned { .. } => {}
            }
        } else if roster.member(&owner).map(|m| m.fingerprint) != Some(sender_fpr) {
            return Err(Error::InviteNotInRoster);
        }
        if book_changed {
            self.save_contact_book(&book)?;
        }
        let sender_state = self.contact_state(&inv.sender, sender_ipk)?;
        let owner_state = self.contact_book()?.check(&owner, &owner_fpr);
        let name = meta.name().map(str::to_owned);
        self.sweep_pending_invites();
        self.pending_invites.insert(
            inv.invite_id,
            PendingInvite {
                vk,
                vault: inv.vault_id,
                generation: inv.generation,
                sender: inv.sender,
                sender_fpr,
                role: inv.role,
                owner,
                owner_fpr,
                opened: Instant::now(),
            },
        );
        Ok(InvitePreview {
            invite_id: inv.invite_id,
            vault_id: inv.vault_id,
            vault_name: name,
            role: inv.role,
            generation: inv.generation,
            sender: inv.sender,
            sender_email: inv.sender_email.clone(),
            sender_code: sender_fpr.safety_code(),
            sender_state,
            owner,
            owner_code: owner_fpr.safety_code(),
            owner_state,
        })
    }

    /// Drops the opened invite keys whose five minutes are over. The sync
    /// layer calls it every run, so a key does not sit in memory after its
    /// time (M6b review L4); [`Self::accept_invite`] refuses an expired one
    /// either way.
    pub fn sweep_pending_invites(&mut self) {
        self.pending_invites
            .retain(|_, p| p.opened.elapsed() <= PENDING_INVITE_TTL);
    }

    /// Whether the sharing row of `vault` names `owner`.
    fn is_owned_by(&self, vault: &VaultId, owner: &AccountId) -> bool {
        self.store
            .vault_sharing(vault)
            .ok()
            .flatten()
            .is_some_and(|r| r.owner_account == *owner)
    }

    /// Moves the opened-at time of every pending invite back (a test cannot
    /// wait five minutes).
    #[cfg(test)]
    pub(super) fn age_pending_invites(&mut self, by: Duration) {
        for p in self.pending_invites.values_mut() {
            if let Some(t) = p.opened.checked_sub(by) {
                p.opened = t;
            }
        }
    }

    /// Whether a vault ID is new to this device (M6b review H1). An invite
    /// or an emergency release hands a vault key to the device; for an ID
    /// the device already knows (an open vault, a generation floor of a
    /// deleted one, or a sharing row) the key would replace or join a
    /// vault the server and a contact have no business with. The one
    /// exception is a vault this account lost access to, offered again by
    /// the same owner.
    ///
    /// # Errors
    /// [`Error::VaultKnown`], [`Error::Store`].
    pub fn check_vault_unknown(&self, vault: &VaultId, owner: &AccountId) -> Result<()> {
        if let Some(row) = self.store.vault_sharing(vault)? {
            return if row.access_lost_at_ms.is_some() && row.owner_account == *owner {
                Ok(())
            } else {
                Err(Error::VaultKnown)
            };
        }
        if self.vaults.contains_key(vault) || self.store.generation_floor(vault)?.is_some() {
            return Err(Error::VaultKnown);
        }
        Ok(())
    }

    /// Accepts a previewed invite (the user's gesture, with the sender's
    /// pin verified): wraps the opened key under the account key. Send it to
    /// the server, then [`Self::commit_invite`].
    ///
    /// # Errors
    /// [`Error::InviteNotOpened`] (not previewed in this session, or the
    /// preview expired), [`Error::ContactUnverified`],
    /// [`Error::ContactKeyChanged`], [`Error::Core`].
    pub fn accept_invite(&self, invite_id: &InviteId) -> Result<AcceptedInvite> {
        let p = self
            .pending_invites
            .get(invite_id)
            .filter(|p| p.opened.elapsed() <= PENDING_INVITE_TTL)
            .ok_or(Error::InviteNotOpened)?;
        match self.contact_book()?.check(&p.sender, &p.sender_fpr) {
            ContactState::Pinned { verified: true } => {}
            ContactState::Pinned { .. } | ContactState::Unknown => {
                return Err(Error::ContactUnverified);
            }
            ContactState::Changed | ContactState::Conflicting => {
                return Err(Error::ContactKeyChanged);
            }
        }
        // Later rotation shares are accepted only from the owner, whose pin
        // must therefore be verified (M6b review M4).
        if p.owner != p.sender {
            match self.contact_book()?.check(&p.owner, &p.owner_fpr) {
                ContactState::Pinned { verified: true } => {}
                ContactState::Pinned { .. } | ContactState::Unknown => {
                    return Err(Error::ContactUnverified);
                }
                ContactState::Changed | ContactState::Conflicting => {
                    return Err(Error::ContactKeyChanged);
                }
            }
        }
        self.check_vault_unknown(&p.vault, &p.owner)?;
        Ok(AcceptedInvite {
            session_tag: self.session_tag,
            invite_id: Some(*invite_id),
            vault_id: p.vault,
            generation: p.generation,
            wrapped: p.vk.wrap(&self.account_key, &self.account_id, &p.vault)?,
            owner: p.owner,
            role: p.role,
        })
    }

    /// Stores the vault after the server added the membership: the key
    /// (origin `Remote`) and the `shared` flag with the role and the owner.
    /// `owner` is the owner the server names; it must be the one the roster
    /// named. The next run pulls the vault from the start.
    ///
    /// # Errors
    /// [`Error::PreparedStale`], [`Error::OwnerMismatch`],
    /// [`Error::RoleForbidden`] for `Owner` or `Removed`, and the errors of
    /// [`Self::apply_remote_vault_key`].
    pub fn commit_invite(
        &mut self,
        a: AcceptedInvite,
        role: Role,
        owner: &AccountId,
    ) -> Result<()> {
        let AcceptedInvite {
            session_tag,
            invite_id,
            vault_id,
            generation,
            wrapped,
            owner: named_owner,
            ..
        } = a;
        if session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        if *owner != named_owner {
            return Err(Error::OwnerMismatch);
        }
        if matches!(role, Role::Owner | Role::Removed) {
            return Err(Error::RoleForbidden);
        }
        // The check and the key acceptance follow each other under this
        // exclusive borrow; the store refuses a vault that exists as well.
        self.check_vault_unknown(&vault_id, owner)?;
        self.apply_remote_vault_key(vault_id, generation, &wrapped)?;
        self.store
            .set_vault_sharing(&vault_id, role_number(role)?, owner, true)?;
        if let Some(id) = invite_id {
            self.pending_invites.remove(&id);
        }
        Ok(())
    }

    // ---- revocation and rotation -------------------------------------------

    /// Queues a revocation of `member` for the vault's owner device (a
    /// manager's removal waits there; only the owner rotates).
    ///
    /// # Errors
    /// [`Error::RoleForbidden`] unless this account is owner or manager,
    /// [`Error::OwnerCannotBeRevoked`], [`Error::UnknownVault`],
    /// [`Error::Store`].
    pub fn request_revocation(
        &mut self,
        vault: VaultId,
        member: &AccountId,
        reason: RevocationReason,
    ) -> Result<()> {
        if !self.vault_own_role(vault)?.can_manage() || *member == self.account_id {
            return Err(Error::RoleForbidden);
        }
        if *member == self.vault_owner(vault)? {
            return Err(Error::OwnerCannotBeRevoked);
        }
        Ok(self
            .store
            .add_pending_revocation(&vault, member, reason.code())?)
    }

    /// The revocations still to carry out.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn pending_revocations(&self) -> Result<Vec<(VaultId, AccountId, RevocationReason)>> {
        self.store
            .pending_revocations()?
            .into_iter()
            .map(|r| {
                Ok((
                    r.vault_id,
                    r.account_id,
                    RevocationReason::from_code(r.reason)
                        .ok_or(Error::Store(crate::storage::Error::Corrupt))?,
                ))
            })
            .collect()
    }

    /// Prepares a rotation of a shared vault (CRYPTO_SPEC §8.3), **on the
    /// owner's device only**: a new key one generation up, a share for every
    /// remaining member and pending invite (each served key must match the
    /// roster fingerprint and the contact pin, else no seal is made), the
    /// roster at the new generation with the revoked members `removed`, and
    /// emergency seals for the grants that cover the vault. Nothing changes
    /// in the store or the session. Attachments this device does not hold
    /// are listed ([`PreparedSharedRotation::unreachable_attachments`]); the
    /// sync layer stores the ones it fetched first
    /// ([`Self::store_remote_attachment`]).
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::NotOwner`], [`Error::NoRoster`],
    /// [`Error::PlanMismatch`], [`Error::OwnerCannotBeRevoked`],
    /// [`Error::MemberKeyMismatch`], [`Error::ContactUnverified`],
    /// [`Error::WrongVaultProtection`], [`Error::Core`], [`Error::Store`].
    #[expect(
        clippy::too_many_lines,
        reason = "the plan checks and the preparation of one rotation belong together"
    )]
    pub fn prepare_shared_rotation(
        &self,
        vault: VaultId,
        plan: &RotationPlan,
    ) -> Result<PreparedSharedRotation> {
        let id = self.own_identity()?;
        let ov = self.vault(vault)?;
        if ov.protection != VaultKeyWrap::AccountKey {
            return Err(Error::WrongVaultProtection);
        }
        ov.current_key()?;
        if self.vault_own_role(vault)? != Role::Owner || self.vault_owner(vault)? != self.account_id
        {
            return Err(Error::NotOwner);
        }
        let roster = self.load_roster(vault)?.ok_or(Error::NoRoster)?;
        roster.validate()?;
        let members = active(roster.members());
        let fpr_of = |a: &AccountId| {
            members
                .iter()
                .find(|m| m.account_id == *a)
                .map(|m| m.fingerprint)
        };
        let remaining: BTreeSet<AccountId> = plan.remaining.iter().map(|(a, _)| *a).collect();
        let revoked: BTreeSet<AccountId> = plan.revoked.iter().copied().collect();
        let invitees: BTreeSet<AccountId> = plan.invites.iter().map(|(_, a, _)| *a).collect();
        if revoked.contains(&self.account_id) {
            return Err(Error::PlanMismatch);
        }
        if remaining.contains(&self.account_id)
            || invitees.contains(&self.account_id)
            || remaining.len() != plan.remaining.len()
            || invitees.len() != plan.invites.len()
            || !remaining.is_disjoint(&revoked)
            || !invitees.is_disjoint(&revoked)
            || !remaining.is_disjoint(&invitees)
        {
            return Err(Error::PlanMismatch);
        }
        // Every active member but the owner stays, is revoked or is invited;
        // everyone who stays or is invited is in the roster.
        for m in &members {
            if m.account_id != self.account_id
                && !(remaining.contains(&m.account_id)
                    || revoked.contains(&m.account_id)
                    || invitees.contains(&m.account_id))
            {
                return Err(Error::PlanMismatch);
            }
        }
        if remaining
            .iter()
            .chain(&invitees)
            .any(|a| fpr_of(a).is_none())
        {
            return Err(Error::PlanMismatch);
        }
        let book = self.contact_book()?;
        let check_member = |account: &AccountId, ipk: &IdentityPublicKey| -> Result<()> {
            let fpr = Fingerprint::of(account, ipk);
            if fpr_of(account) != Some(fpr) {
                return Err(Error::MemberKeyMismatch);
            }
            // A key is sealed only to a contact this account verified (H2):
            // a roster entry written by a member says nothing about who the
            // key would go to.
            match book.check(account, &fpr) {
                ContactState::Changed | ContactState::Conflicting => Err(Error::MemberKeyMismatch),
                ContactState::Pinned { verified: true } => Ok(()),
                ContactState::Pinned { .. } | ContactState::Unknown => {
                    Err(Error::ContactUnverified)
                }
            }
        };
        for (a, ipk) in &plan.remaining {
            check_member(a, ipk)?;
        }
        for (_, a, ipk) in &plan.invites {
            check_member(a, ipk)?;
        }
        for (a, ipk) in &plan.grants {
            match book.check(a, &Fingerprint::of(a, ipk)) {
                ContactState::Pinned { verified: true } => {}
                ContactState::Changed | ContactState::Conflicting => {
                    return Err(Error::MemberKeyMismatch);
                }
                _ => return Err(Error::ContactUnverified),
            }
        }
        if revoked.contains(&roster.owner()) {
            return Err(Error::OwnerCannotBeRevoked);
        }

        let next = ov.generation.next()?;
        let key = VaultKey::generate(next)?;
        let wrapped = key.wrap(&self.account_key, &self.account_id, &vault)?;
        let mut new_roster = self
            .load_roster(vault)?
            .ok_or(Error::NoRoster)?
            .advance(next)?;
        {
            let mut edit = new_roster.edit(self.device, now());
            for m in members.iter().filter(|m| revoked.contains(&m.account_id)) {
                edit.remove_member(&m.account_id, next)?;
            }
        }
        let roster_ct = new_roster.seal(&key, &vault)?;
        let seal_to = |a: &AccountId, ipk: &IdentityPublicKey| {
            seal_vault_key(&key, &vault, &self.account_id, &id.isk, a, ipk)
        };
        let seals = plan
            .remaining
            .iter()
            .map(|(a, ipk)| Ok((*a, seal_to(a, ipk)?)))
            .collect::<Result<Vec<_>>>()?;
        let invite_seals = plan
            .invites
            .iter()
            .map(|(i, a, ipk)| Ok((*i, seal_to(a, ipk)?)))
            .collect::<Result<Vec<_>>>()?;
        let emergency_seals = plan
            .grants
            .iter()
            .map(|(a, ipk)| {
                Ok((
                    *a,
                    seal_emergency_vault_key(&key, &vault, &self.account_id, &id.isk, a, ipk)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut unreachable = Vec::new();
        for (item_id, item) in ov.items.iter().filter(|(_, i)| !i.is_deleted()) {
            for a in item.attachments() {
                if self.store.attachment_info(&a.attachment_id)?.is_none() {
                    unreachable.push((*item_id, a.attachment_id));
                }
            }
        }
        Ok(PreparedSharedRotation {
            session_tag: self.session_tag,
            vault_id: vault,
            base: ov.generation,
            generation: next,
            key,
            wrapped,
            roster: roster_ct,
            seals,
            invite_seals,
            emergency_seals,
            revoke: plan.revoked.clone(),
            unreachable,
        })
    }

    /// Applies a shared rotation after the server accepted it: the key is
    /// accepted as generated here and already uploaded, every record, the
    /// metadata and the roster are re-sealed under it and queued, and every
    /// attachment held here is re-sealed under a fresh attachment ID and
    /// file key (ADR-0017), with the item references updated and the old IDs
    /// recorded for deletion, in ONE store transaction. The caller sends the
    /// recorded attachment deletions only after the referencing records were
    /// accepted. The revocations it carried out leave the queue.
    ///
    /// # Errors
    /// [`Error::PreparedStale`] after a lock,
    /// [`Error::KeyGenerationRollback`] if the vault moved on meanwhile,
    /// [`Error::Core`], [`Error::Store`].
    pub fn commit_shared_rotation(
        &mut self,
        p: PreparedSharedRotation,
    ) -> Result<SharedRotationOutcome> {
        let PreparedSharedRotation {
            session_tag,
            vault_id,
            generation,
            key,
            wrapped,
            roster,
            revoke,
            unreachable,
            ..
        } = p;
        if session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        let ov = self.vaults.get(&vault_id).ok_or(Error::UnknownVault)?;
        ov.current_key()?;
        if ov.generation >= generation {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: ov.generation.get(),
                offered: generation.get(),
            });
        }
        let mut resealed = Resealed::default();
        let built = self.reseal_vault(vault_id, &key, &unreachable, &mut resealed);
        let staged_ids: Vec<AttachmentId> = resealed.staged.iter().map(|(a, _, _)| *a).collect();
        let committed = built.and_then(|records| {
            let meta = ov
                .meta
                .as_ref()
                .map(|m| m.seal(&key, &vault_id))
                .transpose()?;
            Ok(self.store.commit_shared_rotation(&SharedRotationWrite {
                vault_id,
                generation,
                wrapped: &wrapped,
                records: &records,
                meta: meta.as_ref(),
                roster: &roster,
                new_attachments: &resealed.staged,
                old_attachments: &resealed.old,
                revoked: &revoke,
            })?)
        });
        if let Err(e) = committed {
            self.store.discard_staged_attachment_files(&staged_ids)?;
            return Err(e);
        }
        for (item_id, item) in &resealed.changed {
            self.index.update(vault_id, *item_id, item);
        }
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        for (item_id, item) in resealed.changed {
            ov.items.insert(item_id, item);
        }
        ov.keys.insert(generation, key);
        ov.generation = generation;
        Ok(SharedRotationOutcome {
            generation,
            unreachable_attachments: unreachable,
        })
    }

    /// Re-seals every record of the vault under `key`, and the attachments
    /// held here under fresh IDs and keys. The staged attachment files, the
    /// old IDs and the items with their new references are collected in
    /// `out` (the caller removes the staged files if the commit fails).
    fn reseal_vault(
        &self,
        vault_id: VaultId,
        key: &VaultKey,
        unreachable: &[(ItemId, AttachmentId)],
        out: &mut Resealed,
    ) -> Result<Vec<(ItemId, crate::storage::RecordBody)>> {
        let Resealed {
            staged,
            old,
            changed,
        } = out;
        let ov = self.vaults.get(&vault_id).ok_or(Error::UnknownVault)?;
        for (item_id, item) in ov.items.iter().filter(|(_, i)| !i.is_deleted()) {
            if let Some((copy, old_ids)) =
                self.reseal_item_attachments(&vault_id, item_id, item, unreachable, staged)?
            {
                old.extend(old_ids);
                changed.insert(*item_id, copy);
            }
        }
        ov.items
            .iter()
            .map(|(item_id, item)| {
                let item = changed.get(item_id).unwrap_or(item);
                Ok((*item_id, seal_record(item, key, &vault_id, item_id)?))
            })
            .collect()
    }

    /// Re-seals the attachments of one item that are held here: the item
    /// with the new references, and the IDs they replace.
    fn reseal_item_attachments(
        &self,
        vault_id: &VaultId,
        item_id: &ItemId,
        item: &mosaic_core::Item,
        unreachable: &[(ItemId, AttachmentId)],
        staged: &mut Vec<(AttachmentId, ItemId, u64)>,
    ) -> Result<Option<(mosaic_core::Item, Vec<AttachmentId>)>> {
        let mut resealed = Vec::new();
        for old_ref in item
            .attachments()
            .into_iter()
            .filter(|a| !unreachable.iter().any(|(_, u)| *u == a.attachment_id))
        {
            let ct = self.store.attachment(&old_ref.attachment_id)?;
            let plain =
                open_attachment(&old_ref.key, vault_id, item_id, &old_ref.attachment_id, &ct)?;
            let new_ref = AttachmentRef::new(
                AttachmentId::new_v7()?,
                Text::new(old_ref.file_name.as_str()),
                Text::new(old_ref.mime.as_str()),
                old_ref.size,
                AttachmentKey::generate()?,
            );
            let new_ct = seal_attachment(
                &new_ref.key,
                vault_id,
                item_id,
                &new_ref.attachment_id,
                plain.expose_secret(),
            )?;
            self.store
                .stage_attachment_file(&new_ref.attachment_id, &new_ct)?;
            staged.push((
                new_ref.attachment_id,
                *item_id,
                u64::try_from(new_ct.len()).unwrap_or(u64::MAX),
            ));
            resealed.push((old_ref.attachment_id, new_ref));
        }
        if resealed.is_empty() {
            return Ok(None);
        }
        let mut copy = dup_item(item)?;
        let mut edit = copy.edit(self.device, now());
        for (old_id, new_ref) in &resealed {
            edit.remove_attachment(old_id)?;
            edit.set_attachment(new_ref)?;
        }
        Ok(Some((copy, resealed.into_iter().map(|(o, _)| o).collect())))
    }

    // ---- the claim rule ----------------------------------------------------

    /// Checks the key shares of a pull page against the claim rule
    /// (CRYPTO_SPEC §8.3) and opens the chain. For the lowest share above the
    /// held generation, and then for each next one: the sender must be the
    /// vault owner ([`VaultRoster::authorizes_rotation`]) with the served
    /// key's fingerprint equal to the owner's contact pin and the roster's
    /// owner entry; the seal must open; the roster of that generation must
    /// open under the new key and be valid and list this account. Only the
    /// final key is claimed ([`Self::claim_wrap`],
    /// [`Self::commit_claimed_key`]). `rosters` holds the page's roster
    /// history; `ipks` returns the identity key the server serves for an
    /// account ID.
    ///
    /// # Errors
    /// [`Error::ShareUntrusted`] for any failed step (record it with
    /// [`Self::record_sharing_finding`]); [`Error::NothingToClaim`];
    /// [`Error::NoIdentity`], [`Error::NoRoster`], [`Error::Core`],
    /// [`Error::Store`].
    pub fn open_share_chain(
        &self,
        vault: VaultId,
        shares: &[PulledShareView],
        rosters: &[(KeyGeneration, VaultRosterCiphertext)],
        ipks: &dyn Fn(&AccountId) -> Option<IdentityPublicKey>,
    ) -> Result<OpenedChain> {
        let id = self.own_identity()?;
        let ov = self.vault(vault)?;
        let held = ov.generation;
        let owner = self.vault_owner(vault)?;
        let mut ordered: Vec<&PulledShareView> =
            shares.iter().filter(|s| s.generation > held).collect();
        ordered.sort_by_key(|s| s.generation);
        let Some(first) = ordered.first() else {
            return Err(Error::NothingToClaim);
        };
        let untrusted = |g: KeyGeneration| Error::ShareUntrusted {
            generation: g.get(),
        };
        let book = self.contact_book()?;
        // Only a verified pin of the owner authorizes a rotation share (M4).
        let pinned_owner = book
            .get(&owner)
            .filter(|c| !c.conflicting && c.verified)
            .map(|c| c.fingerprint);

        // The roster of the held generation: the stored one, else the page's
        // history, opened with the held key.
        let mut current = match self.load_roster(vault) {
            Ok(Some(r)) if r.generation() == held => r,
            _ => {
                let held_key = ov.keys.get(&held).ok_or(Error::VaultUnavailable)?;
                let ct = rosters
                    .iter()
                    .find(|(g, _)| *g == held)
                    .map(|(_, ct)| ct)
                    .ok_or_else(|| untrusted(first.generation))?;
                VaultRoster::open(ct, held_key, &vault, owner)
                    .map_err(|_| untrusted(first.generation))?
            }
        };
        current
            .validate()
            .map_err(|_| untrusted(first.generation))?;

        let mut last: Option<(KeyGeneration, VaultKey, VaultRosterCiphertext)> = None;
        for share in ordered {
            let g = share.generation;
            if last.as_ref().is_some_and(|(lg, ..)| g <= *lg) {
                continue;
            }
            let sender_ipk = ipks(&share.sender).ok_or_else(|| untrusted(g))?;
            let sender_fpr = Fingerprint::of(&share.sender, &sender_ipk);
            let pinned = pinned_owner.ok_or_else(|| untrusted(g))?;
            current
                .authorizes_rotation(
                    &share.sender,
                    &sender_fpr,
                    &owner,
                    &pinned,
                    current.generation(),
                )
                .map_err(|_| untrusted(g))?;
            if current.member(&owner).map(|m| m.fingerprint) != Some(sender_fpr) {
                return Err(untrusted(g));
            }
            let vk = open_vault_key(
                &share.seal,
                &vault,
                g,
                &share.sender,
                &sender_ipk,
                &self.account_id,
                &id.isk,
            )
            .map_err(|_| untrusted(g))?;
            let ct = rosters
                .iter()
                .find(|(rg, _)| *rg == g)
                .map(|(_, ct)| ct)
                .ok_or_else(|| untrusted(g))?;
            let next = VaultRoster::open(ct, &vk, &vault, owner).map_err(|_| untrusted(g))?;
            next.validate().map_err(|_| untrusted(g))?;
            match next.member(&self.account_id) {
                Some(m) if m.role != Role::Removed => {}
                _ => return Err(untrusted(g)),
            }
            current = next;
            last = Some((g, vk, ct.clone()));
        }
        let (generation, key, roster) = last.ok_or(Error::NothingToClaim)?;
        Ok(OpenedChain {
            session_tag: self.session_tag,
            vault_id: vault,
            generation,
            key,
            roster,
        })
    }

    /// The final key of a chain wrapped under the account key, as claimed on
    /// the server.
    ///
    /// # Errors
    /// [`Error::PreparedStale`], [`Error::Core`].
    pub fn claim_wrap(&self, chain: &OpenedChain) -> Result<WrappedKey> {
        if chain.session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        Ok(chain
            .key
            .wrap(&self.account_key, &self.account_id, &chain.vault_id)?)
    }

    /// Stores the claimed key (origin `Remote`, like any key another device
    /// of the account receives) and the roster of its generation, and
    /// updates this account's role from it.
    ///
    /// # Errors
    /// [`Error::PreparedStale`]; [`Error::Core`] if `wrapped` is not the
    /// chain's key; the errors of [`Self::apply_remote_vault_key`].
    pub fn commit_claimed_key(&mut self, chain: OpenedChain, wrapped: &WrappedKey) -> Result<()> {
        let OpenedChain {
            session_tag,
            vault_id: vault,
            generation,
            key,
            roster,
        } = chain;
        if session_tag != self.session_tag {
            return Err(Error::PreparedStale);
        }
        let unwrapped =
            wrapped.unwrap_vault_key(&self.account_key, &self.account_id, &vault, generation)?;
        if unwrapped != key {
            return Err(Error::Core(mosaic_core::Error::Decryption));
        }
        self.apply_remote_vault_key(vault, generation, wrapped)?;
        self.store.put_vault_roster(&vault, &roster, None, false)?;
        if let Some(row) = self.store.vault_sharing(&vault)?
            && let Some(roster) = self.load_roster(vault)?
            && let Some(me) = roster.member(&self.account_id)
        {
            let n = me.role.number().unwrap_or(row.own_role);
            self.store
                .set_vault_sharing(&vault, n, &row.owner_account, false)?;
        }
        Ok(())
    }

    // ---- emergency access --------------------------------------------------

    /// Seals the current keys of `vaults` to an emergency grantee
    /// (CRYPTO_SPEC §8.4, vault keys only): the grantee must be a verified
    /// contact with the served key, and this account must own every vault.
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::ContactUnverified`],
    /// [`Error::ContactKeyChanged`], [`Error::NotOwner`],
    /// [`Error::WrongVaultProtection`], [`Error::VaultUnavailable`],
    /// [`Error::Core`], [`Error::Store`].
    pub fn seal_emergency(
        &self,
        grantee: &AccountId,
        ipk: &IdentityPublicKey,
        vaults: &[VaultId],
    ) -> Result<Vec<(VaultId, KeyGeneration, ShareSeal)>> {
        let id = self.own_identity()?;
        Self::check_contact(&self.contact_book()?, grantee, ipk, true)?;
        let mut out = Vec::new();
        for vault in vaults {
            let ov = self.vault(*vault)?;
            if ov.protection != VaultKeyWrap::AccountKey {
                return Err(Error::WrongVaultProtection);
            }
            if self.vault_own_role(*vault)? != Role::Owner
                || self.vault_owner(*vault)? != self.account_id
            {
                return Err(Error::NotOwner);
            }
            let vk = ov.current_key()?;
            out.push((
                *vault,
                ov.generation,
                seal_emergency_vault_key(vk, vault, &self.account_id, &id.isk, grantee, ipk)?,
            ));
        }
        Ok(out)
    }

    /// Opens the seals of an emergency release (CRYPTO_SPEC §8.4). An
    /// unverified grantor is allowed (the caller warned the user); a changed
    /// key is not. One bad seal fails the whole release and records
    /// `ShareUntrusted`. Each opened key is returned as an
    /// [`AcceptedInvite`] with the grantor as owner and role `Read`, to be
    /// claimed on the server and stored with [`Self::commit_invite`].
    ///
    /// # Errors
    /// [`Error::NoIdentity`], [`Error::ContactKeyChanged`],
    /// [`Error::ShareUntrusted`], [`Error::Core`], [`Error::Store`].
    pub fn open_emergency_release(
        &mut self,
        grantor: &AccountId,
        ipk: &IdentityPublicKey,
        seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<Vec<AcceptedInvite>> {
        let fpr = Fingerprint::of(grantor, ipk);
        let mut book = self.contact_book()?;
        match book.check(grantor, &fpr) {
            ContactState::Changed | ContactState::Conflicting => {
                return Err(Error::ContactKeyChanged);
            }
            ContactState::Unknown => {
                book.pin(self.device, now(), grantor, &fpr, "")?;
                self.save_contact_book(&book)?;
            }
            ContactState::Pinned { .. } => {}
        }
        let mut out = Vec::new();
        let mut failed = None;
        let mut forged: Vec<(VaultId, KeyGeneration)> = Vec::new();
        {
            let id = self.own_identity()?;
            for (vault, generation, seal) in seals {
                // A vault this device knows gets no key from a release (H1).
                // One that is the grantor's own already (the account is a
                // member) is skipped quietly; any other is a forged seal.
                let known = self.check_vault_unknown(vault, grantor).is_err();
                if known && !self.is_owned_by(vault, grantor) {
                    forged.push((*vault, *generation));
                }
                if known {
                    continue;
                }
                let opened = open_emergency_vault_key(
                    seal,
                    vault,
                    *generation,
                    grantor,
                    ipk,
                    &self.account_id,
                    &id.isk,
                );
                let Ok(vk) = opened else {
                    failed = Some((*vault, *generation));
                    break;
                };
                out.push(AcceptedInvite {
                    session_tag: self.session_tag,
                    invite_id: None,
                    vault_id: *vault,
                    generation: *generation,
                    wrapped: vk.wrap(&self.account_key, &self.account_id, vault)?,
                    owner: *grantor,
                    role: Role::Read,
                });
            }
        }
        for (vault, generation) in forged {
            self.record_sharing_finding(SharingFinding::ShareUntrusted {
                vault,
                generation,
                sender: *grantor,
            })?;
        }
        if let Some((vault, generation)) = failed {
            self.record_sharing_finding(SharingFinding::ShareUntrusted {
                vault,
                generation,
                sender: *grantor,
            })?;
            return Err(Error::ShareUntrusted {
                generation: generation.get(),
            });
        }
        Ok(out)
    }
}

/// Opens a roster whose owner is not known yet (an invite preview): the
/// owner is the unique member with the role `owner`.
fn open_roster_unowned(
    ct: &VaultRosterCiphertext,
    vk: &VaultKey,
    vault: &VaultId,
) -> Result<(VaultRoster, AccountId)> {
    // An account that cannot be a member keeps the probe from forcing any
    // entry to `owner`.
    let probe_owner = AccountId::from_bytes([0xFF; 16])?;
    let probe =
        VaultRoster::open(ct, vk, vault, probe_owner).map_err(|_| Error::InviteUnreadable)?;
    let mut owners = probe
        .members()
        .into_iter()
        .filter(|m| m.role == Role::Owner)
        .map(|m| m.account_id);
    let (Some(owner), None) = (owners.next(), owners.next()) else {
        return Err(Error::InviteNotInRoster);
    };
    let roster = VaultRoster::open(ct, vk, vault, owner).map_err(|_| Error::InviteUnreadable)?;
    Ok((roster, owner))
}

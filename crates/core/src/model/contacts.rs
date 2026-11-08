//! The contact book (CRYPTO_SPEC §8.5, ADR-0054).
//!
//! An account-level replicated document in the layout of
//! [`super::VaultMeta`], sealed under `K_contacts` ([`ContactsCiphertext`]) and
//! synced through the hosted server so that every device of the account
//! shares the same pins. Its keys are `contact:<account_id as 32 hex digits>`
//! and its values `{"email":E,"firstSeen":ms,"fpr":HEX64,"verified":bool}`.
//!
//! Pinning is trust on first use plus an explicit user attestation
//! (`verified`, set after comparing safety codes). A contact whose served key
//! no longer matches the pin is [`ContactState::Changed`]; two devices that
//! pinned different fingerprints at once leave a register with two distinct
//! `fpr` values, which is [`ContactState::Conflicting`]. Both block every
//! seal to and from that account until the user decides
//! ([`ContactBook::replace_pin`]).

use core::fmt;

use super::clock::{Stamp, Timestamp};
use super::doc::{Doc, DocJson, Format, Register, decode};
use super::roster::{from_hex, to_hex};
use super::value::{Object, Value, to_json};
use crate::envelope::{ContactsCiphertext, MAX_CONTACTS_PLAINTEXT_LEN};
use crate::error::{Error, Result};
use crate::identity::Fingerprint;
use crate::ids::{AccountId, DeviceId};
use crate::secret::{ContactsKey, SecretVec};

const CONTACT_PREFIX: &str = "contact:";

/// What the book knows about an account's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactState {
    /// No pin.
    Unknown,
    /// Pinned, and the checked fingerprint matches.
    Pinned {
        /// The user compared safety codes.
        verified: bool,
    },
    /// Pinned, and the checked fingerprint differs: the key changed.
    Changed,
    /// Devices pinned different fingerprints concurrently.
    Conflicting,
}

/// One contact as the book shows it.
#[derive(Clone, PartialEq, Eq)]
pub struct Contact {
    /// The contact's account.
    pub account_id: AccountId,
    /// The pinned fingerprint (the newest one if [`Contact::conflicting`]).
    pub fingerprint: Fingerprint,
    /// The user compared safety codes for this fingerprint.
    pub verified: bool,
    /// The email the contact was looked up by.
    pub email: String,
    /// When the pin was first made, by the pinning device's clock.
    pub first_seen: Timestamp,
    /// Concurrent pins disagree.
    pub conflicting: bool,
}

impl fmt::Debug for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Contact")
            .field("account_id", &self.account_id)
            .field("verified", &self.verified)
            .field("conflicting", &self.conflicting)
            .finish_non_exhaustive()
    }
}

struct Parsed {
    fingerprint: Fingerprint,
    verified: bool,
    email: String,
    first_seen: Timestamp,
}

fn contact_key(account: &AccountId) -> String {
    format!("{CONTACT_PREFIX}{}", to_hex(account.as_bytes()))
}

fn parse_contact_key(key: &str) -> Option<AccountId> {
    let hex = key.strip_prefix(CONTACT_PREFIX)?;
    AccountId::from_bytes(from_hex::<16>(hex).ok()?).ok()
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
    let verified = o
        .get("verified")
        .and_then(Value::as_bool)
        .ok_or(Error::Malformed)?;
    let first_seen = o
        .get("firstSeen")
        .and_then(Value::as_i64)
        .ok_or(Error::Malformed)?;
    Ok(Parsed {
        fingerprint: Fingerprint::from_bytes(&from_hex::<32>(fpr)?)?,
        verified,
        email,
        first_seen: Timestamp::from_unix_millis(first_seen),
    })
}

fn build_value(
    fingerprint: &Fingerprint,
    verified: bool,
    email: &str,
    first_seen: Timestamp,
) -> Value {
    let mut o = Object::new();
    o.insert("email", Value::text(email));
    o.insert("firstSeen", Value::int(first_seen.as_unix_millis()));
    o.insert("fpr", Value::text(&to_hex(fingerprint.as_bytes())));
    o.insert("verified", Value::Bool(verified));
    Value::Object(o)
}

/// The effective view of a register: `None` if no entry parses.
fn effective(reg: &Register) -> Option<(Parsed, bool)> {
    let all: Vec<(Stamp, Parsed)> = reg
        .entries()
        .filter_map(|e| parse_value(e.value()).ok().map(|p| (e.stamp(), p)))
        .collect();
    let (_, newest) = all.iter().max_by_key(|(s, _)| *s)?;
    let conflicting = all.iter().any(|(_, p)| p.fingerprint != newest.fingerprint);
    // Entries that agree on the fingerprint are one pin: verified if any
    // says so, first seen at the earliest.
    let same = all
        .iter()
        .filter(|(_, p)| p.fingerprint == newest.fingerprint);
    let verified = same.clone().any(|(_, p)| p.verified);
    let first_seen = same
        .map(|(_, p)| p.first_seen)
        .min()
        .unwrap_or(newest.first_seen);
    Some((
        Parsed {
            fingerprint: newest.fingerprint,
            verified,
            email: newest.email.clone(),
            first_seen,
        },
        conflicting,
    ))
}

/// The decrypted contact book.
#[derive(PartialEq, Eq)]
pub struct ContactBook {
    doc: Doc,
}

impl fmt::Debug for ContactBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContactBook")
            .field("version", self.doc.vv())
            .finish_non_exhaustive()
    }
}

impl Default for ContactBook {
    fn default() -> Self {
        Self::new()
    }
}

impl ContactBook {
    /// An empty book.
    #[must_use]
    pub fn new() -> Self {
        Self {
            doc: Doc::default(),
        }
    }

    /// What the book says about `account` when the server serves
    /// `fingerprint` for it.
    #[must_use]
    pub fn check(&self, account: &AccountId, fingerprint: &Fingerprint) -> ContactState {
        let Some(reg) = self.doc.register(&contact_key(account)) else {
            return ContactState::Unknown;
        };
        let Some((p, conflicting)) = effective(reg) else {
            return ContactState::Unknown;
        };
        if conflicting {
            ContactState::Conflicting
        } else if p.fingerprint == *fingerprint {
            ContactState::Pinned {
                verified: p.verified,
            }
        } else {
            ContactState::Changed
        }
    }

    /// The contact for `account`, if pinned.
    #[must_use]
    pub fn get(&self, account: &AccountId) -> Option<Contact> {
        let reg = self.doc.register(&contact_key(account))?;
        let (p, conflicting) = effective(reg)?;
        Some(Contact {
            account_id: *account,
            fingerprint: p.fingerprint,
            verified: p.verified,
            email: p.email,
            first_seen: p.first_seen,
            conflicting,
        })
    }

    /// All contacts, by account ID.
    #[must_use]
    pub fn contacts(&self) -> Vec<Contact> {
        let mut out: Vec<Contact> = self
            .doc
            .registers()
            .filter_map(|(key, _)| self.get(&parse_contact_key(key)?))
            .collect();
        out.sort_by_key(|c| *c.account_id.as_bytes());
        out
    }

    fn write(
        &mut self,
        device: DeviceId,
        at: Timestamp,
        account: &AccountId,
        value: Value,
    ) -> Result<()> {
        let stamp = self.doc.tick(device)?;
        self.doc
            .write(&contact_key(account), stamp, at, value, false)
    }

    /// Pins `fingerprint` for `account` (unverified) if there is no pin yet.
    /// A pin that exists is never overwritten here: a different fingerprint
    /// is a key change and goes through [`ContactBook::replace_pin`]. Returns
    /// the state after the call.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn pin(
        &mut self,
        device: DeviceId,
        at: Timestamp,
        account: &AccountId,
        fingerprint: &Fingerprint,
        email: &str,
    ) -> Result<ContactState> {
        let state = self.check(account, fingerprint);
        if state == ContactState::Unknown {
            self.write(
                device,
                at,
                account,
                build_value(fingerprint, false, email, at),
            )?;
            return Ok(self.check(account, fingerprint));
        }
        Ok(state)
    }

    /// Records that the user compared safety codes for `expected`. A single
    /// write that supersedes this replica's entries for the account.
    ///
    /// # Errors
    /// [`Error::ContactChanged`] if the pin is not `expected` (it moved, or
    /// devices disagree); [`Error::Malformed`] if there is no pin;
    /// [`Error::ClockOverflow`].
    pub fn verify(
        &mut self,
        device: DeviceId,
        at: Timestamp,
        account: &AccountId,
        expected: &Fingerprint,
    ) -> Result<()> {
        match self.check(account, expected) {
            ContactState::Pinned { .. } => {}
            ContactState::Unknown => return Err(Error::Malformed),
            ContactState::Changed | ContactState::Conflicting => {
                return Err(Error::ContactChanged);
            }
        }
        let current = self.get(account).ok_or(Error::Malformed)?;
        self.write(
            device,
            at,
            account,
            build_value(expected, true, &current.email, current.first_seen),
        )
    }

    /// Replaces the pin of `account` with `fingerprint`, unverified: the
    /// user saw the key-change warning and accepted the new key (or settled a
    /// conflict). It then needs verification again.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn replace_pin(
        &mut self,
        device: DeviceId,
        at: Timestamp,
        account: &AccountId,
        fingerprint: &Fingerprint,
        email: &str,
    ) -> Result<()> {
        self.write(
            device,
            at,
            account,
            build_value(fingerprint, false, email, at),
        )
    }

    /// Merges two replicas (a pure join).
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            doc: self.doc.merge(&other.doc),
        }
    }

    /// Merges `received` into this local replica, with the device-ID-reuse
    /// checks of [`super::VaultMeta::merge_and_repair`].
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
    /// [`Error::SizeLimit`] above [`MAX_CONTACTS_PLAINTEXT_LEN`],
    /// [`Error::Allocation`].
    pub fn to_json(&self) -> Result<SecretVec> {
        to_json(
            &DocJson {
                doc: &self.doc,
                format: Format::Document,
            },
            MAX_CONTACTS_PLAINTEXT_LEN,
        )
    }

    /// Strictly decodes contact-book plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::UnsupportedVersion`],
    /// [`Error::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_CONTACTS_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Ok(Self {
            doc: decode(bytes, Format::Document)?,
        })
    }

    /// Encrypts the book of `account` under `K_contacts`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::Allocation`], [`Error::Randomness`].
    pub fn seal(&self, key: &ContactsKey, account: &AccountId) -> Result<ContactsCiphertext> {
        let pt = self.to_json()?;
        ContactsCiphertext::seal(key, account, pt.expose_secret())
    }

    /// Decrypts and decodes the book of `account`.
    ///
    /// # Errors
    /// [`Error::Decryption`], and the errors of [`ContactBook::from_json`].
    pub fn open(ct: &ContactsCiphertext, key: &ContactsKey, account: &AccountId) -> Result<Self> {
        let pt = ct.open(key, account)?;
        Self::from_json(pt.expose_secret())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityPublicKey;

    fn dev(b: u8) -> DeviceId {
        DeviceId::from_bytes([b; 16]).unwrap()
    }

    fn acct(b: u8) -> AccountId {
        AccountId::from_bytes([b; 16]).unwrap()
    }

    fn fpr(b: u8) -> Fingerprint {
        Fingerprint::of(&acct(b), &IdentityPublicKey([b; 32]))
    }

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_unix_millis(ms)
    }

    fn copy(b: &ContactBook) -> ContactBook {
        ContactBook::from_json(b.to_json().unwrap().expose_secret()).unwrap()
    }

    #[test]
    fn pin_verify_change_and_replace() {
        let mut b = ContactBook::new();
        let (a, f1, f2) = (acct(7), fpr(7), fpr(8));
        assert_eq!(b.check(&a, &f1), ContactState::Unknown);
        assert_eq!(
            b.pin(dev(1), at(10), &a, &f1, "a@x").unwrap(),
            ContactState::Pinned { verified: false }
        );
        // A second pin never overwrites.
        assert_eq!(
            b.pin(dev(1), at(20), &a, &f2, "a@x").unwrap(),
            ContactState::Changed
        );
        assert_eq!(b.check(&a, &f1), ContactState::Pinned { verified: false });
        b.verify(dev(1), at(30), &a, &f1).unwrap();
        assert_eq!(b.check(&a, &f1), ContactState::Pinned { verified: true });
        let c = b.get(&a).unwrap();
        assert!(c.verified && !c.conflicting);
        assert_eq!((c.email.as_str(), c.first_seen), ("a@x", at(10)));
        // Verifying a fingerprint that is not the pin is refused.
        assert_eq!(
            b.verify(dev(1), at(40), &a, &f2).unwrap_err(),
            Error::ContactChanged
        );
        assert_eq!(
            b.verify(dev(1), at(40), &acct(1), &f1).unwrap_err(),
            Error::Malformed
        );
        // Accepting a key change starts unverified again.
        b.replace_pin(dev(1), at(50), &a, &f2, "a@x").unwrap();
        assert_eq!(b.check(&a, &f2), ContactState::Pinned { verified: false });
        assert_eq!(b.check(&a, &f1), ContactState::Changed);
        assert_eq!(b.contacts().len(), 1);
        assert!(format!("{:?}", b.contacts()[0]).starts_with("Contact"));
    }

    #[test]
    fn conflicting_pins_detected() {
        let base = ContactBook::new();
        let (mut one, mut two) = (copy(&base), copy(&base));
        let a = acct(7);
        one.pin(dev(1), at(1), &a, &fpr(7), "a@x").unwrap();
        two.pin(dev(2), at(2), &a, &fpr(8), "a@x").unwrap();
        let merged = one.merge(&two);
        assert_eq!(merged, two.merge(&one));
        for f in [fpr(7), fpr(8), fpr(9)] {
            assert_eq!(merged.check(&a, &f), ContactState::Conflicting);
        }
        assert!(merged.get(&a).unwrap().conflicting);
        assert_eq!(
            copy(&merged)
                .verify(dev(1), at(3), &a, &fpr(7))
                .unwrap_err(),
            Error::ContactChanged
        );
        // The user settles it.
        let mut settled = copy(&merged);
        settled
            .replace_pin(dev(1), at(4), &a, &fpr(7), "a@x")
            .unwrap();
        assert_eq!(
            settled.check(&a, &fpr(7)),
            ContactState::Pinned { verified: false }
        );
        // Concurrent pins of the same fingerprint agree; one verification wins.
        let (mut x, mut y) = (copy(&base), copy(&base));
        x.pin(dev(3), at(1), &a, &fpr(7), "a@x").unwrap();
        y.pin(dev(4), at(2), &a, &fpr(7), "a@x").unwrap();
        y.verify(dev(4), at(3), &a, &fpr(7)).unwrap();
        let m = x.merge(&y);
        assert_eq!(
            m.check(&a, &fpr(7)),
            ContactState::Pinned { verified: true }
        );
        assert_eq!(m.get(&a).unwrap().first_seen, at(1));
    }

    #[test]
    fn round_trip_under_contacts_key() {
        let ak = crate::AccountKey::generate().unwrap();
        let (me, other) = (acct(1), acct(2));
        let key = ak.contacts_key().unwrap();
        let mut b = ContactBook::new();
        b.pin(dev(1), at(5), &acct(7), &fpr(7), "seven@x").unwrap();
        let ct = b.seal(&key, &me).unwrap();
        assert_eq!(ContactBook::open(&ct, &key, &me).unwrap(), b);
        assert_eq!(
            ContactBook::open(&ct, &key, &other).unwrap_err(),
            Error::Decryption
        );
        let wrong = crate::AccountKey::generate()
            .unwrap()
            .contacts_key()
            .unwrap();
        assert_eq!(
            ContactBook::open(&ct, &wrong, &me).unwrap_err(),
            Error::Decryption
        );
        let again = ContactsCiphertext::from_bytes(ct.as_bytes()).unwrap();
        assert_eq!(again, ct);
        assert_eq!(ct.as_bytes().get(..2), Some([1u8, 1].as_slice()));
        // The key is derived per account key.
        assert_eq!(ak.contacts_key().unwrap(), key);
    }
}

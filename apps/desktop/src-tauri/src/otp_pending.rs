//! Parsed one-time-password setups waiting for the user's confirmation
//! (ADR-0031 item 2). The secret never leaves Rust; the webview holds the ID.

use std::time::{Duration, Instant, SystemTime};

use mosaic_totp::OtpAuth;

use crate::contract::errors::{IpcError, IpcResult};

/// How long a setup waits for its confirmation.
pub const PENDING_TTL: Duration = Duration::from_secs(600);
/// Most setups held at once; the oldest is dropped for a new one.
pub const MAX_PENDING: usize = 4;

/// When a slot was made: both clocks.
pub type Clocks = (Instant, SystemTime);

/// One parsed setup (`OtpAuth` zeroizes on drop).
pub struct PendingOtp {
    pub(crate) auth: OtpAuth,
    created: Clocks,
}

/// The slots, by opaque ID.
#[derive(Default)]
pub struct PendingOtps {
    entries: Vec<(String, PendingOtp)>,
}

/// Expired when either clock passed `created + PENDING_TTL` (the rule of
/// `clipboard::due`: a wall clock set back does not extend it because the
/// monotonic one still runs).
fn expired(created: Clocks, now: Clocks) -> bool {
    let deadline = created
        .0
        .checked_add(PENDING_TTL)
        .zip(created.1.checked_add(PENDING_TTL));
    deadline.is_none_or(|deadline| crate::clipboard::due(deadline, now.0, now.1))
}

impl PendingOtps {
    fn drop_expired(&mut self, now: Clocks) {
        self.entries
            .retain(|(_, entry)| !expired(entry.created, now));
    }

    /// Drops expired entries (the idle tick calls this, so a setup does not
    /// outlive its TTL in memory just because nobody asked for it).
    pub fn purge_expired(&mut self, now: Clocks) {
        self.drop_expired(now);
    }

    /// Drops expired entries, then the oldest while `len == MAX_PENDING`, then
    /// stores `auth` and returns its new opaque ID: 16 random bytes in
    /// lowercase hex.
    ///
    /// # Errors
    /// `internal` if the system randomness fails (nothing is stored).
    pub fn insert(&mut self, auth: OtpAuth, now: Clocks) -> IpcResult<String> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| IpcError::INTERNAL)?;
        let id = bytes.iter().fold(String::with_capacity(32), |mut id, b| {
            id.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
            id.push(char::from_digit(u32::from(b & 15), 16).unwrap_or('0'));
            id
        });
        self.drop_expired(now);
        self.make_room();
        self.entries
            .push((id.clone(), PendingOtp { auth, created: now }));
        Ok(id)
    }

    fn make_room(&mut self) {
        while self.entries.len() >= MAX_PENDING {
            self.entries.remove(0);
        }
    }

    /// The entry, if present and not expired (expired ones are dropped first).
    pub fn get(&mut self, id: &str, now: Clocks) -> Option<&PendingOtp> {
        self.drop_expired(now);
        self.entries
            .iter()
            .find(|(entry_id, _)| entry_id == id)
            .map(|(_, entry)| entry)
    }

    /// Removes and returns the entry (same expiry rule).
    pub fn take(&mut self, id: &str, now: Clocks) -> Option<PendingOtp> {
        self.drop_expired(now);
        let at = self
            .entries
            .iter()
            .position(|(entry_id, _)| entry_id == id)?;
        Some(self.entries.remove(at).1)
    }

    /// Puts a taken entry back under its ID with its original creation time
    /// (when a save fails after `take`). Respects [`MAX_PENDING`].
    pub fn restore(&mut self, id: String, entry: PendingOtp) {
        self.entries.retain(|(entry_id, _)| *entry_id != id);
        self.make_room();
        self.entries.push((id, entry));
    }

    /// Forgets one entry.
    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|(entry_id, _)| entry_id != id);
    }

    /// Forgets every entry (lock). IDs are random, so none is reused.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many slots exist (tests).
    #[cfg(test)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no slot exists (tests).
    #[cfg(test)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> OtpAuth {
        OtpAuth::parse("otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example")
            .unwrap()
    }

    fn at(offset: Duration, base: Clocks) -> Clocks {
        (base.0 + offset, base.1 + offset)
    }

    fn base() -> Clocks {
        (Instant::now(), SystemTime::now())
    }

    #[test]
    fn insert_get_take_remove() {
        let t0 = base();
        let mut slots = PendingOtps::default();
        let a = slots.insert(auth(), t0).unwrap();
        let b = slots.insert(auth(), t0).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        );
        assert!(slots.get(&a, t0).is_some());
        assert!(slots.take(&a, t0).is_some());
        assert!(slots.take(&a, t0).is_none(), "consumed");
        slots.remove(&b);
        assert!(slots.get(&b, t0).is_none());
        slots.remove("otp-99");
    }

    #[test]
    fn expires_at_the_ttl_on_either_clock() {
        let t0 = base();
        let just_before = Duration::from_secs(599);
        let mut slots = PendingOtps::default();
        let id = slots.insert(auth(), t0).unwrap();
        assert!(slots.get(&id, at(just_before, t0)).is_some());
        // Monotonic clock past the TTL, wall clock set back.
        let mono_only = (t0.0 + PENDING_TTL, t0.1);
        assert!(slots.get(&id, mono_only).is_none());
        let id = slots.insert(auth(), t0).unwrap();
        let wall_only = (t0.0, t0.1 + PENDING_TTL);
        assert!(slots.take(&id, wall_only).is_none());
        assert_eq!(slots.len(), 0);
    }

    #[test]
    fn purge_drops_expired_entries_without_an_access() {
        let t0 = base();
        let mut slots = PendingOtps::default();
        slots.insert(auth(), t0).unwrap();
        slots
            .insert(auth(), at(Duration::from_secs(300), t0))
            .unwrap();
        slots.purge_expired(at(Duration::from_secs(599), t0));
        assert_eq!(slots.len(), 2);
        slots.purge_expired(at(PENDING_TTL, t0));
        assert_eq!(slots.len(), 1, "only the older one expired");
        slots.purge_expired(at(Duration::from_mins(15), t0));
        assert!(slots.is_empty());
    }

    #[test]
    fn the_oldest_is_evicted_at_the_limit() {
        let t0 = base();
        let mut slots = PendingOtps::default();
        let ids: Vec<String> = (0..=MAX_PENDING)
            .map(|_| slots.insert(auth(), t0).unwrap())
            .collect();
        assert_eq!(slots.len(), MAX_PENDING);
        assert!(slots.get(&ids[0], t0).is_none(), "oldest dropped");
        assert!(slots.get(&ids[MAX_PENDING], t0).is_some());
    }

    #[test]
    fn restore_keeps_the_creation_time() {
        let t0 = base();
        let mut slots = PendingOtps::default();
        let id = slots.insert(auth(), t0).unwrap();
        let entry = slots.take(&id, at(Duration::from_secs(100), t0)).unwrap();
        slots.restore(id.clone(), entry);
        assert!(slots.get(&id, at(Duration::from_secs(599), t0)).is_some());
        assert!(slots.get(&id, at(PENDING_TTL, t0)).is_none());
    }

    #[test]
    fn ids_never_repeat_after_clear() {
        let t0 = base();
        let mut slots = PendingOtps::default();
        let a = slots.insert(auth(), t0).unwrap();
        slots.clear();
        assert_eq!(slots.len(), 0);
        assert_ne!(slots.insert(auth(), t0).unwrap(), a);
    }
}

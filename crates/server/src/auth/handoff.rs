//! Browser handoffs of the WebAuthn ceremony (CRYPTO_SPEC §9.3, ADR-0077).
//!
//! A handoff lives in server memory only: a restart loses every one. It is
//! bound to the token hash of the session that created it. The ID and the
//! typed match code come from the OS CSPRNG; the code is compared in constant
//! time. Nothing here is logged.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use subtle::ConstantTimeEq;

use super::webauthn::CeremonyState;

/// A handoff lives at most five minutes.
pub const HANDOFF_TTL_MS: u64 = 300_000;
/// Live handoffs per account.
pub const MAX_PER_ACCOUNT: usize = 3;
/// Live handoffs on the server.
pub const MAX_TOTAL: usize = 10_000;
/// Wrong typed codes before the handoff fails.
pub const CODE_ATTEMPTS: u8 = 5;
/// Finish attempts per handoff.
pub const FINISH_ATTEMPTS: u8 = 5;

/// What the handoff is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// Completes a pending login.
    Login,
    /// Registers a credential.
    Register,
    /// Proves a second factor for five minutes.
    Confirm,
}

/// Where a handoff stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for the page.
    Waiting,
    /// The ceremony succeeded.
    Done,
    /// Too many wrong codes or finish attempts.
    Failed,
    /// Past its expiry.
    Expired,
}

/// Why a handoff step was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// No such handoff, or it ran out.
    Expired,
    /// The handoff failed or is done: no more steps.
    Closed,
    /// A wrong code (counted).
    WrongCode,
    /// No ceremony state is held (a second finish).
    NoState,
}

/// Why a handoff could not be created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitReached;

struct Handoff {
    purpose: Purpose,
    token_hash: [u8; 32],
    account_id: [u8; 16],
    device_id: [u8; 16],
    device_name: String,
    label: Option<String>,
    match_code: [u8; 6],
    rp_state: Option<CeremonyState>,
    created: u64,
    expires: u64,
    code_attempts: u8,
    finish_attempts: u8,
    state: State,
}

impl Handoff {
    fn live(&self, now: u64) -> bool {
        self.state == State::Waiting && self.expires > now
    }
}

/// What a handoff is created for.
pub struct NewHandoff<'a> {
    /// What it is for.
    pub purpose: Purpose,
    /// The creating session.
    pub token_hash: [u8; 32],
    /// The account.
    pub account_id: [u8; 16],
    /// The creating session's device.
    pub device_id: [u8; 16],
    /// The device's name, shown on the page.
    pub device_name: &'a str,
    /// The label of a registration.
    pub label: Option<String>,
}

/// A handoff just created.
pub struct Created {
    /// The ID (16 random bytes).
    pub id: [u8; 16],
    /// The six decimal digits the user types.
    pub match_code: [u8; 6],
    /// Expiry, Unix milliseconds.
    pub expires: u64,
}

/// What the page may know after a correct code.
pub struct Verified {
    /// What the handoff is for.
    pub purpose: Purpose,
    /// The account.
    pub account_id: [u8; 16],
    /// The device that asked.
    pub device_name: String,
    /// When the handoff was created.
    pub created: u64,
}

/// What `finish` takes out of a handoff.
pub struct Taken {
    /// What the handoff is for.
    pub purpose: Purpose,
    /// The creating session.
    pub token_hash: [u8; 32],
    /// The account.
    pub account_id: [u8; 16],
    /// The creating session's device.
    pub device_id: [u8; 16],
    /// The label for a registration.
    pub label: Option<String>,
    /// The single-use ceremony state.
    pub rp_state: CeremonyState,
}

/// The handoffs, in memory only.
#[derive(Default)]
pub struct Handoffs {
    map: Mutex<HashMap<[u8; 16], Handoff>>,
}

impl Handoffs {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<[u8; 16], Handoff>> {
        self.map.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Creates a handoff that expires at `now + 5 min`, or earlier at
    /// `not_after` (the pending session's expiry).
    ///
    /// # Errors
    /// [`LimitReached`] at 3 live handoffs of the account or 10 000 in all.
    /// `None` if the OS random source fails.
    pub fn create(
        &self,
        now: u64,
        new: NewHandoff<'_>,
        not_after: u64,
    ) -> Option<Result<Created, LimitReached>> {
        let NewHandoff {
            purpose,
            token_hash,
            account_id,
            device_id,
            device_name,
            label,
        } = new;
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).ok()?;
        let match_code = random_code()?;
        let expires = now.saturating_add(HANDOFF_TTL_MS).min(not_after);
        let mut map = self.lock();
        map.retain(|_, h| h.expires.saturating_add(60_000) > now);
        let mine = map
            .values()
            .filter(|h| h.account_id == account_id && h.live(now))
            .count();
        let live = map.values().filter(|h| h.live(now)).count();
        if mine >= MAX_PER_ACCOUNT || live >= MAX_TOTAL {
            return Some(Err(LimitReached));
        }
        map.insert(
            id,
            Handoff {
                purpose,
                token_hash,
                account_id,
                device_id,
                device_name: device_name.to_owned(),
                label,
                match_code,
                rp_state: None,
                created: now,
                expires,
                code_attempts: 0,
                finish_attempts: 0,
                state: State::Waiting,
            },
        );
        Some(Ok(Created {
            id,
            match_code,
            expires,
        }))
    }

    /// The state of a handoff for the session that created it.
    #[must_use]
    pub fn state(
        &self,
        id: &[u8; 16],
        token_hash: &[u8; 32],
        now: u64,
    ) -> Option<(State, Purpose)> {
        let map = self.lock();
        let h = map.get(id)?;
        if h.token_hash.ct_eq(token_hash).unwrap_u8() != 1 {
            return None;
        }
        let state = if h.state == State::Waiting && h.expires <= now {
            State::Expired
        } else {
            h.state
        };
        Some((state, h.purpose))
    }

    /// The account of a live handoff (to claim its failure budget before a
    /// code is looked at).
    #[must_use]
    pub fn account_of(&self, id: &[u8; 16], now: u64) -> Option<[u8; 16]> {
        let map = self.lock();
        map.get(id).filter(|h| h.live(now)).map(|h| h.account_id)
    }

    /// Checks the typed code in constant time. A wrong code is counted; the
    /// fifth makes the handoff `Failed`.
    ///
    /// # Errors
    /// [`Refused`].
    pub fn verify_code(&self, id: &[u8; 16], code: &[u8], now: u64) -> Result<Verified, Refused> {
        let mut map = self.lock();
        let h = map.get_mut(id).ok_or(Refused::Expired)?;
        if h.state == State::Waiting && h.expires <= now {
            h.state = State::Expired;
        }
        match h.state {
            State::Waiting => {}
            State::Expired => return Err(Refused::Expired),
            State::Done | State::Failed => return Err(Refused::Closed),
        }
        let right = code.len() == 6 && h.match_code.as_slice().ct_eq(code).unwrap_u8() == 1;
        if !right {
            h.code_attempts = h.code_attempts.saturating_add(1);
            if h.code_attempts >= CODE_ATTEMPTS {
                h.state = State::Failed;
                h.rp_state = None;
            }
            return Err(Refused::WrongCode);
        }
        Ok(Verified {
            purpose: h.purpose,
            account_id: h.account_id,
            device_name: h.device_name.clone(),
            created: h.created,
        })
    }

    /// Stores the ceremony state of a handoff (replacing an earlier one: a new
    /// challenge makes the old one useless).
    pub fn set_state(&self, id: &[u8; 16], rp_state: CeremonyState, now: u64) -> bool {
        let mut map = self.lock();
        match map.get_mut(id) {
            Some(h) if h.live(now) => {
                h.rp_state = Some(rp_state);
                true
            }
            _ => false,
        }
    }

    /// Takes the ceremony state out for one finish attempt: single use, and at
    /// most five attempts per handoff.
    ///
    /// # Errors
    /// [`Refused`].
    pub fn take_for_finish(&self, id: &[u8; 16], now: u64) -> Result<Taken, Refused> {
        let mut map = self.lock();
        let h = map.get_mut(id).ok_or(Refused::Expired)?;
        if h.state == State::Waiting && h.expires <= now {
            h.state = State::Expired;
        }
        match h.state {
            State::Waiting => {}
            State::Expired => return Err(Refused::Expired),
            State::Done | State::Failed => return Err(Refused::Closed),
        }
        h.finish_attempts = h.finish_attempts.saturating_add(1);
        if h.finish_attempts >= FINISH_ATTEMPTS {
            h.state = State::Failed;
        }
        let rp_state = h.rp_state.take().ok_or(Refused::NoState)?;
        Ok(Taken {
            purpose: h.purpose,
            token_hash: h.token_hash,
            account_id: h.account_id,
            device_id: h.device_id,
            label: h.label.clone(),
            rp_state,
        })
    }

    /// Marks the handoff done.
    pub fn complete(&self, id: &[u8; 16]) {
        if let Some(h) = self.lock().get_mut(id) {
            h.state = State::Done;
            h.rp_state = None;
        }
    }

    /// Ends every handoff of an account (a password change, a re-key).
    pub fn drop_account(&self, account: &[u8; 16]) {
        self.lock().retain(|_, h| &h.account_id != account);
    }

    /// Ends the handoffs bound to one session (a logout).
    pub fn drop_session(&self, token_hash: &[u8; 32]) {
        self.lock().retain(|_, h| &h.token_hash != token_hash);
    }

    /// Drops handoffs that ended a minute ago or earlier.
    pub fn prune(&self, now: u64) {
        self.lock()
            .retain(|_, h| h.expires.saturating_add(60_000) > now);
    }
}

/// Six decimal digits, uniform over `000000` to `999999`, as ASCII.
fn random_code() -> Option<[u8; 6]> {
    let mut out = [b'0'; 6];
    for d in &mut out {
        // Rejection sampling: 250 is the largest multiple of 10 below 256.
        loop {
            let mut b = [0u8; 1];
            getrandom::fill(&mut b).ok()?;
            let v = b.first().copied().unwrap_or(255);
            if v < 250 {
                *d = b'0' + (v % 10);
                break;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(account: [u8; 16]) -> NewHandoff<'static> {
        NewHandoff {
            purpose: Purpose::Login,
            token_hash: [1; 32],
            account_id: account,
            device_id: [4; 16],
            device_name: "d",
            label: None,
        }
    }

    fn make(h: &Handoffs, now: u64) -> Created {
        h.create(now, new([2; 16]), u64::MAX).unwrap().unwrap()
    }

    #[test]
    fn codes_are_six_digits() {
        let h = Handoffs::new();
        for _ in 0..50 {
            let c = random_code().unwrap();
            assert!(c.iter().all(u8::is_ascii_digit));
        }
        assert_eq!(make(&h, 0).match_code.len(), 6);
    }

    #[test]
    fn five_wrong_codes_fail_the_handoff() {
        let h = Handoffs::new();
        let c = make(&h, 0);
        for _ in 0..4 {
            assert_eq!(
                h.verify_code(&c.id, b"abcdef", 1).err(),
                Some(Refused::WrongCode)
            );
        }
        assert_eq!(
            h.verify_code(&c.id, b"abcdef", 1).err(),
            Some(Refused::WrongCode)
        );
        assert!(matches!(
            h.verify_code(&c.id, &c.match_code, 1),
            Err(Refused::Closed)
        ));
        assert_eq!(h.state(&c.id, &[1; 32], 1).unwrap().0, State::Failed);
        assert!(
            h.state(&c.id, &[9; 32], 1).is_none(),
            "bound to its session"
        );
    }

    #[test]
    fn at_most_three_live_per_account_and_expiry() {
        let h = Handoffs::new();
        for _ in 0..3 {
            make(&h, 0);
        }
        assert!(h.create(0, new([2; 16]), u64::MAX).unwrap().is_err());
        // Another account is unaffected; after expiry the slots are free.
        assert!(h.create(0, new([3; 16]), u64::MAX).unwrap().is_ok());
        assert!(
            h.create(HANDOFF_TTL_MS, new([2; 16]), u64::MAX)
                .unwrap()
                .is_ok()
        );
    }

    #[test]
    fn expiry_is_capped_by_the_pending_session() {
        let h = Handoffs::new();
        let c = h.create(0, new([2; 16]), 1_000).unwrap().unwrap();
        assert_eq!(c.expires, 1_000);
        assert_eq!(h.state(&c.id, &[1; 32], 1_000).unwrap().0, State::Expired);
    }
}

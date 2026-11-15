//! Grants (agent access design §9.3): what the user allowed, in memory only.
//!
//! A grant is bound to (pairing, vault, item, binding) and to the binding's digest, so a changed
//! policy needs a new approval. It ends at its expiry, at lock, on revoke, on unpairing and when
//! agent access is turned off. A grant with a deadline ends at the **earlier** of a monotonic
//! deadline and a wall-clock deadline: a machine that sleeps pauses the monotonic clock, and the
//! wall clock then ends the grant (review focus 1).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mosaic_core::model::ElementId;
use mosaic_core::{ItemId, VaultId};
use serde::{Deserialize, Serialize};

use crate::engine::Now;

/// Requests a grant allows per minute.
pub const REQUESTS_PER_MINUTE: usize = 60;

/// What a pairing may do with a secret right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GrantState {
    /// Nothing is granted.
    None,
    /// One request.
    Once,
    /// Until this wall-clock time (ms since the Unix epoch).
    Until(u64),
    /// Until the vault locks.
    UntilLock,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Expiry {
    Once,
    Until { mono: Instant, wall_ms: u64 },
    UntilLock,
}

impl Expiry {
    pub(crate) fn after(duration: Duration, now: Now) -> Self {
        let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        Self::Until {
            mono: now.mono + duration,
            wall_ms: now.wall_ms.saturating_add(ms),
        }
    }
}

pub(crate) struct Grant {
    pub(crate) id: u64,
    pub(crate) pairing: [u8; 16],
    pub(crate) vault_id: VaultId,
    pub(crate) item_id: ItemId,
    pub(crate) element_id: ElementId,
    pub(crate) digest: [u8; 32],
    pub(crate) expiry: Expiry,
    /// Monotonic times of this grant's requests in the last minute.
    uses: VecDeque<Instant>,
}

impl Grant {
    pub(crate) fn live(&self, now: Now) -> bool {
        match self.expiry {
            Expiry::Once | Expiry::UntilLock => true,
            Expiry::Until { mono, wall_ms } => now.mono < mono && now.wall_ms < wall_ms,
        }
    }

    pub(crate) fn state(&self) -> GrantState {
        match self.expiry {
            Expiry::Once => GrantState::Once,
            Expiry::Until { wall_ms, .. } => GrantState::Until(wall_ms),
            Expiry::UntilLock => GrantState::UntilLock,
        }
    }

    fn is_for(
        &self,
        pairing: &[u8; 16],
        vault: VaultId,
        item: ItemId,
        element: &ElementId,
    ) -> bool {
        self.pairing == *pairing
            && self.vault_id == vault
            && self.item_id == item
            && self.element_id == *element
    }

    /// Counts a request now; `false` when the minute's budget is spent.
    pub(crate) fn take_use(&mut self, now: Now) -> bool {
        let minute = Duration::from_secs(60);
        while self
            .uses
            .front()
            .is_some_and(|t| now.mono.saturating_duration_since(*t) >= minute)
        {
            self.uses.pop_front();
        }
        if self.uses.len() >= REQUESTS_PER_MINUTE {
            return false;
        }
        self.uses.push_back(now.mono);
        true
    }
}

#[derive(Default)]
pub(crate) struct Grants {
    next_id: u64,
    list: Vec<Grant>,
}

/// What the lookup found for a key.
pub(crate) enum Found {
    /// A live grant with the binding's current digest, at this index.
    Live(usize),
    /// A live grant exists, but for another digest: the policy changed.
    Stale,
    /// Nothing.
    Missing,
}

impl Grants {
    /// Drops grants that expired.
    pub(crate) fn sweep(&mut self, now: Now) {
        self.list.retain(|g| g.live(now));
    }

    pub(crate) fn find(
        &self,
        pairing: &[u8; 16],
        vault: VaultId,
        item: ItemId,
        element: &ElementId,
        digest: &[u8; 32],
        now: Now,
    ) -> Found {
        let mut stale = false;
        for (i, g) in self.list.iter().enumerate() {
            if g.is_for(pairing, vault, item, element) && g.live(now) {
                if g.digest == *digest {
                    return Found::Live(i);
                }
                stale = true;
            }
        }
        if stale { Found::Stale } else { Found::Missing }
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut Grant> {
        self.list.get_mut(index)
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Grant> {
        self.list.get(index)
    }

    pub(crate) fn remove_at(&mut self, index: usize) -> Option<Grant> {
        (index < self.list.len()).then(|| self.list.remove(index))
    }

    /// Adds a grant, replacing any other grant for the same key.
    pub(crate) fn insert(
        &mut self,
        pairing: [u8; 16],
        vault_id: VaultId,
        item_id: ItemId,
        element_id: ElementId,
        digest: [u8; 32],
        expiry: Expiry,
    ) -> u64 {
        self.list
            .retain(|g| !g.is_for(&pairing, vault_id, item_id, &element_id));
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.list.push(Grant {
            id,
            pairing,
            vault_id,
            item_id,
            element_id,
            digest,
            expiry,
            uses: VecDeque::new(),
        });
        id
    }

    pub(crate) fn remove(&mut self, id: u64) -> bool {
        let before = self.list.len();
        self.list.retain(|g| g.id != id);
        self.list.len() != before
    }

    pub(crate) fn remove_pairing(&mut self, pairing: &[u8; 16]) -> usize {
        let before = self.list.len();
        self.list.retain(|g| g.pairing != *pairing);
        before - self.list.len()
    }

    pub(crate) fn clear(&mut self) -> usize {
        let n = self.list.len();
        self.list.clear();
        n
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Grant> {
        self.list.iter()
    }
}

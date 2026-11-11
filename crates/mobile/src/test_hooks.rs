//! Test hooks (feature `test-hooks`): local account creation with the
//! Baseline profile, a settable clock offset and a work delay that makes the
//! races of the tests deterministic. Built only for the host tests, the JVM
//! unit tests, the instrumented tests and the `XCTest` builds;
//! `scripts/mobile/check-no-test-hooks.sh` keeps it out of release libraries.

use std::sync::atomic::Ordering;

use mosaic_core::{Argon2Profile, MasterPassword};
use mosaic_vault::storage::{SyncConfig, SyncKind};
use zeroize::Zeroizing;

use crate::Core;
use crate::error::{MobileError, Result};
use crate::state::SessionState;
use crate::storage::secret_key_name;
use crate::unlock::Taken;

/// The hooks of one core.
pub struct TestHooks<'a> {
    core: &'a Core,
}

impl Core {
    /// The test hooks of this core.
    #[must_use]
    pub const fn test_hooks(&self) -> TestHooks<'_> {
        TestHooks { core: self }
    }
}

impl TestHooks<'_> {
    /// Creates an account on this phone without a server (Argon2id profile
    /// Baseline), with one vault "Personal", and unlocks it. The Secret Key goes
    /// into secure storage as after a join. Returns the Secret Key's display
    /// form.
    ///
    /// # Errors
    /// `accountExists`, `busy`, `storageUnavailable`, `internal`.
    pub fn create_local_account(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(SessionState, Zeroizing<String>)> {
        let core = self.core;
        let locked = match core.take_locked()? {
            Taken::NoAccount(ls) => ls,
            other @ Taken::Locked(_) => {
                core.put_back(other);
                return Err(MobileError::AccountExists);
            }
        };
        let created = locked.create_account(
            &MasterPassword::new(password.to_owned()),
            Argon2Profile::Baseline,
        );
        let (mut session, created) = match created {
            Ok(ok) => ok,
            Err(failed) => {
                core.put_back(Taken::NoAccount(failed.session));
                return Err(failed.error.into());
            }
        };
        let account = session.account_id();
        session.set_sync_config(Some(&SyncConfig {
            kind: SyncKind::Hosted,
            server_url: Some("https://sync.invalid".to_owned()),
            email: Some(email.to_owned()),
            folder_path: None,
            device_name: "Test phone".to_owned(),
        }))?;
        session.create_vault("Personal")?;
        core.storage.put(
            &secret_key_name(&account),
            created.secret_key.expose_secret(),
        )?;
        core.record_boot(&mut session);
        let display = Zeroizing::new(created.secret_key.to_display()?.expose_secret().to_owned());
        Ok((core.finish_unlocked(session)?, display))
    }

    /// Moves the core's clock.
    pub fn set_clock_offset_ms(&self, offset: i64) {
        self.core.clock.set_offset_ms(offset);
    }

    /// Makes every long call sleep this long before it takes the core's mutex
    /// again.
    pub fn set_work_delay_ms(&self, ms: u64) {
        self.core.work_delay_ms.store(ms, Ordering::SeqCst);
    }
}

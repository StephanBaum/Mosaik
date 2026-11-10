//! The run context and the unlock rules: store access under the exclusive lock (ADR-0071), the
//! session cache (CRYPTO_SPEC §6.8, ADR-0072) and the password path.
//!
//! - Every invocation first sweeps expired session parts (review decision H1).
//! - A session is opened from the keystore key and the file; any `CliSessionRejected` deletes both
//!   parts (a `Store` error keeps them). A session is created only by `unlock`, right after a
//!   master-password unlock in this process (decision L3).

use std::path::PathBuf;

use mosaic_core::{AccountId, MasterPassword, SecretKey};
use mosaic_vault::session::{Error as VaultError, LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;

use crate::backends::{self, Backends};
use crate::error::{CliError, EXIT_AUTH, EXIT_NOT_FOUND, Result};
use crate::i18n::Msg;
use crate::input::{self, Prompter};
use crate::output::Output;
use crate::spec::Parsed;

/// Which store the command uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// The desktop's store.
    Shared,
    /// The CLI's own device.
    Own,
}

impl Profile {
    /// The name on the command line and in `status`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Own => "own",
        }
    }
}

/// Where the master password comes from.
#[derive(Clone, Copy, Debug)]
pub enum PasswordSource {
    /// The terminal prompt.
    Prompt,
    /// One line of stdin.
    Stdin,
    /// One line of a file descriptor.
    Fd(i32),
}

/// What a command runs with.
pub struct App {
    /// Output and language.
    pub out: Output,
    /// The OS parts.
    pub be: Backends,
    /// Prompts.
    pub prompt: Prompter,
    /// The store file.
    pub store_path: PathBuf,
    /// The profile.
    pub profile: Profile,
    /// Unix milliseconds at the start of the run.
    pub now: u64,
}

/// An unlocked store.
pub struct Unlocked {
    /// The session.
    pub session: UnlockedSession,
    /// The account.
    pub account: AccountId,
}

impl App {
    /// Builds the context from the parsed command line.
    ///
    /// # Errors
    /// A usage error for an unknown `--profile`; `noStore`-class errors come later.
    pub fn new(parsed: &Parsed, out: Output) -> Result<Self> {
        let profile = match parsed.value("profile") {
            None | Some("shared") => Profile::Shared,
            Some("own") => Profile::Own,
            Some(_) => {
                return Err(CliError::usage(Msg::ErrInvalidValue).with("name", "--profile"));
            }
        };
        let store_path = if let Some(p) = parsed.value("store") {
            PathBuf::from(p)
        } else if let Some(p) = std::env::var_os("MOSAIC_STORE").filter(|p| !p.is_empty()) {
            PathBuf::from(p)
        } else {
            let found = match profile {
                Profile::Shared => mosaic_platform::paths::desktop_store(),
                Profile::Own => mosaic_platform::paths::cli_store(),
            };
            found.map_err(|_| CliError::internal())?
        };
        let be = backends::open();
        let prompt = Prompter::new(backends::terminal(), parsed.has("no-input"), out.lang);
        Ok(Self {
            out,
            be,
            prompt,
            store_path,
            profile,
            now: backends::now_ms(),
        })
    }

    /// Deletes the session parts whose expiry has passed. Never fails the command.
    pub fn sweep(&self) {
        if let Ok(sessions) = &self.be.sessions {
            let _ = sessions.sweep_expired(self.now);
        }
    }

    /// Opens the store file under the exclusive lock.
    ///
    /// # Errors
    /// `noStore` (4) when the file does not exist, `storeInUse` (5) when another process holds it.
    pub fn open_store(&self) -> Result<LockedSession> {
        if !self.store_path.exists() {
            return Err(CliError::new("noStore", EXIT_NOT_FOUND, Msg::ErrNoStore)
                .with("path", self.store_path.display().to_string()));
        }
        Ok(LockedSession::open(Store::open(&self.store_path)?))
    }

    /// Opens the store and unlocks it: from a valid session if there is one, else with the master
    /// password.
    ///
    /// # Errors
    /// As [`App::open_store`], plus `wrongCredentials` and `inputRequired` (3).
    pub fn unlock(&mut self) -> Result<Unlocked> {
        let locked = self.open_store()?;
        let account = locked.account_id()?;
        let locked = match self.try_session(locked, account) {
            Ok(unlocked) => return Ok(unlocked),
            Err(locked) => locked,
        };
        self.unlock_with_password(locked, PasswordSource::Prompt, false)
    }

    /// Tries the cached session; gives the locked store back when there is none or it is refused.
    fn try_session(
        &self,
        locked: LockedSession,
        account: AccountId,
    ) -> core::result::Result<Unlocked, LockedSession> {
        let Ok(sessions) = &self.be.sessions else {
            return Err(locked);
        };
        let Ok(install) = locked.store().install_id() else {
            return Err(locked);
        };
        let slot = session_slot(account, install);
        let Ok(Some((key, file))) = sessions.load(slot) else {
            return Err(locked);
        };
        match locked.unlock_cli_session(&key, &file, self.now) {
            Ok(session) => Ok(Unlocked { session, account }),
            Err(failed) => {
                // Every refusal of §6.8 deletes both parts; a store error keeps them.
                if matches!(failed.error, VaultError::CliSessionRejected) {
                    let _ = sessions.delete(slot);
                    self.out.warn("sessionExpired", Msg::ErrSessionExpired, &[]);
                }
                Err(failed.session)
            }
        }
    }

    /// Unlocks with the master password (and the Secret Key from the keystore, else a prompt).
    ///
    /// # Errors
    /// `wrongCredentials`, `secretKeyInvalid` and `inputRequired` (3).
    pub fn unlock_with_password(
        &mut self,
        locked: LockedSession,
        source: PasswordSource,
        force_secret_key_prompt: bool,
    ) -> Result<Unlocked> {
        let account = locked.account_id()?;
        let stored = if force_secret_key_prompt {
            None
        } else {
            self.be.keys.load(account).ok().flatten()
        };
        let secret_key = if let Some(key) = stored {
            key
        } else {
            let typed = self.prompt.secret(Msg::PromptSecretKey, &[])?;
            SecretKey::parse(&typed).map_err(|_| {
                CliError::new("secretKeyInvalid", EXIT_AUTH, Msg::ErrSecretKeyInvalid)
            })?
        };
        let mut typed = match source {
            PasswordSource::Prompt => self.prompt.secret(Msg::PromptPassword, &[])?,
            PasswordSource::Stdin => input::read_secret_line(&mut std::io::stdin().lock())?,
            PasswordSource::Fd(fd) => input::read_secret_fd(fd)?,
        };
        let password = MasterPassword::new(std::mem::take(&mut *typed));
        match locked.unlock(&password, &secret_key) {
            Ok(session) => Ok(Unlocked { session, account }),
            Err(failed) => Err(failed.error.into()),
        }
    }
}

/// The slot a session lives in: the account and the install together, so the `shared` and the `own`
/// store of one account (two installs) each keep their own session and never delete each other's.
/// An identifier, not a secret: the first 16 bytes of SHA-256 over a fixed label, the account ID and
/// the install ID, shaped as an account ID for the platform's session store.
#[must_use]
pub fn session_slot(account: AccountId, install: mosaic_core::InstallId) -> AccountId {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::new()
        .chain_update(b"mosaic/cli/session-slot")
        .chain_update(account.as_bytes())
        .chain_update(install.as_bytes())
        .finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(digest.get(..16).unwrap_or(&[1; 16]));
    // The nil ID is invalid; a collision with it is astronomically unlikely but costs nothing to avoid.
    AccountId::from_bytes(bytes).unwrap_or(account)
}

/// Refuses while a key rotation job is open (exit 10).
///
/// # Errors
/// `rotationInProgress`.
pub fn guard_rotation(session: &UnlockedSession) -> Result<()> {
    if session.rotation_job()?.is_some() {
        return Err(CliError::rotation());
    }
    Ok(())
}

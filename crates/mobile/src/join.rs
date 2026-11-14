//! Joining a hosted account from a phone (CRYPTO_SPEC §9.2 to §9.3, ADR-0091),
//! following the CLI pattern: pre-login, the join keys, an OPAQUE login, a
//! second factor if the account has one, the account fetch, and
//! `LockedSession::join_account_with_keys_at`.
//!
//! The pending join holds **no store**: the iOS app can close the store while
//! the user is in Safari for the security-key handoff. It lives five minutes
//! and is dropped on expiry or `join_cancel()`.

use mosaic_core::{MasterPassword, SecretKey};
use mosaic_sync::http::{Token as _, canonical_origin, hosted};
use mosaic_sync::transport::TransportError;
use mosaic_sync::wire::json::{HandoffState, LoginStatus};
use mosaic_vault::storage::{SyncConfig, SyncKind};
use zeroize::Zeroizing;

use crate::Core;
use crate::config::Platform;
use crate::error::{MobileError, Result};
use crate::secret::text_of;
use crate::state::{Phase, SessionState, UnlockKind};
use crate::storage::{secret_key_name, sync_token_name};
use crate::unlock::Taken;

/// How long a pending join lives.
const JOIN_TTL_MS: u64 = 300_000;

/// Plain `http` to a loopback host is for the host tests and the developer
/// builds only (G1-17): a release build of the phone refuses it, so a typed
/// address or a setup code cannot send the master-password login over plain
/// text. `canonical_origin` itself still accepts loopback `http` for the
/// desktop and the CLI.
pub(crate) const ALLOW_HTTP_LOOPBACK: bool = cfg!(any(debug_assertions, feature = "test-hooks"));

/// `origin` (canonical) as the phone accepts it: `https`, and loopback `http`
/// only where `allow_loopback` says so.
///
/// # Errors
/// `syncServerInsecure`.
pub(crate) fn check_phone_origin(origin: String, allow_loopback: bool) -> Result<String> {
    if !allow_loopback && !origin.starts_with("https://") {
        return Err(MobileError::SyncServerInsecure);
    }
    Ok(origin)
}

/// The canonical origin of a server address typed or scanned on the phone.
///
/// # Errors
/// `syncServerInvalid`, `syncServerInsecure`.
pub(crate) fn phone_origin(server_url: &str) -> Result<String> {
    check_phone_origin(canonical_origin(server_url)?, ALLOW_HTTP_LOOPBACK)
}

/// The server as pre-login sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerInfo {
    /// The canonical origin.
    pub origin: String,
    /// The server can run the security-key handoff.
    pub webauthn: bool,
}

/// What the person typed (or scanned) to join.
pub struct JoinHostedRequest {
    /// The server address.
    pub server_url: String,
    /// The account email.
    pub email: String,
    /// The master password, UTF-8.
    pub password: Vec<u8>,
    /// The Secret Key in its display form, UTF-8.
    pub secret_key: Vec<u8>,
    /// The name of this phone in the device list (default "Android phone" or
    /// "iPhone").
    pub device_name: Option<String>,
}

/// The second factors an account offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecondFactorMethods {
    /// An authenticator-app code.
    pub totp: bool,
    /// A security key through the browser handoff.
    pub webauthn: bool,
}

/// How a join went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinOutcome {
    /// Joined and unlocked.
    Unlocked(SessionState),
    /// The account needs a second factor ([`Core::join_totp`] or
    /// [`Core::join_webauthn_begin`]).
    SecondFactor {
        /// What the account offers.
        methods: SecondFactorMethods,
    },
}

/// A started security-key handoff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandoffInfo {
    /// The page to open in the system browser.
    pub url: String,
    /// The code the user types on that page.
    pub match_code: String,
    /// When the handoff ends (server clock, Unix ms).
    pub expires_at_ms: u64,
}

/// Where a handoff stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandoffStatus {
    /// Still waiting for the key.
    Waiting,
    /// Done: the phone joined.
    Done(SessionState),
    /// The key was refused.
    Failed,
    /// The handoff ran out.
    Expired,
}

/// A join that waits for its second factor.
pub(crate) struct PendingJoin {
    origin: String,
    email: String,
    device_name: String,
    login: hosted::JoinLogin,
    secret_key: SecretKey,
    expires_at_ms: u64,
    handoff: Option<[u8; 16]>,
}

impl PendingJoin {
    /// Whether the join ran out at `now_ms`.
    pub(crate) const fn expired(&self, now_ms: u64) -> bool {
        now_ms > self.expires_at_ms
    }
}

fn default_device_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Android => "Android phone",
        Platform::Ios => "iPhone",
    }
}

impl Core {
    /// Checks a server address: its canonical origin, its protocol and
    /// whether it offers security keys (`GET /v1/info`).
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `syncServerInvalid`,
    /// `syncServerInsecure`, `syncServerUnreachable`, `syncServerMismatch`,
    /// `syncServerIncompatible`.
    pub fn prelogin_check(&self, server_url: String) -> Result<ServerInfo> {
        self.require_app()?;
        let origin = phone_origin(&server_url)?;
        let features = hosted::info_features(&origin)?;
        Ok(ServerInfo {
            origin,
            webauthn: features.webauthn,
        })
    }

    /// Joins a hosted account: the first half runs OPAQUE and Argon2id.
    ///
    /// # Errors
    /// `invalidInput` (the extension context, or a bad email), `accountExists`,
    /// `emptyPassword`, `secretKeyMalformed`, `secretKeyMistyped`,
    /// `wrongCredentials`, `busy`, `storageUnavailable`, and the server codes.
    pub fn join_hosted(&self, request: JoinHostedRequest) -> Result<JoinOutcome> {
        self.require_app()?;
        let JoinHostedRequest {
            server_url,
            email,
            password,
            secret_key,
            device_name,
        } = request;
        let mut password = text_of(password)?;
        let secret_key = SecretKey::parse(&text_of(secret_key)?)?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        let origin = phone_origin(&server_url)?;
        let email =
            mosaic_sync::wire::email::normalize(&email).map_err(|_| MobileError::InvalidInput)?;
        let device_name = device_name
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty() && n.chars().count() <= 64)
            .unwrap_or_else(|| default_device_name(self.config.platform).to_owned());
        // The store must be empty; a join over an account is refused.
        if matches!(self.guard()?.phase, Phase::Locked(_) | Phase::Unlocked(_)) {
            return Err(MobileError::AccountExists);
        }
        let taken = self.take_locked()?;
        let locked = match taken {
            Taken::NoAccount(ls) => ls,
            other @ Taken::Locked(_) => {
                self.put_back(other);
                return Err(MobileError::AccountExists);
            }
        };
        let install = locked.store().install_id();
        let install = match install {
            Ok(i) => i,
            Err(e) => {
                self.put_back(Taken::NoAccount(locked));
                return Err(e.into());
            }
        };
        let master = MasterPassword::new(std::mem::take(&mut *password));
        let login =
            hosted::join_login(&origin, &email, &master, &secret_key, install, &device_name);
        self.work_pause();
        // A `lock()` during this call stays requested when the join goes on
        // (no second factor, or one waiting): `finish_join` ends locked (N3).
        if login.is_ok() {
            self.put_back_keeping_lock(Taken::NoAccount(locked));
        } else {
            self.put_back(Taken::NoAccount(locked));
        }
        let login = login?;
        let mut pending = PendingJoin {
            origin,
            email,
            device_name,
            login,
            secret_key,
            expires_at_ms: self.now_ms().saturating_add(JOIN_TTL_MS),
            handoff: None,
        };
        if pending.login.grant.status == LoginStatus::TwoFactorRequired {
            let methods = hosted::login_methods(&pending.origin, &pending.login.grant.token)?;
            pending.handoff = None;
            self.guard()?.slots.join = Some(pending);
            return Ok(JoinOutcome::SecondFactor {
                methods: SecondFactorMethods {
                    totp: methods.totp,
                    webauthn: methods.webauthn,
                },
            });
        }
        self.finish_join(pending).map(JoinOutcome::Unlocked)
    }

    /// Takes the pending join; one that expired is dropped.
    fn take_pending_join(&self) -> Result<PendingJoin> {
        let now = self.now_ms();
        let pending = self
            .guard()?
            .slots
            .join
            .take()
            .ok_or(MobileError::NotFound)?;
        if pending.expired(now) {
            return Err(MobileError::SyncHandoffExpired);
        }
        Ok(pending)
    }

    /// Puts a pending join back after a failure that allows a retry.
    fn keep_pending_join(&self, pending: PendingJoin) {
        if let Ok(mut guard) = self.guard() {
            guard.slots.join = Some(pending);
        }
    }

    /// Finishes a join with an authenticator-app code.
    ///
    /// # Errors
    /// `notFound` (no join waits), `syncHandoffExpired`, `syncTwoFactorInvalid`
    /// (the join stays pending), `syncRateLimited`, and the codes of
    /// [`Core::join_hosted`].
    pub fn join_totp(&self, code: Vec<u8>) -> Result<SessionState> {
        self.require_app()?;
        let code = text_of(code)?;
        let pending = self.take_pending_join()?;
        match hosted::login_totp(&pending.origin, &pending.login.grant.token, code.trim()) {
            Ok(expires) => {
                let mut pending = pending;
                pending.login.grant.expires_at_ms = expires;
                self.finish_join(pending)
            }
            Err(e) => {
                if !matches!(e, TransportError::Unauthorized) {
                    self.keep_pending_join(pending);
                }
                Err(e.into())
            }
        }
    }

    /// Starts the security-key handoff: the platform opens the URL in the
    /// system browser and polls [`Core::join_webauthn_status`].
    ///
    /// # Errors
    /// `notFound`, `syncHandoffExpired`, `invalidInput` (the account has no
    /// key), `syncRateLimited`, and the server codes.
    pub fn join_webauthn_begin(&self) -> Result<HandoffInfo> {
        self.require_app()?;
        let mut pending = self.take_pending_join()?;
        let started = hosted::login_webauthn_handoff(&pending.origin, &pending.login.grant.token);
        let started = match started {
            Ok(s) => s,
            Err(e) => {
                self.keep_pending_join(pending);
                return Err(e.into());
            }
        };
        let info = hosted::handoff_page_url(&pending.origin, &started.handoff_id.0).map(|url| {
            HandoffInfo {
                url,
                match_code: started.match_code.clone(),
                expires_at_ms: started.expires_at,
            }
        });
        pending.handoff = Some(started.handoff_id.0);
        self.keep_pending_join(pending);
        Ok(info?)
    }

    /// Asks where the handoff stands. `Done` joins the phone.
    ///
    /// # Errors
    /// `notFound` (no handoff), `syncHandoffExpired`, and the server codes.
    pub fn join_webauthn_status(&self) -> Result<HandoffStatus> {
        self.require_app()?;
        let pending = self.take_pending_join()?;
        let Some(handoff) = pending.handoff else {
            self.keep_pending_join(pending);
            return Err(MobileError::NotFound);
        };
        let state =
            hosted::login_webauthn_status(&pending.origin, &pending.login.grant.token, handoff);
        match state {
            Ok(HandoffState::Waiting) => {
                self.keep_pending_join(pending);
                Ok(HandoffStatus::Waiting)
            }
            Ok(HandoffState::Done) => self.finish_join(pending).map(HandoffStatus::Done),
            Ok(HandoffState::Failed) => Ok(HandoffStatus::Failed),
            Ok(HandoffState::Expired) => Ok(HandoffStatus::Expired),
            Err(e) => {
                self.keep_pending_join(pending);
                Err(e.into())
            }
        }
    }

    /// Drops a pending join.
    pub fn join_cancel(&self) {
        if let Ok(mut guard) = self.guard() {
            guard.slots.join = None;
        }
    }

    /// The second half of a join: fetches the account, stores it, unlocks, and
    /// keeps the Secret Key and the session token in secure storage.
    fn finish_join(&self, pending: PendingJoin) -> Result<SessionState> {
        let PendingJoin {
            origin,
            email,
            device_name,
            login,
            secret_key,
            ..
        } = pending;
        let account = hosted::join_fetch_account(&origin, &login.grant.token, &login.keys)?;
        // The store may have been closed (iOS) while the user was in Safari.
        if matches!(self.guard()?.phase, Phase::Closed) {
            self.reopen_store()?;
        }
        let locked = match self.take_locked()? {
            Taken::NoAccount(ls) => ls,
            other @ Taken::Locked(_) => {
                self.put_back(other);
                return Err(MobileError::AccountExists);
            }
        };
        let hosted::JoinLogin { keys, grant } = login;
        let mut session =
            match locked.join_account_with_keys_at(&account.wrapped_ak, keys, account.ak_epoch) {
                Ok(session) => session,
                Err(failed) => {
                    self.put_back(Taken::NoAccount(failed.session));
                    return Err(failed.error.into());
                }
            };
        let config = SyncConfig {
            kind: SyncKind::Hosted,
            server_url: Some(origin),
            email: Some(email),
            folder_path: None,
            device_name,
        };
        let account_id = session.account_id();
        let token = Zeroizing::new(*grant.token.expose_bytes());
        let stored = session
            .set_sync_config(Some(&config))
            .map_err(MobileError::from)
            .and_then(|()| {
                self.storage
                    .put(&secret_key_name(&account_id), secret_key.expose_secret())
                    .map_err(MobileError::from)
            })
            .and_then(|()| {
                self.storage
                    .put(&sync_token_name(&account_id), token.as_slice())
                    .map_err(MobileError::from)
            });
        if let Err(e) = stored {
            // Without its Secret Key the phone's account is of no use: start over.
            drop(session.lock());
            let _ = self.storage.delete(&secret_key_name(&account_id));
            let _ = self.storage.delete(&sync_token_name(&account_id));
            self.note_stored(Some(false), Some(false));
            let _ = self.wipe_files();
            let (phase, busy) = crate::open_phase(&self.config).unwrap_or((Phase::Closed, false));
            if let Ok(mut guard) = self.guard() {
                guard.phase = phase;
                guard.store_busy = busy;
                // The fresh phase meets a lock that came meanwhile and honours
                // a close.
                guard.settle_fresh_phase();
            }
            return Err(e);
        }
        self.note_stored(Some(true), Some(true));
        self.record_boot(&mut session);
        self.finish_unlocked(session, UnlockKind::Password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_builds_refuse_loopback_http() {
        // G1-17: the rule a release build applies (`ALLOW_HTTP_LOOPBACK` is
        // false there), checked directly because the test build allows it.
        let loopback = canonical_origin("http://127.0.0.1:8080").unwrap();
        assert_eq!(
            check_phone_origin(loopback.clone(), false),
            Err(MobileError::SyncServerInsecure)
        );
        assert_eq!(check_phone_origin(loopback.clone(), true), Ok(loopback));
        let secure = canonical_origin("https://sync.example.com").unwrap();
        assert_eq!(check_phone_origin(secure.clone(), false), Ok(secure));
    }
}

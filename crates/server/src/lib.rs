//! Mosaic sync server: OPAQUE login, encrypted record storage and a
//! long-poll change feed (M6, ADR-0044, ADR-0046).
//!
//! The server never sees a master password, a key or a plaintext
//! (CRYPTO_SPEC §12). It stores what the clients upload as ciphertext and
//! checks only the structure of each object, never its content. IP addresses
//! live in memory for rate limiting and are never written to disk, and the
//! logs name the route and the status but no body, email, IP, token or ID.
//!
//! - [`app`]: the axum router over an [`AppState`];
//! - [`Config`] is `mosaic-server.toml`;
//! - [`cli`]: `init`, `serve`, `check-config`, `invite create`, `backup`,
//!   `account delete`.
#![forbid(unsafe_code)]

pub mod accept;
mod api;
mod auth;
mod blobs;
pub mod cli;
mod clock;
mod config;
mod db;
mod error;
mod fsperm;
mod polls;
mod ratelimit;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use mosaic_core::attachment::{MAX_ATTACHMENT_LEN, ciphertext_len};
use mosaic_sync::wire::json::RegistrationMode;
use mosaic_sync::wire::opaque::{Suite, ke::ServerSetup};

pub use auth::secret_file::{SecretFileError, ServerKeys};
/// The relying party, for the fuzz targets (not a stable API).
#[doc(hidden)]
pub use auth::webauthn as webauthn_rp;
pub use clock::{Clock, ManualClock, SystemClock};
pub use config::{Config, ConfigError, Limits, WebauthnConfig};
pub use db::{Db, DbError};
pub use error::ApiError;

use auth::handoff::Handoffs;
use auth::opaque::LoginStates;
use auth::prelogin::ProfileDist;
use auth::webauthn::Rp;
use blobs::Blobs;
use polls::Polls;
use ratelimit::RateLimiter;

/// The database file inside the data directory.
pub const DB_FILE: &str = "mosaic-server.sqlite";

/// Why the server could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StartError {
    /// The configuration is unusable.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The secret file is missing or damaged.
    #[error(transparent)]
    Secret(#[from] SecretFileError),
    /// The database could not be opened.
    #[error(transparent)]
    Db(#[from] DbError),
    /// A directory or the listener could not be set up.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// The shared state of the handlers.
pub struct Inner {
    pub(crate) db: Db,
    pub(crate) blobs: Blobs,
    pub(crate) keys: ServerKeys,
    pub(crate) origin: String,
    pub(crate) registration: RegistrationMode,
    pub(crate) trusted_proxies: Vec<std::net::IpAddr>,
    pub(crate) limits: Limits,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) rate: RateLimiter,
    pub(crate) logins: LoginStates,
    pub(crate) polls: Polls,
    pub(crate) rp: Option<Rp>,
    pub(crate) handoffs: Handoffs,
    pub(crate) dist: Mutex<Option<(u64, ProfileDist)>>,
    pub(crate) max_attachment: usize,
}

/// The application state; cheap to clone.
#[derive(Clone)]
pub struct AppState(Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AppState { .. }")
    }
}

impl AppState {
    /// State over an open database and keys. Blobs go under
    /// `config.data_dir/blobs`.
    #[must_use]
    pub fn new(config: &Config, db: Db, keys: ServerKeys, clock: Arc<dyn Clock>) -> Self {
        let rp = Rp::new(&config.canonical_origin, &config.webauthn);
        if rp.is_none()
            && let Ok(n) = db.run(db::webauthn::accounts_with_credentials)
            && n > 0
        {
            // Logins of these accounts fail closed (CRYPTO_SPEC §9.3).
            tracing::warn!(
                accounts = n,
                "accounts hold security keys but no WebAuthn relying party is configured; \n                 their logins cannot finish until [webauthn] is on and the origin is a host name, \n                 or their second factor is reset"
            );
        }
        Self(Arc::new(Inner {
            db,
            blobs: Blobs::new(&config.data_dir),
            keys,
            origin: config.canonical_origin.clone(),
            registration: config.registration,
            trusted_proxies: config.trusted_proxies.clone(),
            limits: config.limits.clone(),
            clock,
            rate: RateLimiter::new(),
            logins: LoginStates::new(),
            polls: Polls::new(),
            rp,
            handoffs: Handoffs::new(),
            dist: Mutex::new(None),
            max_attachment: ciphertext_len(MAX_ATTACHMENT_LEN).unwrap_or(usize::MAX),
        }))
    }

    /// Opens the database and the secret file named by `config`.
    ///
    /// # Errors
    /// [`StartError`].
    pub fn open(config: &Config, clock: Arc<dyn Clock>) -> Result<Self, StartError> {
        let keys = ServerKeys::load(&config.secret_file)?;
        let db = Db::open(&config.data_dir.join(DB_FILE))?;
        std::fs::create_dir_all(config.data_dir.join("blobs"))?;
        Ok(Self::new(config, db, keys, clock))
    }

    /// The database (tests and tools).
    #[must_use]
    pub fn db(&self) -> &Db {
        &self.0.db
    }

    /// The server's OPAQUE setup.
    pub(crate) fn setup(&self) -> &ServerSetup<Suite> {
        &self.0.keys.setup
    }

    /// The hourly housekeeping: expired sessions, old audit events,
    /// soft-deleted attachments past their retention (with their files),
    /// idle rate-limit buckets, stale logins and counters.
    ///
    /// # Errors
    /// Database or file errors (the next run retries).
    pub fn maintain(&self) -> Result<(), error::ApiError> {
        let now = self.clock.now_ms();
        let retention = self.limits.soft_delete_days.saturating_mul(86_400_000);
        let purged = self.db.run_tx(|tx| {
            db::sessions::purge_expired(tx, now)?;
            db::sessions::purge_revoked(tx, now.saturating_sub(30 * 86_400_000))?;
            db::audit::purge(tx, now.saturating_sub(365 * 86_400_000))?;
            db::sharing::purge_invites(tx, now.saturating_sub(60 * 86_400_000))?;
            db::attachments::purge_deleted(tx, now.saturating_sub(retention))
        })?;
        for id in purged {
            self.blobs.remove(&id)?;
        }
        self.rate.prune(now);
        self.logins.prune(now);
        self.polls.prune();
        self.handoffs.prune(now);
        Ok(())
    }
}

/// The HTTP application.
pub fn app(state: AppState) -> Router {
    api::router(state)
}

/// Serves until Ctrl-C or SIGTERM.
///
/// # Errors
/// [`StartError`].
pub async fn serve(config: Config) -> Result<(), StartError> {
    // The default panic hook prints the panic message; a handler panic must not
    // put request data into the log (CRYPTO_SPEC §9.3 "Panics").
    std::panic::set_hook(Box::new(|_| tracing::error!("a request handler panicked")));
    let state = AppState::open(&config, Arc::new(SystemClock))?;
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let housekeeping = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(3600));
        loop {
            tick.tick().await;
            if housekeeping.maintain().is_err() {
                tracing::error!("housekeeping failed");
            }
        }
    });
    tracing::info!("listening");
    accept::serve_listener(
        listener,
        app(state),
        shutdown(),
        accept::HEADER_READ_TIMEOUT,
        usize::try_from(config.limits.max_connections).unwrap_or(usize::MAX),
    )
    .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! {
                () = ctrl_c => {}
                _ = term.recv() => {}
            }
            return;
        }
    }
    ctrl_c.await;
}

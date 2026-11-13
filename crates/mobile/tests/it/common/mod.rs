//! Shared helpers of the integration tests: an in-memory `SecureStorage`, a
//! phone (a `Core` over a temporary directory), a "desktop" device of the same
//! account that talks to the real server on `127.0.0.1` (the harness of
//! `crates/sync/tests/it`), and the loopback server itself.

#![allow(dead_code, reason = "each test module uses a part of the helpers")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_core::{AccountId, Argon2Profile, InstallId, MasterPassword, SecretKey, VaultId};
use mosaic_mobile::{
    Core, CoreConfig, CoreContext, JoinHostedRequest, Platform, SecureStorage, SessionState,
    StorageError,
};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, OwnedSession, SyncEngine};
use mosaic_sync::http::{HttpTransport, hosted};
use mosaic_sync::transport::TransportKind;
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use zeroize::Zeroizing;

#[path = "../../../../server/tests/it/loopback.rs"]
pub mod loopback;

pub use loopback::LoopbackServer;

pub const PASSWORD: &str = "correct horse battery staple";
pub const BOOT: [u8; 4] = [0, 0, 0, 7];

/// An in-memory secure storage that can be made to fail.
#[derive(Default)]
pub struct MemStorage {
    pub map: Mutex<HashMap<String, Vec<u8>>>,
    pub fail: AtomicBool,
    /// Refuses only `put`; reads, presence checks and deletes keep working.
    pub fail_puts: AtomicBool,
    /// The name of every `delete` call, in order.
    pub deletes: Mutex<Vec<String>>,
    /// The name of every `get` (a value read), in order.
    pub gets: Mutex<Vec<String>>,
    /// The name of every `contains` (presence asked, nothing read).
    pub contains_calls: Mutex<Vec<String>>,
}

impl MemStorage {
    /// How many reads of a name starting with `prefix` happened.
    pub fn reads_of(&self, prefix: &str) -> usize {
        self.gets
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.starts_with(prefix))
            .count()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.map.lock().unwrap().keys().cloned().collect();
        names.sort();
        names
    }
}

impl SecureStorage for MemStorage {
    fn contains(&self, name: &str) -> Result<bool, StorageError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(StorageError);
        }
        self.contains_calls.lock().unwrap().push(name.to_owned());
        Ok(self.map.lock().unwrap().contains_key(name))
    }

    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StorageError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(StorageError);
        }
        self.gets.lock().unwrap().push(name.to_owned());
        Ok(self
            .map
            .lock()
            .unwrap()
            .get(name)
            .map(|v| Zeroizing::new(v.clone())))
    }

    fn put(&self, name: &str, value: &[u8]) -> Result<(), StorageError> {
        if self.fail.load(Ordering::SeqCst) || self.fail_puts.load(Ordering::SeqCst) {
            return Err(StorageError);
        }
        self.map
            .lock()
            .unwrap()
            .insert(name.to_owned(), value.to_vec());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), StorageError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(StorageError);
        }
        self.deletes.lock().unwrap().push(name.to_owned());
        self.map.lock().unwrap().remove(name);
        Ok(())
    }
}

pub fn config(dir: &std::path::Path, platform: Platform, context: CoreContext) -> CoreConfig {
    CoreConfig {
        store_dir: dir.join("mosaic"),
        platform,
        context,
        boot_id: BOOT.to_vec(),
        trust_anchors_der: Vec::new(),
    }
}

/// A phone: a core over a temporary directory with an in-memory storage.
pub struct Phone {
    pub dir: tempfile::TempDir,
    pub storage: Arc<MemStorage>,
    pub core: Core,
}

impl Phone {
    pub fn new() -> Self {
        Self::with(Platform::Ios, CoreContext::App)
    }

    pub fn android() -> Self {
        Self::with(Platform::Android, CoreContext::App)
    }

    pub fn with(platform: Platform, context: CoreContext) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(MemStorage::default());
        let core = Core::open(config(dir.path(), platform, context), storage.clone()).unwrap();
        Self { dir, storage, core }
    }

    /// Another core over the same directory and storage. The previous core
    /// closes its store first, so the store lock is free.
    pub fn reopen_as(&mut self, platform: Platform, context: CoreContext, boot: &[u8]) {
        self.core.close_store();
        let mut cfg = config(self.dir.path(), platform, context);
        cfg.boot_id = boot.to_vec();
        self.core = Core::open(cfg, self.storage.clone()).unwrap();
    }

    /// A second core over the same directory (the first one has closed its
    /// store).
    pub fn second_core(&self, platform: Platform, context: CoreContext, boot: &[u8]) -> Core {
        let mut cfg = config(self.dir.path(), platform, context);
        cfg.boot_id = boot.to_vec();
        Core::open(cfg, self.storage.clone()).unwrap()
    }

    pub fn store_path(&self) -> PathBuf {
        self.dir.path().join("mosaic").join("store.sqlite")
    }

    /// A phone with a local account (no server): unlocked, one vault.
    pub fn with_account() -> (Self, String) {
        Self::with_account_on(Platform::Ios)
    }

    /// As [`Phone::with_account`], on Android.
    pub fn android_with_account() -> (Self, String) {
        Self::with_account_on(Platform::Android)
    }

    pub fn with_account_on(platform: Platform) -> (Self, String) {
        let phone = Self::with(platform, CoreContext::App);
        let (_, sk) = phone
            .core
            .test_hooks()
            .create_local_account("user@example.com", PASSWORD)
            .unwrap();
        (phone, sk.to_string())
    }

    pub fn vault_id(&self) -> String {
        self.core.vaults().unwrap()[0].vault_id.clone()
    }

    pub fn join(&self, server: &LoopbackServer, email: &str, sk: &str) -> SessionState {
        server.advance_ms(60_000);
        match self
            .core
            .join_hosted(JoinHostedRequest {
                server_url: server.base.clone(),
                email: email.to_owned(),
                password: PASSWORD.as_bytes().to_vec(),
                secret_key: sk.as_bytes().to_vec(),
                device_name: None,
            })
            .unwrap()
        {
            mosaic_mobile::JoinOutcome::Unlocked(state) => state,
            other @ mosaic_mobile::JoinOutcome::SecondFactor { .. } => {
                panic!("unexpected second factor: {other:?}")
            }
        }
    }
}

/// A desktop-like device of the account: own store, registered on the server.
pub struct Desktop {
    pub dir: tempfile::TempDir,
    pub owned: OwnedSession,
    pub account: AccountId,
    pub install: InstallId,
    pub email: String,
    pub secret_key: SecretKey,
    pub password: String,
    pub base: String,
}

pub fn pw(s: &str) -> MasterPassword {
    MasterPassword::new(s.to_owned())
}

impl Desktop {
    pub fn register(server: &LoopbackServer, email: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("desktop.db")).unwrap();
        let install = store.install_id().unwrap();
        let (mut session, created) = LockedSession::open(store)
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
        let bootstrap = session.account_bootstrap().unwrap();
        let opw = session
            .opaque_password(&pw(PASSWORD), &created.secret_key)
            .unwrap();
        hosted::register(
            &server.base,
            email,
            None,
            &bootstrap,
            &opw,
            install,
            "Desktop",
        )
        .unwrap();
        let account = session.account_id();
        session.create_vault("Personal").unwrap();
        Self {
            dir,
            owned: OwnedSession::new(session),
            account,
            install,
            email: email.to_owned(),
            secret_key: created.secret_key,
            password: PASSWORD.to_owned(),
            base: server.base.clone(),
        }
    }

    pub fn sk_text(&self) -> String {
        self.secret_key
            .to_display()
            .unwrap()
            .expose_secret()
            .to_owned()
    }

    /// Runs `f` on the session.
    pub fn with<R>(&self, f: impl FnOnce(&mut mosaic_vault::session::UnlockedSession) -> R) -> R {
        use mosaic_sync::wire::SessionHandle as _;
        self.owned.with_session_mut(f).unwrap()
    }

    pub fn vault(&self) -> VaultId {
        self.with(|s| s.vaults()[0].vault_id)
    }

    pub fn add_login(
        &self,
        title: &str,
        username: &str,
        password: &str,
        url: &str,
    ) -> mosaic_core::ItemId {
        let vault = self.vault();
        self.with(|s| {
            s.add_item(vault, &mosaic_core::model::ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_text(mosaic_core::model::TextField::Username, username)?;
                e.set_secret(mosaic_core::model::SecretField::Password, password)?;
                e.add_url(&mosaic_core::model::LoginUrl::new(url))?;
                Ok(())
            })
            .unwrap()
        })
    }

    fn opw(&self) -> mosaic_core::OpaquePassword {
        self.with(|s| {
            s.opaque_password(&pw(&self.password), &self.secret_key)
                .unwrap()
        })
    }

    /// A fresh login and one sync run.
    pub fn sync(&self, server: &LoopbackServer) {
        server.advance_ms(60_000);
        let grant = hosted::login(
            &self.base,
            &self.email,
            self.account,
            &self.opw(),
            self.install,
            "Desktop",
        )
        .unwrap();
        let transport = HttpTransport::connect(&self.base, &grant.token, self.install).unwrap();
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
        let mut access = self.owned.clone();
        engine.sync_once(&mut access, &SystemClock).unwrap();
    }

    /// The server's TOTP code for a base32 secret, at the server's time.
    pub fn code_now(server: &LoopbackServer, secret_b32: &str) -> String {
        use mosaic_server::Clock as _;
        let otp = Otp {
            secret: OtpSecret::from_base32(secret_b32).unwrap(),
            algorithm: Algorithm::Sha1,
            kind: OtpKind::Totp {
                digits: Digits::DEFAULT,
                period: Period::DEFAULT,
            },
        };
        otp.code_at(server.clock.now_ms() / 1000)
            .unwrap()
            .expose_secret()
            .to_owned()
    }

    /// Turns the account's second factor (an authenticator app) on and returns
    /// the base32 secret.
    pub fn enable_totp(&self, server: &LoopbackServer) -> String {
        server.advance_ms(60_000);
        let grant = hosted::login(
            &self.base,
            &self.email,
            self.account,
            &self.opw(),
            self.install,
            "Desktop",
        )
        .unwrap();
        let setup = hosted::totp_setup(&self.base, &grant.token).unwrap();
        let secret = setup.secret.expose_secret().to_owned();
        let code = Self::code_now(server, &secret);
        hosted::totp_confirm(&self.base, &grant.token, &code).unwrap();
        secret
    }
}

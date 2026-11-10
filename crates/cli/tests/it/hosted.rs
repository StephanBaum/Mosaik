//! The hosted flows against a real `mosaic-server` on 127.0.0.1: `setup own-store --server`
//! (with and without a second factor), a wrong password, hosted `sync` between a desktop-style
//! store and the CLI's own store, and `recover` after a password change made elsewhere.
//!
//! The server runs in the test process (manual clock, in-memory database); the CLI is the built
//! binary. The desktop is played in-process with the same `mosaic-vault` and `mosaic-sync` calls
//! the desktop's hosted flow makes. The server clock only moves when a test moves it, so every
//! step advances it first (rate limits, one TOTP step per code).

use std::path::{Path, PathBuf};

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{AccountId, InstallId, MasterPassword, OpaquePassword, SecretKey};
use mosaic_server::Clock as _;
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, OwnedSession, SyncEngine};
use mosaic_sync::http::{HttpTransport, hosted};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::json::RegistrationMode;
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::{Store, SyncConfig, SyncKind};

#[path = "../../../server/tests/it/loopback.rs"]
#[expect(clippy::expect_used, reason = "the server tests' shared harness")]
mod loopback;

use crate::common::{Fixture, PASSWORD};
use loopback::LoopbackServer;

const NEW_PASSWORD: &str = "a new password chosen on the desktop";
const EMAIL: &str = "alice@example.com";

fn pw(text: &str) -> MasterPassword {
    MasterPassword::new(text.to_owned())
}

/// The desktop: the fixture store registered on the server and kept open in this process.
struct Desktop {
    server: LoopbackServer,
    f: Fixture,
    session: Option<UnlockedSession>,
    secret_key: SecretKey,
    opw: OpaquePassword,
    account: AccountId,
    install: InstallId,
    own: PathBuf,
}

impl Desktop {
    fn new() -> Self {
        Self::on(LoopbackServer::start())
    }

    /// On a server whose canonical origin names the host `localhost`: only such a server can run
    /// WebAuthn ceremonies (a relying party needs a host name), so only there does the account's
    /// security key count as a second factor.
    fn with_host_origin() -> Self {
        let mut server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
        let port = server.base.rsplit(':').next().unwrap().to_owned();
        server.base = format!("http://localhost:{port}");
        Self::on(server)
    }

    fn on(server: LoopbackServer) -> Self {
        let f = Fixture::new();
        let secret_key = SecretKey::parse(&f.secret_key).unwrap();
        let mut session = LockedSession::open(Store::open(&f.store).unwrap())
            .unlock(&pw(PASSWORD), &secret_key)
            .map_err(|e| e.error)
            .unwrap();
        let (account, install) = (session.account_id(), session.install_id());
        let opw = session.opaque_password(&pw(PASSWORD), &secret_key).unwrap();
        let boot = session.account_bootstrap().unwrap();
        hosted::register(&server.base, EMAIL, None, &boot, &opw, install, "desktop").unwrap();
        session
            .set_sync_config(Some(&SyncConfig {
                kind: SyncKind::Hosted,
                server_url: Some(server.base.clone()),
                email: Some(EMAIL.to_owned()),
                folder_path: None,
                device_name: "desktop".to_owned(),
            }))
            .unwrap();
        let own = f.tmp.path().join("own").join("vault.sqlite3");
        Self {
            server,
            f,
            session: Some(session),
            secret_key,
            opw,
            account,
            install,
            own,
        }
    }

    fn session(&mut self) -> &mut UnlockedSession {
        self.session.as_mut().unwrap()
    }

    /// A desktop sync over a fresh login (a minute of server time per call).
    fn sync(&mut self) {
        self.server.advance_ms(60_000);
        let grant = hosted::login(
            &self.server.base,
            EMAIL,
            self.account,
            &self.opw,
            self.install,
            "desktop",
        )
        .unwrap();
        let transport =
            HttpTransport::connect(&self.server.base, &grant.token, self.install).unwrap();
        let mut owned = OwnedSession::new(self.session.take().unwrap());
        let synced = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted))
            .sync_once(&mut owned, &SystemClock);
        self.session = owned.take();
        synced.unwrap();
    }

    fn add_login(&mut self, title: &str, password: &str) {
        let vault = self
            .session()
            .vaults()
            .into_iter()
            .find(|v| v.name.as_ref().is_some_and(|n| *n == "Personal"))
            .unwrap()
            .vault_id;
        self.session()
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_secret(SecretField::Password, password)
            })
            .unwrap();
    }

    fn own(&self) -> &str {
        self.own.to_str().unwrap()
    }

    fn url(&self) -> String {
        self.server.base.clone()
    }

    /// `setup own-store --server` with the given prompt lines.
    fn setup(&self, prompts: &[&str]) -> crate::common::Out {
        self.server.advance_ms(60_000);
        self.setup_now(prompts)
    }

    /// As `setup`, without moving the server clock (a typed code is valid for its time step).
    fn setup_now(&self, prompts: &[&str]) -> crate::common::Out {
        self.f
            .cmd(&[
                "--json",
                "setup",
                "own-store",
                "--server",
                &self.url(),
                "--email",
                EMAIL,
                "--store",
                self.own(),
            ])
            .no_store_flag()
            .prompts(prompts)
            .run()
    }

    fn cli(&self, args: &[&str], prompts: &[&str]) -> crate::common::Out {
        self.server.advance_ms(60_000);
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--store", self.own()]);
        self.f.cmd(&all).no_store_flag().prompts(prompts).run()
    }

    fn cli_password(&self, path: &str, field: &str, password: &str) -> String {
        let out = self.cli(&["get", path, "--field", field, "--reveal"], &[password]);
        assert_eq!(out.code, 0, "{}", out.stderr);
        out.json()["data"]["fields"]["login.password"]["value"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn token_files(&self) -> Vec<String> {
        files_with_prefix(&self.f.test_dir.join("keystore"), "sync-token-")
    }
}

fn files_with_prefix(dir: &Path, prefix: &str) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(prefix))
        .collect()
}

fn code_now(server: &LoopbackServer, secret_b32: &str) -> String {
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

/// Turns the account's second factor on; returns the base32 secret.
fn enable_totp(d: &Desktop) -> String {
    d.server.advance_ms(60_000);
    let grant = hosted::login(
        &d.server.base,
        EMAIL,
        d.account,
        &d.opw,
        d.install,
        "desktop",
    )
    .unwrap();
    let setup = hosted::totp_setup(&d.server.base, &grant.token).unwrap();
    let secret = setup.secret.expose_secret().to_owned();
    let code = code_now(&d.server, &secret);
    hosted::totp_confirm(&d.server.base, &grant.token, &code).unwrap();
    secret
}

#[test]
fn hosted_join_sync_and_recover() {
    let mut d = Desktop::new();
    d.sync();
    // The first device is the desktop-style store: a fresh CLI own store joins over the server.
    let joined = d.setup(&[d.f.secret_key.as_str(), PASSWORD]);
    assert_eq!(joined.code, 0, "{}", joined.stderr);
    assert!(d.own.exists());
    assert_eq!(d.token_files().len(), 1, "the session token is stored");
    // Joining replaced no session of the desktop's device and the CLI is not synced yet.
    let first = d.cli(&["sync"], &[PASSWORD]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert!(first.json()["data"]["pulled"].as_u64().unwrap() > 0);
    assert_eq!(
        d.cli_password("mosaic://Personal/GitHub", "password", PASSWORD),
        crate::common::GITHUB_PASSWORD
    );
    let status = d.cli(&["sync", "--status"], &[]);
    assert_eq!(status.json()["data"]["backend"], "hosted");

    // An item the desktop adds reaches the CLI, and the CLI's sync is idempotent.
    d.add_login("Added on desktop", "desktop-added-pw");
    d.sync();
    let pulled = d.cli(&["sync"], &[PASSWORD]);
    assert_eq!(pulled.code, 0, "{}", pulled.stderr);
    assert!(pulled.json()["data"]["pulled"].as_u64().unwrap() >= 1);
    assert_eq!(
        d.cli_password("mosaic://Personal/Added on desktop", "password", PASSWORD),
        "desktop-added-pw"
    );
    let again = d.cli(&["sync"], &[PASSWORD]);
    assert_eq!(again.json()["data"]["pulled"], 0);
    assert_eq!(again.json()["data"]["pushed"], 0);

    // The desktop (also a CLI-session holder via the shared store) has a CLI session, as does the
    // own store: both die with the recovery.
    d.session = None; // closes the desktop store so the CLI can open it for its session
    d.f.unlock("5m");
    let own_unlock =
        d.f.cmd(&[
            "unlock",
            "--password-stdin",
            "--ttl",
            "5m",
            "--store",
            d.own(),
        ])
        .no_store_flag()
        .stdin(PASSWORD)
        .run();
    assert_eq!(own_unlock.code, 0, "{}", own_unlock.stderr);
    assert_eq!(
        files_with_prefix(&d.f.test_dir.join("run"), "session-").len(),
        2
    );
    // The desktop changes the master password and the server takes it.
    let mut session = LockedSession::open(Store::open(&d.f.store).unwrap())
        .unlock(&pw(PASSWORD), &d.secret_key)
        .map_err(|e| e.error)
        .unwrap();
    let prepared = session
        .prepare_master_password_change(&pw(PASSWORD), &pw(NEW_PASSWORD), &d.secret_key)
        .unwrap();
    d.server.advance_ms(60_000);
    hosted::password_change(&d.server.base, EMAIL, &prepared, d.install, "desktop", None).unwrap();
    session.commit_master_password_change(prepared).unwrap();
    drop(session);

    // The CLI still opens with the old password until it recovers.
    let recovered = d.cli(&["recover"], &[NEW_PASSWORD]);
    assert_eq!(recovered.code, 0, "{}", recovered.stderr);
    assert_eq!(recovered.json()["data"]["recovered"], true);
    assert!(
        files_with_prefix(&d.f.test_dir.join("run"), "session-").is_empty(),
        "every CLI session is deleted"
    );
    assert!(
        files_with_prefix(&d.f.test_dir.join("keystore"), "session-").is_empty(),
        "and its keystore part"
    );
    let old = d.cli(&["list"], &[PASSWORD]);
    assert_eq!(old.code, 3, "the old password no longer opens the store");
    assert_eq!(
        d.cli_password("mosaic://Personal/GitHub", "password", NEW_PASSWORD),
        crate::common::GITHUB_PASSWORD
    );
    // The stored token works: a sync after the recovery needs no new sign-in.
    let synced = d.cli(&["sync"], &[NEW_PASSWORD]);
    assert_eq!(synced.code, 0, "{}", synced.stderr);
}

#[test]
fn recover_with_a_wrong_password_changes_nothing() {
    let mut d = Desktop::new();
    d.sync();
    assert_eq!(d.setup(&[d.f.secret_key.as_str(), PASSWORD]).code, 0);
    assert_eq!(d.cli(&["sync"], &[PASSWORD]).code, 0);
    let wrong = d.cli(
        &["recover"],
        &[
            PASSWORD,
            "not the account password",
            d.f.secret_key.as_str(),
        ],
    );
    assert_eq!(wrong.code, 3, "{}", wrong.stderr);
    // The local store is untouched: the original password still opens it.
    assert_eq!(
        d.cli_password("mosaic://Personal/GitHub", "password", PASSWORD),
        crate::common::GITHUB_PASSWORD
    );
}

#[test]
fn hosted_join_with_a_second_factor() {
    let d = Desktop::new();
    let secret = enable_totp(&d);

    // A wrong code stores nothing.
    d.server.advance_ms(60_000);
    let bad = d.setup_now(&[d.f.secret_key.as_str(), PASSWORD, "000000"]);
    assert_eq!(bad.code, 6, "{}", bad.stderr);
    assert!(!d.own.exists(), "a failed join leaves no store");
    assert_eq!(d.token_files().len(), 0);

    // The right one joins.
    d.server.advance_ms(60_000);
    let code = code_now(&d.server, &secret);
    let ok = d.setup_now(&[d.f.secret_key.as_str(), PASSWORD, &code]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert!(d.own.exists());
    assert_eq!(d.token_files().len(), 1);
}

/// Gives the account a security key (a stored row: the ceremony is tested in the server's own
/// tests; the CLI only asks which second factors the account has).
fn add_security_key(d: &Desktop) {
    d.server
        .state
        .db()
        .run(|c| {
            c.execute(
                "INSERT INTO webauthn_credentials (account_id, credential_id, public_key,                  sign_count, label, created_at)                  SELECT account_id, randomblob(16), randomblob(8), 0, 'YubiKey', 0 FROM accounts",
                [],
            )?;
            Ok::<_, mosaic_server::ApiError>(())
        })
        .unwrap();
}

#[test]
fn join_webauthn_only_exits_3_two_factor_method_unsupported() {
    let d = Desktop::with_host_origin();
    add_security_key(&d);
    // The CLI asks for no code and prints no link: it names the way out.
    let out = d.setup(&[d.f.secret_key.as_str(), PASSWORD]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert_eq!(
        out.error_json()["error"]["code"],
        "twoFactorMethodUnsupported"
    );
    assert!(out.stderr.contains("security key"), "{}", out.stderr);
    assert!(!out.stderr.contains("/webauthn/"), "no link is printed");
    assert!(!d.own.exists(), "a failed join leaves no store");
    assert_eq!(d.token_files().len(), 0);
    // The German text is in the catalog too.
    d.server.advance_ms(60_000);
    let de =
        d.f.cmd(&[
            "--lang",
            "de",
            "setup",
            "own-store",
            "--server",
            &d.url(),
            "--email",
            EMAIL,
            "--store",
            d.own(),
        ])
        .no_store_flag()
        .prompts(&[d.f.secret_key.as_str(), PASSWORD])
        .run();
    assert_eq!(de.code, 3, "{}", de.stderr);
    assert!(de.stderr.contains("Sicherheitsschlüssel"), "{}", de.stderr);
}

#[test]
fn join_with_totp_and_webauthn_prompts_for_code() {
    let d = Desktop::with_host_origin();
    let secret = enable_totp(&d);
    add_security_key(&d);
    d.server.advance_ms(60_000);
    let code = code_now(&d.server, &secret);
    // The code prompt appears (the third line); a security key does not take its place.
    let ok = d.setup_now(&[d.f.secret_key.as_str(), PASSWORD, &code]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert!(d.own.exists());
    assert_eq!(d.token_files().len(), 1);
}

#[test]
fn hosted_join_with_a_wrong_password_or_key_stores_nothing() {
    let d = Desktop::new();
    let wrong = d.setup(&[d.f.secret_key.as_str(), "definitely not the password"]);
    assert_eq!(wrong.code, 3, "{}", wrong.stderr);
    assert_eq!(wrong.error_json()["error"]["code"], "wrongCredentials");
    assert!(!d.own.exists(), "a failed join leaves no store");
    assert_eq!(d.token_files().len(), 0, "and no token");
    // The error does not tell a wrong password from a wrong Secret Key.
    let other_key = SecretKey::generate().unwrap();
    let key = other_key.to_display().unwrap();
    let wrong_key = d.setup(&[key.expose_secret(), PASSWORD]);
    assert_eq!(wrong_key.code, 3, "{}", wrong_key.stderr);
    assert!(!d.own.exists());
    // A Secret Key that does not parse.
    let junk = d.setup(&["not a secret key", PASSWORD]);
    assert_eq!(junk.code, 3);
    assert_eq!(junk.error_json()["error"]["code"], "secretKeyInvalid");
    assert_eq!(d.token_files().len(), 0);
    // A missing --email is a usage error.
    let no_email =
        d.f.cmd(&[
            "setup",
            "own-store",
            "--server",
            &d.url(),
            "--store",
            d.own(),
        ])
        .no_store_flag()
        .run();
    assert_eq!(no_email.code, 2);
}

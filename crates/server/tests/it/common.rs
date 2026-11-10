//! Test app: real handlers over an in-memory (or file) database, a temporary
//! blob directory and a manual clock. Requests go through
//! `tower::ServiceExt::oneshot`, no socket. A small OPAQUE client plays the
//! role of the desktop app.
#![allow(dead_code, reason = "each test module uses a part of the helpers")]

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::sync_auth::session_token;
use mosaic_core::{
    AccountId, Argon2Profile, AttachmentKey, InstallId, ItemCiphertext, ItemId, KeyGeneration,
    TombstoneCiphertext, VaultId, VaultKey, VaultMetaCiphertext, WrappedKey,
};
use mosaic_server::{AppState, Config, Db, ManualClock, ServerKeys, app};
use mosaic_sync::wire::json::{
    self, B64, B64Fixed, CreateVaultRequest, ErrorBody, LoginFinishRequest, LoginResponse,
    LoginStartRequest, LoginStartResponse, LoginStatus, PreloginRequest, PushRecord,
    PushRecordsRequest, PushRecordsResponse, RecordKind, RecordResult, RegisterFinishRequest,
    RegisterStartRequest, RegistrationMode, RegistrationStartResponse,
};
use mosaic_sync::wire::opaque::{
    LOGIN_CONTEXT, OsRng, Suite, identifiers,
    ke::{
        ClientLogin, ClientLoginFinishParameters, ClientRegistration,
        ClientRegistrationFinishParameters, CredentialResponse, RegistrationResponse,
    },
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tower::ServiceExt;

/// The test binary's log: one global subscriber for all tests (a scoped one
/// races with the callsite cache of tests running in parallel).
#[derive(Clone, Default)]
pub struct LogBuf(Arc<std::sync::Mutex<Vec<u8>>>);

impl LogBuf {
    pub fn contents(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

impl std::io::Write for LogBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuf {
    type Writer = Self;

    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

pub fn init_logs() -> &'static LogBuf {
    static LOGS: std::sync::OnceLock<LogBuf> = std::sync::OnceLock::new();
    LOGS.get_or_init(|| {
        let buf = LogBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::TRACE)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
        buf
    })
}

pub const ORIGIN: &str = "https://sync.example.com";
pub const T0: u64 = 1_800_000_000_000;
pub const IP: &str = "198.51.100.1:4000";

/// A response.
pub struct Resp {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn json<T: DeserializeOwned>(&self) -> T {
        json::parse(&self.body, usize::MAX).unwrap_or_else(|_| {
            panic!(
                "not the expected JSON: {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }

    pub fn error(&self) -> ErrorBody {
        self.json()
    }

    /// The error code, panicking if the body is not an error.
    pub fn code(&self) -> String {
        self.error().error
    }
}

pub struct TestApp {
    pub state: AppState,
    pub clock: Arc<ManualClock>,
    pub dir: tempfile::TempDir,
    router: Router,
}

pub fn config_for(dir: &Path, mode: RegistrationMode) -> Config {
    let mode = match mode {
        RegistrationMode::Open => "open",
        RegistrationMode::Invite => "invite",
        RegistrationMode::Closed => "closed",
    };
    let d = dir.display();
    Config::parse(&format!(
        "listen = \"127.0.0.1:8743\"\ncanonical_origin = \"{ORIGIN}\"\ndata_dir = '{d}'\n\
         secret_file = '{d}/server.key'\nregistration = \"{mode}\"\n"
    ))
    .expect("test config")
}

impl TestApp {
    pub fn new(mode: RegistrationMode) -> Self {
        Self::with(mode, |_| {})
    }

    pub fn with(mode: RegistrationMode, tweak: impl FnOnce(&mut Config)) -> Self {
        Self::build(mode, false, tweak)
    }

    /// With the database in a file under the temporary directory.
    pub fn with_file_db(mode: RegistrationMode) -> Self {
        Self::build(mode, true, |_| {})
    }

    fn build(mode: RegistrationMode, file: bool, tweak: impl FnOnce(&mut Config)) -> Self {
        init_logs();
        let dir = tempfile::tempdir().expect("tempdir");
        let mut config = config_for(dir.path(), mode);
        tweak(&mut config);
        let db = if file {
            Db::open(&dir.path().join(mosaic_server::DB_FILE)).expect("db")
        } else {
            Db::open_in_memory().expect("db")
        };
        let clock = Arc::new(ManualClock::new(T0));
        let state = AppState::new(
            &config,
            db,
            ServerKeys::generate(T0 - 1000).expect("keys"),
            clock.clone(),
        );
        let router = app(state.clone());
        Self {
            state,
            clock,
            dir,
            router,
        }
    }

    pub async fn send(
        &self,
        method: Method,
        uri: &str,
        token: Option<&[u8; 32]>,
        ip: &str,
        body: Vec<u8>,
    ) -> Resp {
        self.send_with(method, uri, token, ip, &[], body).await
    }

    pub async fn send_with(
        &self,
        method: Method,
        uri: &str,
        token: Option<&[u8; 32]>,
        ip: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Resp {
        let mut req = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        if let Some(t) = token {
            req = req.header(
                header::AUTHORIZATION,
                format!("Bearer {}", Base64UrlUnpadded::encode_string(t)),
            );
        }
        let mut req = req.body(Body::from(body)).expect("request");
        let addr: SocketAddr = ip.parse().expect("socket addr");
        req.extensions_mut().insert(ConnectInfo(addr));
        let resp = self.router.clone().oneshot(req).await.expect("infallible");
        let (parts, body) = resp.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX).await.expect("body");
        Resp {
            status: parts.status,
            headers: parts.headers,
            body: body.to_vec(),
        }
    }

    pub async fn get(&self, uri: &str, token: Option<&[u8; 32]>) -> Resp {
        self.send(Method::GET, uri, token, IP, Vec::new()).await
    }

    pub async fn post<T: Serialize>(&self, uri: &str, token: Option<&[u8; 32]>, body: &T) -> Resp {
        self.send(
            Method::POST,
            uri,
            token,
            IP,
            json::to_vec(body).expect("json"),
        )
        .await
    }

    pub async fn put<T: Serialize>(&self, uri: &str, token: Option<&[u8; 32]>, body: &T) -> Resp {
        self.send(
            Method::PUT,
            uri,
            token,
            IP,
            json::to_vec(body).expect("json"),
        )
        .await
    }

    pub async fn delete(&self, uri: &str, token: Option<&[u8; 32]>) -> Resp {
        self.send(Method::DELETE, uri, token, IP, Vec::new()).await
    }

    // ---- accounts ----------------------------------------------------

    pub async fn register(&self, u: &User, invite: Option<&str>) -> Resp {
        self.register_from(u, invite, IP).await
    }

    pub async fn register_from(&self, u: &User, invite: Option<&str>, ip: &str) -> Resp {
        let mut rng = OsRng;
        let start = ClientRegistration::<Suite>::start(&mut rng, &u.opw).expect("start");
        let r = self
            .send(
                Method::POST,
                "/v1/register/start",
                None,
                ip,
                json::to_vec(&RegisterStartRequest {
                    email: u.email.clone(),
                    account_id: u.account_id,
                    registration_request: B64(start.message.serialize().to_vec()),
                })
                .expect("json"),
            )
            .await;
        if r.status != StatusCode::OK {
            return r;
        }
        let started: RegistrationStartResponse = r.json();
        let fin = start
            .state
            .finish(
                &mut rng,
                &u.opw,
                RegistrationResponse::deserialize(&started.registration_response.0).expect("resp"),
                ClientRegistrationFinishParameters::new(
                    identifiers(u.account_id.as_bytes(), ORIGIN.as_bytes()),
                    None,
                ),
            )
            .expect("finish");
        self.send(
            Method::POST,
            "/v1/register/finish",
            None,
            ip,
            json::to_vec(&RegisterFinishRequest {
                email: u.email.clone(),
                account_id: u.account_id,
                salt: B64Fixed(u.salt),
                profile: Argon2Profile::DEFAULT,
                registration_upload: B64(fin.message.serialize().to_vec()),
                wrapped_ak: wrapped_key(7),
                ak_epoch: 1,
                invite_code: invite.map(str::to_owned),
                device_id: u.device,
                device_name: "Test device".to_owned(),
            })
            .expect("json"),
        )
        .await
    }

    /// Stores an invite code (as the CLI does: only its hash).
    pub fn add_invite(&self, code: &str) {
        use sha2::{Digest, Sha256};
        let hash: [u8; 32] = Sha256::digest(code.as_bytes()).into();
        self.state
            .db()
            .run(|c| {
                c.execute(
                    "INSERT INTO invites (code_hash, created_at) VALUES (?1, 0)",
                    [hash.as_slice()],
                )?;
                Ok(())
            })
            .expect("invite");
    }

    pub async fn prelogin(&self, email: &str) -> Resp {
        self.post(
            "/v1/prelogin",
            None,
            &PreloginRequest {
                email: email.to_owned(),
            },
        )
        .await
    }

    pub async fn login(&self, u: &User) -> LoginResult {
        self.login_with(&u.email, &u.opw, u.account_id, u.device, IP)
            .await
    }

    pub async fn login_with(
        &self,
        email: &str,
        opw: &[u8; 32],
        account_id: AccountId,
        device: InstallId,
        ip: &str,
    ) -> LoginResult {
        self.login_with_pause(email, opw, account_id, device, ip, async || {})
            .await
    }

    /// A login that runs `pause` between `login/start` and `login/finish`
    /// (something happens to the account while the login is in flight).
    pub async fn login_with_pause(
        &self,
        email: &str,
        opw: &[u8; 32],
        account_id: AccountId,
        device: InstallId,
        ip: &str,
        pause: impl AsyncFnOnce(),
    ) -> LoginResult {
        let mut rng = OsRng;
        let start = ClientLogin::<Suite>::start(&mut rng, opw).expect("start");
        let r = self
            .send(
                Method::POST,
                "/v1/login/start",
                None,
                ip,
                json::to_vec(&LoginStartRequest {
                    email: email.to_owned(),
                    credential_request: B64(start.message.serialize().to_vec()),
                })
                .expect("json"),
            )
            .await;
        if r.status != StatusCode::OK {
            return LoginResult::Server(r);
        }
        let started: LoginStartResponse = r.json();
        pause().await;
        let done = start.state.finish(
            &mut rng,
            opw,
            CredentialResponse::deserialize(&started.credential_response.0).expect("resp"),
            ClientLoginFinishParameters::new(
                Some(LOGIN_CONTEXT.as_slice()),
                identifiers(account_id.as_bytes(), ORIGIN.as_bytes()),
                None,
            ),
        );
        let Ok(done) = done else {
            return LoginResult::ClientRejected;
        };
        let token = *session_token(&done.session_key).expect("token").expose();
        let r = self
            .send(
                Method::POST,
                "/v1/login/finish",
                None,
                ip,
                json::to_vec(&LoginFinishRequest {
                    login_id: started.login_id,
                    credential_finalization: B64(done.message.serialize().to_vec()),
                    device_id: device,
                    device_name: "Test device".to_owned(),
                })
                .expect("json"),
            )
            .await;
        if r.status != StatusCode::OK {
            return LoginResult::Server(r);
        }
        let l: LoginResponse = r.json();
        LoginResult::Ok {
            token,
            status: l.status,
            expires_at: l.expires_at,
        }
    }

    /// Registers and logs in a fresh user (open or invite-free registration).
    pub async fn user(&self, name: &str) -> (User, [u8; 32]) {
        let u = User::new(&format!("{name}@example.com"));
        let r = self.register(&u, None).await;
        assert_eq!(
            r.status,
            StatusCode::CREATED,
            "{}",
            String::from_utf8_lossy(&r.body)
        );
        let LoginResult::Ok { token, .. } = self.login(&u).await else {
            panic!("login failed");
        };
        (u, token)
    }

    /// A further login of the same user from another device.
    pub async fn login_device(&self, u: &User, device: InstallId) -> [u8; 32] {
        match self
            .login_with(&u.email, &u.opw, u.account_id, device, IP)
            .await
        {
            LoginResult::Ok { token, .. } => token,
            _ => panic!("login failed"),
        }
    }

    // ---- vaults ------------------------------------------------------

    /// The device a session belongs to.
    pub fn device_of(&self, token: &[u8; 32]) -> InstallId {
        use sha2::{Digest, Sha256};
        let hash: [u8; 32] = Sha256::digest(token).into();
        let id: Vec<u8> = self
            .state
            .db()
            .run(|c| {
                Ok(c.query_row(
                    "SELECT device_id FROM sessions WHERE token_hash = ?1",
                    [hash.as_slice()],
                    |r| r.get(0),
                )?)
            })
            .expect("session");
        InstallId::from_slice(&id).expect("device id")
    }

    pub async fn create_vault(&self, token: &[u8; 32], v: &TestVault) -> Resp {
        self.put(
            &format!("/v1/vaults/{}", v.id),
            Some(token),
            &v.create_request(self.device_of(token)),
        )
        .await
    }

    pub async fn push(&self, token: &[u8; 32], v: &TestVault, records: Vec<PushRecord>) -> Resp {
        self.post(
            &format!("/v1/vaults/{}/records", v.id),
            Some(token),
            &PushRecordsRequest {
                generation: v.vk.generation(),
                records,
            },
        )
        .await
    }

    /// Pushes new items and returns their server sequence numbers.
    pub async fn push_new_items(
        &self,
        token: &[u8; 32],
        v: &TestVault,
        n: usize,
    ) -> Vec<(ItemId, u64)> {
        let mut ids = Vec::new();
        let mut recs = Vec::new();
        for i in 0..n {
            let id = ItemId::new_v7().expect("id");
            recs.push(v.item_record(id, &format!("item {i}"), None));
            ids.push(id);
        }
        let r = self.push(token, v, recs).await;
        assert_eq!(
            r.status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&r.body)
        );
        let out: PushRecordsResponse = r.json();
        out.results
            .into_iter()
            .zip(ids)
            .map(|(res, id)| match res {
                RecordResult::Accepted { server_seq, .. } => (id, server_seq),
                RecordResult::Conflict { .. } => panic!("unexpected conflict"),
            })
            .collect()
    }
}

pub enum LoginResult {
    Ok {
        token: [u8; 32],
        status: LoginStatus,
        expires_at: u64,
    },
    /// The client could not open the server's response (wrong password).
    ClientRejected,
    Server(Resp),
}

impl LoginResult {
    pub fn token(self) -> [u8; 32] {
        match self {
            Self::Ok { token, .. } => token,
            _ => panic!("login did not succeed"),
        }
    }
}

pub struct User {
    pub email: String,
    pub opw: [u8; 32],
    pub account_id: AccountId,
    pub device: InstallId,
    pub salt: [u8; 16],
}

impl User {
    pub fn new(email: &str) -> Self {
        let mut opw = [0u8; 32];
        getrandom::fill(&mut opw).expect("rng");
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).expect("rng");
        Self {
            email: email.to_owned(),
            opw,
            account_id: AccountId::new_random().expect("id"),
            device: InstallId::new_v7().expect("id"),
            salt,
        }
    }
}

/// A well-formed wrapped key (the server checks only the envelope header).
pub fn wrapped_key(fill: u8) -> WrappedKey {
    let mut b = [fill; WrappedKey::LEN];
    b[0] = 1;
    b[1] = 1;
    WrappedKey::from_bytes(&b).expect("wrapped key")
}

pub struct TestVault {
    pub id: VaultId,
    pub vk: VaultKey,
    pub device: InstallId,
}

impl TestVault {
    pub fn new() -> Self {
        Self::at(KeyGeneration::FIRST)
    }

    pub fn at(generation: KeyGeneration) -> Self {
        Self {
            id: VaultId::new_v7().expect("id"),
            vk: VaultKey::generate(generation).expect("vk"),
            device: InstallId::new_v7().expect("id"),
        }
    }

    pub fn meta(&self) -> Vec<u8> {
        VaultMetaCiphertext::seal(&self.vk, &self.id, b"{\"name\":\"x\"}")
            .expect("meta")
            .as_bytes()
            .to_vec()
    }

    pub fn create_request(&self, device: InstallId) -> CreateVaultRequest {
        CreateVaultRequest {
            generation: self.vk.generation(),
            wrapped_key: wrapped_key(u8::try_from(self.vk.generation().get()).unwrap()),
            device_id: device,
            meta: B64(self.meta()),
        }
    }

    pub fn item_bytes(&self, id: ItemId, text: &str) -> Vec<u8> {
        ItemCiphertext::seal(&self.vk, &self.id, &id, 1, text.as_bytes())
            .expect("item")
            .as_bytes()
            .to_vec()
    }

    pub fn item_record(&self, id: ItemId, text: &str, base_seq: Option<u64>) -> PushRecord {
        PushRecord {
            record_id: id,
            kind: RecordKind::Item,
            ciphertext: B64(self.item_bytes(id, text)),
            base_seq,
        }
    }

    pub fn tombstone_record(&self, id: ItemId, base_seq: Option<u64>) -> PushRecord {
        PushRecord {
            record_id: id,
            kind: RecordKind::Tombstone,
            ciphertext: B64(TombstoneCiphertext::seal(&self.vk, &self.id, &id, b"{}")
                .expect("tombstone")
                .as_bytes()
                .to_vec()),
            base_seq,
        }
    }

    /// A STREAM attachment ciphertext of `len` plaintext bytes.
    pub fn attachment(&self, item: ItemId, id: mosaic_core::AttachmentId, len: usize) -> Vec<u8> {
        let fk = AttachmentKey::generate().expect("fk");
        mosaic_core::seal_attachment(&fk, &self.id, &item, &id, &vec![0x5a; len])
            .expect("attachment")
    }
}

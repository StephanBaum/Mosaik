//! Seeds a Mosaic Security Challenge test server (docs/CHALLENGE.md).
//!
//! ```text
//! cargo run -p mosaic-server --example contest_seed -- --server URL --out DIR
//! ```
//!
//! It talks to a running server over the normal client paths (`mosaic-sync`
//! hosted calls, the sync engine, `mosaic-vault` sessions, `mosaic-passkey`
//! as the software authenticator of C4). It invents no cryptography.
//!
//! Output in `DIR`:
//! - `public.json`: what the challenge publishes (addresses, the C4 and C6
//!   credentials the rules publish, artifact hashes). No flag, no secret.
//! - `private.json`: the flags and every secret the rules do not publish.
//!   Never commit it, never publish it.
//! - `c2-store/`, `c3.mosaicx`, `c6-vault/`: the downloads of C2, C3 and C6.
//!
//! Running it twice is safe: it refuses when `DIR` already holds a
//! `public.json` or `private.json`, and a server that already has the
//! accounts refuses the registration. To reseed, wipe the server's data
//! volume and `DIR` first (see `deploy/contest/README.md`).
//!
//! `--server` must be the server's canonical origin, exactly as clients enter
//! it (the sign-in checks the server's identity). The WebAuthn step of C4
//! posts to the handoff routes itself, over `http://` (the loopback server of
//! the test suite) or `https://` (the deployed server; that path has no
//! automated test).

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: its output is the interface"
)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::{ItemType, LoginUrl, SecretField, TextField};
use mosaic_core::{Argon2Profile, ExportPassphrase, MasterPassword, SecretString};
use mosaic_generator::{PassphraseOptions, generate_passphrase};
use mosaic_passkey::{
    Ceremony, Es256Key, attestation_object_none, client_data_json, new_credential_id, rp_id_hash,
};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, SyncEngine};
use mosaic_sync::http::{HttpTransport, canonical_origin, hosted};
use mosaic_sync::testutil::LocalSession;
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::json::HandoffPurpose;
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};
use mosaic_vault::session::{ExportSelection, LockedSession};
use mosaic_vault::storage::Store;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_platform_verifier::BuilderVerifierExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The address domain of the challenge accounts (reserved, never resolves).
const DOMAIN: &str = "contest.invalid";

/// What the seeder needs.
pub struct Options {
    /// The server, as clients enter it.
    pub server: String,
    /// The output directory.
    pub out: PathBuf,
    /// The Argon2id profile of the accounts (the challenge uses `Standard`).
    pub profile: Argon2Profile,
    /// The server's clock in Unix ms (TOTP codes are checked against it).
    pub now_ms: Box<dyn Fn() -> u64>,
    /// Called before each rate-limited request (the real run waits here, a
    /// test with a manual server clock moves it).
    pub pace: Box<dyn Fn()>,
}

/// Where the files went.
pub struct Outcome {
    /// `public.json`.
    pub public: PathBuf,
    /// `private.json`.
    pub private: PathBuf,
}

fn f<E: std::fmt::Debug>(what: &'static str) -> impl FnOnce(E) -> String {
    move |e| format!("{what}: {e:?}")
}

fn b64(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// A random flag such as `MOSAIC{c1-0123456789abcdef}`.
fn flag(label: &str) -> Result<String, String> {
    let mut r = [0u8; 12];
    getrandom::fill(&mut r).map_err(f("random"))?;
    Ok(format!("MOSAIC{{{label}-{}}}", hex(&r)))
}

/// A random six-word passphrase (the generator's default).
fn passphrase() -> Result<String, String> {
    let p = generate_passphrase(&PassphraseOptions::default()).map_err(f("passphrase"))?;
    Ok(p.expose_secret().to_owned())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(f("read artifact"))?;
    Ok(hex(&Sha256::digest(bytes)))
}

/// One local account in `dir/<label>/mosaic.db`.
struct Local {
    session: mosaic_vault::session::UnlockedSession,
    password: String,
    secret_key: String,
}

fn local_account(dir: &Path, label: &str, profile: Argon2Profile) -> Result<Local, String> {
    let folder = dir.join(label);
    fs::create_dir_all(&folder).map_err(f("create folder"))?;
    let store = Store::open(folder.join("mosaic.db")).map_err(f("open store"))?;
    let password = passphrase()?;
    let (session, created) = LockedSession::open(store)
        .create_account(&MasterPassword::new(password.clone()), profile)
        .map_err(|e| format!("create account: {:?}", e.error))?;
    let secret_key = created
        .secret_key
        .to_display()
        .map_err(f("secret key"))?
        .expose_secret()
        .to_owned();
    Ok(Local {
        session,
        password,
        secret_key,
    })
}

fn add_flag_item(
    s: &mut mosaic_vault::session::UnlockedSession,
    vault: mosaic_core::VaultId,
    title: &str,
    url: Option<&str>,
    flag: &str,
) -> Result<(), String> {
    s.add_item(vault, &ItemType::Login, |e| {
        e.set_title(title)?;
        e.set_text(TextField::Username, "challenge")?;
        e.set_secret(SecretField::Password, flag)?;
        if let Some(url) = url {
            e.add_url(&LoginUrl::new(url))?;
        }
        Ok(())
    })
    .map_err(f("add item"))?;
    Ok(())
}

/// A hosted account that is registered, signed in and holds one vault.
struct Hosted {
    email: String,
    password: String,
    secret_key: String,
    access: LocalSession,
    grant: hosted::LoginGrant,
}

fn hosted_account(
    o: &Options,
    dir: &Path,
    label: &str,
    flag_text: Option<&str>,
) -> Result<Hosted, String> {
    let email = format!("{label}@{DOMAIN}");
    let folder = dir.join(format!("{label}-device"));
    fs::create_dir_all(&folder).map_err(f("create folder"))?;
    let store = Store::open(folder.join("mosaic.db")).map_err(f("open store"))?;
    let device = store.install_id().map_err(f("install id"))?;
    let password = passphrase()?;
    let master = MasterPassword::new(password.clone());
    let (mut session, created) = LockedSession::open(store)
        .create_account(&master, o.profile)
        .map_err(|e| format!("create account: {:?}", e.error))?;
    let boot = session.account_bootstrap().map_err(f("bootstrap"))?;
    let opw = session
        .opaque_password(&master, &created.secret_key)
        .map_err(f("opaque password"))?;
    let secret_key = created
        .secret_key
        .to_display()
        .map_err(f("secret key"))?
        .expose_secret()
        .to_owned();
    if let Some(text) = flag_text {
        let vault = session.create_vault("Challenge").map_err(f("vault"))?;
        add_flag_item(&mut session, vault, "Flag", None, text)?;
    }
    let account = session.account_id();
    (o.pace)();
    hosted::register(&o.server, &email, None, &boot, &opw, device, "seed device").map_err(|e| {
        format!("register {email}: {e:?} (already seeded? wipe the server data first)")
    })?;
    (o.pace)();
    let grant = hosted::login(&o.server, &email, account, &opw, device, "seed device")
        .map_err(f("login"))?;
    let mut access = LocalSession::new(session);
    if flag_text.is_some() {
        let transport =
            HttpTransport::connect(&o.server, &grant.token, device).map_err(f("connect"))?;
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
        let report = engine
            .sync_once(&mut access, &SystemClock)
            .map_err(|e| format!("sync: {e:?}"))?;
        if report.pushed == 0 {
            return Err(format!("sync of {email} pushed nothing"));
        }
    }
    Ok(Hosted {
        email,
        password,
        secret_key,
        access,
        grant,
    })
}

/// The bytes of one HTTP/1.1 `POST` with a JSON body, closing the connection.
fn post_request(host: &str, path: &str, body: &Value) -> String {
    let body = body.to_string();
    format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Sends `request` to `host_port` and reads the whole answer. `tls` names the
/// server for HTTPS (TLS 1.3, the system trust store, like the sync client).
fn exchange(host_port: &str, tls: Option<&str>, request: &str) -> Result<Vec<u8>, String> {
    let tcp = TcpStream::connect(host_port).map_err(f("connect"))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(f("timeout"))?;
    let mut raw = Vec::new();
    if let Some(name) = tls {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(f("tls"))?
            .with_platform_verifier()
            .map_err(f("tls verifier"))?
            .with_no_client_auth();
        let server_name = ServerName::try_from(name.to_owned()).map_err(f("server name"))?;
        let conn = ClientConnection::new(Arc::new(config), server_name).map_err(f("tls"))?;
        let mut stream = StreamOwned::new(conn, tcp);
        stream.write_all(request.as_bytes()).map_err(f("write"))?;
        // A server that closes without a TLS goodbye ends the read with an
        // error after the whole answer; keep what came.
        let _ = stream.read_to_end(&mut raw);
    } else {
        let mut stream = tcp;
        stream.write_all(request.as_bytes()).map_err(f("write"))?;
        stream.read_to_end(&mut raw).map_err(f("read"))?;
    }
    Ok(raw)
}

/// One POST of JSON (the unauthenticated handoff routes), over `http://` or
/// `https://`.
fn post_json(base: &str, path: &str, body: &Value) -> Result<(u16, Value), String> {
    let (host, tls) = match base.strip_prefix("https://") {
        Some(rest) => (rest.trim_end_matches('/'), true),
        None => (
            base.strip_prefix("http://")
                .ok_or("the server address must start with http:// or https://")?
                .trim_end_matches('/'),
            false,
        ),
    };
    let name = host.rsplit_once(':').map_or(host, |(h, _)| h);
    let host_port = if tls && !host.contains(':') {
        format!("{host}:443")
    } else {
        host.to_owned()
    };
    let raw = exchange(
        &host_port,
        tls.then_some(name),
        &post_request(host, path, body),
    )?;
    let text = String::from_utf8_lossy(&raw);
    let (head, payload) = text.split_once("\r\n\r\n").ok_or("malformed response")?;
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or("malformed status line")?;
    let value = serde_json::from_str(payload.trim()).unwrap_or(Value::Null);
    Ok((status, value))
}

/// Registers a software authenticator through a `register` handoff and
/// returns its private key (PKCS#8, base64url) and credential ID.
fn register_security_key(o: &Options, h: &Hosted) -> Result<(String, String), String> {
    (o.pace)();
    let handoff = hosted::account_webauthn_handoff(
        &o.server,
        &h.grant.token,
        HandoffPurpose::Register,
        Some("Challenge key"),
    )
    .map_err(f("handoff"))?;
    let id = b64(&handoff.handoff_id.0);
    (o.pace)();
    let (status, options) = post_json(
        &o.server,
        "/v1/webauthn/handoff/options",
        &json!({"handoffId": id, "code": handoff.match_code}),
    )?;
    if status != 200 {
        return Err(format!("handoff options answered {status}"));
    }
    let challenge = Base64UrlUnpadded::decode_vec(
        options
            .pointer("/publicKey/challenge")
            .and_then(Value::as_str)
            .ok_or("no challenge")?,
    )
    .map_err(f("challenge"))?;
    let rp_id = options
        .pointer("/publicKey/rp/id")
        .and_then(Value::as_str)
        .ok_or("no rp id")?;
    let key = Es256Key::generate().map_err(f("key"))?;
    let cred_id = new_credential_id().map_err(f("credential id"))?;
    let mut auth_data = rp_id_hash(rp_id).to_vec();
    // UP | UV | AT.
    auth_data.push(0x45);
    auth_data.extend_from_slice(&0u32.to_be_bytes());
    auth_data.extend_from_slice(&[0u8; 16]);
    auth_data.extend_from_slice(
        &u16::try_from(cred_id.len())
            .map_err(f("credential id length"))?
            .to_be_bytes(),
    );
    auth_data.extend_from_slice(&cred_id);
    auth_data.extend_from_slice(&key.cose_public_key());
    let attestation = attestation_object_none(&auth_data).map_err(f("attestation"))?;
    let origin = canonical_origin(&o.server).map_err(f("origin"))?;
    let client_data = client_data_json(Ceremony::Create, &challenge, &origin);
    let credential = json!({
        "id": b64(&cred_id), "rawId": b64(&cred_id), "type": "public-key",
        "response": {
            "clientDataJSON": b64(&client_data),
            "attestationObject": b64(&attestation),
            "transports": ["usb"],
        },
    });
    (o.pace)();
    let (status, _) = post_json(
        &o.server,
        "/v1/webauthn/handoff/finish",
        &json!({"handoffId": id, "credential": credential}),
    )?;
    if status != 200 {
        return Err(format!("handoff finish answered {status}"));
    }
    let der = key.to_pkcs8_der().map_err(f("key export"))?;
    Ok((b64(der.expose_secret()), b64(&cred_id)))
}

/// Turns on the authenticator-app second factor and returns its secret.
fn enable_totp(o: &Options, h: &Hosted) -> Result<String, String> {
    let setup = hosted::totp_setup(&o.server, &h.grant.token).map_err(f("totp setup"))?;
    let secret = setup.secret.expose_secret().to_owned();
    let otp = Otp {
        secret: OtpSecret::from_base32(&secret).map_err(f("totp secret"))?,
        algorithm: Algorithm::Sha1,
        kind: OtpKind::Totp {
            digits: Digits::DEFAULT,
            period: Period::DEFAULT,
        },
    };
    let code = otp
        .code_at((o.now_ms)() / 1000)
        .map_err(f("totp code"))?
        .expose_secret()
        .to_owned();
    hosted::totp_confirm(&o.server, &h.grant.token, &code).map_err(f("totp confirm"))?;
    Ok(secret)
}

fn move_into(from: &Path, to: &Path) -> Result<(), String> {
    fs::rename(from, to).map_err(f("move artifact"))
}

/// Seeds the server and writes the files. See the module documentation.
///
/// # Errors
/// A description of the first step that failed. Nothing is written to the
/// output directory before every step has worked.
#[expect(
    clippy::too_many_lines,
    reason = "one straight list of the challenge accounts"
)]
pub fn seed(o: &Options) -> Result<Outcome, String> {
    let public_path = o.out.join("public.json");
    let private_path = o.out.join("private.json");
    if public_path.exists() || private_path.exists() {
        return Err(format!(
            "{} already holds a seed. Wipe the server data and this directory to reseed.",
            o.out.display()
        ));
    }
    fs::create_dir_all(&o.out).map_err(f("create output"))?;
    let staging = o.out.join(".staging");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(f("clear staging"))?;
    }
    fs::create_dir_all(&staging).map_err(f("create staging"))?;

    // C1: a vault on the server. The full data directory is the download.
    let c1_flag = flag("c1")?;
    let c1 = hosted_account(o, &staging, "c1", Some(&c1_flag))?;

    // C2: a locked local store, no keystore.
    let c2_flag = flag("c2")?;
    let mut c2 = local_account(&staging, "c2-store", o.profile)?;
    let v = c2.session.create_vault("Challenge").map_err(f("vault"))?;
    add_flag_item(&mut c2.session, v, "Flag", None, &c2_flag)?;

    // C3: an export file under a random six-word passphrase.
    let c3_flag = flag("c3")?;
    let mut c3 = local_account(&staging, "c3-source", o.profile)?;
    let v = c3.session.create_vault("Challenge").map_err(f("vault"))?;
    add_flag_item(&mut c3.session, v, "Flag", None, &c3_flag)?;
    let c3_pass = passphrase()?;
    let export_path = staging.join("c3.mosaicx");
    {
        let mut file = fs::File::create(&export_path).map_err(f("create export"))?;
        let pass =
            ExportPassphrase::new(&SecretString::from_str_copy(&c3_pass).map_err(f("pass"))?)
                .map_err(f("passphrase"))?;
        c3.session
            .write_export(&ExportSelection::default(), &mut file, &pass)
            .map_err(f("export"))?;
    }

    // C4: password and Secret Key published, plus TOTP and a security key.
    let c4_flag = flag("c4")?;
    let c4 = hosted_account(o, &staging, "c4", Some(&c4_flag))?;
    let (key_pkcs8, credential_id) = register_security_key(o, &c4)?;
    let totp_secret = enable_totp(o, &c4)?;

    // C5: the account `hidden@`. Its existence and its records are the
    // target; c1 and c4 are the other accounts to read. (The server allows
    // three registrations per address an hour, which is why there are three
    // hosted accounts and no more.)
    let c5_flag = flag("c5")?;
    let hidden = hosted_account(o, &staging, "hidden", Some(&c5_flag))?;

    // C6: a published test vault with the flag login.
    let c6_flag = flag("c6")?;
    let mut c6 = local_account(&staging, "c6-vault", o.profile)?;
    let v = c6.session.create_vault("Challenge").map_err(f("vault"))?;
    add_flag_item(
        &mut c6.session,
        v,
        "Flag login",
        Some("https://flag.contest.invalid"),
        &c6_flag,
    )?;

    // Close every session before the files move.
    drop((c2.session, c3.session, c6.session));
    drop((c1.access, c4.access, hidden.access));

    let c2_hash = sha256_file(&staging.join("c2-store").join("mosaic.db"))?;
    let c3_hash = sha256_file(&export_path)?;
    let c6_hash = sha256_file(&staging.join("c6-vault").join("mosaic.db"))?;

    let public = json!({
        "version": 1,
        "server": o.server,
        "accounts": {
            "c1": { "email": c1.email },
            "c4": {
                "email": c4.email,
                "password": c4.password,
                "secretKey": c4.secret_key,
                "secondFactors": ["totp", "webauthn"],
            },
            "c5": { "registration": "open", "hiddenAddress": format!("hidden@{DOMAIN}") },
        },
        "artifacts": {
            "c2-store/mosaic.db": c2_hash,
            "c3.mosaicx": c3_hash,
            "c6-vault/mosaic.db": c6_hash,
        },
        "c6": {
            "password": c6.password,
            "secretKey": c6.secret_key,
            "flagLogin": "https://flag.contest.invalid",
        },
    });
    let private = json!({
        "version": 1,
        "flags": {
            "c1": c1_flag, "c2": c2_flag, "c3": c3_flag, "c4": c4_flag,
            "c5": c5_flag, "c6": c6_flag,
        },
        "secrets": {
            "c1": { "password": c1.password, "secretKey": c1.secret_key },
            "c2": { "password": c2.password, "secretKey": c2.secret_key },
            "c3": { "exportPassphrase": c3_pass },
            "c4": {
                "totpSecret": totp_secret,
                "webauthnKeyPkcs8": key_pkcs8,
                "webauthnCredentialId": credential_id,
            },
            "c5": { "password": hidden.password, "secretKey": hidden.secret_key },
        },
    });

    move_into(&staging.join("c2-store"), &o.out.join("c2-store"))?;
    move_into(&staging.join("c6-vault"), &o.out.join("c6-vault"))?;
    move_into(&export_path, &o.out.join("c3.mosaicx"))?;
    let _ = fs::remove_dir_all(&staging);
    let text = |v: &Value| serde_json::to_string_pretty(v).map_err(f("json"));
    fs::write(&private_path, text(&private)?).map_err(f("write private.json"))?;
    fs::write(&public_path, text(&public)?).map_err(f("write public.json"))?;
    Ok(Outcome {
        public: public_path,
        private: private_path,
    })
}

fn main() -> std::process::ExitCode {
    let mut server = None;
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => server = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            _ => {
                eprintln!("usage: contest_seed --server URL --out DIR");
                return std::process::ExitCode::from(2);
            }
        }
    }
    let (Some(server), Some(out)) = (server, out) else {
        eprintln!("usage: contest_seed --server URL --out DIR");
        return std::process::ExitCode::from(2);
    };
    let options = Options {
        server,
        out,
        profile: Argon2Profile::Standard,
        now_ms: Box::new(|| mosaic_sync::clock::Clock::now_ms(&SystemClock)),
        // The unauthenticated routes allow 10 a minute with a burst of 20.
        pace: Box::new(|| std::thread::sleep(Duration::from_secs(7))),
    };
    match seed(&options) {
        Ok(done) => {
            println!("wrote {}", done.public.display());
            println!(
                "wrote {} (keep it private, never commit it)",
                done.private.display()
            );
            std::process::ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("contest_seed: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

//! Shared helpers: one fixture store created once per test process and copied per test, and a
//! runner for the built `mosaic` binary with the file-based test backends.

use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use mosaic_core::model::{
    CustomField, CustomFieldValue, ItemType, LoginUrl, OtpConfig, OtpKind, SecretField, Text,
    TextField,
};
use mosaic_core::{Argon2Profile, MasterPassword, SecretString, SecretVec};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

pub const PASSWORD: &str = "correct horse battery staple";
pub const GITHUB_PASSWORD: &str = "gh-secret-pw-4711";
pub const T0: u64 = 1_800_000_000_000;
/// RFC 4226 / 6238 test secret.
pub const OTP_SECRET: &[u8] = b"12345678901234567890";

pub struct Master {
    pub dir: PathBuf,
    pub secret_key: String,
    pub account_hex: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Creates an account (and items); the store is closed again when this returns.
pub fn create_store(dir: &Path, vault_names: &[&str], with_items: bool) -> (String, String) {
    std::fs::create_dir_all(dir).unwrap();
    let store = Store::open(dir.join("vault.sqlite3")).unwrap();
    let (mut session, created) = LockedSession::open(store)
        .create_account(
            &MasterPassword::new(PASSWORD.to_owned()),
            Argon2Profile::Baseline,
        )
        .map_err(|e| e.error)
        .unwrap();
    let secret_key = created
        .secret_key
        .to_display()
        .unwrap()
        .expose_secret()
        .to_owned();
    let account_hex = hex(created.params.account_id.as_bytes());
    let vaults: Vec<_> = vault_names
        .iter()
        .map(|n| session.create_vault(n).unwrap())
        .collect();
    if with_items {
        let personal = vaults[0];
        let work = vaults[1];
        session
            .add_item(personal, &ItemType::Login, |e| {
                e.set_title("GitHub")?;
                e.set_text(TextField::Username, "alice")?;
                e.set_secret(SecretField::Password, GITHUB_PASSWORD)?;
                e.add_url(&LoginUrl::new("https://github.com"))?;
                e.set_otp(&OtpConfig::totp(SecretVec::from_slice(OTP_SECRET)?))?;
                e.add_custom_field(&CustomField::new(
                    Text::new("recovery"),
                    CustomFieldValue::Hidden(SecretString::from_str_copy("recov-123")?),
                ))?;
                Ok(())
            })
            .unwrap();
        session
            .add_item(personal, &ItemType::Login, |e| {
                e.set_title("Mail")?;
                e.set_text(TextField::Username, "bob")?;
                e.set_secret(SecretField::Password, "mail-pw-xyz")
            })
            .unwrap();
        session
            .add_item(personal, &ItemType::SecureNote, |e| {
                e.set_title("Shopping list")?;
                e.set_text(TextField::Notes, "milk and eggs")
            })
            .unwrap();
        session
            .add_item(personal, &ItemType::Login, |e| {
                e.set_title("evil\u{202e}title")
            })
            .unwrap();
        session
            .add_item(personal, &ItemType::Login, |e| {
                e.set_title("HotpThing")?;
                let mut config = OtpConfig::totp(SecretVec::from_slice(OTP_SECRET)?);
                config.kind = OtpKind::Hotp;
                e.set_otp(&config)
            })
            .unwrap();
        session
            .add_item(work, &ItemType::Login, |e| {
                e.set_title("GitHub")?;
                e.set_secret(SecretField::Password, "work-gh-pw")
            })
            .unwrap();
    }
    drop(session.lock());
    (secret_key, account_hex)
}

pub fn master() -> &'static Master {
    static MASTER: OnceLock<Master> = OnceLock::new();
    MASTER.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap().keep();
        let (secret_key, account_hex) = create_store(&dir, &["Personal", "Work"], true);
        Master {
            dir,
            secret_key,
            account_hex,
        }
    })
}

pub struct Fixture {
    pub tmp: tempfile::TempDir,
    pub store: PathBuf,
    pub test_dir: PathBuf,
    pub account_hex: String,
    pub secret_key: String,
}

impl Fixture {
    /// A copy of the shared fixture store (two vaults, six items).
    pub fn new() -> Self {
        let m = master();
        let tmp = tempfile::tempdir().unwrap();
        let store_dir = tmp.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::copy(m.dir.join("vault.sqlite3"), store_dir.join("vault.sqlite3")).unwrap();
        Self::at(tmp, m.account_hex.clone(), m.secret_key.clone())
    }

    /// A fixture over a freshly created store without items.
    pub fn fresh(vault_names: &[&str]) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let (secret_key, account_hex) = create_store(&tmp.path().join("store"), vault_names, false);
        Self::at(tmp, account_hex, secret_key)
    }

    fn at(tmp: tempfile::TempDir, account_hex: String, secret_key: String) -> Self {
        let test_dir = tmp.path().join("test");
        let keystore = test_dir.join("keystore");
        std::fs::create_dir_all(&keystore).unwrap();
        std::fs::write(
            keystore.join(format!("secret-key-{account_hex}")),
            &secret_key,
        )
        .unwrap();
        Self {
            store: tmp.path().join("store").join("vault.sqlite3"),
            test_dir,
            tmp,
            account_hex,
            secret_key,
        }
    }

    pub fn session_file(&self) -> PathBuf {
        self.test_dir
            .join("run")
            .join(format!("session-{}.mcls", self.account_hex))
    }

    pub fn session_key(&self) -> PathBuf {
        self.test_dir
            .join("keystore")
            .join(format!("session-{}.key", self.account_hex))
    }

    pub fn clipboard(&self) -> String {
        std::fs::read_to_string(self.test_dir.join("clipboard.txt")).unwrap_or_default()
    }

    pub fn cmd(&self, args: &[&str]) -> Run<'_> {
        Run {
            fixture: self,
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            stdin: None,
            env: vec![("MOSAIC_TEST_NOW_MS".to_owned(), T0.to_string())],
            store: true,
        }
    }

    /// Unlocks with the right password and a session of `ttl` (for example `5m`).
    pub fn unlock(&self, ttl: &str) {
        let out = self
            .cmd(&["unlock", "--password-stdin", "--ttl", ttl])
            .stdin(PASSWORD)
            .run();
        assert_eq!(out.code, 0, "unlock failed: {}", out.stderr);
    }
}

pub struct Run<'a> {
    fixture: &'a Fixture,
    args: Vec<String>,
    stdin: Option<String>,
    env: Vec<(String, String)>,
    store: bool,
}

pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(self.stdout.trim())
            .unwrap_or_else(|e| panic!("not JSON ({e}): {}", self.stdout))
    }

    pub fn error_json(&self) -> serde_json::Value {
        serde_json::from_str(self.stderr.trim().lines().last().unwrap_or("")).unwrap()
    }
}

impl Run<'_> {
    pub fn stdin(mut self, text: &str) -> Self {
        self.stdin = Some(format!("{text}\n"));
        self
    }

    pub fn env(mut self, name: &str, value: &str) -> Self {
        self.env.retain(|(n, _)| n != name);
        self.env.push((name.to_owned(), value.to_owned()));
        self
    }

    pub fn at(self, now_ms: u64) -> Self {
        self.env("MOSAIC_TEST_NOW_MS", &now_ms.to_string())
    }

    /// Lines for the scripted terminal (one per prompt).
    pub fn prompts(self, lines: &[&str]) -> Self {
        let path = self.fixture.tmp.path().join("prompts.txt");
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
        self.env("MOSAIC_TEST_PROMPTS", path.to_str().unwrap())
    }

    /// Without the fixture's `--store` (the test passes its own).
    pub fn no_store_flag(mut self) -> Self {
        self.store = false;
        self
    }

    pub fn run(self) -> Out {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mosaic"));
        command.env_clear();
        for key in ["PATH", "SystemRoot", "TEMP", "TMP", "HOME", "USERPROFILE"] {
            if let Some(v) = std::env::var_os(key) {
                command.env(key, v);
            }
        }
        command
            .env("MOSAIC_TEST_DIR", &self.fixture.test_dir)
            .env("MOSAIC_LANG", "en");
        for (k, v) in &self.env {
            command.env(k, v);
        }
        if self.store {
            command.arg("--store").arg(&self.fixture.store);
        }
        command.args(&self.args);
        // Output goes to files, not pipes: the clipboard helper outlives the command and may hold
        // inherited pipe handles (Windows), which would make a pipe reader wait for it.
        let (mut out_file, mut err_file) =
            (tempfile::tempfile().unwrap(), tempfile::tempfile().unwrap());
        command
            .stdout(out_file.try_clone().unwrap())
            .stderr(err_file.try_clone().unwrap());
        command.stdin(if self.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = command.spawn().unwrap();
        if let Some(text) = &self.stdin {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(text.as_bytes())
                .unwrap();
        }
        let status = child.wait().unwrap();
        let read = |file: &mut std::fs::File| {
            let mut bytes = Vec::new();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read_to_end(&mut bytes).unwrap();
            String::from_utf8_lossy(&bytes).into_owned()
        };
        Out {
            code: status.code().unwrap_or(-1),
            stdout: read(&mut out_file),
            stderr: read(&mut err_file),
        }
    }
}

/// Replaces every UUID by `<uuid>` so a golden file does not depend on random IDs.
pub fn normalize(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let end = chars.len().min(i + 36);
        let window: &[char] = &chars[i..end];
        let uuid_like = window.len() == 36
            && window.iter().enumerate().all(|(n, c)| {
                if matches!(n, 8 | 13 | 18 | 23) {
                    *c == '-'
                } else {
                    c.is_ascii_hexdigit()
                }
            });
        if uuid_like {
            out.push_str("<uuid>");
            i += 36;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Compares `actual` (UUIDs normalized) with `tests/it/golden/<name>`; `UPDATE_GOLDEN=1` rewrites it.
pub fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/it/golden")
        .join(name);
    let actual = normalize(actual);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(actual.trim_end(), expected.trim_end(), "golden file {name}");
}

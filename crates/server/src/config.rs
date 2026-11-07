//! `mosaic-server.toml` (M6 spec §6.5). Unknown keys are an error.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

use mosaic_sync::wire::json::RegistrationMode;
use serde::Deserialize;

/// The largest config file the server reads.
const MAX_CONFIG_LEN: u64 = 64 * 1024;
/// The smallest `max_request_bytes` a config file may set: a record is up to
/// 1 MiB, which is 1.4 MiB as base64url in a JSON body.
pub const MIN_REQUEST_BYTES: u64 = 2 * 1024 * 1024;
/// Vaults per account.
pub const MAX_VAULTS_PER_ACCOUNT: u64 = 1_000;
/// Non-revoked devices per account.
pub const MAX_DEVICES_PER_ACCOUNT: u64 = 32;

/// Why the configuration is unusable. The message names the key, never a
/// secret (the file holds none).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The file could not be read.
    #[error("cannot read the config file")]
    Read,
    /// The file is longer than the limit.
    #[error("the config file is too large")]
    TooLarge,
    /// The TOML is malformed or has an unknown or mistyped key.
    #[error("invalid config: {0}")]
    Parse(String),
    /// A value is out of range.
    #[error("invalid config value: {0}")]
    Invalid(&'static str),
}

/// The `[limits]` table.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct Limits {
    /// Largest JSON request body.
    pub max_request_bytes: u64,
    /// Attachment bytes per account (soft-deleted ones count until purged).
    pub account_attachment_bytes: u64,
    /// Records per account.
    pub account_record_count: u64,
    /// Record ciphertext bytes per account.
    pub account_record_bytes: u64,
    /// Idle lifetime of a session.
    pub session_idle_days: u64,
    /// Absolute lifetime of a session.
    pub session_max_days: u64,
    /// Days a soft-deleted attachment is kept.
    pub soft_delete_days: u64,
    /// Requests handled at once, server-wide (long polls excepted).
    pub max_concurrent_requests: u64,
    /// Key generations per vault.
    pub max_key_generations: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_request_bytes: 8 * 1024 * 1024,
            account_attachment_bytes: 1024 * 1024 * 1024,
            account_record_count: 50_000,
            account_record_bytes: 256 * 1024 * 1024,
            session_idle_days: 30,
            session_max_days: 180,
            soft_delete_days: 30,
            max_concurrent_requests: 512,
            max_key_generations: 10_000,
        }
    }
}

/// The server configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Address to listen on.
    pub listen: SocketAddr,
    /// `https://host[:port]`: the OPAQUE server identity, echoed by
    /// `GET /v1/info`.
    pub canonical_origin: String,
    /// Holds the database and the attachments.
    pub data_dir: PathBuf,
    /// The secret file (mode 0600, created by `init`).
    pub secret_file: PathBuf,
    /// Who may register.
    pub registration: RegistrationMode,
    /// Peers whose `X-Forwarded-For` is believed.
    #[serde(default = "default_proxies")]
    pub trusted_proxies: Vec<IpAddr>,
    /// Limits and quotas.
    #[serde(default)]
    pub limits: Limits,
}

fn default_proxies() -> Vec<IpAddr> {
    vec![
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ]
}

impl Config {
    /// Reads and validates a config file.
    ///
    /// # Errors
    /// [`ConfigError`].
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let meta = std::fs::metadata(path).map_err(|_| ConfigError::Read)?;
        if meta.len() > MAX_CONFIG_LEN {
            return Err(ConfigError::TooLarge);
        }
        let text = std::fs::read_to_string(path).map_err(|_| ConfigError::Read)?;
        Self::parse(&text)
    }

    /// Parses and validates config text.
    ///
    /// # Errors
    /// [`ConfigError`].
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config: Self =
            toml::from_str(text).map_err(|e| ConfigError::Parse(e.message().to_owned()))?;
        config.canonical_origin = normalize_origin(&config.canonical_origin)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let l = &self.limits;
        if l.max_request_bytes < MIN_REQUEST_BYTES || l.max_request_bytes > 64 * 1024 * 1024 {
            return Err(ConfigError::Invalid(
                "limits.max_request_bytes (2 MiB to 64 MiB)",
            ));
        }
        if l.account_attachment_bytes == 0
            || l.account_record_count == 0
            || l.account_record_bytes == 0
        {
            return Err(ConfigError::Invalid("limits quotas must be positive"));
        }
        if l.session_idle_days == 0 || l.session_idle_days > l.session_max_days {
            return Err(ConfigError::Invalid(
                "limits.session_idle_days must be 1..=session_max_days",
            ));
        }
        if l.session_max_days > 3650 || l.soft_delete_days == 0 || l.soft_delete_days > 3650 {
            return Err(ConfigError::Invalid("limits day counts (1 to 3650)"));
        }
        if !(1..=100_000).contains(&l.max_concurrent_requests) {
            return Err(ConfigError::Invalid(
                "limits.max_concurrent_requests (1 to 100000)",
            ));
        }
        if !(1..=1_000_000).contains(&l.max_key_generations) {
            return Err(ConfigError::Invalid(
                "limits.max_key_generations (1 to 1000000)",
            ));
        }
        Ok(())
    }
}

/// Validates the canonical origin and returns its canonical form:
/// `https://host[:port]`, lowercase, no path. `http://` is accepted only for
/// a loopback host (local testing).
///
/// # Errors
/// [`ConfigError::Invalid`].
pub fn normalize_origin(origin: &str) -> Result<String, ConfigError> {
    const BAD: ConfigError = ConfigError::Invalid("canonical_origin must be https://host[:port]");
    let (scheme, rest) = origin.split_once("://").ok_or(BAD)?;
    let scheme = scheme.to_ascii_lowercase();
    if rest.is_empty() || rest.contains(['/', '?', '#', '@', ' ', '\\']) {
        return Err(BAD);
    }
    let rest = rest.to_ascii_lowercase();
    let (host, port) = match rest.strip_prefix('[') {
        Some(v6) => {
            let (h, p) = v6.split_once(']').ok_or(BAD)?;
            let port = if p.is_empty() {
                None
            } else {
                Some(p.strip_prefix(':').ok_or(BAD)?)
            };
            (format!("[{h}]"), port)
        }
        None => match rest.split_once(':') {
            Some((h, p)) => (h.to_owned(), Some(p)),
            None => (rest.clone(), None),
        },
    };
    if host.is_empty() || host.starts_with('.') || host.ends_with('.') {
        return Err(BAD);
    }
    if let Some(p) = port
        && (p.is_empty() || p.parse::<u16>().is_err())
    {
        return Err(BAD);
    }
    let loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]");
    match scheme.as_str() {
        "https" => {}
        "http" if loopback => {}
        _ => return Err(BAD),
    }
    Ok(match port {
        Some(p) => format!("{scheme}://{host}:{p}"),
        None => format!("{scheme}://{host}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
listen = "127.0.0.1:8743"
canonical_origin = "https://Sync.Example.com"
data_dir = "/var/lib/mosaic"
secret_file = "/var/lib/mosaic/server.key"
registration = "invite"
"#;

    #[test]
    fn parses_and_canonicalizes() {
        let c = Config::parse(GOOD).unwrap();
        assert_eq!(c.canonical_origin, "https://sync.example.com");
        assert_eq!(c.limits, Limits::default());
        assert_eq!(c.trusted_proxies.len(), 2);
    }

    #[test]
    fn rejects_unknown_keys_and_bad_origins() {
        assert!(matches!(
            Config::parse(&format!("{GOOD}\nbogus = 1\n")),
            Err(ConfigError::Parse(_))
        ));
        assert!(matches!(
            Config::parse(&format!("{GOOD}\n[limits]\nbogus = 1\n")),
            Err(ConfigError::Parse(_))
        ));
        for bad in [
            "http://sync.example.com",
            "https://sync.example.com/",
            "https://sync.example.com/v1",
            "ftp://x",
            "sync.example.com",
            "https://",
            "https://u@h",
            "https://h:99999",
        ] {
            assert!(normalize_origin(bad).is_err(), "{bad}");
        }
        assert_eq!(
            normalize_origin("http://127.0.0.1:8743").unwrap(),
            "http://127.0.0.1:8743"
        );
        assert_eq!(
            normalize_origin("http://[::1]:1").unwrap(),
            "http://[::1]:1"
        );
    }
}

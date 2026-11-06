//! The secret file (M6 spec §6.3), created by `mosaic-server init` with mode
//! 0600:
//!
//! ```text
//! "MSRV" || version:u8 (1) || init_ms:u64 || server_secret[32]
//!        || setup_len:u32 || ServerSetup::serialize()
//! ```
//!
//! `server_secret` keys the pre-login fakes and the TOTP-at-rest key.
//! `ServerSetup` holds the OPAQUE OPRF seed and the server's static key pair.
//! Losing the file locks every account out of the server (the vault data on
//! the clients is unaffected); leaking it does not reveal a password, but
//! lets an attacker impersonate this server to a client that holds a login
//! record (back it up like a private key).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use mosaic_sync::wire::opaque::{OsRng, Suite, ke::ServerSetup};
use zeroize::Zeroizing;

const MAGIC: &[u8; 4] = b"MSRV";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 8 + 32 + 4;
const MAX_LEN: u64 = 4096;

/// Why the secret file is unusable.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SecretFileError {
    /// The file could not be read or written.
    #[error("cannot access the secret file")]
    Io,
    /// The file already exists (`init` never overwrites it).
    #[error("the secret file already exists")]
    Exists,
    /// The bytes are not a secret file of this version.
    #[error("the secret file is malformed")]
    Malformed,
    /// The OS random number generator failed.
    #[error("the random number generator failed")]
    Randomness,
}

/// The server's secrets.
pub struct ServerKeys {
    /// When the server was initialised (Unix ms).
    pub init_ms: u64,
    /// Keys the pre-login fakes and the TOTP-at-rest key.
    pub server_secret: Zeroizing<[u8; 32]>,
    /// The OPAQUE server setup.
    pub setup: ServerSetup<Suite>,
}

impl ServerKeys {
    /// Fresh random keys.
    ///
    /// # Errors
    /// [`SecretFileError::Randomness`].
    pub fn generate(init_ms: u64) -> Result<Self, SecretFileError> {
        let mut secret = Zeroizing::new([0u8; 32]);
        getrandom::fill(secret.as_mut_slice()).map_err(|_| SecretFileError::Randomness)?;
        Ok(Self {
            init_ms,
            server_secret: secret,
            setup: ServerSetup::<Suite>::new(&mut OsRng),
        })
    }

    /// The file bytes.
    #[must_use]
    pub fn encode(&self) -> Zeroizing<Vec<u8>> {
        let setup = Zeroizing::new(self.setup.serialize().to_vec());
        let mut out = Zeroizing::new(Vec::with_capacity(HEADER_LEN + setup.len()));
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&self.init_ms.to_be_bytes());
        out.extend_from_slice(self.server_secret.as_slice());
        out.extend_from_slice(&u32::try_from(setup.len()).unwrap_or(u32::MAX).to_be_bytes());
        out.extend_from_slice(&setup);
        out
    }

    /// Parses file bytes.
    ///
    /// # Errors
    /// [`SecretFileError::Malformed`].
    pub fn decode(bytes: &[u8]) -> Result<Self, SecretFileError> {
        let bad = || SecretFileError::Malformed;
        let rest = bytes.strip_prefix(MAGIC.as_slice()).ok_or_else(bad)?;
        let (&version, rest) = rest.split_first().ok_or_else(bad)?;
        if version != VERSION {
            return Err(bad());
        }
        let (init, rest) = rest.split_at_checked(8).ok_or_else(bad)?;
        let (secret, rest) = rest.split_at_checked(32).ok_or_else(bad)?;
        let (len, setup) = rest.split_at_checked(4).ok_or_else(bad)?;
        let len = u32::from_be_bytes(len.try_into().map_err(|_| bad())?);
        if usize::try_from(len).ok() != Some(setup.len()) {
            return Err(bad());
        }
        let mut server_secret = Zeroizing::new([0u8; 32]);
        server_secret.copy_from_slice(secret);
        Ok(Self {
            init_ms: u64::from_be_bytes(init.try_into().map_err(|_| bad())?),
            server_secret,
            setup: ServerSetup::<Suite>::deserialize(setup).map_err(|_| bad())?,
        })
    }

    /// Writes the file; fails if it exists.
    ///
    /// # Errors
    /// [`SecretFileError`].
    pub fn create(&self, path: &Path) -> Result<(), SecretFileError> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => SecretFileError::Exists,
            _ => SecretFileError::Io,
        })?;
        let bytes = self.encode();
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| SecretFileError::Io)
    }

    /// Reads the file.
    ///
    /// # Errors
    /// [`SecretFileError`].
    pub fn load(path: &Path) -> Result<Self, SecretFileError> {
        let meta = fs::metadata(path).map_err(|_| SecretFileError::Io)?;
        if meta.len() > MAX_LEN {
            return Err(SecretFileError::Malformed);
        }
        let bytes = Zeroizing::new(fs::read(path).map_err(|_| SecretFileError::Io)?);
        Self::decode(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_rejections() {
        let keys = ServerKeys::generate(123).unwrap();
        let bytes = keys.encode();
        let back = ServerKeys::decode(&bytes).unwrap();
        assert_eq!(back.init_ms, 123);
        assert_eq!(*back.server_secret, *keys.server_secret);
        assert_eq!(back.setup.serialize(), keys.setup.serialize());
        assert!(ServerKeys::decode(&bytes[..bytes.len() - 1]).is_err());
        let mut extra = bytes.to_vec();
        extra.push(0);
        assert!(ServerKeys::decode(&extra).is_err());
        let mut v2 = bytes.to_vec();
        v2[4] = 2;
        assert!(ServerKeys::decode(&v2).is_err());
        assert!(ServerKeys::decode(b"").is_err());
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("k");
        keys.create(&p).unwrap();
        assert!(matches!(keys.create(&p), Err(SecretFileError::Exists)));
        assert_eq!(ServerKeys::load(&p).unwrap().init_ms, 123);
    }
}

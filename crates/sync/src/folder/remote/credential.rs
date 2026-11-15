//! What the phone keeps to reach a provider account: an app password
//! (`WebDAV`) or a refresh token (Dropbox, Graph). Zeroizing, never `Clone`,
//! redacted in `Debug`, and never part of an error or a log.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::transport::TransportError;

/// The version byte of an encoded credential.
const VERSION: u8 = 1;
/// The longest field of an encoded credential, in bytes.
const MAX_FIELD: usize = 4096;

/// Where the folder lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Dropbox, over its API.
    Dropbox,
    /// `OneDrive`, over Microsoft Graph.
    OneDrive,
    /// A `WebDAV` server (Nextcloud or a typed URL).
    WebDav,
}

/// A provider account's secret.
pub enum ProviderCredential {
    /// A `WebDAV` login name and app password.
    WebDav {
        /// The login name.
        login: String,
        /// The app password.
        app_password: Zeroizing<String>,
    },
    /// A Dropbox refresh token.
    Dropbox {
        /// The refresh token.
        refresh_token: Zeroizing<String>,
    },
    /// A Microsoft Graph refresh token.
    Graph {
        /// The refresh token.
        refresh_token: Zeroizing<String>,
    },
}

impl core::fmt::Debug for ProviderCredential {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::WebDav { .. } => "ProviderCredential::WebDav([REDACTED])",
            Self::Dropbox { .. } => "ProviderCredential::Dropbox([REDACTED])",
            Self::Graph { .. } => "ProviderCredential::Graph([REDACTED])",
        })
    }
}

fn put(out: &mut Zeroizing<Vec<u8>>, field: &str) {
    let len = u16::try_from(field.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(field.as_bytes());
}

/// A cursor over the encoded bytes.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], TransportError> {
        let (head, rest) = self
            .0
            .split_at_checked(n)
            .ok_or(TransportError::Protocol)?;
        self.0 = rest;
        Ok(head)
    }

    fn field(&mut self) -> Result<Zeroizing<String>, TransportError> {
        let len = <[u8; 2]>::try_from(self.take(2)?).map_err(|_| TransportError::Protocol)?;
        let len = usize::from(u16::from_be_bytes(len));
        if len > MAX_FIELD {
            return Err(TransportError::Protocol);
        }
        let text = core::str::from_utf8(self.take(len)?).map_err(|_| TransportError::Protocol)?;
        Ok(Zeroizing::new(text.to_owned()))
    }
}

impl ProviderCredential {
    /// The provider this credential is for.
    #[must_use]
    pub const fn provider(&self) -> Provider {
        match self {
            Self::WebDav { .. } => Provider::WebDav,
            Self::Dropbox { .. } => Provider::Dropbox,
            Self::Graph { .. } => Provider::OneDrive,
        }
    }

    /// The bytes the secure storage keeps: version `0x01`, the kind (`1`
    /// `WebDAV`, `2` Dropbox, `3` Graph), then the fields as UTF-8 with a
    /// big-endian `u16` length each (`WebDAV`: login, password; the others:
    /// the token).
    #[must_use]
    pub fn encode(&self) -> Zeroizing<Vec<u8>> {
        let mut out = Zeroizing::new(vec![VERSION]);
        match self {
            Self::WebDav {
                login,
                app_password,
            } => {
                out.push(1);
                put(&mut out, login);
                put(&mut out, app_password);
            }
            Self::Dropbox { refresh_token } => {
                out.push(2);
                put(&mut out, refresh_token);
            }
            Self::Graph { refresh_token } => {
                out.push(3);
                put(&mut out, refresh_token);
            }
        }
        out
    }

    /// The inverse of [`encode`](Self::encode).
    ///
    /// # Errors
    /// [`TransportError::Protocol`] for any deviation: another version or
    /// kind, a field over 4096 bytes, bad UTF-8, a short or a longer input.
    pub fn decode(bytes: &[u8]) -> Result<Self, TransportError> {
        let mut r = Reader(bytes);
        let [version, kind] =
            <[u8; 2]>::try_from(r.take(2)?).map_err(|_| TransportError::Protocol)?;
        if version != VERSION {
            return Err(TransportError::Protocol);
        }
        let credential = match kind {
            1 => {
                let login = r.field()?;
                Self::WebDav {
                    login: login.as_str().to_owned(),
                    app_password: r.field()?,
                }
            }
            2 => Self::Dropbox {
                refresh_token: r.field()?,
            },
            3 => Self::Graph {
                refresh_token: r.field()?,
            },
            _ => return Err(TransportError::Protocol),
        };
        if r.0.is_empty() {
            Ok(credential)
        } else {
            Err(TransportError::Protocol)
        }
    }
}

/// Receives a credential that changed during a run (a rotated refresh
/// token), so it is stored before the old one is used again.
pub trait CredentialSink: Send + Sync {
    /// Stores the new credential.
    ///
    /// # Errors
    /// [`TransportError`] if it could not be stored.
    fn rotated(&self, credential: &ProviderCredential) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn webdav() -> ProviderCredential {
        ProviderCredential::WebDav {
            login: "alice".to_owned(),
            app_password: Zeroizing::new("abcde-fghij".to_owned()),
        }
    }

    #[test]
    fn credential_round_trip_and_strict_decode() {
        for c in [
            webdav(),
            ProviderCredential::Dropbox {
                refresh_token: Zeroizing::new("r-token".to_owned()),
            },
            ProviderCredential::Graph {
                refresh_token: Zeroizing::new("ü".repeat(2000)),
            },
        ] {
            let bytes = c.encode();
            let back = ProviderCredential::decode(&bytes).unwrap();
            assert_eq!(back.provider(), c.provider());
            assert_eq!(*back.encode(), *bytes);
        }
        let good = webdav().encode();
        let mut trailing = good.to_vec();
        trailing.push(0);
        let mut version2 = good.to_vec();
        version2[0] = 2;
        let mut kind4 = good.to_vec();
        kind4[1] = 4;
        let mut overlong = vec![VERSION, 2];
        overlong.extend_from_slice(&u16::try_from(MAX_FIELD + 1).unwrap().to_be_bytes());
        overlong.extend(std::iter::repeat_n(b'a', MAX_FIELD + 1));
        let truncated = good[..good.len() - 1].to_vec();
        let bad_utf8 = vec![VERSION, 2, 0, 1, 0xff];
        for bad in [trailing, version2, kind4, overlong, truncated, bad_utf8, vec![], vec![1]] {
            assert!(
                matches!(
                    ProviderCredential::decode(&bad),
                    Err(TransportError::Protocol)
                ),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn credential_debug_is_redacted() {
        let c = webdav();
        let text = format!("{c:?}");
        assert!(!text.contains("abcde"));
        assert!(!text.contains("alice"));
        assert_eq!(text, "ProviderCredential::WebDav([REDACTED])");
    }
}

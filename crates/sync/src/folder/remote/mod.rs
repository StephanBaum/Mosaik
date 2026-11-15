//! Provider stores for the folder backend (ADR-0162): a
//! [`FolderStore`](crate::folder::FolderStore) over a provider's HTTPS API, so the phone can
//! join and sync a folder account without a file-sync client.
//!
//! - [`http`]: the HTTP layer every provider store shares (TLS 1.3 only, no
//!   automatic redirects, response caps, the status mapping);
//! - [`time`]: the date formats of the providers;
//! - [`credential`], [`oauth`], [`location`]: what the phone keeps for a
//!   provider account, PKCE and callback parsing, and where the folder is;
//! - [`dropbox`]: the Dropbox API v2 store and its sign-in;
//! - [`webdav`], [`davxml`], [`nextcloud`]: the `WebDAV` store and Nextcloud's
//!   Login Flow v2.
//!
//! Nothing here logs. A `TransportError` carries no data from a provider, and
//! secrets live in zeroizing types that are never `Clone`.

pub mod credential;
pub mod davxml;
pub mod dropbox;
pub mod http;
pub mod location;
pub mod nextcloud;
pub mod oauth;
pub mod time;
pub(crate) mod tokens;
pub mod webdav;

/// The largest listing response that is read.
pub const MAX_LISTING_BYTES: u64 = 4 * 1024 * 1024;
/// The largest token, login-flow or metadata response that is read.
pub const MAX_SMALL_JSON: u64 = 64 * 1024;
/// The most continuation pages of one listing.
pub const MAX_PAGES: usize = 32;

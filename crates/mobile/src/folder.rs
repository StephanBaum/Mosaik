//! Folder accounts on the phone (ADR-0162): sign in to the provider that holds the
//! account's folder (Dropbox and `OneDrive` with PKCE in the system browser, Nextcloud with
//! Login Flow v2, or a typed `WebDAV` app password), find and browse the folder, and join the
//! account from it. The phone never creates, initialises or repairs a folder: it reads what a
//! desktop wrote, and then syncs like any device of the account.
//!
//! The provider credential lives in secure storage (`folder-credential/<account>`) once an
//! account exists; before that it waits in the pending link, which a lock drops (like the
//! pending join) and which runs out after five minutes. The mutex of the core is never held
//! across the network.

use std::sync::{Arc, Mutex};

use mosaic_core::{AccountId, MasterPassword, SecretKey};
use mosaic_sync::engine::{OwnedSession, SyncEngine};
use mosaic_sync::folder::remote::credential::{CredentialSink, Provider, ProviderCredential};
use mosaic_sync::folder::remote::dropbox::{
    DropboxAuth, DropboxStore, dropbox_authorize_url, dropbox_exchange,
};
use mosaic_sync::folder::remote::graph::{
    GraphAuth, GraphStore, graph_authorize_url, graph_browse, graph_exchange, graph_resolve_root,
};
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::location::FolderLocation;
use mosaic_sync::folder::remote::nextcloud::{LoginPoll, login_flow_begin, login_flow_poll};
use mosaic_sync::folder::remote::oauth::{
    AuthState, Endpoints, Pkce, REDIRECT_URI, parse_callback,
};
use mosaic_sync::folder::remote::webdav::WebDavStore;
use mosaic_sync::folder::{
    FolderContents, FolderStore, FolderTransport, JoinError, RelPath, has_account_in, inspect_in,
    join_from_folder_in,
};
use mosaic_sync::transport::TransportError;
use mosaic_sync::wire::SessionCrypto;
use mosaic_vault::storage::{SyncConfig, SyncKind};
use zeroize::Zeroizing;

use crate::Core;
use crate::config::Platform;
use crate::error::{MobileError, Result, folder_error};
use crate::join::{JoinFlight, clean_device_name, phone_origin};
use crate::secret::text_of;
use crate::state::{Phase, SessionState, UnlockKind};
use crate::storage::{SecureStorage, folder_credential_name, hex, secret_key_name};
use crate::unlock::Taken;

/// How long a pending link lives.
pub(crate) const LINK_TTL_MS: u64 = 300_000;

/// The most folders `folder_browse` lists.
const MAX_DIRS: usize = 200;

/// The most accounts a candidate lists.
const MAX_CANDIDATE_ACCOUNTS: usize = 64;

/// The Dropbox app key of this build (`MOSAIC_DROPBOX_APP_KEY` at compile time). Not a secret.
const DROPBOX_APP_KEY: Option<&str> = option_env!("MOSAIC_DROPBOX_APP_KEY");

/// The Microsoft application (client) ID of this build (`MOSAIC_MS_CLIENT_ID`). Not a secret.
const MS_CLIENT_ID: Option<&str> = option_env!("MOSAIC_MS_CLIENT_ID");

/// The cloud provider of a folder account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderProvider {
    /// Dropbox.
    Dropbox,
    /// `OneDrive` (personal or work/school).
    OneDrive,
    /// Nextcloud or another `WebDAV` server.
    WebDav,
}

impl From<FolderProvider> for Provider {
    fn from(p: FolderProvider) -> Self {
        match p {
            FolderProvider::Dropbox => Self::Dropbox,
            FolderProvider::OneDrive => Self::OneDrive,
            FolderProvider::WebDav => Self::WebDav,
        }
    }
}

impl From<Provider> for FolderProvider {
    fn from(p: Provider) -> Self {
        match p {
            Provider::Dropbox => Self::Dropbox,
            Provider::OneDrive => Self::OneDrive,
            Provider::WebDav => Self::WebDav,
        }
    }
}

/// How the platform completes a sign-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// The browser ends on the app's redirect: the platform passes that URL to
    /// [`Core::folder_link_finish`].
    Redirect,
    /// The user approves in the browser and the platform polls
    /// [`Core::folder_link_status`].
    Poll,
}

/// A started sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkInfo {
    /// The page to open in the system browser.
    pub url: String,
    /// How it ends.
    pub kind: LinkKind,
}

/// Where a sign-in stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    /// Waiting for the user.
    Waiting,
    /// Signed in: the platform lists and joins the folder.
    Linked,
    /// The sign-in ran out.
    Expired,
}

/// What a folder setup code says (the provider is a hint, the account is exact).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderSetupInfo {
    /// The folder account's ID, 32 lowercase hex characters.
    pub account: String,
    /// The provider, `None` for "other".
    pub provider: Option<FolderProvider>,
    /// The canonical `https` origin of the `WebDAV` server.
    pub server: Option<String>,
    /// The path hint below the provider root (`/Mosaic/Family`).
    pub path: Option<String>,
}

/// A folder that holds Mosaic accounts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderCandidate {
    /// The folder, as `folder_browse` and `join_folder` name it (`Mosaic/Family`).
    pub path: String,
    /// The accounts in it (32 hex characters each), at most 64.
    pub accounts: Vec<String>,
}

/// A folder below another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderDirEntry {
    /// Its name.
    pub name: String,
    /// Its path (`dir/name`).
    pub path: String,
}

/// What the person typed to join a folder account.
pub struct JoinFolderRequest {
    /// The folder, as `folder_locate` and `folder_browse` name it.
    pub path: String,
    /// The account, 32 hex characters.
    pub account: String,
    /// The master password, UTF-8.
    pub password: Vec<u8>,
    /// The Secret Key in its display form, UTF-8.
    pub secret_key: Vec<u8>,
    /// The name of this phone in the device list.
    pub device_name: Option<String>,
}

/// Where a folder account's folder is, for the settings screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderOverview {
    /// The provider.
    pub provider: FolderProvider,
    /// The folder (`/Mosaic/Family`).
    pub path: String,
}

/// Where a link or a store runs: the provider endpoints and client IDs. The
/// test hooks replace them.
#[derive(Clone)]
pub(crate) struct FolderEnv {
    pub(crate) endpoints: Endpoints,
    dropbox_key: Option<String>,
    ms_client: Option<String>,
    /// Loopback `http` is accepted (the host tests only).
    #[cfg(feature = "test-hooks")]
    loopback: bool,
}

impl FolderEnv {
    pub(crate) fn production() -> Self {
        let id = |v: Option<&str>| v.filter(|s| !s.is_empty()).map(str::to_owned);
        Self {
            endpoints: Endpoints::production(),
            dropbox_key: id(DROPBOX_APP_KEY),
            ms_client: id(MS_CLIENT_ID),
            #[cfg(feature = "test-hooks")]
            loopback: false,
        }
    }

    /// Every endpoint on one loopback server and these client IDs (empty: none).
    #[cfg(feature = "test-hooks")]
    pub(crate) fn loopback(origin: &str, dropbox_key: &str, ms_client: &str) -> Self {
        let id = |v: &str| (!v.is_empty()).then(|| v.to_owned());
        Self {
            endpoints: Endpoints::loopback(origin),
            dropbox_key: id(dropbox_key),
            ms_client: id(ms_client),
            loopback: true,
        }
    }

    pub(crate) fn http(&self) -> Result<RemoteHttp> {
        #[cfg(feature = "test-hooks")]
        if self.loopback {
            return RemoteHttp::new_loopback().map_err(folder_error);
        }
        RemoteHttp::new().map_err(folder_error)
    }

    pub(crate) fn dropbox_key(&self) -> Result<&str> {
        self.dropbox_key
            .as_deref()
            .ok_or(MobileError::FolderProviderUnsupported)
    }

    fn ms_client(&self) -> Result<&str> {
        self.ms_client
            .as_deref()
            .ok_or(MobileError::FolderProviderUnsupported)
    }
}

/// A copy of a credential (encoded and decoded: the copy is zeroizing too).
fn dup(
    credential: &ProviderCredential,
) -> core::result::Result<ProviderCredential, TransportError> {
    ProviderCredential::decode(&credential.encode())
}

/// The credential of a pending link, shared with the sink that hears of a rotated refresh
/// token, so a rotation during browsing is not lost.
#[derive(Clone, Default)]
pub(crate) struct CredCell(Arc<Mutex<Option<ProviderCredential>>>);

impl CredCell {
    fn set(&self, credential: ProviderCredential) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(credential);
        }
    }

    fn is_set(&self) -> bool {
        self.0.lock().is_ok_and(|slot| slot.is_some())
    }

    fn get(&self) -> Option<ProviderCredential> {
        dup(self.0.lock().ok()?.as_ref()?).ok()
    }
}

impl CredentialSink for CredCell {
    fn rotated(&self, credential: &ProviderCredential) -> core::result::Result<(), TransportError> {
        let copy = dup(credential)?;
        let mut slot = self.0.lock().map_err(|_| TransportError::Io)?;
        *slot = Some(copy);
        Ok(())
    }
}

/// Stores a rotated credential in secure storage, before the new access token is used.
pub(crate) struct SecureSink {
    pub storage: Arc<dyn SecureStorage>,
    pub name: String,
}

impl CredentialSink for SecureSink {
    fn rotated(&self, credential: &ProviderCredential) -> core::result::Result<(), TransportError> {
        self.storage
            .put(&self.name, &credential.encode())
            .map_err(|_| TransportError::Io)
    }
}

/// Ignores a rotated credential (a grant that is about to be revoked).
pub(crate) struct NoSink;

impl CredentialSink for NoSink {
    fn rotated(&self, _: &ProviderCredential) -> core::result::Result<(), TransportError> {
        Ok(())
    }
}

/// A sign-in in progress, or done and waiting for the account to join.
pub(crate) struct PendingLink {
    provider: Provider,
    pkce: Option<Pkce>,
    state: Option<AuthState>,
    poll: Option<LoginPoll>,
    /// The canonical origin of the `WebDAV` server the flow runs on.
    server: Option<String>,
    /// The collection URL of the `WebDAV` account.
    dav_base: Option<String>,
    credential: CredCell,
    /// Dropbox: the team space the account's root is.
    path_root: Option<String>,
    expires_at_ms: u64,
    /// Signing in again to the folder of the existing account.
    relink: bool,
}

impl PendingLink {
    fn new(provider: Provider, now_ms: u64, relink: bool) -> Self {
        Self {
            provider,
            pkce: None,
            state: None,
            poll: None,
            server: None,
            dav_base: None,
            credential: CredCell::default(),
            path_root: None,
            expires_at_ms: now_ms.saturating_add(LINK_TTL_MS),
            relink,
        }
    }

    /// Whether the link ran out at `now_ms`.
    pub(crate) const fn expired(&self, now_ms: u64) -> bool {
        now_ms > self.expires_at_ms
    }
}

/// What a folder operation of the link phase needs of the pending link.
struct LinkCtx {
    provider: Provider,
    dav_base: Option<String>,
    path_root: Option<String>,
    credential: CredCell,
}

/// The kept engine of a folder account: built on the first run after an unlock, valid for the
/// unlock it was built in (`generation`).
pub(crate) struct KeptEngine {
    pub generation: u64,
    pub engine: FolderEngine,
}

/// The engine of a folder account over the unlocked session.
pub(crate) type FolderEngine = SyncEngine<FolderTransport<SessionCrypto<OwnedSession>>>;

/// `path` as the components `a/b` (no leading or trailing `/`, `""` is the provider root).
///
/// # Errors
/// `invalidInput` for a component the folder format does not accept.
fn normalize_path(path: &str) -> Result<String> {
    if path.len() > 2048 {
        return Err(MobileError::InvalidInput);
    }
    let mut rel = RelPath::root();
    for component in path.split('/').filter(|c| !c.is_empty()) {
        rel = rel.child(component).ok_or(MobileError::InvalidInput)?;
    }
    Ok(rel.components().collect::<Vec<_>>().join("/"))
}

fn rel_of(path: &str) -> Result<RelPath> {
    let mut rel = RelPath::root();
    for component in path.split('/').filter(|c| !c.is_empty()) {
        rel = rel.child(component).ok_or(MobileError::InvalidInput)?;
    }
    Ok(rel)
}

/// The account of 32 hex characters.
fn parse_account(text: &str) -> Result<AccountId> {
    if text.len() != 32 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(MobileError::InvalidInput);
    }
    let mut bytes = [0u8; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        let pair = text
            .get(i * 2..i * 2 + 2)
            .ok_or(MobileError::InvalidInput)?;
        *byte = u8::from_str_radix(pair, 16).map_err(|_| MobileError::InvalidInput)?;
    }
    AccountId::from_bytes(bytes).map_err(|_| MobileError::InvalidInput)
}

/// The folders to offer from the names of a directory's subfolders: valid components only,
/// sorted, at most [`MAX_DIRS`].
fn dir_entries(dir: &str, names: impl IntoIterator<Item = String>) -> Vec<FolderDirEntry> {
    let mut names: Vec<String> = names
        .into_iter()
        .filter(|n| RelPath::root().child(n).is_some())
        .collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .take(MAX_DIRS)
        .map(|name| FolderDirEntry {
            path: if dir.is_empty() {
                name.clone()
            } else {
                format!("{dir}/{name}")
            },
            name,
        })
        .collect()
}

/// `https://host/dir`, trimmed and without a trailing `/`, as a typed `WebDAV` address.
fn typed_dav_url(text: &str) -> Result<String> {
    let url = text.trim().trim_end_matches('/');
    if url.is_empty() || url.contains(['?', '#']) {
        return Err(MobileError::SyncServerInvalid);
    }
    Ok(url.to_owned())
}

/// The origin of a Nextcloud collection URL (`<origin>/remote.php/dav/files/<login>`); `None`
/// for any other `WebDAV` address, which no app password is revoked on.
pub(crate) fn nextcloud_origin(base: &str) -> Option<String> {
    let (origin, _) = base.split_once("/remote.php/dav/files/")?;
    mosaic_sync::http::canonical_origin(origin).ok()
}

/// A store for `location` with `credential`; a rotated credential goes to `sink`.
pub(crate) fn open_store(
    env: &FolderEnv,
    http: RemoteHttp,
    location: &FolderLocation,
    credential: ProviderCredential,
    sink: Arc<dyn CredentialSink>,
) -> Result<Box<dyn FolderStore>> {
    Ok(match location.provider {
        Provider::Dropbox => {
            let auth = DropboxAuth::new(env.dropbox_key()?.to_owned(), credential, sink);
            Box::new(DropboxStore::new(http, &env.endpoints, location, auth).map_err(folder_error)?)
        }
        Provider::OneDrive => {
            let auth = GraphAuth::new(env.ms_client()?.to_owned(), credential, sink);
            Box::new(GraphStore::new(http, &env.endpoints, location, auth).map_err(folder_error)?)
        }
        Provider::WebDav => {
            let base = location
                .server
                .as_deref()
                .ok_or(MobileError::FolderProviderUnsupported)?;
            Box::new(
                WebDavStore::new(http, base, &location.root, credential).map_err(folder_error)?,
            )
        }
    })
}

impl Core {
    pub(crate) fn folder_env(&self) -> Result<FolderEnv> {
        Ok(self
            .folder_env
            .lock()
            .map_err(|_| MobileError::Internal)?
            .clone())
    }

    fn user_agent(&self) -> &'static str {
        match self.config.platform {
            Platform::Android => "Mosaic Android",
            Platform::Ios => "Mosaic iOS",
        }
    }

    /// The location of the folder when the phone is unlocked on a folder account (a sign-in is
    /// then a sign in again); `None` when there is no account (a first sign-in).
    fn link_relink_target(&self) -> Result<Option<FolderLocation>> {
        let unlocked = {
            let guard = self.guard()?;
            match &guard.phase {
                Phase::NoAccount(_) => false,
                Phase::Unlocked(_) => true,
                Phase::Locked(_) => return Err(MobileError::Locked),
                Phase::Closed => return Err(MobileError::StoreUnavailable),
                Phase::Unlocking | Phase::Busy => return Err(MobileError::Busy),
            }
        };
        if !unlocked {
            return Ok(None);
        }
        let (config, remote) = self.with_session(|s| Ok((s.sync_config()?, s.folder_remote()?)))?;
        match (config, remote) {
            (Some(c), Some(text)) if c.kind == SyncKind::Folder => FolderLocation::decode(&text)
                .map(Some)
                .map_err(|_| MobileError::FolderProviderUnsupported),
            _ => Err(MobileError::InvalidInput),
        }
    }

    /// Puts a link back after a call that left it usable, unless a lock came meanwhile.
    fn keep_link(&self, link: PendingLink, generation: u64) {
        if let Ok(mut guard) = self.guard()
            && self.generation() == generation
        {
            guard.slots.link = Some(link);
        }
    }

    /// Takes the pending link for a call that changes it.
    fn take_link(&self) -> Result<(PendingLink, u64)> {
        let now = self.now_ms();
        let mut guard = self.guard()?;
        let generation = self.generation();
        let link = guard
            .slots
            .link
            .take()
            .ok_or(MobileError::FolderLinkInvalid)?;
        if link.expired(now) {
            return Err(MobileError::FolderLinkExpired);
        }
        Ok((link, generation))
    }

    /// What the folder calls of the link phase need: the pending link, signed in, not a sign in
    /// again.
    fn link_ctx(&self) -> Result<LinkCtx> {
        let now = self.now_ms();
        let mut guard = self.guard()?;
        let Some(link) = guard.slots.link.as_ref() else {
            return Err(MobileError::NotFound);
        };
        if link.expired(now) {
            guard.slots.link = None;
            return Err(MobileError::FolderLinkExpired);
        }
        if link.relink || !link.credential.is_set() {
            return Err(MobileError::NotFound);
        }
        Ok(LinkCtx {
            provider: link.provider,
            dav_base: link.dav_base.clone(),
            path_root: link.path_root.clone(),
            credential: link.credential.clone(),
        })
    }

    /// A store at `root` (`a/b`) for the signed-in link and the location it stands for. `OneDrive`
    /// resolves the folder first (`folderNotFound` if it is not there).
    fn link_store(
        &self,
        ctx: &LinkCtx,
        root: &str,
    ) -> Result<(Box<dyn FolderStore>, FolderLocation)> {
        let env = self.folder_env()?;
        let http = env.http()?;
        let sink: Arc<dyn CredentialSink> = Arc::new(ctx.credential.clone());
        let credential = |c: &CredCell| c.get().ok_or(MobileError::NotFound);
        let location = match ctx.provider {
            Provider::Dropbox => FolderLocation {
                provider: Provider::Dropbox,
                root: root.to_owned(),
                server: None,
                drive_id: None,
                item_id: None,
                path_root: ctx.path_root.clone(),
            },
            Provider::WebDav => FolderLocation {
                provider: Provider::WebDav,
                root: root.to_owned(),
                server: Some(ctx.dav_base.clone().ok_or(MobileError::NotFound)?),
                drive_id: None,
                item_id: None,
                path_root: None,
            },
            Provider::OneDrive => {
                let auth = GraphAuth::new(
                    env.ms_client()?.to_owned(),
                    credential(&ctx.credential)?,
                    Arc::clone(&sink),
                );
                graph_resolve_root(&http, &env.endpoints, &auth, root).map_err(folder_error)?
            }
        };
        // A rotation during the resolve is in the cell: the store starts from the newest.
        let store = open_store(&env, http, &location, credential(&ctx.credential)?, sink)?;
        Ok((store, location))
    }

    /// Starts a sign-in with a provider.
    ///
    /// Without an account this is the first sign-in of a join. Unlocked on a folder account it
    /// signs in again (to the same provider): `folderLinkFinish`, `folderLinkStatus` and
    /// `folderLinkPassword` then check that the folder still holds the account before they
    /// store anything.
    ///
    /// # Errors
    /// `invalidInput` (another provider than the account's, or no server for `WebDAV`),
    /// `folderProviderUnsupported` (no client ID in this build), `locked`, `syncServerInvalid`,
    /// `syncServerInsecure`, `folderServerTlsTooOld`, `syncServerUnreachable`, `busy`.
    pub fn folder_link_begin(
        &self,
        provider: FolderProvider,
        server: Option<String>,
    ) -> Result<LinkInfo> {
        self.require_app()?;
        let relink = self.link_relink_target()?;
        let provider = Provider::from(provider);
        if relink.as_ref().is_some_and(|l| l.provider != provider) {
            return Err(MobileError::InvalidInput);
        }
        let generation = self.generation();
        let env = self.folder_env()?;
        let mut link = PendingLink::new(provider, self.now_ms(), relink.is_some());
        let info = match provider {
            Provider::Dropbox | Provider::OneDrive => {
                let pkce = Pkce::new().map_err(folder_error)?;
                let state = AuthState::new().map_err(folder_error)?;
                let url = if provider == Provider::Dropbox {
                    dropbox_authorize_url(&env.endpoints, env.dropbox_key()?, &pkce, &state)
                } else {
                    graph_authorize_url(&env.endpoints, env.ms_client()?, &pkce, &state)
                };
                link.pkce = Some(pkce);
                link.state = Some(state);
                LinkInfo {
                    url,
                    kind: LinkKind::Redirect,
                }
            }
            Provider::WebDav => {
                let origin = match &relink {
                    Some(location) => {
                        let stored = location.server.as_deref().and_then(nextcloud_origin);
                        let stored = stored.ok_or(MobileError::InvalidInput)?;
                        if let Some(typed) = server
                            && phone_origin(&typed)? != stored
                        {
                            return Err(MobileError::InvalidInput);
                        }
                        stored
                    }
                    None => phone_origin(&server.ok_or(MobileError::InvalidInput)?)?,
                };
                let http = env.http()?;
                let started =
                    login_flow_begin(&http, &origin, self.user_agent()).map_err(folder_error)?;
                link.poll = Some(started.poll);
                link.server = Some(origin);
                LinkInfo {
                    url: started.login_url,
                    kind: LinkKind::Poll,
                }
            }
        };
        let mut guard = self.guard()?;
        if relink.is_some() && self.generation() != generation {
            return Err(MobileError::Locked);
        }
        guard.slots.link = Some(link);
        drop(guard);
        Ok(info)
    }

    /// Finishes a sign-in of kind `Redirect` with the URL the browser ended on.
    ///
    /// # Errors
    /// `folderLinkInvalid` (no link, a denied or forged answer, a code the provider refuses; the
    /// link is gone), `folderLinkExpired`, `syncFolderNoAccount` (a sign in again to a folder
    /// without the account), `syncServerUnreachable` and the other network codes (the link
    /// stays).
    pub fn folder_link_finish(&self, callback_url: String) -> Result<()> {
        self.require_app()?;
        let (mut link, generation) = self.take_link()?;
        let callback = parse_callback(&callback_url, REDIRECT_URI)
            .map_err(|_| MobileError::FolderLinkInvalid)?;
        let (Some(state), Some(pkce)) = (link.state.as_ref(), link.pkce.as_ref()) else {
            return Err(MobileError::FolderLinkInvalid);
        };
        if !state.matches(&callback.state) {
            return Err(MobileError::FolderLinkInvalid);
        }
        let env = self.folder_env()?;
        let http = env.http()?;
        let exchanged = match link.provider {
            Provider::Dropbox => dropbox_exchange(
                &http,
                &env.endpoints,
                env.dropbox_key()?,
                &callback.code,
                pkce,
            ),
            Provider::OneDrive => graph_exchange(
                &http,
                &env.endpoints,
                env.ms_client()?,
                &callback.code,
                pkce,
            )
            .map(|credential| (credential, None)),
            Provider::WebDav => return Err(MobileError::FolderLinkInvalid),
        };
        self.work_pause();
        match exchanged {
            Ok((credential, path_root)) => {
                link.credential.set(credential);
                link.path_root = path_root;
                link.state = None;
                link.pkce = None;
                if link.relink {
                    self.complete_relink(&link)
                } else {
                    self.keep_link(link, generation);
                    Ok(())
                }
            }
            Err(error) => {
                if matches!(
                    error,
                    TransportError::Unreachable
                        | TransportError::Timeout
                        | TransportError::RateLimited { .. }
                ) {
                    // The code is probably unused: the same answer can be sent again.
                    self.keep_link(link, generation);
                }
                Err(match error {
                    TransportError::Unauthorized => MobileError::FolderLinkInvalid,
                    other => folder_error(other),
                })
            }
        }
    }

    /// Where the sign-in stands. For `Poll` it asks Nextcloud once; for `Redirect` it only
    /// reports whether the answer was taken.
    ///
    /// # Errors
    /// `notFound` (no link), `folderLinkInvalid` (the server's answer is not the flow's; the
    /// link is gone), `syncFolderNoAccount` (a sign in again to a folder without the account) and
    /// the network codes (the link stays).
    pub fn folder_link_status(&self) -> Result<LinkStatus> {
        self.require_app()?;
        let now = self.now_ms();
        let (mut link, generation) = {
            let guard = &mut *self.guard()?;
            let generation = self.generation();
            let link = guard.slots.link.take().ok_or(MobileError::NotFound)?;
            (link, generation)
        };
        if link.expired(now) {
            return Ok(LinkStatus::Expired);
        }
        if link.credential.is_set() {
            self.keep_link(link, generation);
            return Ok(LinkStatus::Linked);
        }
        let Some(poll) = link.poll.as_ref() else {
            self.keep_link(link, generation);
            return Ok(LinkStatus::Waiting);
        };
        let http = self.folder_env()?.http()?;
        let polled = login_flow_poll(&http, poll);
        self.work_pause();
        match polled {
            Ok(None) => {
                self.keep_link(link, generation);
                Ok(LinkStatus::Waiting)
            }
            Ok(Some((credential, base))) => {
                link.credential.set(credential);
                link.dav_base = Some(base);
                link.poll = None;
                if link.relink {
                    self.complete_relink(&link).map(|()| LinkStatus::Linked)
                } else {
                    self.keep_link(link, generation);
                    Ok(LinkStatus::Linked)
                }
            }
            Err(TransportError::Protocol) => Err(MobileError::FolderLinkInvalid),
            Err(error) => {
                self.keep_link(link, generation);
                Err(folder_error(error))
            }
        }
    }

    /// Signs in to a `WebDAV` server with a typed address, login and app password: the address
    /// must be an `https` collection URL that answers a `PROPFIND` for the account.
    ///
    /// # Errors
    /// `emptyPassword`, `invalidInput`, `syncServerInvalid`, `syncServerInsecure`,
    /// `folderProviderSignedOut` (the server refuses the login), `folderNotFound` (the address is
    /// not a collection), `folderServerTlsTooOld`, `syncServerUnreachable`,
    /// `syncFolderNoAccount` (a sign in again to a folder without the account).
    pub fn folder_link_password(
        &self,
        dav_url: String,
        login: String,
        password: Vec<u8>,
    ) -> Result<()> {
        self.require_app()?;
        let password = text_of(password)?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        let login = login.trim().to_owned();
        if login.is_empty() || login.contains(':') {
            return Err(MobileError::InvalidInput);
        }
        let relink = self.link_relink_target()?;
        if relink
            .as_ref()
            .is_some_and(|l| l.provider != Provider::WebDav)
        {
            return Err(MobileError::InvalidInput);
        }
        let base = typed_dav_url(&dav_url)?;
        if let Some(location) = &relink
            && location.server.as_deref() != Some(base.as_str())
        {
            return Err(MobileError::InvalidInput);
        }
        let generation = self.generation();
        let http = self.folder_env()?.http()?;
        let credential = ProviderCredential::WebDav {
            login,
            app_password: Zeroizing::new(password.to_string()),
        };
        let store = WebDavStore::new(http, &base, "", dup(&credential).map_err(folder_error)?)
            .map_err(MobileError::from)?;
        // One `PROPFIND` of the collection itself: the login and the address are right.
        match store.stat(&RelPath::root()) {
            Ok(Some(entry)) if entry.is_dir() => {}
            Ok(_) => return Err(MobileError::FolderNotFound),
            Err(e) => return Err(folder_error(e)),
        }
        self.work_pause();
        let mut link = PendingLink::new(Provider::WebDav, self.now_ms(), relink.is_some());
        link.dav_base = Some(base);
        link.credential.set(credential);
        if link.relink {
            self.complete_relink(&link)
        } else {
            let mut guard = self.guard()?;
            if self.generation() != generation {
                return Err(MobileError::Locked);
            }
            guard.slots.link = Some(link);
            Ok(())
        }
    }

    /// Drops the pending sign-in.
    pub fn folder_link_cancel(&self) {
        if let Ok(mut guard) = self.guard() {
            guard.slots.link = None;
        }
    }

    /// Ends a sign in again: the folder must still hold this account, then the new credential
    /// replaces the old one. Nothing is stored otherwise.
    fn complete_relink(&self, link: &PendingLink) -> Result<()> {
        use mosaic_sync::wire::SessionHandle as _;
        let owned = self.owned_session()?;
        let (account, remote) = owned
            .with_session(|s| (s.account_id(), s.folder_remote()))
            .ok_or(MobileError::Locked)?;
        let mut location =
            FolderLocation::decode(&remote?.ok_or(MobileError::FolderProviderUnsupported)?)
                .map_err(|_| MobileError::FolderProviderUnsupported)?;
        if link.provider == Provider::Dropbox {
            // A sign in again may land on another namespace: the account decides it.
            location.path_root.clone_from(&link.path_root);
        }
        let credential = link.credential.get().ok_or(MobileError::NotFound)?;
        let env = self.folder_env()?;
        let http = env.http()?;
        let sink: Arc<dyn CredentialSink> = Arc::new(link.credential.clone());
        let store = open_store(
            &env,
            http,
            &location,
            dup(&credential).map_err(folder_error)?,
            sink,
        )?;
        self.work_pause();
        if let Err(e) = store.stat(&RelPath::root()) {
            return Err(folder_error(e));
        }
        if !has_account_in(store.as_ref(), &account) {
            return Err(MobileError::SyncFolderNoAccount);
        }
        let name = folder_credential_name(&account);
        let newest = link.credential.get().ok_or(MobileError::NotFound)?;
        self.storage.put(&name, &newest.encode())?;
        self.note_stored(None, Some(true));
        if let Ok(mut guard) = self.guard() {
            guard.folder_engine = None;
            guard.last_sync_error = None;
        }
        Ok(())
    }

    /// Whether a folder is the folder of a Mosaic account: checks only the folder at
    /// `path_hint` (`Mosaic/Family`, or the provider root without a hint); there is no search of
    /// the drive.
    ///
    /// # Errors
    /// `notFound` (no signed-in link), `folderLinkExpired`, `invalidInput`,
    /// `folderProviderSignedOut` and the network codes.
    pub fn folder_locate(&self, path_hint: Option<String>) -> Result<Vec<FolderCandidate>> {
        self.require_app()?;
        let root = normalize_path(&path_hint.unwrap_or_default())?;
        let ctx = self.link_ctx()?;
        let (store, _) = match self.link_store(&ctx, &root) {
            Ok(opened) => opened,
            Err(MobileError::FolderNotFound) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        self.work_pause();
        // A refused grant is an error, a missing folder is no candidate.
        match store.stat(&RelPath::root()) {
            Ok(Some(_)) => {}
            Ok(None) | Err(TransportError::NotFound | TransportError::FolderUnavailable) => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(folder_error(e)),
        }
        Ok(match inspect_in(store.as_ref()) {
            FolderContents::Mosaic { accounts, .. } => vec![FolderCandidate {
                path: root,
                accounts: accounts
                    .iter()
                    .take(MAX_CANDIDATE_ACCOUNTS)
                    .map(|a| hex(a.as_bytes()))
                    .collect(),
            }],
            _ => Vec::new(),
        })
    }

    /// The folders directly below `dir` (`""` is the provider root), at most 200, by name.
    ///
    /// # Errors
    /// `notFound` (no signed-in link), `folderLinkExpired`, `invalidInput`,
    /// `folderNotFound`, `folderProviderSignedOut` and the network codes.
    pub fn folder_browse(&self, dir: String) -> Result<Vec<FolderDirEntry>> {
        self.require_app()?;
        let dir = normalize_path(&dir)?;
        let ctx = self.link_ctx()?;
        let names: Vec<String> = if ctx.provider == Provider::OneDrive {
            let env = self.folder_env()?;
            let http = env.http()?;
            let sink: Arc<dyn CredentialSink> = Arc::new(ctx.credential.clone());
            let auth = GraphAuth::new(
                env.ms_client()?.to_owned(),
                ctx.credential.get().ok_or(MobileError::NotFound)?,
                sink,
            );
            let target = if dir.is_empty() {
                None
            } else {
                Some(graph_resolve_root(&http, &env.endpoints, &auth, &dir).map_err(folder_error)?)
            };
            let ids = target
                .as_ref()
                .map(|t| (t.drive_id.as_deref(), t.item_id.as_deref()));
            let (drive, item) = ids.unwrap_or((None, None));
            graph_browse(&http, &env.endpoints, &auth, drive, item)
                .map_err(folder_error)?
                .into_iter()
                .map(|(name, _)| name)
                .collect()
        } else {
            let (store, _) = self.link_store(&ctx, "")?;
            store
                .list(&rel_of(&dir)?)
                .map_err(folder_error)?
                .into_iter()
                .filter(mosaic_sync::folder::Entry::is_dir)
                .filter_map(|e| e.name.to_str().map(str::to_owned))
                .collect()
        };
        self.work_pause();
        Ok(dir_entries(&dir, names))
    }

    /// Joins a folder account from its folder with the master password and the Secret Key. The
    /// phone only reads the folder (it never creates or initialises one). The join counts once
    /// the account is stored; the first pull follows, and a failure of it is `last_error`.
    ///
    /// # Errors
    /// `notFound` (no signed-in link), `folderLinkExpired`, `accountExists`, `invalidInput`,
    /// `emptyPassword`, `secretKeyMalformed`, `secretKeyMistyped`, `folderNotFound` (not a Mosaic
    /// folder), `syncFolderNoAccount`, `syncFolderDamaged`, `wrongCredentials` (the link stays
    /// usable), `busy`, `storageUnavailable`.
    pub fn join_folder(&self, request: JoinFolderRequest) -> Result<SessionState> {
        self.require_app()?;
        let JoinFolderRequest {
            path,
            account,
            password,
            secret_key,
            device_name,
        } = request;
        let mut password = text_of(password)?;
        let secret_key = SecretKey::parse(&text_of(secret_key)?)?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        let account = parse_account(&account)?;
        let root = normalize_path(&path)?;
        let device_name = clean_device_name(self.config.platform, device_name);
        if matches!(self.guard()?.phase, Phase::Locked(_) | Phase::Unlocked(_)) {
            return Err(MobileError::AccountExists);
        }
        let ctx = self.link_ctx()?;
        let (store, location) = self.link_store(&ctx, &root)?;
        let locked = match self.take_locked()? {
            Taken::NoAccount(ls) => ls,
            other @ Taken::Locked(_) => {
                self.put_back(other);
                return Err(MobileError::AccountExists);
            }
        };
        let flight = JoinFlight::start(self);
        let master = MasterPassword::new(std::mem::take(&mut *password));
        let joined = join_from_folder_in(store.as_ref(), account, locked, &master, &secret_key);
        self.work_pause();
        drop(store);
        let mut session = match joined {
            Ok(session) => session,
            Err(failure) => {
                let failure = *failure;
                match failure.session {
                    Some(session) => self.put_back(Taken::NoAccount(session)),
                    None => self.start_over(),
                }
                return Err(join_error(failure.error));
            }
        };
        let display = format!("/{root}");
        let config = SyncConfig {
            kind: SyncKind::Folder,
            server_url: None,
            email: None,
            folder_path: Some(display.into()),
            device_name,
        };
        let credential_name = folder_credential_name(&account);
        let stored = session
            .set_sync_config(Some(&config))
            .and_then(|()| session.set_folder_remote(Some(&location.encode())))
            .map_err(MobileError::from)
            .and_then(|()| {
                self.storage
                    .put(&secret_key_name(&account), secret_key.expose_secret())
                    .map_err(MobileError::from)
            })
            .and_then(|()| {
                let newest = ctx.credential.get().ok_or(MobileError::NotFound)?;
                self.storage
                    .put(&credential_name, &newest.encode())
                    .map_err(MobileError::from)
            });
        if let Err(e) = stored {
            // Without its Secret Key and its credential the account is of no use: start over.
            drop(session.lock());
            let _ = self.storage.delete(&secret_key_name(&account));
            let _ = self.storage.delete(&credential_name);
            self.note_stored(Some(false), Some(false));
            self.start_over();
            return Err(e);
        }
        self.note_stored(Some(true), Some(true));
        self.record_boot(&mut session);
        let state = self.finish_unlocked(session, UnlockKind::Password)?;
        drop(flight);
        if let Ok(mut guard) = self.guard() {
            guard.slots.link = None;
        }
        // The join counts; the first pull reports its failure in the overview.
        if matches!(state, SessionState::Unlocked { .. }) {
            let _ = self.sync_recorded();
        }
        Ok(self.state())
    }

    /// Wipes the store and opens a fresh one (a join that cannot be undone half way).
    fn start_over(&self) {
        let _ = self.wipe_files();
        let (phase, busy) = crate::open_phase(&self.config).unwrap_or((Phase::Closed, false));
        if let Ok(mut guard) = self.guard() {
            guard.phase = phase;
            guard.store_busy = busy;
            guard.settle_fresh_phase();
        }
    }
}

/// The error of a failed join from a folder.
fn join_error(error: JoinError) -> MobileError {
    match error {
        JoinError::NoAccountFile => MobileError::SyncFolderNoAccount,
        JoinError::FolderDamaged => MobileError::SyncFolderDamaged,
        JoinError::WrongCredentials => MobileError::WrongCredentials,
        JoinError::Folder(_) => MobileError::FolderNotFound,
        JoinError::Session(e) => e.into(),
        _ => MobileError::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_entries_skip_invalid_names_dedupe_and_cap() {
        let mut names: Vec<String> = (0..250).map(|i| format!("d{i:03}")).collect();
        names.extend(["a:b", "..", "x\ty", "", "d000"].map(str::to_owned));
        let listed = dir_entries("Mosaic", names);
        assert_eq!(listed.len(), MAX_DIRS);
        assert_eq!(listed[0].path, "Mosaic/d000");
        assert!(
            listed
                .iter()
                .all(|e| e.path == format!("Mosaic/{}", e.name))
        );
        assert_eq!(
            dir_entries("", ["b".to_owned(), "a".to_owned()])[1].path,
            "b"
        );
    }

    #[test]
    fn paths_and_accounts_are_checked() {
        assert_eq!(
            normalize_path("/Mosaic//Family/").as_deref(),
            Ok("Mosaic/Family")
        );
        assert_eq!(normalize_path("").as_deref(), Ok(""));
        for bad in ["a/../b", "a/./b", "a\\b", "a:b"] {
            assert_eq!(normalize_path(bad), Err(MobileError::InvalidInput), "{bad}");
        }
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(hex(parse_account(id).unwrap().as_bytes()), id);
        for bad in [
            "",
            "ABCDEF0123456789abcdef0123456789",
            "0123",
            &format!("{id}0"),
        ] {
            assert_eq!(
                parse_account(bad).err(),
                Some(MobileError::InvalidInput),
                "{bad}"
            );
        }
    }

    #[test]
    fn nextcloud_origin_needs_the_login_flow_path() {
        assert_eq!(
            nextcloud_origin("https://cloud.example.com/remote.php/dav/files/alice").as_deref(),
            Some("https://cloud.example.com")
        );
        assert_eq!(
            nextcloud_origin("https://dav.example.com/files/alice"),
            None
        );
    }
}

//! A [`FolderStore`] over Microsoft Graph (`OneDrive`): children listings, path
//! addressed content, upload sessions, folder creation and delete, always on
//! `drives/<drive>/items/<root>:/<RelPath>:` for the folder the user chose, so
//! what the server names cannot lead out of it. The app has the full
//! `Files.ReadWrite` scope; this file is what keeps it inside the folder
//! (ADR-0162).
//!
//! A write is atomic on the server (a simple `PUT`, or an upload session that
//! commits with its last chunk) and write-once names use
//! `conflictBehavior=fail`. A delete carries `If-Match` with the entry's
//! `eTag` and never touches a folder that was not looked at. A download is
//! one redirect to a pre-authenticated link that is fetched without the
//! `Authorization` header.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::credential::{CredentialSink, Provider, ProviderCredential};
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest, status_error};
use super::location::FolderLocation;
use super::oauth::{AuthState, Endpoints, Pkce, REDIRECT_URI, path_segment};
use super::time::parse_rfc3339;
use super::tokens::{bearer, grant_is_gone, parse_tokens, post_form};
use super::{MAX_LISTING_BYTES, MAX_PAGES, MAX_SMALL_JSON};
use crate::folder::store::{Entry, EntryKind, FolderStore, MAX_SCANNED, RelPath, valid_component};
use crate::http::client::{ATTACHMENT_TIMEOUT, REQUEST_TIMEOUT};
use crate::transport::TransportError;

/// The largest body of a simple `PUT`.
const SIMPLE_UPLOAD_MAX: usize = 4 * 1024 * 1024;
/// The size of one upload-session chunk: a multiple of 320 KiB.
pub const GRAPH_CHUNK: usize = 10 * 320 * 1024;
/// Children asked for per listing page.
const PAGE_TOP: u32 = 200;
/// The fields of an item that are asked for.
const SELECT: &str = "name,size,lastModifiedDateTime,eTag,file,folder,package,remoteItem,deleted";
/// The longest id of a drive or an item.
const MAX_ID: usize = 512;

/// What authorises the requests of a [`GraphStore`]: the refresh token of the
/// account and the access token that came from it (kept in memory only).
///
/// Microsoft rotates the refresh token: when an answer carries a new one it is
/// handed to the [`CredentialSink`] and must be stored (`Ok`) **before** the
/// new access token is used. If the sink fails the request fails with
/// [`TransportError::Io`] and the old credential stays in place. A refresh the
/// server refuses (`invalid_grant`, 401) is [`TransportError::Unauthorized`].
pub struct GraphAuth {
    client_id: String,
    sink: Option<Arc<dyn CredentialSink>>,
    inner: Mutex<AuthInner>,
    /// The client and the token endpoint, set by whatever uses this.
    net: Mutex<Option<(RemoteHttp, String)>>,
}

struct AuthInner {
    access: Option<Zeroizing<String>>,
    refresh: Option<Zeroizing<String>>,
}

impl core::fmt::Debug for GraphAuth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("GraphAuth([REDACTED])")
    }
}

impl GraphAuth {
    /// An authorisation from the stored credential; `sink` stores the
    /// refresh token the provider replaces.
    #[must_use]
    pub fn new(
        client_id: String,
        credential: ProviderCredential,
        sink: Arc<dyn CredentialSink>,
    ) -> Self {
        let refresh = match credential {
            ProviderCredential::Graph { refresh_token } => Some(refresh_token),
            _ => None,
        };
        Self {
            client_id,
            sink: Some(sink),
            inner: Mutex::new(AuthInner {
                access: None,
                refresh,
            }),
            net: Mutex::new(None),
        }
    }

    /// An authorisation with one fixed access token that is never refreshed.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    pub fn fixed_for_test(token: &str) -> Self {
        Self {
            client_id: String::new(),
            sink: None,
            inner: Mutex::new(AuthInner {
                access: Some(Zeroizing::new(token.to_owned())),
                refresh: None,
            }),
            net: Mutex::new(None),
        }
    }

    /// Where the refresh goes.
    fn bind(&self, http: &RemoteHttp, endpoints: &Endpoints) {
        if let Ok(mut net) = self.net.lock() {
            *net = Some((
                http.clone(),
                format!("{}/token", endpoints.ms_login.trim_end_matches('/')),
            ));
        }
    }

    /// The `Authorization` header value (marked sensitive); an access token
    /// is fetched first if there is none.
    ///
    /// # Errors
    /// [`TransportError::Unauthorized`] without an access token and a usable
    /// refresh token; [`TransportError::Io`] for a token that cannot be a
    /// header value; the errors of the refresh.
    pub fn bearer(&self) -> Result<HeaderValue, TransportError> {
        let has_token = self
            .inner
            .lock()
            .map_err(|_| TransportError::Io)?
            .access
            .is_some();
        if !has_token {
            self.refresh()?;
        }
        let guard = self.inner.lock().map_err(|_| TransportError::Io)?;
        bearer(guard.access.as_ref().ok_or(TransportError::Unauthorized)?)
    }

    /// Gets a new access token with the refresh token (after a 401, or when
    /// there is none yet).
    pub(crate) fn refresh(&self) -> Result<(), TransportError> {
        let (http, url) = self
            .net
            .lock()
            .map_err(|_| TransportError::Io)?
            .clone()
            .ok_or(TransportError::Unauthorized)?;
        let mut inner = self.inner.lock().map_err(|_| TransportError::Io)?;
        let refresh = inner.refresh.clone().ok_or(TransportError::Unauthorized)?;
        inner.access = None;
        let reply = post_form(
            &http,
            &url,
            &[
                ("client_id", &self.client_id),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
                ("scope", SCOPES),
            ],
        )?;
        if !reply.is_success() {
            return Err(if grant_is_gone(&reply) {
                TransportError::Unauthorized
            } else {
                status_error(&reply)
            });
        }
        let tokens = parse_tokens(reply.body)?;
        if let Some(new) = tokens.refresh
            && *new != *refresh
        {
            // Stored before the new access token is used. If it cannot be
            // stored, the old credential stays and the request fails.
            let sink = self.sink.as_ref().ok_or(TransportError::Io)?;
            sink.rotated(&ProviderCredential::Graph {
                refresh_token: new.clone(),
            })
            .map_err(|_| TransportError::Io)?;
            inner.refresh = Some(new);
        }
        inner.access = Some(tokens.access);
        Ok(())
    }
}

/// The scopes the app asks for, separated by spaces.
const SCOPES: &str = "Files.ReadWrite offline_access";

/// The page the user signs in on: `S256` PKCE, the app's one redirect URI,
/// `Files.ReadWrite` and `offline_access`.
#[must_use]
pub fn graph_authorize_url(
    endpoints: &Endpoints,
    client_id: &str,
    pkce: &Pkce,
    state: &AuthState,
) -> String {
    format!(
        "{}/authorize?client_id={}&response_type=code&redirect_uri={}&response_mode=query\
         &scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        endpoints.ms_login.trim_end_matches('/'),
        path_segment(client_id),
        path_segment(REDIRECT_URI),
        path_segment(SCOPES),
        path_segment(&state.as_param()),
        path_segment(&pkce.challenge()),
    )
}

/// Trades the code of the redirect for a refresh credential.
///
/// # Errors
/// [`TransportError::Unauthorized`] for a code the server refuses;
/// [`TransportError::Protocol`] for an answer without a refresh token; the
/// usual transport errors.
pub fn graph_exchange(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    client_id: &str,
    code: &str,
    pkce: &Pkce,
) -> Result<ProviderCredential, TransportError> {
    let reply = post_form(
        http,
        &format!("{}/token", endpoints.ms_login.trim_end_matches('/')),
        &[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", pkce.verifier()),
            ("scope", SCOPES),
        ],
    )?;
    if !reply.is_success() {
        return Err(if grant_is_gone(&reply) {
            TransportError::Unauthorized
        } else {
            status_error(&reply)
        });
    }
    let tokens = parse_tokens(reply.body)?;
    let refresh_token = tokens.refresh.ok_or(TransportError::Protocol)?;
    Ok(ProviderCredential::Graph { refresh_token })
}

/// A `GET` with the bearer (one refresh and retry after a 401).
fn authed_get(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    auth: &GraphAuth,
    url: &str,
) -> Result<RemoteReply, TransportError> {
    auth.bind(http, endpoints);
    let mut refreshed = false;
    loop {
        let reply = http.send(RemoteRequest {
            method: Method::GET,
            url,
            headers: &[(AUTHORIZATION, auth.bearer()?)],
            body: RemoteBody::None,
            timeout: REQUEST_TIMEOUT,
            cap: MAX_LISTING_BYTES,
        })?;
        if reply.status == 401 && !refreshed {
            refreshed = true;
            auth.refresh()?;
            continue;
        }
        return Ok(reply);
    }
}

#[derive(Deserialize)]
struct WireRoot {
    id: String,
    #[serde(default)]
    folder: Option<Value>,
    #[serde(default)]
    package: Option<Value>,
    #[serde(rename = "remoteItem", default)]
    remote_item: Option<Value>,
    #[serde(rename = "parentReference", default)]
    parent: Option<WireParent>,
}

#[derive(Deserialize)]
struct WireParent {
    #[serde(rename = "driveId", default)]
    drive_id: String,
}

/// Finds the folder at `path` (below the root of the user's own drive) and
/// returns the ids that address it from then on.
///
/// # Errors
/// [`TransportError::Protocol`] for a path with an invalid component;
/// [`TransportError::FolderUnavailable`] if there is no folder at `path`; the
/// usual transport errors.
pub fn graph_resolve_root(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    auth: &GraphAuth,
    path: &str,
) -> Result<FolderLocation, TransportError> {
    let comps: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
    if comps.iter().any(|c| !valid_component(c)) {
        return Err(TransportError::Protocol);
    }
    let graph = endpoints.graph.trim_end_matches('/');
    let url = if comps.is_empty() {
        format!("{graph}/me/drive/root")
    } else {
        let parts: Vec<String> = comps.iter().map(|c| path_segment(c)).collect();
        format!("{graph}/me/drive/root:/{}:", parts.join("/"))
    };
    let reply = authed_get(
        http,
        endpoints,
        auth,
        &format!("{url}?$select=id,folder,package,remoteItem,parentReference"),
    )?;
    if reply.status == 404 {
        return Err(TransportError::FolderUnavailable);
    }
    if !reply.is_success() {
        return Err(status_error(&reply));
    }
    let item: WireRoot =
        serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
    let drive_id = item.parent.map(|p| p.drive_id).unwrap_or_default();
    if item.folder.is_none()
        || item.package.is_some()
        || item.remote_item.is_some()
        || !valid_id(&item.id)
        || !valid_id(&drive_id)
    {
        return Err(TransportError::FolderUnavailable);
    }
    Ok(FolderLocation {
        provider: Provider::OneDrive,
        root: format!("/{}", comps.join("/")),
        server: None,
        drive_id: Some(drive_id),
        item_id: Some(item.id),
        path_root: None,
    })
}

/// The most folders `graph_browse` returns.
pub const MAX_BROWSE: usize = 200;

#[derive(Deserialize)]
struct WireBrowse {
    #[serde(default)]
    value: Vec<WireFolder>,
}

#[derive(Deserialize)]
struct WireFolder {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    folder: Option<Value>,
    #[serde(default)]
    package: Option<Value>,
    #[serde(rename = "remoteItem", default)]
    remote_item: Option<Value>,
}

/// The folders (name and item id) directly below an item: the root of the
/// user's drive without ids, else the item `item_id` of `drive_id`. At most
/// [`MAX_BROWSE`], sorted by name; shortcuts and packages are left out.
///
/// # Errors
/// [`TransportError::Protocol`] for an id that is not usable;
/// [`TransportError::FolderUnavailable`] if the item is gone; the usual
/// transport errors.
pub fn graph_browse(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    auth: &GraphAuth,
    drive_id: Option<&str>,
    item_id: Option<&str>,
) -> Result<Vec<(String, String)>, TransportError> {
    let graph = endpoints.graph.trim_end_matches('/');
    let base = match (drive_id, item_id) {
        (None, None) => format!("{graph}/me/drive/root"),
        (Some(d), Some(i)) if valid_id(d) && valid_id(i) => {
            format!(
                "{graph}/drives/{}/items/{}",
                path_segment(d),
                path_segment(i)
            )
        }
        _ => return Err(TransportError::Protocol),
    };
    let url =
        format!("{base}/children?$select=id,name,folder,package,remoteItem&$top={MAX_BROWSE}");
    let reply = authed_get(http, endpoints, auth, &url)?;
    if reply.status == 404 {
        return Err(TransportError::FolderUnavailable);
    }
    if !reply.is_success() {
        return Err(status_error(&reply));
    }
    let page: WireBrowse =
        serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
    let mut out: Vec<(String, String)> = page
        .value
        .into_iter()
        .filter(|f| {
            f.folder.is_some()
                && f.package.is_none()
                && f.remote_item.is_none()
                && valid_component(&f.name)
                && valid_id(&f.id)
        })
        .map(|f| (f.name, f.id))
        .take(MAX_BROWSE)
        .collect();
    out.sort();
    Ok(out)
}

#[derive(Default)]
struct State {
    in_run: bool,
    listings: HashMap<RelPath, Vec<Entry>>,
    /// Directories known to exist in this run (created or found).
    dirs: HashSet<RelPath>,
}

/// A folder on `OneDrive`.
pub struct GraphStore {
    http: RemoteHttp,
    /// `https://graph.microsoft.com/v1.0` (no trailing slash).
    graph: String,
    drive: String,
    item: String,
    auth: GraphAuth,
    state: Mutex<State>,
}

impl core::fmt::Debug for GraphStore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GraphStore").finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct WireItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(rename = "lastModifiedDateTime", default)]
    modified: Option<String>,
    #[serde(rename = "eTag", default)]
    etag: Option<String>,
    #[serde(default)]
    file: Option<Value>,
    #[serde(default)]
    folder: Option<Value>,
    #[serde(default)]
    package: Option<Value>,
    #[serde(rename = "remoteItem", default)]
    remote_item: Option<Value>,
    #[serde(default)]
    deleted: Option<Value>,
}

#[derive(Deserialize)]
struct WirePage {
    #[serde(default)]
    value: Vec<WireItem>,
    #[serde(rename = "@odata.nextLink", default)]
    next_link: Option<String>,
}

#[derive(Deserialize)]
struct WireError {
    error: WireErrorBody,
}

#[derive(Deserialize)]
struct WireErrorBody {
    #[serde(default)]
    code: String,
}

#[derive(Deserialize)]
struct WireSession {
    #[serde(rename = "uploadUrl")]
    upload_url: String,
}

fn error_code(reply: &RemoteReply) -> String {
    serde_json::from_slice::<WireError>(&reply.body)
        .map(|e| e.error.code)
        .unwrap_or_default()
}

/// The error of a failure answer. A name that is taken is `Io` (the write-once
/// rule), a full drive `QuotaExceeded`.
fn api_error(reply: &RemoteReply) -> TransportError {
    let code = error_code(reply);
    if reply.status == 507 || code.eq_ignore_ascii_case("quotaLimitReached") {
        TransportError::QuotaExceeded
    } else if code.eq_ignore_ascii_case("nameAlreadyExists")
        || matches!(reply.status, 409 | 412 | 423)
    {
        TransportError::Io
    } else {
        status_error(reply)
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID && !id.chars().any(|c| c.is_control() || c == '/')
}

/// The entry of a wire item; `None` for what is not a plain file or folder.
fn entry_of(item: WireItem, name: &str) -> Option<Entry> {
    if item.package.is_some() || item.remote_item.is_some() || item.deleted.is_some() {
        return None;
    }
    let kind = match (item.file.is_some(), item.folder.is_some()) {
        (true, false) => EntryKind::File,
        (false, true) => EntryKind::Dir,
        _ => return None,
    };
    Some(Entry {
        name: name.into(),
        kind,
        len: if kind == EntryKind::File {
            item.size.unwrap_or(0)
        } else {
            0
        },
        modified: item.modified.as_deref().and_then(parse_rfc3339),
        version: item.etag.map(|t| t.into_bytes().into_boxed_slice()),
    })
}

impl GraphStore {
    /// A store for the folder of `location` (`drive_id` and `item_id` name it).
    ///
    /// # Errors
    /// [`TransportError::Protocol`] for a location of another provider or
    /// without a usable drive or item id.
    pub fn new(
        http: RemoteHttp,
        endpoints: &Endpoints,
        location: &FolderLocation,
        auth: GraphAuth,
    ) -> Result<Self, TransportError> {
        if location.provider != Provider::OneDrive {
            return Err(TransportError::Protocol);
        }
        http.check(&endpoints.graph)?;
        let drive = location.drive_id.clone().ok_or(TransportError::Protocol)?;
        let item = location.item_id.clone().ok_or(TransportError::Protocol)?;
        if !valid_id(&drive) || !valid_id(&item) {
            return Err(TransportError::Protocol);
        }
        auth.bind(&http, endpoints);
        Ok(Self {
            http,
            graph: endpoints.graph.trim_end_matches('/').to_owned(),
            drive,
            item,
            auth,
            state: Mutex::new(State::default()),
        })
    }

    fn state(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        self.state.lock().map_err(|_| TransportError::Io)
    }

    /// `<graph>/drives/<drive>/`, the only place a link may lead to.
    fn drive_prefixes(&self) -> [String; 2] {
        [
            format!("{}/drives/{}/", self.graph, path_segment(&self.drive)),
            format!("{}/drives/{}/", self.graph, self.drive),
        ]
    }

    /// The URL of the item `rel`: the folder itself or a path below it.
    fn item_url(&self, rel: &RelPath) -> String {
        let mut url = format!(
            "{}/drives/{}/items/{}",
            self.graph,
            path_segment(&self.drive),
            path_segment(&self.item)
        );
        if rel.file_name().is_some() {
            url.push_str(":/");
            let parts: Vec<String> = rel.components().map(path_segment).collect();
            url.push_str(&parts.join("/"));
            url.push(':');
        }
        url
    }

    /// `<item>/<tail>`; a path-addressed item needs `:/` before a segment.
    fn sub_url(&self, rel: &RelPath, tail: &str) -> String {
        format!("{}/{tail}", self.item_url(rel))
    }

    /// One request with the bearer, and one refresh and retry after a 401.
    fn call(
        &self,
        method: &Method,
        url: &str,
        extra: &[(HeaderName, HeaderValue)],
        body: &RemoteBody<'_>,
        timeout: Duration,
        cap: u64,
    ) -> Result<RemoteReply, TransportError> {
        let mut refreshed = false;
        loop {
            let mut headers = Vec::with_capacity(extra.len() + 1);
            headers.push((AUTHORIZATION, self.auth.bearer()?));
            headers.extend(extra.iter().cloned());
            let reply = self.http.send(RemoteRequest {
                method: method.clone(),
                url,
                headers: &headers,
                body: match body {
                    RemoteBody::None => RemoteBody::None,
                    RemoteBody::Bytes(b, t) => RemoteBody::Bytes(b, t),
                    RemoteBody::Form(f) => RemoteBody::Form(f.clone()),
                },
                timeout,
                cap,
            })?;
            if reply.status == 401 && !refreshed {
                refreshed = true;
                self.auth.refresh()?;
                continue;
            }
            return Ok(reply);
        }
    }

    fn json_request(
        &self,
        method: &Method,
        url: &str,
        body: &Value,
        cap: u64,
    ) -> Result<RemoteReply, TransportError> {
        let bytes = Zeroizing::new(body.to_string().into_bytes());
        self.call(
            method,
            url,
            &[],
            &RemoteBody::Bytes(&bytes, "application/json"),
            REQUEST_TIMEOUT,
            cap,
        )
    }

    fn fetch_list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        let mut url = format!(
            "{}?$select={SELECT}&$top={PAGE_TOP}",
            self.sub_url(dir, "children")
        );
        let prefixes = self.drive_prefixes();
        let mut entries: Vec<Entry> = Vec::new();
        let mut seen = HashSet::new();
        let mut scanned = 0usize;
        for _ in 0..MAX_PAGES {
            let reply = self.call(
                &Method::GET,
                &url,
                &[],
                &RemoteBody::None,
                REQUEST_TIMEOUT,
                MAX_LISTING_BYTES,
            )?;
            if reply.status == 404 && scanned == 0 {
                return Ok(Vec::new());
            }
            if !reply.is_success() {
                return Err(api_error(&reply));
            }
            let page: WirePage =
                serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
            scanned = scanned.saturating_add(page.value.len());
            if scanned > MAX_SCANNED {
                return Err(TransportError::Protocol);
            }
            for item in page.value {
                let name = item.name.clone();
                if dir.child(&name).is_some()
                    && seen.insert(name.clone())
                    && let Some(entry) = entry_of(item, &name)
                {
                    entries.push(entry);
                }
            }
            match page.next_link {
                None => {
                    entries.sort_by(|a, b| a.name.cmp(&b.name));
                    return Ok(entries);
                }
                Some(link) => {
                    // Only a link into this drive is followed.
                    if !prefixes.iter().any(|p| link.starts_with(p.as_str())) {
                        return Err(TransportError::Protocol);
                    }
                    url = link;
                }
            }
        }
        Err(TransportError::Protocol)
    }

    /// Records a write in this run's listing of `dir`, so a lagging listing
    /// does not make it vanish from the run's own later `list` answers.
    fn note_write(&self, dir: &RelPath, entry: Entry) {
        let Ok(mut state) = self.state() else {
            return;
        };
        if !state.in_run {
            return;
        }
        state.dirs.insert(dir.clone());
        if let Some(list) = state.listings.get_mut(dir) {
            list.retain(|e| e.name != entry.name);
            list.push(entry);
            list.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    fn note_removed(&self, path: &RelPath) {
        let Ok(mut state) = self.state() else {
            return;
        };
        if let (Some(dir), Some(name)) = (path.parent(), path.file_name())
            && let Some(list) = state.listings.get_mut(&dir)
        {
            list.retain(|e| e.name.to_str() != Some(name));
        }
        state.dirs.remove(path);
        state.listings.remove(path);
    }

    /// The entry of `path` as this run's listing knows it, if it has an
    /// `eTag`; otherwise a direct `stat`.
    fn known_entry(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        if let (Ok(state), Some(dir), Some(name)) = (self.state(), path.parent(), path.file_name())
            && let Some(e) = state
                .listings
                .get(&dir)
                .and_then(|l| l.iter().find(|e| e.name.to_str() == Some(name)))
            && e.version.is_some()
        {
            return Ok(Some(e.clone()));
        }
        self.stat(path)
    }

    fn delete(&self, path: &RelPath, etag: Option<&[u8]>) -> Result<(), TransportError> {
        // The chosen folder itself is never deleted.
        if path.file_name().is_none() {
            return Err(TransportError::Protocol);
        }
        let mut headers = Vec::new();
        if let Some(tag) = etag {
            let value = HeaderValue::from_bytes(tag).map_err(|_| TransportError::Protocol)?;
            headers.push((HeaderName::from_static("if-match"), value));
        }
        let reply = self.call(
            &Method::DELETE,
            &self.item_url(path),
            &headers,
            &RemoteBody::None,
            REQUEST_TIMEOUT,
            MAX_SMALL_JSON,
        )?;
        if reply.is_success() || reply.status == 404 {
            self.note_removed(path);
            Ok(())
        } else {
            Err(api_error(&reply))
        }
    }

    fn ensure_dir(&self, dir: &RelPath) -> Result<(), TransportError> {
        let mut parent = RelPath::root();
        for component in dir.components() {
            let current = parent.known(component);
            if !self.state()?.dirs.contains(&current) {
                let reply = self.json_request(
                    &Method::POST,
                    &self.sub_url(&parent, "children"),
                    &json!({
                        "name": component,
                        "folder": {},
                        "@microsoft.graph.conflictBehavior": "fail",
                    }),
                    MAX_SMALL_JSON,
                )?;
                // A folder that is there already is fine; a file on the name is not.
                let there_already = reply.status == 409 && self.is_real_dir(&current);
                if !(reply.is_success() || there_already) {
                    return Err(api_error(&reply));
                }
                let mut state = self.state()?;
                if state.in_run {
                    state.dirs.insert(current.clone());
                }
            }
            parent = current;
        }
        Ok(())
    }

    /// The entry in the answer of an upload, checked against the name.
    fn uploaded(name: &str, body: &[u8], len: usize) -> Result<Entry, TransportError> {
        let item: WireItem = serde_json::from_slice(body).map_err(|_| TransportError::Protocol)?;
        if !item.name.eq_ignore_ascii_case(name) {
            return Err(TransportError::Protocol);
        }
        let mut entry = entry_of(item, name).ok_or(TransportError::Protocol)?;
        if entry.kind != EntryKind::File {
            return Err(TransportError::Protocol);
        }
        entry.len = u64::try_from(len).unwrap_or(u64::MAX);
        entry.modified = entry.modified.or_else(|| Some(SystemTime::now()));
        Ok(entry)
    }

    /// A simple `PUT` of at most 4 MiB.
    fn put_simple(
        &self,
        target: &RelPath,
        bytes: &[u8],
        behavior: &str,
    ) -> Result<RemoteReply, TransportError> {
        let url = format!(
            "{}?@microsoft.graph.conflictBehavior={behavior}",
            self.sub_url(target, "content")
        );
        self.call(
            &Method::PUT,
            &url,
            &[],
            &RemoteBody::Bytes(bytes, "application/octet-stream"),
            ATTACHMENT_TIMEOUT,
            MAX_SMALL_JSON,
        )
    }

    /// An upload session: chunks of [`GRAPH_CHUNK`] bytes to the session URL
    /// (sent without `Authorization`); the last chunk commits the file.
    fn put_session(
        &self,
        target: &RelPath,
        bytes: &[u8],
        behavior: &str,
    ) -> Result<RemoteReply, TransportError> {
        let reply = self.json_request(
            &Method::POST,
            &self.sub_url(target, "createUploadSession"),
            &json!({ "item": { "@microsoft.graph.conflictBehavior": behavior } }),
            MAX_SMALL_JSON,
        )?;
        if !reply.is_success() {
            return Err(api_error(&reply));
        }
        let session: WireSession =
            serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
        // https only (or loopback in tests), no user information.
        self.http.check(&session.upload_url)?;
        let total = bytes.len();
        let mut start = 0usize;
        let mut last = None;
        while start < total {
            let end = (start + GRAPH_CHUNK).min(total);
            let chunk = bytes.get(start..end).ok_or(TransportError::Io)?;
            let range = HeaderValue::from_str(&format!("bytes {start}-{}/{total}", end - 1))
                .map_err(|_| TransportError::Io)?;
            let sent = self.http.send(RemoteRequest {
                method: Method::PUT,
                url: &session.upload_url,
                headers: &[(HeaderName::from_static("content-range"), range)],
                body: RemoteBody::Bytes(chunk, "application/octet-stream"),
                timeout: ATTACHMENT_TIMEOUT,
                cap: MAX_SMALL_JSON,
            });
            let reply = match sent {
                Ok(r) if r.is_success() => r,
                Ok(r) => {
                    self.cancel_session(&session.upload_url);
                    return Err(api_error(&r));
                }
                Err(e) => {
                    self.cancel_session(&session.upload_url);
                    return Err(e);
                }
            };
            start = end;
            if matches!(reply.status, 200 | 201) {
                if start != total {
                    return Err(TransportError::Protocol);
                }
                last = Some(reply);
            } else if reply.status != 202 || start == total {
                self.cancel_session(&session.upload_url);
                return Err(TransportError::Protocol);
            }
        }
        last.ok_or(TransportError::Protocol)
    }

    /// Cancels an upload session (best effort, without `Authorization`).
    fn cancel_session(&self, url: &str) {
        let _ = self.http.send(RemoteRequest {
            method: Method::DELETE,
            url,
            headers: &[],
            body: RemoteBody::None,
            timeout: REQUEST_TIMEOUT,
            cap: MAX_SMALL_JSON,
        });
    }
}

impl FolderStore for GraphStore {
    fn begin_run(&self) -> Result<(), TransportError> {
        let mut state = self.state()?;
        state.in_run = true;
        state.listings.clear();
        state.dirs.clear();
        Ok(())
    }

    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        {
            let state = self.state()?;
            if state.in_run
                && let Some(cached) = state.listings.get(dir)
            {
                return Ok(cached.clone());
            }
        }
        let entries = self.fetch_list(dir)?;
        let mut state = self.state()?;
        if state.in_run {
            state.listings.insert(dir.clone(), entries.clone());
        }
        Ok(entries)
    }

    fn list_capped(
        &self,
        dir: &RelPath,
        limit: usize,
        keep: &dyn Fn(&Entry) -> bool,
    ) -> Result<Vec<Entry>, TransportError> {
        let mut entries = self.list(dir)?;
        entries.retain(|e| keep(e));
        entries.truncate(limit);
        Ok(entries)
    }

    fn read_capped(&self, path: &RelPath, max: u64) -> Result<Option<Vec<u8>>, TransportError> {
        let url = self.sub_url(path, "content");
        let mut refreshed = false;
        loop {
            let reply = self.http.get_following_one_redirect(
                &url,
                Some(&self.auth.bearer()?),
                ATTACHMENT_TIMEOUT,
                max,
            )?;
            if reply.status == 401 && !refreshed {
                refreshed = true;
                self.auth.refresh()?;
                continue;
            }
            return match reply.status {
                200 => Ok(Some(reply.body)),
                // Missing, or a folder (it has no content).
                400 | 404 => Ok(None),
                _ if reply.is_success() => Err(TransportError::Protocol),
                _ => Err(api_error(&reply)),
            };
        }
    }

    fn write_atomic(
        &self,
        dir: &RelPath,
        name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), TransportError> {
        let target = dir.child(name).ok_or(TransportError::Protocol)?;
        self.ensure_dir(dir)?;
        let behavior = if replace { "replace" } else { "fail" };
        let reply = if bytes.len() <= SIMPLE_UPLOAD_MAX {
            self.put_simple(&target, bytes, behavior)?
        } else {
            self.put_session(&target, bytes, behavior)?
        };
        if !matches!(reply.status, 200 | 201) {
            return Err(if reply.is_success() {
                TransportError::Protocol
            } else {
                api_error(&reply)
            });
        }
        let entry = Self::uploaded(name, &reply.body, bytes.len())?;
        self.note_write(dir, entry);
        Ok(())
    }

    /// Deletes a file with `If-Match`. A folder on the name, or a file that
    /// changed since it was looked at, is never deleted.
    fn remove(&self, path: &RelPath) -> Result<(), TransportError> {
        match self.known_entry(path)? {
            None => Ok(()),
            Some(e) if e.kind != EntryKind::File => Ok(()),
            Some(e) => self.delete(path, e.version.as_deref()),
        }
    }

    fn remove_empty_dir(&self, path: &RelPath) {
        let Ok(Some(entry)) = self.stat(path) else {
            return;
        };
        if entry.is_dir() && matches!(self.fetch_list(path), Ok(list) if list.is_empty()) {
            let _ = self.delete(path, entry.version.as_deref());
        }
    }

    fn remove_squatter(&self, path: &RelPath) {
        let _ = self.delete(path, None);
    }

    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        let url = format!("{}?$select={SELECT}", self.item_url(path));
        let reply = self.call(
            &Method::GET,
            &url,
            &[],
            &RemoteBody::None,
            REQUEST_TIMEOUT,
            MAX_SMALL_JSON,
        )?;
        if reply.status == 404 {
            return Ok(None);
        }
        if !reply.is_success() {
            return Err(api_error(&reply));
        }
        let item: WireItem =
            serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
        if let Some(name) = path.file_name()
            && !item.name.eq_ignore_ascii_case(name)
        {
            return Err(TransportError::Protocol);
        }
        let name = path.file_name().unwrap_or_default().to_owned();
        // A shortcut or a package is not a file or a folder of Mosaic's.
        if item.package.is_some() || item.remote_item.is_some() {
            return Ok(Some(Entry {
                name: name.into(),
                kind: EntryKind::Link,
                len: 0,
                modified: None,
                version: None,
            }));
        }
        Ok(entry_of(item, &name))
    }

    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError> {
        self.ensure_dir(dir)
    }
}

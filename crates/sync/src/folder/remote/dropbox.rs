//! A [`FolderStore`] over the Dropbox API v2: `list_folder`, `download`,
//! `upload`, `get_metadata`, `create_folder_v2` and `delete_v2`, always on a
//! path built from the chosen folder and a [`RelPath`], so what the server
//! names cannot lead out of the folder. The app has the *Full Dropbox*
//! scope; this file is what keeps it inside the folder (ADR-0162).
//!
//! A run starts with one recursive listing that every later `list` of the run
//! answers from (plus the run's own writes). An upload is atomic on the
//! server: the final name appears whole or not at all. A delete carries the
//! listed `rev` and never touches a folder that was not looked at.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::credential::{CredentialSink, ProviderCredential};
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest, status_error};
use super::location::FolderLocation;
use super::oauth::{AuthState, Endpoints, Pkce, REDIRECT_URI, path_segment};
use super::time::parse_rfc3339;
use super::tokens::{bearer, grant_is_gone, parse_tokens, post_form};
use super::{MAX_LISTING_BYTES, MAX_PAGES, MAX_SMALL_JSON};
use crate::folder::remote::credential::Provider;
use crate::folder::store::{Entry, EntryKind, FolderStore, MAX_SCANNED, RelPath, valid_component};
use crate::http::client::{ATTACHMENT_TIMEOUT, REQUEST_TIMEOUT};
use crate::transport::TransportError;

/// The most entries of the recursive listing of a run; more fails the run.
pub const MAX_RUN_ENTRIES: usize = 50_000;
/// Entries asked for per listing page.
const PAGE_LIMIT: u32 = 2000;
/// The largest single upload Dropbox accepts (150 MiB); larger is refused.
const MAX_SINGLE_UPLOAD: usize = 150 * 1024 * 1024;
/// The longest cursor that is passed back.
const MAX_CURSOR: usize = 4096;

/// What authorises the requests of a [`DropboxStore`]: the refresh token of
/// the account and the access token that came from it (kept in memory only).
///
/// The access token is fetched when there is none and after a 401; a refresh
/// that the server refuses (`invalid_grant`, 401) is
/// [`TransportError::Unauthorized`] and the caller signs the account out.
pub struct DropboxAuth {
    app_key: String,
    sink: Option<Arc<dyn CredentialSink>>,
    inner: Mutex<AuthInner>,
    /// The client and the token endpoint, set by the store that uses this.
    net: Mutex<Option<(RemoteHttp, String)>>,
}

struct AuthInner {
    access: Option<Zeroizing<String>>,
    refresh: Option<Zeroizing<String>>,
}

impl core::fmt::Debug for DropboxAuth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("DropboxAuth([REDACTED])")
    }
}

impl DropboxAuth {
    /// An authorisation from the stored credential; `sink` hears of a refresh
    /// token the provider replaces.
    #[must_use]
    pub fn new(
        app_key: String,
        credential: ProviderCredential,
        sink: Arc<dyn CredentialSink>,
    ) -> Self {
        let refresh = match credential {
            ProviderCredential::Dropbox { refresh_token } => Some(refresh_token),
            _ => None,
        };
        Self {
            app_key,
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
            app_key: String::new(),
            sink: None,
            inner: Mutex::new(AuthInner {
                access: Some(Zeroizing::new(token.to_owned())),
                refresh: None,
            }),
            net: Mutex::new(None),
        }
    }

    /// Where the refresh goes (called by [`DropboxStore::new`]).
    fn bind(&self, http: &RemoteHttp, endpoints: &Endpoints) {
        if let Ok(mut net) = self.net.lock() {
            *net = Some((
                http.clone(),
                format!(
                    "{}/oauth2/token",
                    endpoints.dropbox_api.trim_end_matches('/')
                ),
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
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
                ("client_id", &self.app_key),
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
            // Stored before it is used (Dropbox does not rotate today).
            if let Some(sink) = &self.sink {
                sink.rotated(&ProviderCredential::Dropbox {
                    refresh_token: new.clone(),
                })?;
            }
            inner.refresh = Some(new);
        }
        inner.access = Some(tokens.access);
        Ok(())
    }
}

/// The page the user signs in on: `S256` PKCE, the offline access that
/// yields a refresh token, the app's one redirect URI and the four scopes of
/// ADR-0162.
#[must_use]
pub fn dropbox_authorize_url(
    endpoints: &Endpoints,
    app_key: &str,
    pkce: &Pkce,
    state: &AuthState,
) -> String {
    format!(
        "{}?client_id={}&response_type=code&code_challenge={}&code_challenge_method=S256\
         &token_access_type=offline&redirect_uri={}&state={}&scope={}",
        endpoints.dropbox_auth,
        path_segment(app_key),
        path_segment(&pkce.challenge()),
        path_segment(REDIRECT_URI),
        path_segment(&state.as_param()),
        path_segment(SCOPES),
    )
}

/// The scopes the app asks for, separated by spaces.
const SCOPES: &str = "files.metadata.read files.content.read files.content.write account_info.read";

#[derive(Deserialize)]
struct WireAccount {
    root_info: WireRootInfo,
}

#[derive(Deserialize)]
struct WireRootInfo {
    root_namespace_id: String,
    home_namespace_id: String,
}

/// Trades the code of the redirect for a refresh credential, and asks which
/// namespace the account's root is (a team member's root is the team space:
/// the second part of the answer is that namespace, `None` for a personal
/// account).
///
/// # Errors
/// [`TransportError::Unauthorized`] for a code the server refuses;
/// [`TransportError::Protocol`] for an answer without a refresh token or a
/// namespace; the usual transport errors.
pub fn dropbox_exchange(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    app_key: &str,
    code: &str,
    pkce: &Pkce,
) -> Result<(ProviderCredential, Option<String>), TransportError> {
    let api = endpoints.dropbox_api.trim_end_matches('/');
    let reply = post_form(
        http,
        &format!("{api}/oauth2/token"),
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", pkce.verifier()),
            ("client_id", app_key),
            ("redirect_uri", REDIRECT_URI),
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
    let account = http.send(RemoteRequest {
        method: Method::POST,
        url: &format!("{api}/2/users/get_current_account"),
        headers: &[(AUTHORIZATION, bearer(&tokens.access)?)],
        body: RemoteBody::None,
        timeout: REQUEST_TIMEOUT,
        cap: MAX_SMALL_JSON,
    })?;
    if !account.is_success() {
        return Err(status_error(&account));
    }
    let info: WireAccount =
        serde_json::from_slice(&account.body).map_err(|_| TransportError::Protocol)?;
    let root = info.root_info;
    if root.root_namespace_id.is_empty() || root.home_namespace_id.is_empty() {
        return Err(TransportError::Protocol);
    }
    let team = (root.root_namespace_id != root.home_namespace_id).then_some(root.root_namespace_id);
    Ok((ProviderCredential::Dropbox { refresh_token }, team))
}

/// Revokes the grant at Dropbox (`/2/auth/token/revoke`).
///
/// # Errors
/// [`TransportError::Unauthorized`] if the grant is gone already; the usual
/// transport errors.
pub fn dropbox_revoke(
    http: &RemoteHttp,
    endpoints: &Endpoints,
    auth: &DropboxAuth,
) -> Result<(), TransportError> {
    let url = format!(
        "{}/2/auth/token/revoke",
        endpoints.dropbox_api.trim_end_matches('/')
    );
    let send = |bearer: HeaderValue| {
        http.send(RemoteRequest {
            method: Method::POST,
            url: &url,
            headers: &[(AUTHORIZATION, bearer)],
            body: RemoteBody::None,
            timeout: REQUEST_TIMEOUT,
            cap: MAX_SMALL_JSON,
        })
    };
    let mut reply = send(auth.bearer()?)?;
    if reply.status == 401 {
        auth.refresh()?;
        reply = send(auth.bearer()?)?;
    }
    if reply.is_success() {
        Ok(())
    } else {
        Err(status_error(&reply))
    }
}

#[derive(Default)]
struct State {
    in_run: bool,
    /// The children of each directory, as the run's listing and writes know them.
    children: HashMap<RelPath, Vec<Entry>>,
    /// Directories known to exist in this run.
    dirs: HashSet<RelPath>,
}

/// A folder on Dropbox.
pub struct DropboxStore {
    http: RemoteHttp,
    api: String,
    content: String,
    /// The components of the chosen folder, as the user chose them.
    root: Vec<String>,
    /// The same, lower-cased (Dropbox paths are case-insensitive).
    root_lower: Vec<String>,
    /// The `Dropbox-API-Path-Root` value of a team space.
    path_root: Option<HeaderValue>,
    auth: DropboxAuth,
    state: Mutex<State>,
}

impl core::fmt::Debug for DropboxStore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DropboxStore")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct WireEntry {
    #[serde(rename = ".tag")]
    tag: String,
    name: String,
    path_lower: Option<String>,
    path_display: Option<String>,
    rev: Option<String>,
    size: Option<u64>,
    server_modified: Option<String>,
}

#[derive(Deserialize)]
struct WireList {
    entries: Vec<WireEntry>,
    cursor: String,
    has_more: bool,
}

#[derive(Deserialize)]
struct WireError {
    #[serde(default)]
    error_summary: String,
}

/// The JSON text with every character above `~` written as `\uXXXX`, so it
/// is a valid header value (`Dropbox-API-Arg`).
fn ascii_json(value: &Value) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if (' '..='~').contains(&c) {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                let _ = write!(out, "\\u{unit:04x}");
            }
        }
    }
    out
}

fn arg_header(value: &Value) -> Result<(HeaderName, HeaderValue), TransportError> {
    Ok((
        HeaderName::from_static("dropbox-api-arg"),
        HeaderValue::from_str(&ascii_json(value)).map_err(|_| TransportError::Protocol)?,
    ))
}

/// The error summary of a 409 answer (empty if it has none).
fn summary(reply: &RemoteReply) -> String {
    serde_json::from_slice::<WireError>(&reply.body)
        .map(|e| e.error_summary)
        .unwrap_or_default()
}

/// The error of a failure answer: Dropbox's 409 names its cause in
/// `error_summary`.
fn api_error(reply: &RemoteReply) -> TransportError {
    if reply.status != 409 {
        return status_error(reply);
    }
    let s = summary(reply);
    if s.contains("insufficient_space") {
        TransportError::QuotaExceeded
    } else if s.contains("conflict") {
        TransportError::Io
    } else if s.contains("not_found") {
        TransportError::NotFound
    } else {
        TransportError::Protocol
    }
}

fn is_not_found(reply: &RemoteReply) -> bool {
    reply.status == 409 && {
        let s = summary(reply);
        s.contains("not_found") || s.contains("not_file") || s.contains("not_folder")
    }
}

impl DropboxStore {
    /// A store for the folder of `location` (`root` is the path below the
    /// account's root, or below the team space's when `path_root` is set).
    ///
    /// # Errors
    /// [`TransportError::Protocol`] for a location of another provider or a
    /// root with an invalid component; [`TransportError::Io`] for a team
    /// space that cannot be a header value.
    pub fn new(
        http: RemoteHttp,
        endpoints: &Endpoints,
        location: &FolderLocation,
        auth: DropboxAuth,
    ) -> Result<Self, TransportError> {
        if location.provider != Provider::Dropbox {
            return Err(TransportError::Protocol);
        }
        http.check(&endpoints.dropbox_api)?;
        http.check(&endpoints.dropbox_content)?;
        let root: Vec<String> = location
            .root
            .split('/')
            .filter(|c| !c.is_empty())
            .map(str::to_owned)
            .collect();
        if root.iter().any(|c| !valid_component(c)) {
            return Err(TransportError::Protocol);
        }
        let path_root = location
            .path_root
            .as_deref()
            .map(|ns| {
                HeaderValue::from_str(&ascii_json(&json!({".tag": "root", "root": ns})))
                    .map_err(|_| TransportError::Io)
            })
            .transpose()?;
        auth.bind(&http, endpoints);
        Ok(Self {
            http,
            api: endpoints.dropbox_api.trim_end_matches('/').to_owned(),
            content: endpoints.dropbox_content.trim_end_matches('/').to_owned(),
            root_lower: root.iter().map(|c| c.to_lowercase()).collect(),
            root,
            path_root,
            auth,
            state: Mutex::new(State::default()),
        })
    }

    fn state(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        self.state.lock().map_err(|_| TransportError::Io)
    }

    /// The Dropbox path of `rel`: `""` for the root of the account, else
    /// `/<root>/<rel>`.
    fn path_of(&self, rel: &RelPath) -> String {
        let mut path = String::new();
        for c in self.root.iter().map(String::as_str).chain(rel.components()) {
            path.push('/');
            path.push_str(c);
        }
        path
    }

    /// One request, with one refresh and retry after a 401.
    fn call(
        &self,
        base: &str,
        route: &str,
        extra: &[(HeaderName, HeaderValue)],
        body: &RemoteBody<'_>,
        timeout: Duration,
        cap: u64,
    ) -> Result<RemoteReply, TransportError> {
        let url = format!("{base}{route}");
        let mut refreshed = false;
        loop {
            let mut headers = Vec::with_capacity(extra.len() + 2);
            headers.push((AUTHORIZATION, self.auth.bearer()?));
            if let Some(root) = &self.path_root {
                headers.push((
                    HeaderName::from_static("dropbox-api-path-root"),
                    root.clone(),
                ));
            }
            headers.extend(extra.iter().cloned());
            let reply = self.http.send(RemoteRequest {
                method: Method::POST,
                url: &url,
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

    /// An RPC call: a JSON body, a JSON answer of at most `cap` bytes.
    fn rpc(&self, route: &str, body: &Value, cap: u64) -> Result<RemoteReply, TransportError> {
        let bytes = Zeroizing::new(body.to_string().into_bytes());
        self.call(
            &self.api,
            route,
            &[],
            &RemoteBody::Bytes(&bytes, "application/json"),
            REQUEST_TIMEOUT,
            cap,
        )
    }

    /// Turns a listed entry into its path below the root and an [`Entry`];
    /// `None` for anything that is not strictly below the root: another
    /// folder, a sibling whose name starts like the root, an entry whose
    /// display path differs from its lower-case path, an invalid component,
    /// a name that is not the last component, a deleted entry.
    fn confine(&self, wire: WireEntry) -> Option<(RelPath, Entry)> {
        let kind = match wire.tag.as_str() {
            "file" => EntryKind::File,
            "folder" => EntryKind::Dir,
            _ => return None,
        };
        let lower = wire.path_lower?;
        let display = wire.path_display?;
        let lower_parts: Vec<&str> = lower.strip_prefix('/')?.split('/').collect();
        let display_parts: Vec<&str> = display.strip_prefix('/')?.split('/').collect();
        if lower_parts.len() != display_parts.len() || lower_parts.len() <= self.root.len() {
            return None;
        }
        if lower_parts
            .iter()
            .zip(&self.root_lower)
            .any(|(have, want)| have != want)
        {
            return None;
        }
        let mut rel = RelPath::root();
        for (l, d) in lower_parts.iter().zip(&display_parts).skip(self.root.len()) {
            if d.to_lowercase() != *l {
                return None;
            }
            rel = rel.child(d)?;
        }
        if wire.name.to_lowercase() != rel.file_name()?.to_lowercase() {
            return None;
        }
        let entry = Entry {
            name: rel.file_name()?.into(),
            kind,
            len: if kind == EntryKind::File {
                wire.size.unwrap_or(0)
            } else {
                0
            },
            modified: wire.server_modified.as_deref().and_then(parse_rfc3339),
            version: wire.rev.map(|r| r.into_bytes().into_boxed_slice()),
        };
        Some((rel, entry))
    }

    /// The confined entries of a listing of `dir`; `None` if it is missing.
    fn list_pages(
        &self,
        dir: &RelPath,
        recursive: bool,
        max_entries: usize,
    ) -> Result<Option<Vec<(RelPath, Entry)>>, TransportError> {
        let mut out = Vec::new();
        let mut seen = 0usize;
        let first = json!({
            "path": self.path_of(dir),
            "recursive": recursive,
            "include_deleted": false,
            "limit": PAGE_LIMIT,
        });
        let mut reply = self.rpc("/2/files/list_folder", &first, MAX_LISTING_BYTES)?;
        for _ in 0..MAX_PAGES {
            if is_not_found(&reply) && out.is_empty() && seen == 0 {
                return Ok(None);
            }
            if !reply.is_success() {
                return Err(api_error(&reply));
            }
            let page: WireList =
                serde_json::from_slice(&reply.body).map_err(|_| TransportError::Protocol)?;
            seen = seen.saturating_add(page.entries.len());
            if seen > max_entries {
                return Err(TransportError::Protocol);
            }
            out.extend(page.entries.into_iter().filter_map(|e| self.confine(e)));
            if !page.has_more {
                return Ok(Some(out));
            }
            if page.cursor.is_empty() || page.cursor.len() > MAX_CURSOR {
                return Err(TransportError::Protocol);
            }
            reply = self.rpc(
                "/2/files/list_folder/continue",
                &json!({ "cursor": page.cursor }),
                MAX_LISTING_BYTES,
            )?;
        }
        // Still more pages after the last one that is allowed.
        Err(TransportError::Protocol)
    }

    fn fetch_list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        let Some(found) = self.list_pages(dir, false, MAX_SCANNED)? else {
            return Ok(Vec::new());
        };
        let mut seen = HashSet::new();
        let mut entries: Vec<Entry> = found
            .into_iter()
            .filter(|(rel, _)| rel.parent().as_ref() == Some(dir) && seen.insert(rel.clone()))
            .map(|(_, e)| e)
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Records a write in this run's listing of `dir`.
    fn note_write(&self, dir: &RelPath, entry: Entry) {
        let Ok(mut state) = self.state() else {
            return;
        };
        if !state.in_run {
            return;
        }
        let mut current = dir.clone();
        loop {
            state.dirs.insert(current.clone());
            match current.parent() {
                Some(p) => current = p,
                None => break,
            }
        }
        let list = state.children.entry(dir.clone()).or_default();
        list.retain(|e| e.name != entry.name);
        list.push(entry);
        list.sort_by(|a, b| a.name.cmp(&b.name));
    }

    fn note_removed(&self, path: &RelPath) {
        let Ok(mut state) = self.state() else {
            return;
        };
        if let (Some(dir), Some(name)) = (path.parent(), path.file_name())
            && let Some(list) = state.children.get_mut(&dir)
        {
            list.retain(|e| e.name.to_str() != Some(name));
        }
        state.dirs.remove(path);
        state.children.remove(path);
    }

    /// The entry of `path` as this run's listing knows it, if it has a `rev`;
    /// otherwise a direct `stat`.
    fn known_entry(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        if let (Ok(state), Some(dir), Some(name)) = (self.state(), path.parent(), path.file_name())
            && let Some(e) = state
                .children
                .get(&dir)
                .and_then(|l| l.iter().find(|e| e.name.to_str() == Some(name)))
            && e.version.is_some()
        {
            return Ok(Some(e.clone()));
        }
        self.stat(path)
    }

    fn delete(&self, path: &RelPath, rev: Option<&[u8]>) -> Result<(), TransportError> {
        // The chosen folder itself is never deleted.
        if path.file_name().is_none() {
            return Err(TransportError::Protocol);
        }
        let mut body = json!({ "path": self.path_of(path) });
        if let (Some(rev), Some(map)) = (rev, body.as_object_mut()) {
            map.insert(
                "parent_rev".to_owned(),
                Value::String(String::from_utf8_lossy(rev).into_owned()),
            );
        }
        let reply = self.rpc("/2/files/delete_v2", &body, MAX_SMALL_JSON)?;
        if reply.is_success() || is_not_found(&reply) {
            self.note_removed(path);
            Ok(())
        } else {
            Err(api_error(&reply))
        }
    }

    /// The entry in a `get_metadata` or `upload` answer for `rel`.
    fn entry_from_answer(&self, rel: &RelPath, body: &[u8]) -> Result<Entry, TransportError> {
        let wire: WireMeta = serde_json::from_slice(body).map_err(|_| TransportError::Protocol)?;
        let name = rel
            .file_name()
            .or_else(|| self.root.last().map(String::as_str))
            .ok_or(TransportError::Protocol)?;
        if wire.name.to_lowercase() != name.to_lowercase() {
            return Err(TransportError::Protocol);
        }
        let kind = match wire.tag.as_str() {
            "file" => EntryKind::File,
            "folder" => EntryKind::Dir,
            _ => return Err(TransportError::Protocol),
        };
        Ok(Entry {
            name: name.into(),
            kind,
            len: if kind == EntryKind::File {
                wire.size.unwrap_or(0)
            } else {
                0
            },
            modified: wire.server_modified.as_deref().and_then(parse_rfc3339),
            version: wire.rev.map(|r| r.into_bytes().into_boxed_slice()),
        })
    }

    fn ensure_dir(&self, dir: &RelPath) -> Result<(), TransportError> {
        let mut current = RelPath::root();
        for component in dir.components() {
            current = current.known(component);
            if self.state()?.dirs.contains(&current) {
                continue;
            }
            let reply = self.rpc(
                "/2/files/create_folder_v2",
                &json!({ "path": self.path_of(&current), "autorename": false }),
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
        Ok(())
    }
}

#[derive(Deserialize)]
struct WireMeta {
    #[serde(rename = ".tag", default)]
    tag: String,
    name: String,
    rev: Option<String>,
    size: Option<u64>,
    server_modified: Option<String>,
}

/// The `Dropbox-API-Result` header of a download.
#[derive(Deserialize)]
struct WireResult {
    size: u64,
}

impl FolderStore for DropboxStore {
    fn begin_run(&self) -> Result<(), TransportError> {
        {
            let mut state = self.state()?;
            *state = State::default();
        }
        let found = self
            .list_pages(&RelPath::root(), true, MAX_RUN_ENTRIES)?
            .unwrap_or_default();
        let mut state = self.state()?;
        let mut seen = HashSet::new();
        for (rel, entry) in found {
            if !seen.insert(rel.clone()) {
                continue;
            }
            if entry.is_dir() {
                state.dirs.insert(rel.clone());
            }
            if let Some(parent) = rel.parent() {
                state.children.entry(parent).or_default().push(entry);
            }
        }
        for list in state.children.values_mut() {
            list.sort_by(|a, b| a.name.cmp(&b.name));
            if list.len() > MAX_SCANNED {
                return Err(TransportError::Protocol);
            }
        }
        state.in_run = true;
        Ok(())
    }

    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        {
            let state = self.state()?;
            if state.in_run {
                return Ok(state.children.get(dir).cloned().unwrap_or_default());
            }
        }
        self.fetch_list(dir)
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
        let arg = arg_header(&json!({ "path": self.path_of(path) }))?;
        let reply = self.call(
            &self.content,
            "/2/files/download",
            &[arg],
            &RemoteBody::None,
            ATTACHMENT_TIMEOUT,
            max,
        )?;
        if reply.is_success() {
            let announced: WireResult = reply
                .header("dropbox-api-result")
                .and_then(|h| serde_json::from_str(h).ok())
                .ok_or(TransportError::Protocol)?;
            if announced.size > max
                || announced.size != u64::try_from(reply.body.len()).unwrap_or(0)
            {
                return Err(TransportError::Protocol);
            }
            return Ok(Some(reply.body));
        }
        if is_not_found(&reply) {
            return Ok(None);
        }
        Err(api_error(&reply))
    }

    fn write_atomic(
        &self,
        dir: &RelPath,
        name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), TransportError> {
        let target = dir.child(name).ok_or(TransportError::Protocol)?;
        if bytes.len() > MAX_SINGLE_UPLOAD {
            return Err(TransportError::Protocol);
        }
        let mut arg = json!({
            "path": self.path_of(&target),
            "mode": if replace { "overwrite" } else { "add" },
            "autorename": false,
            "mute": true,
        });
        if !replace && let Some(map) = arg.as_object_mut() {
            map.insert("strict_conflict".to_owned(), Value::Bool(true));
        }
        let reply = self.call(
            &self.content,
            "/2/files/upload",
            &[arg_header(&arg)?],
            &RemoteBody::Bytes(bytes, "application/octet-stream"),
            ATTACHMENT_TIMEOUT,
            MAX_SMALL_JSON,
        )?;
        if !reply.is_success() {
            return Err(api_error(&reply));
        }
        let mut entry = self.entry_from_answer(&target, &reply.body)?;
        if entry.kind != EntryKind::File {
            return Err(TransportError::Protocol);
        }
        entry.len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        entry.modified = entry.modified.or_else(|| Some(SystemTime::now()));
        self.note_write(dir, entry);
        Ok(())
    }

    /// Deletes a file with its `rev`. A folder on the name is never deleted.
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
            let _ = self.delete(path, None);
        }
    }

    fn remove_squatter(&self, path: &RelPath) {
        let _ = self.delete(path, None);
    }

    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        if path.file_name().is_none() && self.root.is_empty() {
            // The account's own root has no metadata call: it exists if it lists.
            let reply = self.rpc(
                "/2/files/list_folder",
                &json!({ "path": "", "limit": 1 }),
                MAX_LISTING_BYTES,
            )?;
            return if reply.is_success() {
                Ok(Some(Entry {
                    name: "".into(),
                    kind: EntryKind::Dir,
                    len: 0,
                    modified: None,
                    version: None,
                }))
            } else {
                Err(api_error(&reply))
            };
        }
        let reply = self.rpc(
            "/2/files/get_metadata",
            &json!({ "path": self.path_of(path) }),
            MAX_SMALL_JSON,
        )?;
        if is_not_found(&reply) {
            return Ok(None);
        }
        if !reply.is_success() {
            return Err(api_error(&reply));
        }
        self.entry_from_answer(path, &reply.body).map(Some)
    }

    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError> {
        self.ensure_dir(dir)
    }
}

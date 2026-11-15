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

use super::credential::{CredentialSink, Provider};
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest, status_error};
use super::location::FolderLocation;
use super::oauth::{Endpoints, path_segment};
use super::time::parse_rfc3339;
use super::tokens::bearer;
use super::{MAX_LISTING_BYTES, MAX_PAGES, MAX_SMALL_JSON};
use crate::folder::store::{Entry, EntryKind, FolderStore, MAX_SCANNED, RelPath};
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

/// What authorises the requests of a [`GraphStore`]: an access token.
pub struct GraphAuth {
    access: Mutex<Option<Zeroizing<String>>>,
    _sink: Option<Arc<dyn CredentialSink>>,
}

impl core::fmt::Debug for GraphAuth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("GraphAuth([REDACTED])")
    }
}

impl GraphAuth {
    /// An authorisation with one fixed access token that is never refreshed.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    pub fn fixed_for_test(token: &str) -> Self {
        Self {
            access: Mutex::new(Some(Zeroizing::new(token.to_owned()))),
            _sink: None,
        }
    }

    /// The `Authorization` header value (marked sensitive).
    ///
    /// # Errors
    /// [`TransportError::Unauthorized`] without an access token;
    /// [`TransportError::Io`] for a token that cannot be a header value.
    pub fn bearer(&self) -> Result<HeaderValue, TransportError> {
        let guard = self.access.lock().map_err(|_| TransportError::Io)?;
        bearer(guard.as_ref().ok_or(TransportError::Unauthorized)?)
    }

    /// Gets a new access token after a 401.
    pub(crate) fn refresh(&self) -> Result<(), TransportError> {
        // Nothing to refresh with: the token that was refused is dropped.
        *self.access.lock().map_err(|_| TransportError::Io)? = None;
        Err(TransportError::Unauthorized)
    }
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

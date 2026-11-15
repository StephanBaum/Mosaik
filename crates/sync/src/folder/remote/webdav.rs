//! A [`FolderStore`] over `WebDAV` (RFC 4918), for Nextcloud and for a typed
//! `WebDAV` URL: `PROPFIND`, `GET`, `PUT`, `MKCOL`, `MOVE`, `DELETE`, always
//! below `base/<root>` and only on [`RelPath`]s, so a path a server names
//! cannot lead out of the folder.
//!
//! A write goes to `.<16 hex>.mtmp` and is moved over the final name, so the
//! final name never holds half a file; a write-once name is moved with
//! `Overwrite: F`. A delete carries `If-Match` with the entry's `ETag` and
//! never touches a collection (an `ETag` that changed, or a collection planted
//! on the name, is a refusal, not a delete).

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use base64ct::{Base64, Encoding};
use reqwest::Method;
use reqwest::header::{AUTHORIZATION, HeaderName, HeaderValue};
use zeroize::Zeroizing;

use super::MAX_LISTING_BYTES;
use super::credential::ProviderCredential;
use super::davxml::{parse_multistatus, parse_stat};
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest, status_error};
use super::oauth::{path_segment, percent_decode};
use crate::folder::store::{
    Entry, EntryKind, FolderStore, MAX_SCANNED, RelPath, TEMP_EXTENSION, random_hex,
    valid_component,
};
use crate::http::client::{ATTACHMENT_TIMEOUT, REQUEST_TIMEOUT};
use crate::transport::TransportError;

const PROPFIND_BODY: &[u8] = b"<?xml version=\"1.0\"?><d:propfind xmlns:d=\"DAV:\"><d:prop>\
<d:getetag/><d:getcontentlength/><d:getlastmodified/><d:resourcetype/></d:prop></d:propfind>";
/// A directory's `PROPFIND` answer is small; a file's own metadata smaller.
const MAX_STAT_BYTES: u64 = 256 * 1024;

#[derive(Default)]
struct State {
    /// Between `begin_run` and the next one the listings are cached.
    in_run: bool,
    listings: HashMap<RelPath, Vec<Entry>>,
    /// Directories known to exist in this run (created or found).
    dirs: HashSet<RelPath>,
}

/// A folder on a `WebDAV` server.
pub struct WebDavStore {
    http: RemoteHttp,
    /// The collection URL of the account, no trailing slash.
    base: String,
    /// The decoded path of `base`.
    base_path: String,
    /// The components of the chosen folder below `base`.
    root: Vec<String>,
    auth: HeaderValue,
    state: Mutex<State>,
}

impl core::fmt::Debug for WebDavStore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebDavStore")
            .field("base", &self.base)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

/// `Basic base64(login:password)`, built in zeroizing buffers and marked
/// sensitive. A login with a colon cannot be sent in this scheme.
pub(crate) fn basic_auth(login: &str, password: &str) -> Result<HeaderValue, TransportError> {
    if login.is_empty() || login.contains(':') {
        return Err(TransportError::Protocol);
    }
    let mut raw = Zeroizing::new(Vec::with_capacity(login.len() + password.len() + 1));
    raw.extend_from_slice(login.as_bytes());
    raw.push(b':');
    raw.extend_from_slice(password.as_bytes());
    let mut text = Zeroizing::new(b"Basic ".to_vec());
    let mut encoded = Zeroizing::new(vec![0u8; Base64::encoded_len(&raw)]);
    let n = Base64::encode(&raw, &mut encoded)
        .map_err(|_| TransportError::Io)?
        .len();
    text.extend_from_slice(encoded.get(..n).ok_or(TransportError::Io)?);
    let mut value = HeaderValue::from_bytes(&text).map_err(|_| TransportError::Io)?;
    value.set_sensitive(true);
    Ok(value)
}

fn method(name: &'static str) -> Method {
    // The names used here are fixed valid tokens.
    Method::from_bytes(name.as_bytes()).unwrap_or(Method::GET)
}

impl WebDavStore {
    /// A store for the folder `root` (components separated by `/`, may be
    /// empty) below `base` (`https://host/remote.php/dav/files/<login>` or a
    /// typed `WebDAV` URL, canonical, no trailing `/`).
    ///
    /// # Errors
    /// [`TransportError::InsecureUrl`] or `InvalidUrl` for a bad `base`;
    /// [`TransportError::Protocol`] for a credential that is not a `WebDAV` one,
    /// a login with a colon, or a `root` with an invalid component.
    pub fn new(
        http: RemoteHttp,
        base: &str,
        root: &str,
        credential: ProviderCredential,
    ) -> Result<Self, TransportError> {
        http.check(base)?;
        let ProviderCredential::WebDav {
            login,
            app_password,
        } = credential
        else {
            return Err(TransportError::Protocol);
        };
        let base = base.trim_end_matches('/').to_owned();
        let after_scheme = base.split_once("://").map_or("", |(_, rest)| rest);
        let path = after_scheme.find('/').map_or("", |i| after_scheme.get(i..).unwrap_or(""));
        let base_path = percent_decode(path, false).ok_or(TransportError::InvalidUrl)?;
        let root: Vec<String> = root
            .split('/')
            .filter(|c| !c.is_empty())
            .map(str::to_owned)
            .collect();
        if root.iter().any(|c| !valid_component(c)) {
            return Err(TransportError::Protocol);
        }
        Ok(Self {
            auth: basic_auth(&login, &app_password)?,
            http,
            base,
            base_path,
            root,
            state: Mutex::new(State::default()),
        })
    }

    fn state(&self) -> Result<MutexGuard<'_, State>, TransportError> {
        self.state.lock().map_err(|_| TransportError::Io)
    }

    /// The URL of `rel`: the base, the root and `rel`, each component encoded.
    fn url(&self, rel: &RelPath) -> String {
        let mut url = self.base.clone();
        for c in self.root.iter().map(String::as_str).chain(rel.components()) {
            url.push('/');
            url.push_str(&path_segment(c));
        }
        url
    }

    /// The decoded path of `rel`, as an `href` in an answer names it.
    fn href(&self, rel: &RelPath) -> String {
        let mut path = self.base_path.clone();
        for c in self.root.iter().map(String::as_str).chain(rel.components()) {
            path.push('/');
            path.push_str(c);
        }
        path
    }

    fn call(
        &self,
        method: Method,
        url: &str,
        headers: &[(HeaderName, HeaderValue)],
        body: RemoteBody<'_>,
        timeout: Duration,
        cap: u64,
    ) -> Result<RemoteReply, TransportError> {
        let mut all = Vec::with_capacity(headers.len() + 1);
        all.push((AUTHORIZATION, self.auth.clone()));
        all.extend(headers.iter().cloned());
        self.http.send(RemoteRequest {
            method,
            url,
            headers: &all,
            body,
            timeout,
            cap,
        })
    }

    /// `PROPFIND` with `depth`; `None` for a 404.
    fn propfind(
        &self,
        rel: &RelPath,
        depth: &'static str,
        cap: u64,
    ) -> Result<Option<RemoteReply>, TransportError> {
        let reply = self.call(
            method("PROPFIND"),
            &self.url(rel),
            &[(
                HeaderName::from_static("depth"),
                HeaderValue::from_static(depth),
            )],
            RemoteBody::Bytes(PROPFIND_BODY, "application/xml"),
            REQUEST_TIMEOUT,
            cap,
        )?;
        match reply.status {
            207 => Ok(Some(reply)),
            404 => Ok(None),
            _ if reply.is_success() => Err(TransportError::Protocol),
            _ => Err(status_error(&reply)),
        }
    }

    fn fetch_list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        let Some(reply) = self.propfind(dir, "1", MAX_LISTING_BYTES)? else {
            return Ok(Vec::new());
        };
        let mut entries = parse_multistatus(&reply.body, &self.href(dir))?;
        if entries.len() > MAX_SCANNED {
            return Err(TransportError::Protocol);
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Records a write in this run's listing of `dir`, so a lagging server
    /// does not make it vanish from the run's own later `list` answers.
    fn note_write(&self, dir: &RelPath, name: &str, len: usize) {
        let Ok(mut state) = self.state() else {
            return;
        };
        if !state.in_run {
            return;
        }
        state.dirs.insert(dir.clone());
        if let Some(list) = state.listings.get_mut(dir) {
            list.retain(|e| e.name.to_str() != Some(name));
            list.push(Entry {
                name: name.into(),
                kind: EntryKind::File,
                len: u64::try_from(len).unwrap_or(u64::MAX),
                modified: Some(SystemTime::now()),
                version: None,
            });
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

    /// The entry of `path` as this run's listing knows it, if it has a
    /// version token; otherwise a direct `stat`.
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

    /// Creates `dir` and its parents below the root (the root itself is the
    /// folder the user chose and is never created).
    fn ensure_dir(&self, dir: &RelPath) -> Result<(), TransportError> {
        if self.state()?.dirs.contains(dir) {
            return Ok(());
        }
        let mut current = RelPath::root();
        for component in dir.components() {
            current = current.known(component);
            if self.state()?.dirs.contains(&current) {
                continue;
            }
            let reply = self.call(
                method("MKCOL"),
                &self.url(&current),
                &[],
                RemoteBody::None,
                REQUEST_TIMEOUT,
                0,
            )?;
            match reply.status {
                201 => {}
                // Exists (or the parent is missing, which a concurrent
                // delete can cause): fine if a collection is there now.
                405 | 409 if self.is_real_dir(&current) => {}
                405 | 409 => return Err(TransportError::Io),
                _ => return Err(status_error(&reply)),
            }
            let mut state = self.state()?;
            if state.in_run {
                state.dirs.insert(current.clone());
            }
        }
        Ok(())
    }

    fn delete(&self, path: &RelPath, if_match: Option<&[u8]>) -> Result<(), TransportError> {
        let mut headers = Vec::new();
        if let Some(tag) = if_match {
            let value = HeaderValue::from_bytes(tag).map_err(|_| TransportError::Protocol)?;
            headers.push((HeaderName::from_static("if-match"), value));
        }
        let reply = self.call(
            Method::DELETE,
            &self.url(path),
            &headers,
            RemoteBody::None,
            REQUEST_TIMEOUT,
            0,
        )?;
        match reply.status {
            200 | 202 | 204 | 404 => {
                self.note_removed(path);
                Ok(())
            }
            412 => Err(TransportError::Io),
            _ if reply.is_success() => Err(TransportError::Protocol),
            _ => Err(status_error(&reply)),
        }
    }
}

impl FolderStore for WebDavStore {
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
        let reply = self.call(
            Method::GET,
            &self.url(path),
            &[],
            RemoteBody::None,
            ATTACHMENT_TIMEOUT,
            max,
        )?;
        match reply.status {
            // A collection (or a missing file) is not a regular file.
            404 | 405 => Ok(None),
            200 => Ok(Some(reply.body)),
            _ if reply.is_success() => Err(TransportError::Protocol),
            _ => Err(status_error(&reply)),
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
        let temp_name = format!(".{}.{TEMP_EXTENSION}", random_hex()?);
        let temp = dir.child(&temp_name).ok_or(TransportError::Protocol)?;
        let put = self.call(
            Method::PUT,
            &self.url(&temp),
            &[],
            RemoteBody::Bytes(bytes, "application/octet-stream"),
            ATTACHMENT_TIMEOUT,
            0,
        )?;
        if !put.is_success() {
            // The temp file may exist even so; best effort.
            let _ = self.delete(&temp, None);
            return Err(status_error(&put));
        }
        let destination = HeaderValue::from_str(&self.url(&target)).map_err(|_| TransportError::Io)?;
        let moved = self.call(
            method("MOVE"),
            &self.url(&temp),
            &[
                (HeaderName::from_static("destination"), destination),
                (
                    HeaderName::from_static("overwrite"),
                    HeaderValue::from_static(if replace { "T" } else { "F" }),
                ),
            ],
            RemoteBody::None,
            REQUEST_TIMEOUT,
            0,
        );
        let result = match moved {
            Ok(r) if matches!(r.status, 201 | 204) => Ok(()),
            // Write-once name already taken.
            Ok(r) if r.status == 412 => Err(TransportError::Io),
            Ok(r) if r.is_success() => Err(TransportError::Protocol),
            Ok(r) => Err(status_error(&r)),
            Err(e) => Err(e),
        };
        if result.is_err() {
            let _ = self.delete(&temp, None);
            return result;
        }
        self.note_write(dir, name, bytes.len());
        Ok(())
    }

    /// Deletes a file with `If-Match`. A collection on the name, or one that
    /// replaced the file since it was looked at, is never deleted.
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
        if !entry.is_dir() {
            return;
        }
        if matches!(self.fetch_list(path), Ok(list) if list.is_empty()) {
            let _ = self.delete(path, entry.version.as_deref());
        }
    }

    fn remove_squatter(&self, path: &RelPath) {
        let _ = self.delete(path, None);
    }

    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        let Some(reply) = self.propfind(path, "0", MAX_STAT_BYTES)? else {
            return Ok(None);
        };
        parse_stat(&reply.body, &self.href(path)).map(Some)
    }

    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError> {
        self.ensure_dir(dir)
    }
}

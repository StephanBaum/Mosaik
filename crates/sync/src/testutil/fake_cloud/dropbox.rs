//! A Dropbox API v2 server over a directory: `list_folder` (with paging and
//! `continue`), `download`, `upload` (write-once or overwrite, parents
//! created), `get_metadata`, `create_folder_v2` and `delete_v2` (with
//! `parent_rev`). The `rev` is the hex of the SHA-256 of the length and the
//! modification time. Dropbox path `/a/b` is the directory `<dir>/a/b`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use base64ct::{Base64UrlUnpadded, Encoding};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::Faults;
use super::dav::{children_of, empty, etag_of, json, rfc3339};
use crate::folder::remote::oauth::{REDIRECT_URI, percent_decode};
use crate::folder::store::valid_component;
use crate::testutil::{FakeHttp, FakeRequest, FakeResponse};

/// The code the fake accepts at the token endpoint.
pub const AUTHORIZATION_CODE: &str = "auth-code";

#[derive(Default)]
struct Auth {
    /// The access token that is valid now.
    access: Option<String>,
    /// The refresh token that is valid now.
    refresh: Option<String>,
    /// The `client_id` the token endpoint expects, if set.
    app_key: Option<String>,
    /// The PKCE challenge the code was issued for, if set.
    challenge: Option<String>,
    /// The team root namespace `get_current_account` reports.
    team_root: Option<String>,
    /// The access token expires once this many API requests have been seen.
    expire_at: Option<usize>,
    /// The grant is revoked once this many API requests have been seen.
    revoke_at: Option<usize>,
}

struct Shared {
    dir: PathBuf,
    auth: Mutex<Auth>,
    api_seen: AtomicUsize,
    refreshes: AtomicUsize,
    revoked: AtomicBool,
    faults: Mutex<Faults>,
    count: AtomicUsize,
    writes: AtomicUsize,
    /// Cursor id to the entries not yet sent and the page size.
    cursors: Mutex<HashMap<String, (Vec<Value>, usize)>>,
    next_cursor: AtomicUsize,
    /// Serialises the file operations.
    ops: Mutex<()>,
}

/// A Dropbox API server over a directory.
pub struct FakeDropbox {
    server: FakeHttp,
    shared: Arc<Shared>,
}

impl FakeDropbox {
    /// Serves `dir` as the root of an account for this access token.
    #[must_use]
    pub fn serve(dir: PathBuf, token: &str) -> Self {
        let shared = Arc::new(Shared {
            dir,
            auth: Mutex::new(Auth {
                access: Some(token.to_owned()),
                refresh: Some("refresh-token-1".to_owned()),
                ..Auth::default()
            }),
            api_seen: AtomicUsize::new(0),
            refreshes: AtomicUsize::new(0),
            revoked: AtomicBool::new(false),
            faults: Mutex::new(Faults::default()),
            count: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            cursors: Mutex::new(HashMap::new()),
            next_cursor: AtomicUsize::new(0),
            ops: Mutex::new(()),
        });
        let state = Arc::clone(&shared);
        let server = FakeHttp::start(move |r| handle(&state, r));
        Self { server, shared }
    }

    /// `http://127.0.0.1:<port>`.
    #[must_use]
    pub fn origin(&self) -> &str {
        self.server.origin()
    }

    /// Every request so far.
    #[must_use]
    pub fn requests(&self) -> Vec<FakeRequest> {
        self.server.requests()
    }

    /// The number of uploads, folder creations and deletions that did something.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.shared.writes.load(Ordering::SeqCst)
    }

    /// The refresh token that is valid now.
    #[must_use]
    pub fn refresh_token(&self) -> String {
        self.shared
            .auth
            .lock()
            .ok()
            .and_then(|a| a.refresh.clone())
            .unwrap_or_default()
    }

    /// The token endpoint expects this `client_id`.
    pub fn expect_app_key(&self, key: &str) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.app_key = Some(key.to_owned());
        }
    }

    /// The token endpoint checks the code verifier against this challenge.
    pub fn expect_challenge(&self, challenge: &str) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.challenge = Some(challenge.to_owned());
        }
    }

    /// `get_current_account` reports this team root namespace.
    pub fn set_team_root(&self, namespace: &str) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.team_root = Some(namespace.to_owned());
        }
    }

    /// The access token expires after this many more API requests.
    pub fn expire_access_after(&self, requests: usize) {
        let now = self.shared.api_seen.load(Ordering::SeqCst);
        if let Ok(mut a) = self.shared.auth.lock() {
            a.expire_at = Some(now + requests);
        }
    }

    /// The user revokes the app after this many more API requests: the
    /// access and the refresh token stop working.
    pub fn revoke_after(&self, requests: usize) {
        let now = self.shared.api_seen.load(Ordering::SeqCst);
        if let Ok(mut a) = self.shared.auth.lock() {
            a.revoke_at = Some(now + requests);
        }
    }

    /// How many access tokens were issued by refreshing.
    #[must_use]
    pub fn refreshes(&self) -> usize {
        self.shared.refreshes.load(Ordering::SeqCst)
    }

    /// How many API requests (`/2/...`) were made.
    #[must_use]
    pub fn api_requests(&self) -> usize {
        self.shared.api_seen.load(Ordering::SeqCst)
    }

    /// Whether the app was revoked through `/2/auth/token/revoke`.
    #[must_use]
    pub fn revoked(&self) -> bool {
        self.shared.revoked.load(Ordering::SeqCst)
    }

    /// Sets the faults and restarts the request count.
    pub fn set_faults(&self, faults: Faults) {
        let _guard = self.shared.ops.lock();
        if let Ok(mut f) = self.shared.faults.lock() {
            *f = faults;
        }
        self.shared.count.store(0, Ordering::SeqCst);
    }
}

fn conflict(kind: &str) -> FakeResponse {
    api_error(&format!("path/conflict/{kind}/.."))
}

fn api_error(summary: &str) -> FakeResponse {
    json(
        409,
        &json!({ "error_summary": summary, "error": {} }).to_string(),
    )
}

/// The directory and the components of a Dropbox path; `None` if a component
/// is not valid.
fn fs_path(root: &Path, path: &str) -> Option<(PathBuf, Vec<String>)> {
    let mut out = root.to_path_buf();
    let mut comps = Vec::new();
    if !path.is_empty() {
        for c in path.strip_prefix('/')?.split('/') {
            if !valid_component(c) {
                return None;
            }
            out.push(c);
            comps.push(c.to_owned());
        }
    }
    Some((out, comps))
}

fn ascii(value: &Value) -> String {
    let mut out = String::new();
    for c in value.to_string().chars() {
        if (' '..='~').contains(&c) {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                let _ = write!(out, "\\u{u:04x}");
            }
        }
    }
    out
}

fn meta(comps: &[String], dir: bool, len: u64, mtime: std::time::SystemTime) -> Value {
    let display = format!("/{}", comps.join("/"));
    let mut v = json!({
        ".tag": if dir { "folder" } else { "file" },
        "name": comps.last().cloned().unwrap_or_default(),
        "path_lower": display.to_lowercase(),
        "path_display": display,
        "id": format!("id:{}", display.len()),
    });
    if !dir && let Some(map) = v.as_object_mut() {
        map.insert(
            "rev".to_owned(),
            Value::String(etag_of(len, mtime).trim_matches('"').to_owned()),
        );
        map.insert("size".to_owned(), json!(len));
        map.insert("server_modified".to_owned(), Value::String(rfc3339(mtime)));
    }
    v
}

fn meta_of_path(path: &Path, comps: &[String]) -> Option<Value> {
    let m = fs::metadata(path).ok()?;
    Some(meta(
        comps,
        m.is_dir(),
        m.len(),
        m.modified().unwrap_or(UNIX_EPOCH),
    ))
}

fn handle(s: &Shared, r: &FakeRequest) -> FakeResponse {
    let n = s.count.fetch_add(1, Ordering::SeqCst) + 1;
    let faults = s.faults.lock().map(|f| *f).unwrap_or_default();
    if let Some((nth, status, retry)) = faults.fail_nth
        && nth == n
    {
        let retry = retry.map(|v| v.to_string());
        let headers: Vec<(&str, &str)> =
            retry.iter().map(|v| ("Retry-After", v.as_str())).collect();
        return FakeResponse::new(status, &headers, Vec::new());
    }
    let route = r.path_and_query.split('?').next().unwrap_or_default();
    if route == "/oauth2/token" {
        return token_route(s, r);
    }
    let seen = s.api_seen.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let Ok(mut a) = s.auth.lock() else {
            return empty(500);
        };
        if a.revoke_at.is_some_and(|at| seen > at) {
            a.revoke_at = None;
            a.access = None;
            a.refresh = None;
        }
        if a.expire_at.is_some_and(|at| seen > at) {
            a.expire_at = None;
            a.access = None;
        }
        let valid = a
            .access
            .as_ref()
            .is_some_and(|t| r.header("authorization") == Some(format!("Bearer {t}").as_str()));
        if !valid {
            return json(
                401,
                &json!({ "error_summary": "expired_access_token/", "error": {} }).to_string(),
            );
        }
    }
    let arg: Value = match route {
        "/2/files/upload" | "/2/files/download" => r
            .header("dropbox-api-arg")
            .and_then(|h| serde_json::from_str(h).ok()),
        _ => serde_json::from_slice(&r.body).ok(),
    }
    .unwrap_or(Value::Null);
    let Ok(_guard) = s.ops.lock() else {
        return empty(500);
    };
    let path = arg.get("path").and_then(Value::as_str).unwrap_or_default();
    match (r.method.as_str(), route) {
        ("POST", "/2/files/list_folder") => list_folder(s, &arg, path, faults),
        ("POST", "/2/files/list_folder/continue") => list_continue(s, &arg),
        ("POST", "/2/files/get_metadata") => {
            let Some((p, comps)) = fs_path(&s.dir, path) else {
                return api_error("path/malformed_path/");
            };
            meta_of_path(&p, &comps).map_or_else(
                || api_error("path/not_found/."),
                |m| json(200, &m.to_string()),
            )
        }
        ("POST", "/2/files/create_folder_v2") => create_folder(s, path),
        ("POST", "/2/files/delete_v2") => delete(s, &arg, path),
        ("POST", "/2/files/upload") => upload(s, &arg, path, &r.body),
        ("POST", "/2/files/download") => download(s, path),
        ("POST", "/2/users/get_current_account") => {
            let team = s.auth.lock().ok().and_then(|a| a.team_root.clone());
            let home = "dbid-home".to_owned();
            json(
                200,
                &json!({ "root_info": {
                    ".tag": "user",
                    "root_namespace_id": team.unwrap_or_else(|| home.clone()),
                    "home_namespace_id": home,
                }})
                .to_string(),
            )
        }
        ("POST", "/2/auth/token/revoke") => {
            if let Ok(mut a) = s.auth.lock() {
                a.access = None;
                a.refresh = None;
            }
            s.revoked.store(true, Ordering::SeqCst);
            empty(200)
        }
        _ => empty(404),
    }
}

fn list_folder(s: &Shared, arg: &Value, path: &str, faults: Faults) -> FakeResponse {
    let Some((p, comps)) = fs_path(&s.dir, path) else {
        return api_error("path/malformed_path/");
    };
    match fs::metadata(&p) {
        Err(_) => return api_error("path/not_found/."),
        Ok(m) if !m.is_dir() => return api_error("path/not_folder/."),
        Ok(_) => {}
    }
    let recursive = arg
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut entries = Vec::new();
    collect(&p, &comps, recursive, &mut entries);
    let mtime = std::time::SystemTime::now();
    if faults.outside_path {
        entries.push(meta(
            &["Other".to_owned(), "a".to_owned(), "x.mseg".to_owned()],
            false,
            1,
            mtime,
        ));
        // A sibling whose name starts like the folder's own.
        let mut sibling = comps.clone();
        if let Some(last) = sibling.last_mut() {
            last.push_str("-evil");
        }
        sibling.push("x".to_owned());
        entries.push(meta(&sibling, false, 1, mtime));
        // A name that is not the last component of its path.
        let mut mismatch = comps.clone();
        mismatch.extend(["d".to_owned(), "real.mseg".to_owned()]);
        let mut wrong = meta(&mismatch, false, 1, mtime);
        if let Some(map) = wrong.as_object_mut() {
            map.insert("name".to_owned(), json!("other.mseg"));
        }
        entries.push(wrong);
    }
    if recursive {
        for i in 0..faults.phantom_entries {
            let mut c = comps.clone();
            c.extend(["phantom".to_owned(), format!("p{i:06}.bin")]);
            entries.push(meta(&c, false, 1, mtime));
        }
    }
    let limit = arg
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|l| usize::try_from(l).ok())
        .unwrap_or(2000)
        .max(1);
    page(s, entries, limit)
}

fn collect(dir: &Path, comps: &[String], recursive: bool, out: &mut Vec<Value>) {
    let mut children = children_of(dir);
    children.sort_by(|a, b| a.name.cmp(&b.name));
    for child in children {
        let mut c = comps.to_vec();
        c.push(child.name.clone());
        out.push(meta(&c, child.dir, child.len, child.mtime));
        if recursive && child.dir {
            collect(&dir.join(&child.name), &c, true, out);
        }
    }
}

fn page(s: &Shared, mut entries: Vec<Value>, limit: usize) -> FakeResponse {
    let rest = if entries.len() > limit {
        entries.split_off(limit)
    } else {
        Vec::new()
    };
    let has_more = !rest.is_empty();
    let cursor = format!("cursor-{}", s.next_cursor.fetch_add(1, Ordering::SeqCst));
    if has_more && let Ok(mut c) = s.cursors.lock() {
        c.insert(cursor.clone(), (rest, limit));
    }
    json(
        200,
        &json!({ "entries": entries, "cursor": cursor, "has_more": has_more }).to_string(),
    )
}

fn list_continue(s: &Shared, arg: &Value) -> FakeResponse {
    let Some(cursor) = arg.get("cursor").and_then(Value::as_str) else {
        return api_error("reset/");
    };
    let rest = {
        let Ok(mut c) = s.cursors.lock() else {
            return empty(500);
        };
        c.remove(cursor)
    };
    match rest {
        Some((entries, limit)) => page(s, entries, limit),
        None => api_error("reset/"),
    }
}

fn create_folder(s: &Shared, path: &str) -> FakeResponse {
    let Some((p, comps)) = fs_path(&s.dir, path) else {
        return api_error("path/malformed_path/");
    };
    if comps.is_empty() {
        return conflict("folder");
    }
    if let Ok(m) = fs::metadata(&p) {
        return conflict(if m.is_dir() { "folder" } else { "file" });
    }
    if fs::create_dir_all(&p).is_err() {
        return conflict("file");
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    json(
        200,
        &json!({ "metadata": meta(&comps, true, 0, std::time::SystemTime::now()) }).to_string(),
    )
}

fn delete(s: &Shared, arg: &Value, path: &str) -> FakeResponse {
    let Some((p, comps)) = fs_path(&s.dir, path) else {
        return api_error("path/malformed_path/");
    };
    if comps.is_empty() {
        return api_error("path_lookup/not_found/.");
    }
    let Ok(m) = fs::metadata(&p) else {
        return api_error("path_lookup/not_found/.");
    };
    if let Some(rev) = arg.get("parent_rev").and_then(Value::as_str)
        && (m.is_dir()
            || rev != etag_of(m.len(), m.modified().unwrap_or(UNIX_EPOCH)).trim_matches('"'))
    {
        return api_error("path_write/conflict/file/..");
    }
    let before = meta_of_path(&p, &comps).unwrap_or(Value::Null);
    let done = if m.is_dir() {
        fs::remove_dir_all(&p)
    } else {
        fs::remove_file(&p)
    };
    if done.is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    json(200, &json!({ "metadata": before }).to_string())
}

fn upload(s: &Shared, arg: &Value, path: &str, body: &[u8]) -> FakeResponse {
    let Some((p, comps)) = fs_path(&s.dir, path) else {
        return api_error("path/malformed_path/");
    };
    if comps.is_empty() {
        return conflict("folder");
    }
    let overwrite = match arg.get("mode") {
        Some(Value::String(m)) => m == "overwrite",
        Some(Value::Object(o)) => o.get(".tag").and_then(Value::as_str) == Some("overwrite"),
        _ => false,
    };
    if let Ok(m) = fs::metadata(&p) {
        if m.is_dir() {
            return conflict("folder");
        }
        if !overwrite {
            return conflict("file");
        }
    }
    // Every parent is created; a file on the way is a conflict.
    let mut cur = s.dir.clone();
    for c in comps.iter().take(comps.len().saturating_sub(1)) {
        cur.push(c);
        match fs::metadata(&cur) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return conflict("file"),
            Err(_) => {
                if fs::create_dir(&cur).is_err() {
                    return empty(500);
                }
            }
        }
    }
    if fs::write(&p, body).is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    meta_of_path(&p, &comps).map_or_else(|| empty(500), |m| json(200, &m.to_string()))
}

fn download(s: &Shared, path: &str) -> FakeResponse {
    let Some((p, comps)) = fs_path(&s.dir, path) else {
        return api_error("path/malformed_path/");
    };
    match fs::metadata(&p) {
        Err(_) => api_error("path/not_found/."),
        Ok(m) if m.is_dir() => api_error("path/not_file/."),
        Ok(_) => {
            let result = meta_of_path(&p, &comps).unwrap_or(Value::Null);
            fs::read(&p).map_or_else(
                |_| empty(500),
                |bytes| FakeResponse::new(200, &[("Dropbox-API-Result", &ascii(&result))], bytes),
            )
        }
    }
}

fn bad_grant() -> FakeResponse {
    json(400, &json!({ "error": "invalid_grant" }).to_string())
}

/// `POST /oauth2/token`: the code exchange and the refresh.
fn token_route(s: &Shared, r: &FakeRequest) -> FakeResponse {
    let body = String::from_utf8_lossy(&r.body).into_owned();
    let field = |name: &str| -> Option<String> {
        body.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k == name).then(|| percent_decode(v, true)).flatten()
        })
    };
    let Ok(mut a) = s.auth.lock() else {
        return empty(500);
    };
    if let Some(key) = &a.app_key
        && field("client_id").as_deref() != Some(key.as_str())
    {
        return bad_grant();
    }
    match field("grant_type").as_deref() {
        Some("refresh_token") => {
            if a.refresh.is_none() || field("refresh_token") != a.refresh {
                return bad_grant();
            }
        }
        Some("authorization_code") => {
            let verifier = field("code_verifier").unwrap_or_default();
            let challenge = Base64UrlUnpadded::encode_string(&Sha256::digest(verifier.as_bytes()));
            if field("code").as_deref() != Some(AUTHORIZATION_CODE)
                || field("redirect_uri").as_deref() != Some(REDIRECT_URI)
                || a.challenge.as_ref().is_some_and(|c| *c != challenge)
            {
                return bad_grant();
            }
            a.refresh
                .get_or_insert_with(|| "refresh-token-1".to_owned());
        }
        _ => return bad_grant(),
    }
    let n = s.refreshes.fetch_add(1, Ordering::SeqCst) + 1;
    let access = format!("sl.at-{n}");
    a.access = Some(access.clone());
    json(
        200,
        &json!({
            "access_token": access,
            "token_type": "bearer",
            "expires_in": 14400,
            "refresh_token": a.refresh,
        })
        .to_string(),
    )
}

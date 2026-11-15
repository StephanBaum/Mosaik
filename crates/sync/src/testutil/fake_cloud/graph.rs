//! A Microsoft Graph server over a directory (the drive's root): children
//! listings with paging, path-addressed items, content downloads that redirect
//! to a second server (without `Authorization`), simple uploads, upload
//! sessions, folder creation and `DELETE` with `If-Match`. An item's id is
//! `root` for the drive's root and `id-<hex of its path>` for the rest.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use base64ct::{Base64UrlUnpadded, Encoding};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::Faults;
use super::dav::{children_of, empty, etag_of, json, rfc3339};
use crate::folder::remote::oauth::{REDIRECT_URI, path_segment, percent_decode};
use crate::folder::store::valid_component;
use crate::testutil::{FakeHttp, FakeRequest, FakeResponse};

/// The drive the server serves.
pub const DRIVE_ID: &str = "b!fake-drive-1";
const PAGE: usize = 200;

struct Session {
    rel: Vec<String>,
    replace: bool,
    total: usize,
    data: Vec<u8>,
}

/// The code the fake accepts at the token endpoint.
pub const GRAPH_AUTHORIZATION_CODE: &str = "ms-auth-code";

#[derive(Default)]
struct Auth {
    /// The access token that is valid now.
    access: Option<String>,
    /// The refresh tokens that are valid (all but the newest only unless the
    /// rotation is strict).
    refresh: Vec<String>,
    /// The newest refresh token.
    newest: String,
    strict: bool,
    /// The `client_id` the token endpoint expects, if set.
    client_id: Option<String>,
    /// The PKCE challenge the code was issued for, if set.
    challenge: Option<String>,
    issued: usize,
}

struct Shared {
    dir: PathBuf,
    auth: Mutex<Auth>,
    origin: OnceLock<String>,
    download_origin: OnceLock<String>,
    faults: Mutex<Faults>,
    count: AtomicUsize,
    writes: AtomicUsize,
    sessions: Mutex<HashMap<String, Session>>,
    next_session: AtomicUsize,
    chunks: Mutex<Vec<usize>>,
    /// Serialises the file operations.
    ops: Mutex<()>,
}

/// A Microsoft Graph server over a directory.
pub struct FakeGraph {
    server: FakeHttp,
    download: FakeHttp,
    shared: Arc<Shared>,
}

/// The item id of the path `rel` (components joined by `/`; empty is the
/// drive's root).
#[must_use]
pub fn id_of(rel: &str) -> String {
    if rel.is_empty() {
        "root".to_owned()
    } else {
        format!("id-{}", crate::wire::hex(rel.as_bytes()))
    }
}

fn rel_of_id(id: &str) -> Option<Vec<String>> {
    if id == "root" {
        return Some(Vec::new());
    }
    let hex = id.strip_prefix("id-")?;
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    let text = String::from_utf8(bytes?).ok()?;
    let comps: Vec<String> = text.split('/').map(str::to_owned).collect();
    comps.iter().all(|c| valid_component(c)).then_some(comps)
}

impl FakeGraph {
    /// Serves `dir` as the root of the drive [`DRIVE_ID`] for this access token.
    #[must_use]
    pub fn serve(dir: PathBuf, token: &str) -> Self {
        let shared = Arc::new(Shared {
            dir,
            auth: Mutex::new(Auth {
                access: Some(token.to_owned()),
                refresh: vec!["ms-rt-0".to_owned()],
                newest: "ms-rt-0".to_owned(),
                ..Auth::default()
            }),
            origin: OnceLock::new(),
            download_origin: OnceLock::new(),
            faults: Mutex::new(Faults::default()),
            count: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            sessions: Mutex::new(HashMap::new()),
            next_session: AtomicUsize::new(0),
            chunks: Mutex::new(Vec::new()),
            ops: Mutex::new(()),
        });
        let state = Arc::clone(&shared);
        let server = FakeHttp::start(move |r| handle(&state, r));
        let _ = shared.origin.set(server.origin().to_owned());
        let dl_state = Arc::clone(&shared);
        let download = FakeHttp::start(move |r| download_route(&dl_state, r));
        let _ = shared.download_origin.set(download.origin().to_owned());
        Self {
            server,
            download,
            shared,
        }
    }

    /// `http://127.0.0.1:<port>`.
    #[must_use]
    pub fn origin(&self) -> &str {
        self.server.origin()
    }

    /// The newest refresh token.
    #[must_use]
    pub fn refresh_token(&self) -> String {
        self.shared
            .auth
            .lock()
            .map(|a| a.newest.clone())
            .unwrap_or_default()
    }

    /// The access token that is valid now.
    #[must_use]
    pub fn access_token(&self) -> String {
        self.shared
            .auth
            .lock()
            .ok()
            .and_then(|a| a.access.clone())
            .unwrap_or_default()
    }

    /// The token endpoint expects this `client_id`.
    pub fn expect_client_id(&self, id: &str) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.client_id = Some(id.to_owned());
        }
    }

    /// The token endpoint checks the code verifier against this challenge.
    pub fn expect_challenge(&self, challenge: &str) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.challenge = Some(challenge.to_owned());
        }
    }

    /// With `true` a refresh token stops working once it was used; otherwise
    /// it stays valid next to the new one (as Microsoft keeps it for a while).
    pub fn strict_rotation(&self, strict: bool) {
        if let Ok(mut a) = self.shared.auth.lock() {
            a.strict = strict;
        }
    }

    /// How many token answers were issued.
    #[must_use]
    pub fn token_answers(&self) -> usize {
        self.shared.auth.lock().map_or(0, |a| a.issued)
    }

    /// The requests the download server saw (they must carry no `Authorization`).
    #[must_use]
    pub fn download_requests(&self) -> Vec<FakeRequest> {
        self.download.requests()
    }

    /// Every request of the main server so far.
    #[must_use]
    pub fn requests(&self) -> Vec<FakeRequest> {
        self.server.requests()
    }

    /// The sizes of the upload-session chunks received, in order.
    #[must_use]
    pub fn upload_chunks(&self) -> Vec<usize> {
        self.shared
            .chunks
            .lock()
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    /// The number of uploads, folder creations and deletions that did something.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.shared.writes.load(Ordering::SeqCst)
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

fn graph_error(status: u16, code: &str) -> FakeResponse {
    json(
        status,
        &json!({ "error": { "code": code, "message": "fake" } }).to_string(),
    )
}

fn fs_of(root: &Path, rel: &[String]) -> PathBuf {
    let mut p = root.to_path_buf();
    p.extend(rel);
    p
}

fn item_json(rel: &[String], dir: bool, len: u64, mtime: SystemTime) -> Value {
    let mut v = json!({
        "id": id_of(&rel.join("/")),
        "name": rel.last().cloned().unwrap_or_else(|| "root".to_owned()),
        "lastModifiedDateTime": rfc3339(mtime),
        "eTag": etag_of(len, mtime),
        "parentReference": { "driveId": DRIVE_ID },
    });
    if let Some(map) = v.as_object_mut() {
        if dir {
            map.insert("folder".to_owned(), json!({ "childCount": 0 }));
        } else {
            map.insert("file".to_owned(), json!({}));
            map.insert("size".to_owned(), json!(len));
        }
    }
    v
}

fn item_of_path(root: &Path, rel: &[String]) -> Option<Value> {
    let m = fs::metadata(fs_of(root, rel)).ok()?;
    Some(item_json(
        rel,
        m.is_dir(),
        m.len(),
        m.modified().unwrap_or(UNIX_EPOCH),
    ))
}

/// What a request path names.
struct Target {
    rel: Vec<String>,
    tail: Option<String>,
}

fn parse_target(path: &str) -> Option<Target> {
    if let Some(rest) = path.strip_prefix("/v1.0/me/drive/root") {
        return match rest.strip_prefix(':') {
            Some(after) => parse_after(Vec::new(), after),
            None => Some(Target {
                rel: Vec::new(),
                tail: rest
                    .strip_prefix('/')
                    .map(str::to_owned)
                    .filter(|t| !t.is_empty()),
            }),
        };
    }
    let rest = path.strip_prefix("/v1.0/drives/")?;
    let (drive, rest) = rest.split_once("/items/")?;
    if percent_decode(drive, false)? != DRIVE_ID {
        return None;
    }
    let (id, after) = match rest.find(':') {
        Some(i) => (rest.get(..i)?, rest.get(i + 1..)?),
        None => match rest.split_once('/') {
            Some((id, tail)) => {
                return Some(Target {
                    rel: rel_of_id(&percent_decode(id, false)?)?,
                    tail: Some(tail.to_owned()),
                });
            }
            None => {
                return Some(Target {
                    rel: rel_of_id(&percent_decode(rest, false)?)?,
                    tail: None,
                });
            }
        },
    };
    parse_after(rel_of_id(&percent_decode(id, false)?)?, after)
}

/// `after` is what follows the first `:` of `<item>:/a/b:/tail`.
fn parse_after(mut rel: Vec<String>, after: &str) -> Option<Target> {
    let inner = after.strip_prefix('/')?;
    let (path_part, tail) = inner.split_once(':')?;
    for c in path_part.split('/').filter(|c| !c.is_empty()) {
        let c = percent_decode(c, false)?;
        if !valid_component(&c) {
            return None;
        }
        rel.push(c);
    }
    let tail = tail
        .strip_prefix('/')
        .map(str::to_owned)
        .filter(|t| !t.is_empty());
    Some(Target { rel, tail })
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
    let (path, query) = r
        .path_and_query
        .split_once('?')
        .unwrap_or((r.path_and_query.as_str(), ""));
    // An upload session is addressed by its own URL and needs no token.
    if let Some(id) = path.strip_prefix("/upload/") {
        return upload_chunk(s, r, id);
    }
    if path == "/common/oauth2/v2.0/token" {
        return token_route(s, r);
    }
    let valid = s.auth.lock().is_ok_and(|a| {
        a.access
            .as_ref()
            .is_some_and(|t| r.header("authorization") == Some(format!("Bearer {t}").as_str()))
    });
    if !valid {
        return graph_error(401, "InvalidAuthenticationToken");
    }
    let Some(target) = parse_target(path) else {
        return graph_error(404, "itemNotFound");
    };
    let Ok(_guard) = s.ops.lock() else {
        return empty(500);
    };
    let fs_path = fs_of(&s.dir, &target.rel);
    match (r.method.as_str(), target.tail.as_deref()) {
        ("GET", None) => item_of_path(&s.dir, &target.rel).map_or_else(
            || graph_error(404, "itemNotFound"),
            |v| json(200, &v.to_string()),
        ),
        ("GET", Some("children")) => children(s, &target, query, faults),
        ("GET", Some("content")) => match fs::metadata(&fs_path) {
            Ok(m) if m.is_file() => {
                let origin = s.download_origin.get().cloned().unwrap_or_default();
                let location = format!(
                    "{origin}/dl/{}",
                    crate::wire::hex(target.rel.join("/").as_bytes())
                );
                FakeResponse::new(302, &[("Location", &location)], Vec::new())
            }
            _ => graph_error(404, "itemNotFound"),
        },
        ("PUT", Some("content")) => {
            let replace = query.contains("conflictBehavior=replace");
            put_content(s, &target.rel, replace, &r.body)
        }
        ("POST", Some("createUploadSession")) => create_session(s, &target.rel, &r.body),
        ("POST", Some("children")) => create_folder(s, &target.rel, &r.body),
        ("DELETE", None) => delete(s, r, &target.rel),
        _ => empty(405),
    }
}

fn children(s: &Shared, target: &Target, query: &str, faults: Faults) -> FakeResponse {
    let dir = fs_of(&s.dir, &target.rel);
    match fs::metadata(&dir) {
        Ok(m) if m.is_dir() => {}
        _ => return graph_error(404, "itemNotFound"),
    }
    let mut kids = children_of(&dir);
    kids.sort_by(|a, b| a.name.cmp(&b.name));
    let mut items: Vec<Value> = kids
        .iter()
        .map(|k| {
            let mut rel = target.rel.clone();
            rel.push(k.name.clone());
            item_json(&rel, k.dir, k.len, k.mtime)
        })
        .collect();
    if faults.outside_path {
        // A shortcut into another drive, and a name with a slash.
        let mut shortcut =
            json!({ "name": "shortcut.mseg", "remoteItem": { "id": "x" }, "file": {} });
        if let Some(m) = shortcut.as_object_mut() {
            m.insert("size".to_owned(), json!(1));
        }
        items.push(shortcut);
        items.push(json!({ "name": "a/b.mseg", "file": {}, "size": 1 }));
    }
    let skip: usize = query
        .split('&')
        .find_map(|p| p.strip_prefix("$skiptoken="))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let page: Vec<Value> = items.iter().skip(skip).take(PAGE).cloned().collect();
    let mut body = json!({ "value": page });
    let origin = s.origin.get().cloned().unwrap_or_default();
    if let Some(map) = body.as_object_mut() {
        if skip + PAGE < items.len() {
            let mut link = format!(
                "{origin}/v1.0/drives/{}/items/{}/children?$top={PAGE}&$skiptoken={}",
                path_segment(DRIVE_ID),
                path_segment(&id_of(&target.rel.join("/"))),
                skip + PAGE
            );
            if faults.foreign_next_link {
                link = format!("http://127.0.0.2:1/v1.0/drives/{DRIVE_ID}/items/x/children");
            }
            map.insert("@odata.nextLink".to_owned(), json!(link));
        } else if faults.foreign_next_link && skip == 0 {
            map.insert(
                "@odata.nextLink".to_owned(),
                json!("http://127.0.0.2:1/v1.0/drives/other/items/x/children"),
            );
        }
    }
    json(200, &body.to_string())
}

fn create_parents(s: &Shared, rel: &[String]) -> Result<(), FakeResponse> {
    let mut cur = s.dir.clone();
    for c in rel.iter().take(rel.len().saturating_sub(1)) {
        cur.push(c);
        match fs::metadata(&cur) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(graph_error(409, "nameAlreadyExists")),
            Err(_) => {
                if fs::create_dir(&cur).is_err() {
                    return Err(empty(500));
                }
            }
        }
    }
    Ok(())
}

fn finish_write(s: &Shared, rel: &[String], replace: bool, body: &[u8]) -> FakeResponse {
    let p = fs_of(&s.dir, rel);
    let existed = match fs::metadata(&p) {
        Ok(m) if m.is_dir() => return graph_error(409, "nameAlreadyExists"),
        Ok(_) if !replace => return graph_error(409, "nameAlreadyExists"),
        Ok(_) => true,
        Err(_) => false,
    };
    if let Err(e) = create_parents(s, rel) {
        return e;
    }
    if fs::write(&p, body).is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    item_of_path(&s.dir, rel).map_or_else(
        || empty(500),
        |v| json(if existed { 200 } else { 201 }, &v.to_string()),
    )
}

fn put_content(s: &Shared, rel: &[String], replace: bool, body: &[u8]) -> FakeResponse {
    if rel.is_empty() {
        return graph_error(409, "nameAlreadyExists");
    }
    finish_write(s, rel, replace, body)
}

fn create_session(s: &Shared, rel: &[String], body: &[u8]) -> FakeResponse {
    if rel.is_empty() {
        return graph_error(409, "nameAlreadyExists");
    }
    let arg: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let replace = arg
        .pointer("/item/@microsoft.graph.conflictBehavior")
        .and_then(Value::as_str)
        == Some("replace");
    let p = fs_of(&s.dir, rel);
    if p.exists() && !replace {
        return graph_error(409, "nameAlreadyExists");
    }
    let id = format!("session-{}", s.next_session.fetch_add(1, Ordering::SeqCst));
    if let Ok(mut sessions) = s.sessions.lock() {
        sessions.insert(
            id.clone(),
            Session {
                rel: rel.to_vec(),
                replace,
                total: 0,
                data: Vec::new(),
            },
        );
    }
    let origin = s.origin.get().cloned().unwrap_or_default();
    json(
        200,
        &json!({ "uploadUrl": format!("{origin}/upload/{id}"), "expirationDateTime": "2099-01-01T00:00:00Z" })
            .to_string(),
    )
}

fn upload_chunk(s: &Shared, r: &FakeRequest, id: &str) -> FakeResponse {
    let Ok(mut sessions) = s.sessions.lock() else {
        return empty(500);
    };
    if r.method == "DELETE" {
        sessions.remove(id);
        return empty(204);
    }
    if r.header("authorization").is_some() {
        // A session URL is pre-authenticated; a token sent to it is a leak.
        return graph_error(400, "invalidRequest");
    }
    let Some(session) = sessions.get_mut(id) else {
        return graph_error(404, "itemNotFound");
    };
    // `bytes <start>-<end>/<total>`
    let parsed = r
        .header("content-range")
        .and_then(|h| h.strip_prefix("bytes "))
        .and_then(|h| {
            let (range, total) = h.split_once('/')?;
            let (a, b) = range.split_once('-')?;
            Some((
                a.parse::<usize>().ok()?,
                b.parse::<usize>().ok()?,
                total.parse::<usize>().ok()?,
            ))
        });
    let Some((start, end, total)) = parsed else {
        return graph_error(400, "invalidRange");
    };
    if start != session.data.len() || end + 1 - start != r.body.len() || end + 1 > total {
        return graph_error(416, "invalidRange");
    }
    let last = end + 1 == total;
    if !last && !r.body.len().is_multiple_of(320 * 1024) {
        return graph_error(400, "invalidRange");
    }
    session.total = total;
    session.data.extend_from_slice(&r.body);
    if let Ok(mut c) = s.chunks.lock() {
        c.push(r.body.len());
    }
    if !last {
        return json(202, "{\"nextExpectedRanges\":[]}");
    }
    let (rel, replace, data) = (session.rel.clone(), session.replace, session.data.clone());
    sessions.remove(id);
    drop(sessions);
    let Ok(_guard) = s.ops.lock() else {
        return empty(500);
    };
    finish_write(s, &rel, replace, &data)
}

fn create_folder(s: &Shared, rel: &[String], body: &[u8]) -> FakeResponse {
    let arg: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let Some(name) = arg
        .get("name")
        .and_then(Value::as_str)
        .filter(|n| valid_component(n))
    else {
        return graph_error(400, "invalidRequest");
    };
    let mut child = rel.to_vec();
    child.push(name.to_owned());
    let p = fs_of(&s.dir, &child);
    if p.exists() {
        return graph_error(409, "nameAlreadyExists");
    }
    if !fs_of(&s.dir, rel).is_dir() {
        return graph_error(404, "itemNotFound");
    }
    if fs::create_dir(&p).is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    item_of_path(&s.dir, &child).map_or_else(|| empty(500), |v| json(201, &v.to_string()))
}

fn delete(s: &Shared, r: &FakeRequest, rel: &[String]) -> FakeResponse {
    if rel.is_empty() {
        return graph_error(403, "accessDenied");
    }
    let p = fs_of(&s.dir, rel);
    let Ok(m) = fs::metadata(&p) else {
        return graph_error(404, "itemNotFound");
    };
    if let Some(tag) = r.header("if-match")
        && tag != etag_of(m.len(), m.modified().unwrap_or(UNIX_EPOCH))
    {
        return graph_error(412, "preconditionFailed");
    }
    let done = if m.is_dir() {
        fs::remove_dir_all(&p)
    } else {
        fs::remove_file(&p)
    };
    if done.is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    empty(204)
}

/// The second server: the pre-authenticated download links.
fn download_route(s: &Shared, r: &FakeRequest) -> FakeResponse {
    if r.header("authorization").is_some() {
        return graph_error(400, "invalidRequest");
    }
    let Some(hex) = r.path_and_query.strip_prefix("/dl/") else {
        return empty(404);
    };
    let Some(rel) = rel_of_id(&format!("id-{hex}")) else {
        return empty(404);
    };
    match fs::read(fs_of(&s.dir, &rel)) {
        Ok(bytes) => FakeResponse::new(200, &[], bytes),
        Err(_) => empty(404),
    }
}

fn bad_grant() -> FakeResponse {
    json(400, &json!({ "error": "invalid_grant" }).to_string())
}

/// `POST /common/oauth2/v2.0/token`: the code exchange and the refresh. Every
/// answer carries a new refresh token.
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
    if let Some(id) = &a.client_id
        && field("client_id").as_deref() != Some(id.as_str())
    {
        return bad_grant();
    }
    match field("grant_type").as_deref() {
        Some("refresh_token") => {
            let Some(given) = field("refresh_token").filter(|t| a.refresh.contains(t)) else {
                return bad_grant();
            };
            if a.strict {
                a.refresh.retain(|t| *t != given);
            }
        }
        Some("authorization_code") => {
            let verifier = field("code_verifier").unwrap_or_default();
            let challenge = Base64UrlUnpadded::encode_string(&Sha256::digest(verifier.as_bytes()));
            if field("code").as_deref() != Some(GRAPH_AUTHORIZATION_CODE)
                || field("redirect_uri").as_deref() != Some(REDIRECT_URI)
                || a.challenge.as_ref().is_some_and(|c| *c != challenge)
            {
                return bad_grant();
            }
        }
        _ => return bad_grant(),
    }
    a.issued += 1;
    let n = a.issued;
    let (access, refresh) = (format!("ms-at-{n}"), format!("ms-rt-{n}"));
    a.access = Some(access.clone());
    a.refresh.push(refresh.clone());
    a.newest.clone_from(&refresh);
    json(
        200,
        &json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": 3599,
            "scope": "Files.ReadWrite",
            "refresh_token": refresh,
        })
        .to_string(),
    )
}

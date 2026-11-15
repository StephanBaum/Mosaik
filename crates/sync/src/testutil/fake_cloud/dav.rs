//! A `WebDAV` server over a directory, with Nextcloud's Login Flow v2 and
//! app-password routes. `PROPFIND`, `GET`, `PUT`, `MKCOL`, `MOVE` (with
//! `Overwrite`) and `DELETE` (with `If-Match`); the `ETag` is the hex of the
//! SHA-256 of the length and the modification time.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};

use crate::folder::remote::oauth::{path_segment, percent_decode};
use crate::folder::store::valid_component;
use crate::testutil::{FakeHttp, FakeRequest, FakeResponse};

/// What goes wrong on request.
#[derive(Clone, Copy, Debug, Default)]
pub struct Faults {
    /// `Depth: 1` listings show the tree as it was when this was switched
    /// on: later files are missing from them, deleted ones are still there.
    /// `GET` and `Depth: 0` stay right.
    pub stale_listing: bool,
    /// The n-th request after the faults were set (counting from 1) fails
    /// with this status and this `Retry-After`.
    pub fail_nth: Option<(usize, u16, Option<u32>)>,
    /// Listings carry one entry outside the folder and one with `%2F` in its
    /// name.
    pub outside_href: bool,
}

#[derive(Clone)]
struct Child {
    name: String,
    dir: bool,
    len: u64,
    mtime: SystemTime,
}

#[derive(Default)]
struct Login {
    complete: AtomicBool,
    foreign_endpoint: AtomicBool,
    foreign_server: AtomicBool,
}

struct Shared {
    dir: PathBuf,
    login: String,
    password: String,
    expected_auth: String,
    origin: OnceLock<String>,
    faults: Mutex<Faults>,
    snapshot: Mutex<Option<HashMap<PathBuf, Vec<Child>>>>,
    count: AtomicUsize,
    writes: AtomicUsize,
    flow: Login,
    /// Serialises the file operations.
    ops: Mutex<()>,
}

/// A `WebDAV` server over a directory.
pub struct FakeDav {
    server: FakeHttp,
    shared: Arc<Shared>,
}

impl FakeDav {
    /// Serves `dir` as `/remote.php/dav/files/<login>/` for this login and
    /// app password.
    #[must_use]
    pub fn serve(dir: PathBuf, login: &str, password: &str) -> Self {
        let expected_auth = format!(
            "Basic {}",
            Base64::encode_string(format!("{login}:{password}").as_bytes())
        );
        let shared = Arc::new(Shared {
            dir,
            login: login.to_owned(),
            password: password.to_owned(),
            expected_auth,
            origin: OnceLock::new(),
            faults: Mutex::new(Faults::default()),
            snapshot: Mutex::new(None),
            count: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            flow: Login::default(),
            ops: Mutex::new(()),
        });
        let handler_state = Arc::clone(&shared);
        let server = FakeHttp::start(move |r| handle(&handler_state, r));
        let _ = shared.origin.set(server.origin().to_owned());
        Self { server, shared }
    }

    /// `http://127.0.0.1:<port>`.
    #[must_use]
    pub fn origin(&self) -> &str {
        self.server.origin()
    }

    /// The account's collection URL, `<origin>/remote.php/dav/files/<login>`.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("{}/remote.php/dav/files/{}", self.origin(), self.shared.login)
    }

    /// Every request so far.
    #[must_use]
    pub fn requests(&self) -> Vec<FakeRequest> {
        self.server.requests()
    }

    /// The number of `PUT`, `MKCOL`, `MOVE` and `DELETE` requests that did something.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.shared.writes.load(Ordering::SeqCst)
    }

    /// Sets the faults and restarts the request count. Switching
    /// `stale_listing` on takes the snapshot the listings then show.
    pub fn set_faults(&self, faults: Faults) {
        let _guard = self.shared.ops.lock();
        if let Ok(mut snap) = self.shared.snapshot.lock() {
            *snap = faults.stale_listing.then(|| {
                let mut tree = HashMap::new();
                walk(&self.shared.dir, &mut tree);
                tree
            });
        }
        if let Ok(mut f) = self.shared.faults.lock() {
            *f = faults;
        }
        self.shared.count.store(0, Ordering::SeqCst);
    }

    /// The user has approved the Login Flow v2 in the browser.
    pub fn complete_login(&self) {
        self.shared.flow.complete.store(true, Ordering::SeqCst);
    }

    /// Makes the login flow name another origin for polling and/or in the
    /// result.
    pub fn set_login_lies(&self, foreign_endpoint: bool, foreign_server: bool) {
        self.shared
            .flow
            .foreign_endpoint
            .store(foreign_endpoint, Ordering::SeqCst);
        self.shared
            .flow
            .foreign_server
            .store(foreign_server, Ordering::SeqCst);
    }
}

/// The direct children of a directory.
fn children_of(dir: &Path) -> Vec<Child> {
    let mut children = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for item in read.filter_map(Result::ok) {
            let Ok(meta) = item.metadata() else { continue };
            children.push(Child {
                name: item.file_name().to_string_lossy().into_owned(),
                dir: meta.is_dir(),
                len: meta.len(),
                mtime: meta.modified().unwrap_or(UNIX_EPOCH),
            });
        }
    }
    children
}

/// Every directory below `dir` (itself included) with its children.
fn walk(dir: &Path, tree: &mut HashMap<PathBuf, Vec<Child>>) {
    let children = children_of(dir);
    for child in children.iter().filter(|c| c.dir) {
        walk(&dir.join(&child.name), tree);
    }
    tree.insert(dir.to_path_buf(), children);
}

fn etag_of(len: u64, mtime: SystemTime) -> String {
    let nanos = mtime.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let mut hash = Sha256::new();
    hash.update(len.to_be_bytes());
    hash.update(nanos.to_be_bytes());
    let digest = hash.finalize();
    format!("\"{}\"", crate::wire::hex(digest.as_slice().get(..16).unwrap_or(&[])))
}

/// `Sun, 15 Nov 2026 10:20:30 GMT`.
fn http_date(t: SystemTime) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let secs = i64::try_from(t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())).unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil_from_days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{}, {day:02} {} {year} {:02}:{:02}:{:02} GMT",
        DAYS.get(usize::try_from(days.rem_euclid(7)).unwrap_or(0)).unwrap_or(&"Thu"),
        MONTHS.get(usize::try_from(month - 1).unwrap_or(0)).unwrap_or(&"Jan"),
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn response_xml(href: &str, dir: bool, len: u64, mtime: SystemTime) -> String {
    let kind = if dir {
        "<d:resourcetype><d:collection/></d:resourcetype>".to_owned()
    } else {
        format!("<d:resourcetype/><d:getcontentlength>{len}</d:getcontentlength>")
    };
    format!(
        "<d:response><d:href>{}</d:href><d:propstat><d:prop>{kind}\
         <d:getlastmodified>{}</d:getlastmodified><d:getetag>{}</d:getetag></d:prop>\
         <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>",
        xml_escape(href),
        http_date(mtime),
        xml_escape(&etag_of(len, mtime)),
    )
}

fn json(status: u16, body: &str) -> FakeResponse {
    FakeResponse::new(status, &[("Content-Type", "application/json")], body)
}

fn empty(status: u16) -> FakeResponse {
    FakeResponse::new(status, &[], Vec::new())
}

fn handle(s: &Shared, r: &FakeRequest) -> FakeResponse {
    let path = r.path_and_query.split('?').next().unwrap_or_default();
    let prefix = format!("/remote.php/dav/files/{}", s.login);
    if let Some(rest) = path.strip_prefix(&prefix)
        && (rest.is_empty() || rest.starts_with('/'))
    {
        return dav(s, r, rest);
    }
    login_routes(s, r, path)
}

fn login_routes(s: &Shared, r: &FakeRequest, path: &str) -> FakeResponse {
    let origin = s.origin.get().cloned().unwrap_or_default();
    let foreign = "http://127.0.0.2:1";
    match (r.method.as_str(), path) {
        ("POST", "/index.php/login/v2") => {
            let endpoint = if s.flow.foreign_endpoint.load(Ordering::SeqCst) {
                format!("{foreign}/index.php/login/v2/poll")
            } else {
                format!("{origin}/index.php/login/v2/poll")
            };
            json(
                200,
                &format!(
                    "{{\"poll\":{{\"token\":\"poll-token\",\"endpoint\":\"{endpoint}\"}},\
                     \"login\":\"{origin}/index.php/login/v2/flow/flow-token\"}}"
                ),
            )
        }
        ("POST", "/index.php/login/v2/poll") => {
            if r.body != b"token=poll-token" || !s.flow.complete.load(Ordering::SeqCst) {
                return empty(404);
            }
            let server = if s.flow.foreign_server.load(Ordering::SeqCst) {
                foreign.to_owned()
            } else {
                origin
            };
            json(
                200,
                &format!(
                    "{{\"server\":\"{server}\",\"loginName\":\"{}\",\"appPassword\":\"{}\"}}",
                    s.login, s.password
                ),
            )
        }
        ("DELETE", "/ocs/v2.php/core/apppassword") => {
            if r.header("authorization") != Some(s.expected_auth.as_str()) {
                empty(401)
            } else if r.header("ocs-apirequest") != Some("true") {
                empty(412)
            } else {
                json(200, "{\"ocs\":{\"meta\":{\"status\":\"ok\"},\"data\":[]}}")
            }
        }
        _ => empty(404),
    }
}

/// The file-system path of the components of a request path below the
/// account collection, or `None` if one of them is not a valid component.
fn fs_path(root: &Path, rest: &str) -> Option<(PathBuf, Vec<String>)> {
    let mut out = root.to_path_buf();
    let mut comps = Vec::new();
    for raw in rest.split('/').filter(|c| !c.is_empty()) {
        let c = percent_decode(raw, false)?;
        if !valid_component(&c) {
            return None;
        }
        out.push(&c);
        comps.push(c);
    }
    Some((out, comps))
}

fn href_of(login: &str, comps: &[String], dir: bool) -> String {
    let mut href = format!("/remote.php/dav/files/{login}");
    for c in comps {
        href.push('/');
        href.push_str(&path_segment(c));
    }
    if dir {
        href.push('/');
    }
    href
}

fn dav(s: &Shared, r: &FakeRequest, rest: &str) -> FakeResponse {
    let n = s.count.fetch_add(1, Ordering::SeqCst) + 1;
    let faults = s.faults.lock().map(|f| *f).unwrap_or_default();
    if let Some((nth, status, retry)) = faults.fail_nth
        && nth == n
    {
        let retry = retry.map(|v| v.to_string());
        let headers: Vec<(&str, &str)> = retry.iter().map(|v| ("Retry-After", v.as_str())).collect();
        return FakeResponse::new(status, &headers, Vec::new());
    }
    if r.header("authorization") != Some(s.expected_auth.as_str()) {
        return FakeResponse::new(401, &[("WWW-Authenticate", "Basic realm=\"fake\"")], Vec::new());
    }
    let Some((path, comps)) = fs_path(&s.dir, rest) else {
        return empty(400);
    };
    let Ok(_guard) = s.ops.lock() else {
        return empty(500);
    };
    match r.method.as_str() {
        "PROPFIND" => propfind(s, r, &path, &comps, faults),
        "GET" => match fs::metadata(&path) {
            Ok(m) if m.is_file() => fs::read(&path).map_or_else(
                |_| empty(500),
                |body| {
                    let etag = etag_of(m.len(), m.modified().unwrap_or(UNIX_EPOCH));
                    FakeResponse::new(200, &[("ETag", &etag)], body)
                },
            ),
            Ok(_) => empty(405),
            Err(_) => empty(404),
        },
        "PUT" => put(s, &path, &r.body),
        "MKCOL" => {
            if path.exists() {
                empty(405)
            } else if !path.parent().is_some_and(Path::is_dir) {
                empty(409)
            } else if fs::create_dir(&path).is_ok() {
                s.writes.fetch_add(1, Ordering::SeqCst);
                empty(201)
            } else {
                empty(500)
            }
        }
        "MOVE" => move_to(s, r, &path),
        "DELETE" => delete(s, r, &path),
        _ => empty(405),
    }
}

fn propfind(s: &Shared, r: &FakeRequest, path: &Path, comps: &[String], faults: Faults) -> FakeResponse {
    let Ok(meta) = fs::metadata(path) else {
        return empty(404);
    };
    let is_dir = meta.is_dir();
    let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
    let mut xml = response_xml(&href_of(&s.login, comps, is_dir), is_dir, meta.len(), mtime);
    if is_dir && r.header("depth") == Some("1") {
        let snapshot = s.snapshot.lock().ok().and_then(|g| {
            g.as_ref()
                .map(|tree| tree.get(path).cloned().unwrap_or_default())
        });
        let children = snapshot.unwrap_or_else(|| children_of(path));
        for child in children {
            let mut c = comps.to_vec();
            c.push(child.name.clone());
            xml.push_str(&response_xml(
                &href_of(&s.login, &c, child.dir),
                child.dir,
                child.len,
                child.mtime,
            ));
        }
        if faults.outside_href {
            let mut elsewhere = vec!["Other".to_owned()];
            elsewhere.push("x.mseg".to_owned());
            xml.push_str(&response_xml(&href_of(&s.login, &elsewhere, false), false, 1, mtime));
            let mut slash = href_of(&s.login, comps, true);
            slash.push_str("a%2Fb.mseg");
            xml.push_str(&response_xml(&slash, false, 1, mtime));
        }
    }
    FakeResponse::new(
        207,
        &[("Content-Type", "application/xml; charset=utf-8")],
        format!("<?xml version=\"1.0\"?><d:multistatus xmlns:d=\"DAV:\">{xml}</d:multistatus>"),
    )
}

fn put(s: &Shared, path: &Path, body: &[u8]) -> FakeResponse {
    if path.is_dir() {
        return empty(405);
    }
    if !path.parent().is_some_and(Path::is_dir) {
        return empty(409);
    }
    let existed = path.exists();
    if fs::write(path, body).is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    empty(if existed { 204 } else { 201 })
}

fn move_to(s: &Shared, r: &FakeRequest, from: &Path) -> FakeResponse {
    let prefix = format!("/remote.php/dav/files/{}", s.login);
    let Some(dest_rest) = r
        .header("destination")
        .and_then(|d| d.split_once("://"))
        .and_then(|(_, rest)| rest.find('/').and_then(|i| rest.get(i..)))
        .and_then(|p| p.strip_prefix(&prefix))
    else {
        return empty(400);
    };
    let Some((to, _)) = fs_path(&s.dir, dest_rest) else {
        return empty(400);
    };
    if !from.exists() {
        return empty(404);
    }
    if !to.parent().is_some_and(Path::is_dir) {
        return empty(409);
    }
    let existed = to.exists();
    if existed && (r.header("overwrite") == Some("F") || to.is_dir()) {
        return empty(412);
    }
    if fs::rename(from, &to).is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    empty(if existed { 204 } else { 201 })
}

fn delete(s: &Shared, r: &FakeRequest, path: &Path) -> FakeResponse {
    let Ok(meta) = fs::metadata(path) else {
        return empty(404);
    };
    if let Some(tag) = r.header("if-match")
        && tag != etag_of(meta.len(), meta.modified().unwrap_or(UNIX_EPOCH))
    {
        return empty(412);
    }
    let done = if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    if done.is_err() {
        return empty(500);
    }
    s.writes.fetch_add(1, Ordering::SeqCst);
    empty(204)
}

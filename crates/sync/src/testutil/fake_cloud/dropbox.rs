//! A Dropbox API v2 server over a directory: `list_folder` (with paging and
//! `continue`), `download`, `upload` (write-once or overwrite, parents
//! created), `get_metadata`, `create_folder_v2` and `delete_v2` (with
//! `parent_rev`). The `rev` is the hex of the SHA-256 of the length and the
//! modification time. Dropbox path `/a/b` is the directory `<dir>/a/b`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde_json::{Value, json};

use super::Faults;
use super::dav::{children_of, empty, etag_of, json, rfc3339};
use crate::folder::store::valid_component;
use crate::testutil::{FakeHttp, FakeRequest, FakeResponse};

struct Shared {
    dir: PathBuf,
    token: String,
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
            token: token.to_owned(),
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
                out.push_str(&format!("\\u{u:04x}"));
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
    if r.header("authorization") != Some(format!("Bearer {}", s.token).as_str()) {
        return json(
            401,
            &json!({ "error_summary": "invalid_access_token/", "error": {} }).to_string(),
        );
    }
    let route = r.path_and_query.split('?').next().unwrap_or_default();
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

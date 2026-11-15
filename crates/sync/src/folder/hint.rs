//! A best-effort hint for the folder setup code (CRYPTO_SPEC §6.9.7, ADR-0161):
//! which provider the desktop's local folder belongs to, and where that folder
//! sits below the provider's root. The sources are the provider clients'
//! configuration files, read by the caller; this module only interprets their
//! bytes and touches no file.
//!
//! Anything unknown gives [`FolderProviderHint::Other`] with an empty server and
//! path, and the phone asks the user to choose. The result always satisfies the
//! setup code's path and server rules, so [`FolderHint`] can go straight into a
//! [`FolderSetupCode`](crate::setup_code::FolderSetupCode).

use std::path::{Path, PathBuf};

use mosaic_core::setup_code::valid_path_hint;

use crate::setup_code::FolderProviderHint;
use crate::wire::origin::canonical_origin;

/// The largest configuration file the hint reads; a larger one is ignored.
pub const MAX_HINT_SOURCE: usize = 1024 * 1024;

/// The raw bytes of the provider client configuration, as far as the caller
/// could read them (a missing or unreadable source is `None` or empty).
#[derive(Debug, Default, Clone)]
pub struct HintSources {
    /// Dropbox's `info.json`.
    pub dropbox_info_json: Option<Vec<u8>>,
    /// The roots of the `OneDrive` sync folders (the `OneDrive*` variables).
    pub onedrive_roots: Vec<PathBuf>,
    /// The Nextcloud client's `nextcloud.cfg`.
    pub nextcloud_cfg: Option<Vec<u8>>,
}

/// The hint for the setup code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderHint {
    /// The provider.
    pub provider: FolderProviderHint,
    /// The canonical `https` origin for `WebDav`, otherwise empty.
    pub server: String,
    /// The path below the provider's root, or empty.
    pub path: String,
}

impl FolderHint {
    fn other() -> Self {
        Self {
            provider: FolderProviderHint::Other,
            server: String::new(),
            path: String::new(),
        }
    }
}

/// The path components of `s`, split on both separators.
fn parts(s: &str) -> Vec<&str> {
    s.split(['/', '\\']).filter(|c| !c.is_empty()).collect()
}

/// Component equality: case-insensitive on Windows only.
fn same(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// The components of `folder` below `root`, if `folder` is at or below it.
fn below<'a>(folder: &[&'a str], root: &[&str]) -> Option<Vec<&'a str>> {
    if root.is_empty() || root.len() > folder.len() {
        return None;
    }
    root.iter()
        .zip(folder)
        .all(|(r, f)| same(r, f))
        .then(|| folder[root.len()..].to_vec())
}

/// `prefix` (empty or `/a/b`) followed by the relative components.
fn join(prefix: &str, rel: &[&str]) -> String {
    let mut out = prefix.trim_end_matches('/').to_owned();
    for part in rel {
        out.push('/');
        out.push_str(part);
    }
    out
}

/// The best (longest root) match so far.
struct Best {
    root_len: usize,
    hint: FolderHint,
}

impl Best {
    fn offer(best: &mut Option<Self>, root_len: usize, hint: FolderHint) {
        if !valid_path_hint(&hint.path) {
            return;
        }
        if best.as_ref().is_none_or(|b| root_len > b.root_len) {
            *best = Some(Self { root_len, hint });
        }
    }
}

/// Detects the provider and the path of `folder`.
///
/// The provider is the one whose root is the longest prefix of `folder`. The
/// path comparison is case-insensitive on Windows only.
#[must_use]
pub fn detect_hint(folder: &Path, sources: &HintSources) -> FolderHint {
    let folder_text = folder.to_string_lossy();
    let folder_parts = parts(&folder_text);
    let mut best: Option<Best> = None;

    if let Some(bytes) = sources
        .dropbox_info_json
        .as_deref()
        .filter(|b| b.len() <= MAX_HINT_SOURCE)
    {
        for root in dropbox_roots(bytes) {
            let root_parts = parts(&root);
            if let Some(rel) = below(&folder_parts, &root_parts) {
                let hint = FolderHint {
                    provider: FolderProviderHint::Dropbox,
                    server: String::new(),
                    path: join("", &rel),
                };
                Best::offer(&mut best, root_parts.len(), hint);
            }
        }
    }

    for root in &sources.onedrive_roots {
        let root_text = root.to_string_lossy();
        let root_parts = parts(&root_text);
        if let Some(rel) = below(&folder_parts, &root_parts) {
            let hint = FolderHint {
                provider: FolderProviderHint::OneDrive,
                server: String::new(),
                path: join("", &rel),
            };
            Best::offer(&mut best, root_parts.len(), hint);
        }
    }

    if let Some(bytes) = sources
        .nextcloud_cfg
        .as_deref()
        .filter(|b| b.len() <= MAX_HINT_SOURCE)
    {
        for sync in nextcloud_folders(bytes) {
            let local = parts(&sync.local);
            let Some(rel) = below(&folder_parts, &local) else {
                continue;
            };
            let hint = FolderHint {
                provider: FolderProviderHint::WebDav,
                server: sync.server,
                path: join(&sync.target, &rel),
            };
            Best::offer(&mut best, local.len(), hint);
        }
    }

    best.map_or_else(FolderHint::other, |b| b.hint)
}

/// The sync roots of a Dropbox `info.json`: `{"personal":{"path":".."},
/// "business":{"path":".."}}`. Malformed JSON gives none.
fn dropbox_roots(bytes: &[u8]) -> Vec<String> {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Vec::new();
    };
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    object
        .values()
        .filter_map(|account| account.get("path")?.as_str().map(str::to_owned))
        .collect()
}

/// One synced folder of the Nextcloud client.
struct NextcloudFolder {
    server: String,
    local: String,
    /// Empty (the server root) or `/a/b`.
    target: String,
}

/// The synced folders of `nextcloud.cfg` whose account URL is a canonical
/// `https` origin and whose target path is a path hint. Only the `[Accounts]`
/// section counts (a file without section headers is read as that section).
fn nextcloud_folders(bytes: &[u8]) -> Vec<NextcloudFolder> {
    use std::collections::BTreeMap;

    let Ok(text) = core::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut urls: BTreeMap<&str, &str> = BTreeMap::new();
    // (account, folder) -> (local, target)
    let mut folders: BTreeMap<(&str, &str), (Option<&str>, Option<&str>)> = BTreeMap::new();
    let mut in_accounts = true;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_accounts = section == "Accounts";
            continue;
        }
        if !in_accounts {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let Some((account, rest)) = key.split_once('\\') else {
            continue;
        };
        if account.is_empty() || !account.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if rest == "url" {
            urls.insert(account, value);
        } else if let Some(rest) = rest.strip_prefix("Folders\\") {
            let Some((index, field)) = rest.split_once('\\') else {
                continue;
            };
            let entry = folders.entry((account, index)).or_default();
            match field {
                "localPath" => entry.0 = Some(value),
                "targetPath" => entry.1 = Some(value),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for ((account, _), (local, target)) in folders {
        let (Some(local), Some(target), Some(url)) = (local, target, urls.get(account)) else {
            continue;
        };
        let Ok(server) = canonical_origin(url) else {
            continue;
        };
        if !server.starts_with("https://") || server != *url {
            continue;
        }
        let target = target.trim_end_matches('/');
        if !valid_path_hint(target) {
            continue;
        }
        out.push(NextcloudFolder {
            server,
            local: local.to_owned(),
            target: target.to_owned(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dropbox(json: &str) -> HintSources {
        HintSources {
            dropbox_info_json: Some(json.as_bytes().to_vec()),
            ..HintSources::default()
        }
    }

    fn other() -> FolderHint {
        FolderHint::other()
    }

    #[test]
    fn dropbox_personal_path_is_relative_with_slashes() {
        let sources = dropbox(r#"{"personal":{"path":"C:\\Users\\a\\Dropbox"}}"#);
        let hint = detect_hint(Path::new(r"C:\Users\a\Dropbox\Apps\Mosaic"), &sources);
        assert_eq!(hint.provider, FolderProviderHint::Dropbox);
        assert_eq!(hint.server, "");
        assert_eq!(hint.path, "/Apps/Mosaic");
    }

    #[test]
    fn dropbox_business_path_is_used_when_the_folder_is_under_it() {
        let sources = dropbox(
            r#"{"personal":{"path":"/home/a/Dropbox"},"business":{"path":"/home/a/Dropbox (Firma)"}}"#,
        );
        let hint = detect_hint(Path::new("/home/a/Dropbox (Firma)/Team/Mosaic"), &sources);
        assert_eq!(hint.provider, FolderProviderHint::Dropbox);
        assert_eq!(hint.path, "/Team/Mosaic");
        // The folder is the root itself: an empty path.
        let hint = detect_hint(Path::new("/home/a/Dropbox"), &sources);
        assert_eq!(hint.provider, FolderProviderHint::Dropbox);
        assert_eq!(hint.path, "");
    }

    #[test]
    fn onedrive_root_from_env_list() {
        let sources = HintSources {
            onedrive_roots: vec![PathBuf::from(r"C:\Users\a\OneDrive - Firma")],
            ..HintSources::default()
        };
        let hint = detect_hint(
            Path::new(r"C:\Users\a\OneDrive - Firma\Documents\Mosaic"),
            &sources,
        );
        assert_eq!(hint.provider, FolderProviderHint::OneDrive);
        assert_eq!(hint.server, "");
        assert_eq!(hint.path, "/Documents/Mosaic");
    }

    #[test]
    fn nextcloud_cfg_maps_local_to_target_path() {
        let cfg = "[Accounts]\n0\\url=https://cloud.example.com\n0\\Folders\\1\\localPath=C:/Users/a/Nextcloud/\n0\\Folders\\1\\targetPath=/Sync\n";
        let sources = HintSources {
            nextcloud_cfg: Some(cfg.as_bytes().to_vec()),
            ..HintSources::default()
        };
        let hint = detect_hint(Path::new("C:/Users/a/Nextcloud/Mosaic"), &sources);
        assert_eq!(hint.provider, FolderProviderHint::WebDav);
        assert_eq!(hint.server, "https://cloud.example.com");
        assert_eq!(hint.path, "/Sync/Mosaic");
        // A file without section headers reads as the same section.
        let bare = cfg.replace("[Accounts]\n", "");
        let sources = HintSources {
            nextcloud_cfg: Some(bare.into_bytes()),
            ..HintSources::default()
        };
        assert_eq!(
            detect_hint(Path::new("C:/Users/a/Nextcloud/Mosaic"), &sources).path,
            "/Sync/Mosaic"
        );
        // The server root as the target.
        let root = cfg.replace("targetPath=/Sync", "targetPath=/");
        let sources = HintSources {
            nextcloud_cfg: Some(root.into_bytes()),
            ..HintSources::default()
        };
        assert_eq!(
            detect_hint(Path::new("C:/Users/a/Nextcloud/Mosaic"), &sources).path,
            "/Mosaic"
        );
    }

    #[test]
    fn the_longest_root_wins() {
        let sources = HintSources {
            dropbox_info_json: Some(br#"{"personal":{"path":"/home/a"}}"#.to_vec()),
            onedrive_roots: vec![PathBuf::from("/home/a/OneDrive")],
            ..HintSources::default()
        };
        let hint = detect_hint(Path::new("/home/a/OneDrive/Mosaic"), &sources);
        assert_eq!(hint.provider, FolderProviderHint::OneDrive);
        assert_eq!(hint.path, "/Mosaic");
    }

    #[test]
    fn unknown_or_hostile_sources_give_other() {
        let folder = Path::new("/home/a/Dropbox/Mosaic");
        // Malformed JSON, and JSON of the wrong shape.
        assert_eq!(detect_hint(folder, &dropbox("{not json")), other());
        assert_eq!(detect_hint(folder, &dropbox("[1,2]")), other());
        // A folder outside every root.
        let outside = dropbox(r#"{"personal":{"path":"/home/a/Other"}}"#);
        assert_eq!(detect_hint(folder, &outside), other());
        // A path with a quote in it.
        let quote = HintSources {
            onedrive_roots: vec![PathBuf::from("/home/a")],
            ..HintSources::default()
        };
        assert_eq!(
            detect_hint(Path::new("/home/a/we\"ird"), &quote),
            other()
        );
        // A control character is refused.
        assert_eq!(
            detect_hint(Path::new("/home/a/bad\u{7}name"), &quote),
            other()
        );
        // A cfg URL that is not https, has a path or is not canonical.
        for url in [
            "http://cloud.example.com",
            "https://cloud.example.com/nextcloud",
            "https://Cloud.example.com",
        ] {
            let cfg = format!(
                "[Accounts]\n0\\url={url}\n0\\Folders\\1\\localPath=/home/a/Nextcloud\n0\\Folders\\1\\targetPath=/Sync\n"
            );
            let sources = HintSources {
                nextcloud_cfg: Some(cfg.into_bytes()),
                ..HintSources::default()
            };
            assert_eq!(
                detect_hint(Path::new("/home/a/Nextcloud/Mosaic"), &sources),
                other(),
                "{url}"
            );
        }
        // A target path that is not a path hint.
        let cfg = "[Accounts]\n0\\url=https://cloud.example.com\n0\\Folders\\1\\localPath=/home/a/Nextcloud\n0\\Folders\\1\\targetPath=/a/../b\n";
        let sources = HintSources {
            nextcloud_cfg: Some(cfg.as_bytes().to_vec()),
            ..HintSources::default()
        };
        assert_eq!(
            detect_hint(Path::new("/home/a/Nextcloud/Mosaic"), &sources),
            other()
        );
        // A source over the cap is ignored.
        let mut big = br#"{"personal":{"path":"/home/a/Dropbox"}}"#.to_vec();
        big.resize(MAX_HINT_SOURCE + 1, b' ');
        let sources = HintSources {
            dropbox_info_json: Some(big),
            ..HintSources::default()
        };
        assert_eq!(detect_hint(folder, &sources), other());
        let mut cfg = "[Accounts]\n0\\url=https://cloud.example.com\n0\\Folders\\1\\localPath=/home/a/Nextcloud\n0\\Folders\\1\\targetPath=/Sync\n".as_bytes().to_vec();
        cfg.resize(MAX_HINT_SOURCE + 1, b'\n');
        let sources = HintSources {
            nextcloud_cfg: Some(cfg),
            ..HintSources::default()
        };
        assert_eq!(
            detect_hint(Path::new("/home/a/Nextcloud/Mosaic"), &sources),
            other()
        );
    }

    #[test]
    fn path_case_matters_off_windows_only() {
        let sources = dropbox(r#"{"personal":{"path":"/home/a/dropbox"}}"#);
        let hint = detect_hint(Path::new("/home/a/Dropbox/Mosaic"), &sources);
        if cfg!(windows) {
            assert_eq!(hint.provider, FolderProviderHint::Dropbox);
            assert_eq!(hint.path, "/Mosaic");
        } else {
            assert_eq!(hint, other());
        }
    }
}

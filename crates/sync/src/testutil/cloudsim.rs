//! A simulated file-sync client between several replica directories.
//!
//! Each replica is a directory one device works in (a device's copy of the
//! shared folder). [`CloudSim::round`] propagates what changed in each
//! replica to the others through a "cloud" the way Dropbox, `OneDrive` or
//! Syncthing do, with the faults such clients have:
//!
//! - creates, replaces and deletes arrive in a seeded random order and some
//!   rounds late;
//! - a new file can arrive truncated first, and complete a round later;
//! - when two replicas changed one path, the later change becomes a
//!   provider-style conflict copy and the original path takes the cloud
//!   version;
//! - a path can be withheld from one replica for some rounds.
//!
//! The simulation is deterministic for a seed and never sleeps.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::SimRng;

/// The faults of a [`CloudSim`], in per mille.
#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// A new file arrives truncated first.
    pub truncate_permille: u32,
    /// A change is delivered a round late.
    pub delay_permille: u32,
    /// Conflicting changes make conflict copies (otherwise the cloud wins).
    pub conflict_copies: bool,
}

impl SimConfig {
    /// No faults: every change arrives whole in the round it is made.
    pub const CALM: Self = Self {
        truncate_permille: 0,
        delay_permille: 0,
        conflict_copies: true,
    };

    /// A flaky client: some truncated files, some late changes.
    pub const FLAKY: Self = Self {
        truncate_permille: 150,
        delay_permille: 250,
        conflict_copies: true,
    };
}

/// What one [`CloudSim::round`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoundStats {
    /// Files taken from a replica into the cloud.
    pub uploads: u32,
    /// Files written into a replica (partial ones included).
    pub downloads: u32,
    /// Files deleted (in the cloud or in a replica).
    pub deletes: u32,
    /// Conflict copies made.
    pub conflicts: u32,
}

impl RoundStats {
    const fn total(self) -> u32 {
        self.uploads + self.downloads + self.deletes + self.conflicts
    }
}

#[derive(Clone, Debug)]
struct CloudFile {
    content: Vec<u8>,
    version: u64,
    deleted: bool,
}

/// What a replica knows of a path from its last sync.
#[derive(Clone, Copy, Debug)]
struct Known {
    version: u64,
    digest: [u8; 32],
    /// The replica holds a truncated copy; the full file follows.
    partial: bool,
}

struct Hold {
    replica: usize,
    needle: String,
    rounds: u32,
}

/// See the [module documentation](self).
pub struct CloudSim {
    replicas: Vec<PathBuf>,
    cloud: BTreeMap<String, CloudFile>,
    known: Vec<BTreeMap<String, Known>>,
    rng: SimRng,
    config: SimConfig,
    holds: Vec<Hold>,
    conflict_counter: u32,
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Every file under `dir` as a `/`-separated relative path.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> io::Result<()> {
    let mut entries: Vec<_> = match fs::read_dir(dir) {
        Ok(r) => r.collect::<Result<_, _>>()?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if entry.file_type()?.is_dir() {
            walk(&entry.path(), &rel, out)?;
        } else {
            out.push(rel);
        }
    }
    Ok(())
}

/// A provider-style conflict-copy name for `rel`; `n` picks the style.
fn conflict_name(rel: &str, n: u32) -> String {
    let (dir, file) = rel.rsplit_once('/').map_or(("", rel), |(d, f)| (d, f));
    let (stem, ext) = file.rsplit_once('.').map_or((file, ""), |(s, e)| (s, e));
    let ext = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    let name = match n % 3 {
        0 => format!("{stem} (conflicted copy {n}){ext}"),
        1 => format!("{stem}-DESKTOP-{n:04X}{ext}"),
        _ => format!("{stem}.sync-conflict-20260101-0000{n:02}-ABCDEFG{ext}"),
    };
    if dir.is_empty() {
        name
    } else {
        format!("{dir}/{name}")
    }
}

impl CloudSim {
    /// A simulation over the given replica directories (which must exist).
    #[must_use]
    pub fn new(seed: u64, replicas: Vec<PathBuf>) -> Self {
        let known = vec![BTreeMap::new(); replicas.len()];
        Self {
            replicas,
            cloud: BTreeMap::new(),
            known,
            rng: SimRng::new(seed),
            config: SimConfig::FLAKY,
            holds: Vec::new(),
            conflict_counter: 0,
        }
    }

    /// The directory of replica `i`.
    #[must_use]
    pub fn replica(&self, i: usize) -> &Path {
        &self.replicas[i]
    }

    /// Sets the faults.
    pub const fn set_config(&mut self, config: SimConfig) {
        self.config = config;
    }

    /// Withholds every path containing `needle` from replica `replica` for
    /// `rounds` rounds.
    pub fn withhold(&mut self, replica: usize, needle: &str, rounds: u32) {
        self.holds.push(Hold {
            replica,
            needle: needle.to_owned(),
            rounds,
        });
    }

    fn held(&self, replica: usize, rel: &str) -> bool {
        self.holds
            .iter()
            .any(|h| h.replica == replica && h.rounds > 0 && rel.contains(&h.needle))
    }

    fn full_path(&self, replica: usize, rel: &str) -> PathBuf {
        let mut p = self.replicas[replica].clone();
        p.extend(rel.split('/'));
        p
    }

    /// One propagation pass over all replicas, in random order.
    ///
    /// # Errors
    /// File-system errors of the replicas.
    pub fn round(&mut self) -> io::Result<RoundStats> {
        let mut stats = RoundStats::default();
        let mut order: Vec<usize> = (0..self.replicas.len()).collect();
        self.rng.shuffle(&mut order);
        for r in order {
            self.upload(r, &mut stats)?;
            self.download(r, &mut stats)?;
        }
        for h in &mut self.holds {
            h.rounds = h.rounds.saturating_sub(1);
        }
        self.holds.retain(|h| h.rounds > 0);
        Ok(stats)
    }

    /// Replica `r`'s local changes go into the cloud.
    fn upload(&mut self, r: usize, stats: &mut RoundStats) -> io::Result<()> {
        let mut files = Vec::new();
        walk(&self.replicas[r], "", &mut files)?;
        for rel in &files {
            let path = self.full_path(r, rel);
            let content = match fs::read(&path) {
                Ok(c) => c,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            let d = digest(&content);
            match self.known[r].get(rel).copied() {
                Some(k) if k.partial || k.digest == d => {}
                Some(k) => self.changed(r, rel, content, k, stats)?,
                None => self.created(r, rel, content, d, stats),
            }
        }
        // Deleted locally.
        let gone: Vec<String> = self.known[r]
            .iter()
            .filter(|(rel, k)| {
                !k.partial
                    && !files.contains(rel)
                    && !self.cloud.get(*rel).is_some_and(|c| c.deleted)
            })
            .map(|(rel, _)| rel.clone())
            .collect();
        for rel in gone {
            let cloud_version = self.cloud.get(&rel).map_or(0, |c| c.version);
            let known = self.known[r].remove(&rel);
            if known.is_some_and(|k| k.version == cloud_version)
                && let Some(c) = self.cloud.get_mut(&rel)
                && !c.deleted
            {
                c.deleted = true;
                c.version += 1;
                stats.deletes += 1;
                let version = c.version;
                self.known[r].insert(
                    rel,
                    Known {
                        version,
                        digest: [0; 32],
                        partial: false,
                    },
                );
                // A deletion is known by its version only.
            }
        }
        Ok(())
    }

    fn created(
        &mut self,
        r: usize,
        rel: &str,
        content: Vec<u8>,
        d: [u8; 32],
        stats: &mut RoundStats,
    ) {
        match self.cloud.get_mut(rel) {
            Some(c) if !c.deleted => {
                // Someone else created the path meanwhile. The same bytes
                // are the same file; different bytes make the download step
                // keep the local file as a conflict copy.
                let version = if digest(&c.content) == d {
                    c.version
                } else {
                    0
                };
                self.known[r].insert(
                    rel.to_owned(),
                    Known {
                        version,
                        digest: d,
                        partial: false,
                    },
                );
            }
            Some(c) => {
                c.content = content;
                c.deleted = false;
                c.version += 1;
                let version = c.version;
                self.known[r].insert(
                    rel.to_owned(),
                    Known {
                        version,
                        digest: d,
                        partial: false,
                    },
                );
                stats.uploads += 1;
            }
            None => {
                self.cloud.insert(
                    rel.to_owned(),
                    CloudFile {
                        content,
                        version: 1,
                        deleted: false,
                    },
                );
                self.known[r].insert(
                    rel.to_owned(),
                    Known {
                        version: 1,
                        digest: d,
                        partial: false,
                    },
                );
                stats.uploads += 1;
            }
        }
    }

    fn changed(
        &mut self,
        r: usize,
        rel: &str,
        content: Vec<u8>,
        known: Known,
        stats: &mut RoundStats,
    ) -> io::Result<()> {
        let cloud_version = self.cloud.get(rel).map_or(0, |c| c.version);
        if known.version == cloud_version {
            let d = digest(&content);
            if let Some(c) = self.cloud.get_mut(rel) {
                c.content = content;
                c.deleted = false;
                c.version += 1;
                let version = c.version;
                self.known[r].insert(
                    rel.to_owned(),
                    Known {
                        version,
                        digest: d,
                        partial: false,
                    },
                );
                stats.uploads += 1;
            }
            return Ok(());
        }
        // The cloud moved on: this replica's change becomes a conflict copy
        // (or is lost, if copies are off), and the path follows the cloud.
        if self.config.conflict_copies {
            self.conflict_counter += 1;
            let copy = conflict_name(rel, self.conflict_counter);
            fs::rename(self.full_path(r, rel), self.full_path(r, &copy))?;
            stats.conflicts += 1;
        } else {
            fs::remove_file(self.full_path(r, rel))?;
        }
        self.known[r].remove(rel);
        Ok(())
    }

    /// The cloud's changes go into replica `r`.
    fn download(&mut self, r: usize, stats: &mut RoundStats) -> io::Result<()> {
        let mut paths: Vec<String> = self.cloud.keys().cloned().collect();
        self.rng.shuffle(&mut paths);
        for rel in paths {
            let Some(cloud) = self.cloud.get(&rel).cloned() else {
                continue;
            };
            let known = self.known[r].get(&rel).copied();
            let current = known.is_some_and(|k| k.version == cloud.version && !k.partial);
            if current || self.held(r, &rel) {
                continue;
            }
            let partial = known.is_some_and(|k| k.partial && k.version == cloud.version);
            if !partial && self.rng.chance(self.config.delay_permille) {
                continue;
            }
            self.deliver(r, &rel, &cloud, partial, stats)?;
        }
        Ok(())
    }

    fn deliver(
        &mut self,
        r: usize,
        rel: &str,
        cloud: &CloudFile,
        completing: bool,
        stats: &mut RoundStats,
    ) -> io::Result<()> {
        let path = self.full_path(r, rel);
        if cloud.deleted {
            match fs::remove_file(&path) {
                Ok(()) => stats.deletes += 1,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            self.known[r].insert(
                rel.to_owned(),
                Known {
                    version: cloud.version,
                    digest: [0; 32],
                    partial: false,
                },
            );
            return Ok(());
        }
        let truncate = !completing
            && cloud.content.len() > 1
            && self.rng.chance(self.config.truncate_permille);
        let bytes = if truncate {
            let keep = 1 + self.rng.below(cloud.content.len() - 1);
            &cloud.content[..keep]
        } else {
            &cloud.content[..]
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // A local file that differs and was never synced becomes a conflict
        // copy before the cloud version lands.
        if path.exists() && self.known[r].get(rel).is_some_and(|k| k.version == 0) {
            self.conflict_counter += 1;
            let copy = conflict_name(rel, self.conflict_counter);
            fs::rename(&path, self.full_path(r, &copy))?;
            stats.conflicts += 1;
        }
        fs::write(&path, bytes)?;
        stats.downloads += 1;
        self.known[r].insert(
            rel.to_owned(),
            Known {
                version: cloud.version,
                digest: digest(&cloud.content),
                partial: truncate,
            },
        );
        Ok(())
    }

    /// Runs rounds without faults until nothing moves any more (all held
    /// paths released, all partial files completed). Returns the rounds it
    /// took.
    ///
    /// # Errors
    /// File-system errors of the replicas.
    ///
    /// # Panics
    /// If the simulation does not settle in 500 rounds.
    pub fn settle(&mut self) -> io::Result<u32> {
        let saved = self.config;
        self.config = SimConfig {
            conflict_copies: saved.conflict_copies,
            ..SimConfig::CALM
        };
        let mut rounds = 0;
        let mut quiet = 0;
        while quiet < 2 {
            let stats = self.round()?;
            rounds += 1;
            let pending =
                !self.holds.is_empty() || self.known.iter().any(|m| m.values().any(|k| k.partial));
            quiet = if stats.total() == 0 && !pending {
                quiet + 1
            } else {
                0
            };
            assert!(rounds < 500, "the cloud simulation did not settle");
        }
        self.config = saved;
        Ok(rounds)
    }
}

//! The simulated file-sync client itself: what it delivers, when, and the
//! faults it makes.

#![expect(
    clippy::unwrap_used,
    reason = "the helper below is not a test function"
)]

use std::fs;

use mosaic_sync::testutil::{CloudSim, SimConfig};

fn replicas(n: usize) -> (tempfile::TempDir, Vec<std::path::PathBuf>) {
    let tmp = tempfile::tempdir().unwrap();
    let dirs: Vec<_> = (0..n)
        .map(|i| {
            let p = tmp.path().join(format!("r{i}"));
            fs::create_dir_all(&p).unwrap();
            p
        })
        .collect();
    (tmp, dirs)
}

#[test]
fn creates_replaces_and_deletes_reach_every_replica() {
    let (_tmp, dirs) = replicas(3);
    let mut sim = CloudSim::new(1, dirs.clone());
    sim.set_config(SimConfig::CALM);
    fs::create_dir_all(dirs[0].join("a/b")).unwrap();
    fs::write(dirs[0].join("a/b/file.bin"), b"one").unwrap();
    sim.settle().unwrap();
    for d in &dirs {
        assert_eq!(fs::read(d.join("a/b/file.bin")).unwrap(), b"one");
    }
    fs::write(dirs[2].join("a/b/file.bin"), b"two, longer").unwrap();
    sim.settle().unwrap();
    for d in &dirs {
        assert_eq!(fs::read(d.join("a/b/file.bin")).unwrap(), b"two, longer");
    }
    fs::remove_file(dirs[1].join("a/b/file.bin")).unwrap();
    sim.settle().unwrap();
    for d in &dirs {
        assert!(!d.join("a/b/file.bin").exists());
    }
}

#[test]
fn a_new_file_can_arrive_truncated_first() {
    let (_tmp, dirs) = replicas(2);
    let mut sim = CloudSim::new(2, dirs.clone());
    sim.set_config(SimConfig {
        truncate_permille: 1000,
        delay_permille: 0,
        conflict_copies: true,
    });
    let content = vec![7u8; 4096];
    fs::write(dirs[0].join("seg.bin"), &content).unwrap();
    let mut saw_partial = false;
    for _ in 0..4 {
        sim.round().unwrap();
        if let Ok(b) = fs::read(dirs[1].join("seg.bin"))
            && b.len() < content.len()
        {
            saw_partial = true;
        }
    }
    assert!(saw_partial, "the file arrived short first");
    sim.settle().unwrap();
    assert_eq!(fs::read(dirs[1].join("seg.bin")).unwrap(), content);
}

#[test]
fn two_changes_to_one_path_make_a_conflict_copy() {
    let (_tmp, dirs) = replicas(2);
    let mut sim = CloudSim::new(3, dirs.clone());
    sim.set_config(SimConfig::CALM);
    fs::write(dirs[0].join("head.mck"), b"v1").unwrap();
    sim.settle().unwrap();
    fs::write(dirs[0].join("head.mck"), b"from zero").unwrap();
    fs::write(dirs[1].join("head.mck"), b"from one").unwrap();
    sim.settle().unwrap();
    let names = |d: &std::path::Path| -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(&dirs[0]), names(&dirs[1]));
    let all = names(&dirs[0]);
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(all.iter().any(|n| n == "head.mck"));
    let copy = all.iter().find(|n| *n != "head.mck").unwrap();
    assert_eq!(copy.rsplit_once('.').map(|(_, e)| e), Some("mck"));
    let contents: Vec<Vec<u8>> = all
        .iter()
        .map(|n| fs::read(dirs[0].join(n)).unwrap())
        .collect();
    assert!(contents.contains(&b"from zero".to_vec()));
    assert!(contents.contains(&b"from one".to_vec()));
}

#[test]
fn a_path_can_be_withheld_for_some_rounds() {
    let (_tmp, dirs) = replicas(2);
    let mut sim = CloudSim::new(4, dirs.clone());
    sim.set_config(SimConfig::CALM);
    sim.withhold(1, "secret", 3);
    fs::write(dirs[0].join("secret.bin"), b"x").unwrap();
    fs::write(dirs[0].join("other.bin"), b"y").unwrap();
    sim.round().unwrap();
    sim.round().unwrap();
    assert!(dirs[1].join("other.bin").exists());
    assert!(!dirs[1].join("secret.bin").exists());
    sim.settle().unwrap();
    assert!(dirs[1].join("secret.bin").exists());
}

#[test]
fn a_seed_replays_the_same_run() {
    let run = |seed: u64| {
        let (_tmp, dirs) = replicas(3);
        let mut sim = CloudSim::new(seed, dirs.clone());
        let mut stats = Vec::new();
        for i in 0..6u8 {
            fs::write(
                dirs[usize::from(i) % 3].join(format!("f{i}.bin")),
                vec![i; 300],
            )
            .unwrap();
            let s = sim.round().unwrap();
            stats.push((s.uploads, s.downloads, s.deletes, s.conflicts));
        }
        stats
    };
    assert_eq!(run(9), run(9));
}

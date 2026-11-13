//! The challenge seeder (`examples/contest_seed.rs`) against a loopback
//! server: it writes the files, `public.json` holds no flag and no secret of
//! `private.json`, and a second run is refused without touching the first.

use mosaic_core::Argon2Profile;
use mosaic_sync::wire::json::RegistrationMode;
use serde_json::Value;

use crate::loopback::LoopbackServer;

#[path = "../../examples/contest_seed.rs"]
#[expect(dead_code, reason = "the example's main is not used by the test")]
mod seeder;

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(m) => m.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

#[test]
fn contest_seed_writes_public_json_without_flags_or_secrets() {
    // WebAuthn needs a host name for the relying party: `localhost`.
    let mut server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
    let port = server.base.rsplit(':').next().unwrap().to_owned();
    server.base = format!("http://localhost:{port}");
    let out = tempfile::tempdir().unwrap();
    let clock = server.clock.clone();
    let tick = server.clock.clone();
    let options = || seeder::Options {
        server: server.base.clone(),
        out: out.path().to_owned(),
        profile: Argon2Profile::Baseline,
        now_ms: Box::new({
            let clock = clock.clone();
            move || mosaic_server::Clock::now_ms(&*clock)
        }),
        // A minute of server time per step keeps the rate limits out of the way.
        pace: Box::new({
            let tick = tick.clone();
            move || tick.advance_ms(60_000)
        }),
    };

    let done = seeder::seed(&options()).unwrap();
    let public_text = std::fs::read_to_string(&done.public).unwrap();
    let private: Value =
        serde_json::from_str(&std::fs::read_to_string(&done.private).unwrap()).unwrap();
    let public: Value = serde_json::from_str(&public_text).unwrap();

    // No flag anywhere in public.json, and none of private.json's strings.
    assert!(!public_text.contains("MOSAIC{"));
    let mut secrets = Vec::new();
    strings(&private, &mut secrets);
    assert!(secrets.len() >= 11);
    for s in &secrets {
        assert!(
            !public_text.contains(s.as_str()),
            "public.json leaks a secret"
        );
    }
    // What the rules publish is there.
    assert!(public["accounts"]["c4"]["password"].is_string());
    assert!(public["accounts"]["c4"]["secretKey"].is_string());
    assert!(public["artifacts"]["c3.mosaicx"].is_string());
    for artifact in ["c2-store/mosaic.db", "c3.mosaicx", "c6-vault/mosaic.db"] {
        assert!(out.path().join(artifact).is_file(), "{artifact} missing");
    }
    assert!(!out.path().join(".staging").exists());

    // A second run is refused and leaves the first seed alone.
    let again = seeder::seed(&options()).err().unwrap();
    assert!(again.contains("already holds a seed"), "{again}");
    assert_eq!(std::fs::read_to_string(&done.public).unwrap(), public_text);
}

fn table_count(server: &LoopbackServer, table: &str) -> i64 {
    server
        .state
        .db()
        .run(|c| {
            Ok::<_, mosaic_server::ApiError>(c.query_row(
                &format!("SELECT COUNT(*) FROM {table}"),
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

/// M9 T12: running the seeder a second time is refused before it sends a
/// single request: against a server that cannot be reached at all, with the
/// pace hook (called before every request) never called, and with the server
/// untouched when it can be reached. A directory holding only one of the two
/// files counts as seeded. A rerun into a clean directory against the seeded
/// server fails at the first registration and writes no result file.
#[test]
fn contest_seed_second_run_is_refused_before_it_touches_the_server() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
    let port = server.base.rsplit(':').next().unwrap().to_owned();
    server.base = format!("http://localhost:{port}");
    let out = tempfile::tempdir().unwrap();
    let clock = server.clock.clone();
    let paced = Rc::new(Cell::new(0u32));
    let options = |base: String, dir: &std::path::Path| seeder::Options {
        server: base,
        out: dir.to_owned(),
        profile: Argon2Profile::Baseline,
        now_ms: Box::new({
            let clock = clock.clone();
            move || mosaic_server::Clock::now_ms(&*clock)
        }),
        pace: Box::new({
            let clock = clock.clone();
            let paced = Rc::clone(&paced);
            move || {
                paced.set(paced.get() + 1);
                clock.advance_ms(60_000);
            }
        }),
    };

    let done = seeder::seed(&options(server.base.clone(), out.path())).unwrap();
    let public = std::fs::read(&done.public).unwrap();
    let private = std::fs::read(&done.private).unwrap();
    let accounts = table_count(&server, "accounts");
    let sessions = table_count(&server, "sessions");
    let audit = table_count(&server, "audit");
    assert!(accounts >= 3, "{accounts}");

    // Refused up front: an address nothing listens on proves no request left.
    let paced_before = paced.get();
    let dead = "http://localhost:1".to_owned();
    let err = seeder::seed(&options(dead, out.path())).err().unwrap();
    assert!(err.contains("already holds a seed"), "{err}");
    assert_eq!(paced.get(), paced_before, "a request was paced");
    // Through the live server as well: nothing changed.
    let err = seeder::seed(&options(server.base.clone(), out.path()))
        .err()
        .unwrap();
    assert!(err.contains("already holds a seed"), "{err}");
    assert_eq!(paced.get(), paced_before, "a request was paced");
    assert_eq!(table_count(&server, "accounts"), accounts);
    assert_eq!(table_count(&server, "sessions"), sessions);
    assert_eq!(table_count(&server, "audit"), audit);
    assert_eq!(std::fs::read(&done.public).unwrap(), public);
    assert_eq!(std::fs::read(&done.private).unwrap(), private);

    // Either file alone is a seed too.
    for name in ["public.json", "private.json"] {
        let half = tempfile::tempdir().unwrap();
        std::fs::write(half.path().join(name), b"{}").unwrap();
        let err = seeder::seed(&options("http://localhost:1".to_owned(), half.path()))
            .err()
            .unwrap();
        assert!(err.contains("already holds a seed"), "{name}: {err}");
        assert_eq!(paced.get(), paced_before);
    }

    // A clean directory against the seeded server: the registration is
    // refused, no result file appears and the server gained no account.
    let fresh = tempfile::tempdir().unwrap();
    let err = seeder::seed(&options(server.base.clone(), fresh.path()))
        .err()
        .unwrap();
    assert_ne!(err.len(), 0);
    assert!(!fresh.path().join("public.json").exists());
    assert!(!fresh.path().join("private.json").exists());
    assert_eq!(table_count(&server, "accounts"), accounts);
}

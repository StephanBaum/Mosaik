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

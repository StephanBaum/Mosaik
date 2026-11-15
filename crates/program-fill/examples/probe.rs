//! Manual Windows check harness (`docs/PROGRAM_FILL.md`): waits 5 s so a field of
//! another program can be focused, captures that field and its program, prints
//! the identity and what linking would store, then re-verifies every 500 ms
//! for 30 s. Writes nothing into any program.
//!
//! `cargo run -p mosaic-program-fill --example probe [-- <saved winapp:// URL>...]`
//! Saved URLs are checked with `link_again_needed` (signer or file changed).
//!
//! `cargo run -p mosaic-program-fill --example probe -- --host` starts a
//! `WindowsHost` instead and prints each event (program, class, role, rectangle,
//! never a value). Type `f` and Enter in this console to fill the latest offer
//! with the user name `mosaic-test` and a test password (submit off), `q` to
//! quit. The fill is the real one: the program must be a test dialog.
#![cfg_attr(
    windows,
    expect(clippy::print_stdout, reason = "manual Windows check harness")
)]

#[cfg(windows)]
fn main() {
    harness::run();
}

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod harness {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use mosaic_core::SecretString;
    use mosaic_core::model::LoginUrl;
    use mosaic_program_fill::host::{FillRequest, Host, HostEvent, monotonic_ms};
    use mosaic_program_fill::identity::CapturedIdentity;
    use mosaic_program_fill::policy::{link_again_needed, propose_binding};
    use mosaic_program_fill::verify::{FieldKind, WriteKind, check};
    use mosaic_program_fill::windows::{CaptureError, WindowsHost, WindowsProbe};

    const WAIT: Duration = Duration::from_secs(5);
    const LOOP_FOR: Duration = Duration::from_secs(30);
    const EVERY: Duration = Duration::from_millis(500);

    fn hex8(bytes: &[u8; 32]) -> String {
        use std::fmt::Write as _;
        bytes.iter().take(8).fold(String::new(), |mut out, b| {
            // Writing to a String cannot fail.
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    fn print_identity(identity: &CapturedIdentity) {
        println!("exe:      {}", identity.exe_name);
        println!("class:    {:?}", identity.class);
        println!(
            "package:  {}",
            identity.package_family.as_deref().unwrap_or("-")
        );
        match &identity.signer_display {
            Some(d) => println!(
                "signer:   CN={} O={}",
                d.common_name.as_deref().unwrap_or("-"),
                d.organization.as_deref().unwrap_or("-")
            ),
            None => println!("signer:   -"),
        }
        println!(
            "hash:     signer {} / file {}",
            identity.signer_id.as_ref().map_or_else(|| "-".into(), hex8),
            identity
                .file_sha256
                .as_ref()
                .map_or_else(|| "-".into(), hex8)
        );
        println!("path:     {}", identity.final_path);
    }

    /// The password of the `--host` fill: accents, a character outside the
    /// BMP and more than eight UTF-16 units, so batches and pairs are exercised.
    const TEST_PASSWORD: &str = "P\u{e4}ssw\u{f6}rd-\u{1F600}-12345678";

    fn describe(event: &HostEvent) -> String {
        match event {
            HostEvent::Field(offer) => format!(
                "field #{}: {} ({:?}) {:?} at {:?}",
                offer.token, offer.identity.exe_name, offer.identity.class, offer.role, offer.rect
            ),
            other => format!("{other:?}"),
        }
    }

    fn run_host() {
        let latest: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
        let seen = Arc::clone(&latest);
        let host = WindowsHost::new();
        let started = host.start(Arc::new(move |event| {
            println!("event: {}", describe(&event));
            if let Ok(mut slot) = seen.lock() {
                match &event {
                    HostEvent::Field(offer) | HostEvent::ContinueOffer(offer) => {
                        *slot = Some(offer.token);
                    }
                    HostEvent::Continue { token } => *slot = Some(*token),
                    _ => {}
                }
            }
        }));
        if let Err(err) = started {
            return println!("host failed to start: {err}");
        }
        println!("Watching focus. f + Enter fills the latest offer, q + Enter quits.");
        for line in std::io::stdin().lines().map_while(Result::ok) {
            match line.trim() {
                "q" => break,
                "f" => {
                    let token = latest.lock().ok().and_then(|slot| *slot);
                    let Some(token) = token else {
                        println!("no offer yet");
                        continue;
                    };
                    let request = FillRequest {
                        token,
                        username: Some("mosaic-test".to_owned()),
                        password: SecretString::from_str_copy(TEST_PASSWORD).ok(),
                        submit: false,
                        picked_at_ms: monotonic_ms(),
                        continuation: false,
                    };
                    println!("fill: {:?}", host.fill(request));
                }
                _ => {}
            }
        }
        host.stop();
    }

    pub fn run() {
        if std::env::args().any(|a| a == "--host") {
            return run_host();
        }
        let saved: Vec<LoginUrl> = std::env::args()
            .skip(1)
            .map(|a| LoginUrl::new(&a))
            .collect();
        println!(
            "Focus a text field of another program within {} s.",
            WAIT.as_secs()
        );
        std::thread::sleep(WAIT);
        let start = Instant::now();
        let now_ms = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        let probe = match WindowsProbe::new() {
            Ok(probe) => probe,
            Err(err) => return println!("probe failed to start: {err}"),
        };
        let captured = match probe.capture_focused(now_ms()) {
            Ok(captured) => captured,
            Err(CaptureError::NotAField) => return println!("not a field: nothing captured"),
            Err(err) => return println!("nothing captured: {err}"),
        };
        print_identity(&captured.identity);
        println!("field:    {:?}", captured.session.target_kind);
        match propose_binding(&captured.identity, &[]) {
            Ok(proposed) => println!(
                "binding:  {} ({:?} confirmation(s), warnings {:?})",
                proposed.url.canonical(),
                proposed.confirmations,
                proposed.warnings
            ),
            Err(err) => println!("binding:  refused ({err})"),
        }
        if !saved.is_empty() {
            println!(
                "link again needed: {:?}",
                link_again_needed(&saved, &captured.identity, None)
            );
        }

        let write = match captured.session.target_kind {
            FieldKind::Password => WriteKind::Password,
            FieldKind::Text => WriteKind::Username,
        };
        let until = Instant::now() + LOOP_FOR;
        while Instant::now() < until {
            let verdict = check(&probe, &captured.session, write, now_ms());
            println!("[{:>5} ms] {verdict:?}", now_ms());
            std::thread::sleep(EVERY);
        }
    }
}

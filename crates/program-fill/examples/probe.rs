//! Manual Windows check harness (`docs/PROGRAM_FILL.md`): waits 5 s so a field of
//! another program can be focused, captures that field and its program, prints
//! the identity and what linking would store, then re-verifies every 500 ms
//! for 30 s. Writes nothing into any program.
//!
//! `cargo run -p mosaic-program-fill --example probe [-- <saved winapp:// URL>...]`
//! Saved URLs are checked with `link_again_needed` (signer or file changed).
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
    use std::time::{Duration, Instant};

    use mosaic_core::model::LoginUrl;
    use mosaic_program_fill::identity::CapturedIdentity;
    use mosaic_program_fill::policy::{link_again_needed, propose_binding};
    use mosaic_program_fill::verify::{FieldKind, WriteKind, check};
    use mosaic_program_fill::windows::{CaptureError, WindowsProbe};

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

    pub fn run() {
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

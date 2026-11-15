//! Build script: Tauri's context, the Windows resource file, and the ACL.
//!
//! The app manifest lists every IPC command, so Tauri generates an
//! `allow-<command>` permission per command and checks every invoke against
//! the capabilities in `capabilities/` (with an app manifest, commands the
//! capabilities do not grant are refused). The list is shared with the
//! crate (`src/commands/names.rs`), and a test checks that the capability
//! grants exactly these commands.
#![forbid(unsafe_code)]

include!("src/commands/names.rs");

fn main() {
    // The app manifest knows every command (the main window's and the two picker windows');
    // which window may call which is in `capabilities/`.
    let mut all: Vec<&str> = COMMANDS.to_vec();
    for command in CHIP_COMMANDS.iter().chain(LIST_COMMANDS) {
        if !all.contains(command) {
            all.push(command);
        }
    }
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(all.leak()));
    if let Err(error) = tauri_build::try_build(attributes) {
        // A build script reports failure through cargo's protocol; there is
        // no secret anywhere near this code.
        #[expect(
            clippy::print_stderr,
            reason = "a build script reports failure on stderr"
        )]
        {
            eprintln!("tauri-build failed: {error:#}");
        }
        std::process::exit(1);
    }
}

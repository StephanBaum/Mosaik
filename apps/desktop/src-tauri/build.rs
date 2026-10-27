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
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(error) = tauri_build::try_build(attributes) {
        // A build script reports failure through cargo's protocol; there is
        // no secret anywhere near this code.
        #[allow(clippy::print_stderr)]
        {
            eprintln!("tauri-build failed: {error:#}");
        }
        std::process::exit(1);
    }
}

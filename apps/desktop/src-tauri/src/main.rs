//! Entry point of the Mosaic desktop app.
#![forbid(unsafe_code)]
// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if mosaic_desktop::run().is_err() {
        // Nothing secret exists yet when startup fails; exit non-zero.
        std::process::exit(1);
    }
}

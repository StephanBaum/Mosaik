//! `mosaic` command-line client.
//!
//! Status: scaffold only. Implementation lands in M8; see `docs/PROGRESS.md`.
#![forbid(unsafe_code)]
#![allow(clippy::print_stdout, clippy::print_stderr)] // a CLI talks to the terminal

use std::process::ExitCode;

fn main() -> ExitCode {
    if let Some("--version" | "-V") = std::env::args().nth(1).as_deref() {
        println!("mosaic {}", env!("CARGO_PKG_VERSION"));
        ExitCode::SUCCESS
    } else {
        eprintln!("mosaic: the CLI is not implemented yet (planned for M8).");
        ExitCode::from(2)
    }
}

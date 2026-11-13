//! `mosaic`: the Mosaic command-line client binary. All code lives in the `mosaic_cli` library.
#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(mosaic_cli::real_main())
}

//! The `mosaic-server` binary. All logic is in the library ([`mosaic_server::cli`]);
//! this file only wires stdout, stderr and the exit code.
#![forbid(unsafe_code)]
#![expect(
    clippy::print_stderr,
    reason = "the binary's top-level output layer reports a failed command"
)]

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = std::io::stdout().lock();
    let result = mosaic_server::cli::run(&args, &mut out);
    let _ = out.flush();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mosaic-server: {e}");
            ExitCode::FAILURE
        }
    }
}

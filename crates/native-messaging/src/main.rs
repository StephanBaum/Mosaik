//! `mosaic-native-host`: the native messaging host of the Mosaic browser
//! extension.
//!
//! - `mosaic-native-host install [--browsers chrome,edge,chromium,firefox]`
//!   registers this binary with the browsers; `uninstall` removes
//!   everything `install` wrote.
//! - Started by a browser, it checks who called it, then relays the
//!   browser's stdio messages to the running desktop app.
//!
//! Exit codes: 0 normal end (the browser closed stdin), 1 other failure,
//! 2 the caller is not an allowed extension (nothing connected), 3 a
//! malformed frame from the browser.
//!
//! This file is the output layer of the host: it is the only place that
//! prints, and it prints only fixed texts and file paths, never message
//! content.
#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the command-line output layer of the host"
)]

use std::ffi::OsString;
use std::process::ExitCode;

use mosaic_native_messaging::error::Error;
use mosaic_native_messaging::ids::caller_from_args;
use mosaic_native_messaging::install::{self, Browser, Layout, RegExe};
use mosaic_native_messaging::relay::{EndpointSource, run_host};

const EXIT_FAILURE: u8 = 1;
const EXIT_CALLER: u8 = 2;
const EXIT_MALFORMED: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match args.get(1).and_then(|a| a.to_str()) {
        Some("install") => install_command(args.get(2..).unwrap_or_default()),
        Some("uninstall") => uninstall_command(args.get(2..).unwrap_or_default()),
        _ => relay(&args),
    }
}

fn relay(args: &[OsString]) -> ExitCode {
    let Ok(caller) = caller_from_args(args) else {
        eprintln!("mosaic-native-host: not started by the Mosaic browser extension");
        return ExitCode::from(EXIT_CALLER);
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return ExitCode::from(EXIT_FAILURE);
    };
    let result = runtime.block_on(run_host(
        caller,
        tokio::io::stdin(),
        tokio::io::stdout(),
        EndpointSource::Default,
    ));
    // A pending blocking read of stdin must not hold the process open.
    runtime.shutdown_background();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(
            Error::FrameEmpty | Error::FrameTooLarge | Error::FrameTruncated | Error::Malformed,
        ) => ExitCode::from(EXIT_MALFORMED),
        Err(_) => ExitCode::from(EXIT_FAILURE),
    }
}

/// `--browsers a,b,c`; all browsers by default.
fn browsers_from(args: &[OsString]) -> Result<Vec<Browser>, Error> {
    match args {
        [] => Ok(Browser::ALL.to_vec()),
        [flag, list] if flag == "--browsers" => {
            Browser::parse_list(list.to_str().ok_or(Error::Install)?)
        }
        _ => Err(Error::Install),
    }
}

fn install_command(args: &[OsString]) -> ExitCode {
    let result = (|| {
        let browsers = browsers_from(args)?;
        let host = std::env::current_exe().map_err(|_| Error::Install)?;
        let layout = Layout::current()?;
        install::install(&layout, &browsers, &host, &mut RegExe)
    })();
    let Ok(plan) = result else {
        eprintln!("usage: mosaic-native-host install [--browsers chrome,edge,chromium,firefox]");
        return ExitCode::from(EXIT_FAILURE);
    };
    for file in &plan.files {
        println!("wrote {}", file.path.display());
    }
    for entry in &plan.registry {
        println!("registered HKCU\\{}", entry.key);
    }
    ExitCode::SUCCESS
}

fn uninstall_command(args: &[OsString]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: mosaic-native-host uninstall");
        return ExitCode::from(EXIT_FAILURE);
    }
    let Ok(plan) = Layout::current().and_then(|layout| install::uninstall(&layout, &mut RegExe))
    else {
        eprintln!("mosaic-native-host: uninstall failed");
        return ExitCode::from(EXIT_FAILURE);
    };
    for file in &plan.files {
        println!("removed {}", file.path.display());
    }
    ExitCode::SUCCESS
}

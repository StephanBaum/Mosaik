//! `mosaic-native-host`: the native messaging host of the Mosaic browser
//! extension.
//!
//! - `mosaic-native-host install [--browsers chrome,edge,chromium,firefox]`
//!   registers this binary with the browsers; `uninstall` removes
//!   everything `install` wrote.
//! - `mosaic-native-host manifest --system --os <os> --out <dir> [--host <path>]`
//!   writes the system manifests of the installers (build time).
//! - `mosaic-native-host check` lists the per-user manifests and registry
//!   keys that point to another host (exit 0 none, 1 some).
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
use mosaic_native_messaging::install::{self, Browser, Layout, Os, RegExe};
use mosaic_native_messaging::relay::{EndpointSource, run_host};

const EXIT_FAILURE: u8 = 1;
const EXIT_CALLER: u8 = 2;
const EXIT_MALFORMED: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match args.get(1).and_then(|a| a.to_str()) {
        Some("install") => install_command(args.get(2..).unwrap_or_default()),
        Some("uninstall") => uninstall_command(args.get(2..).unwrap_or_default()),
        Some("manifest") => manifest_command(args.get(2..).unwrap_or_default()),
        Some("check") => check_command(args.get(2..).unwrap_or_default()),
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
    let Ok(done) = Layout::current().and_then(|layout| install::uninstall(&layout, &mut RegExe))
    else {
        eprintln!("mosaic-native-host: uninstall failed");
        return ExitCode::from(EXIT_FAILURE);
    };
    for file in &done.plan.files {
        println!("removed {}", file.path.display());
    }
    for entry in &done.kept {
        println!(
            "kept HKCU\\{}: its value is not {}",
            entry.key,
            entry.manifest.display()
        );
    }
    ExitCode::SUCCESS
}

const MANIFEST_USAGE: &str =
    "usage: mosaic-native-host manifest --system --os <windows|linux> --out <dir> [--host <path>]";

/// `--system --os <os> --out <dir> [--host <path>]`, in any order.
fn manifest_args(args: &[OsString]) -> Result<(Os, String, std::path::PathBuf), Error> {
    let (mut system, mut os, mut out, mut host) = (false, None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.to_str() {
            Some("--system") => system = true,
            Some("--os") => os = rest.next().and_then(|v| v.to_str()).map(Os::parse),
            Some("--out") => out = rest.next().map(std::path::PathBuf::from),
            Some("--host") => host = rest.next().and_then(|v| v.to_str()).map(str::to_owned),
            _ => return Err(Error::Install),
        }
    }
    let os = os.ok_or(Error::Install)??;
    let host = match host {
        Some(host) => host,
        None => install::system_host(os).ok_or(Error::Install)?.to_owned(),
    };
    if !system {
        return Err(Error::Install);
    }
    Ok((os, host, out.ok_or(Error::Install)?))
}

fn manifest_command(args: &[OsString]) -> ExitCode {
    let Ok((os, host, out)) = manifest_args(args) else {
        eprintln!("{MANIFEST_USAGE}");
        return ExitCode::from(EXIT_FAILURE);
    };
    let Ok(files) = install::write_system(os, &host, &out) else {
        eprintln!("{MANIFEST_USAGE}");
        return ExitCode::from(EXIT_FAILURE);
    };
    for file in files {
        println!("wrote {}", file.display());
    }
    ExitCode::SUCCESS
}

fn check_command(args: &[OsString]) -> ExitCode {
    let Ok(host) = std::env::current_exe() else {
        eprintln!("mosaic-native-host: cannot find its own path");
        return ExitCode::from(EXIT_FAILURE);
    };
    if !args.is_empty() {
        eprintln!("usage: mosaic-native-host check");
        return ExitCode::from(EXIT_FAILURE);
    }
    let findings = install::audit_manifests(&host);
    if findings.is_empty() {
        println!("ok: no per-user entry points to another host");
        return ExitCode::SUCCESS;
    }
    for finding in &findings {
        println!(
            "{} ({}): points to {}, not {}",
            finding.location,
            finding.browser.name(),
            finding.path.display(),
            host.display()
        );
    }
    println!("run `mosaic-native-host install` to point them at this host");
    ExitCode::from(EXIT_FAILURE)
}

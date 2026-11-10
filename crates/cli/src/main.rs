//! `mosaic`: the Mosaic command-line client (M8, `docs/CLI.md`). Pre-audit.
//!
//! - No secret ever comes from argv or the environment (`spec.rs`, `input.rs`).
//! - Only `output.rs` prints. Only `testing.rs` (feature `test-util`, debug builds) fakes the OS.
#![forbid(unsafe_code)]

mod app;
mod backends;
mod clipboard;
mod cmd;
mod error;
mod i18n;
mod input;
mod model;
mod output;
mod refs;
mod spec;
#[cfg(feature = "test-util")]
mod testing;
mod util;

#[cfg(all(feature = "test-util", not(debug_assertions)))]
compile_error!(
    "the `test-util` feature replaces the OS keystore and must never be in a release build"
);

use std::ffi::OsString;
use std::process::ExitCode;

use app::App;
use error::{CliError, EXIT_OTHER, Result};
use i18n::{Lang, Msg};
use output::Output;
use spec::{Parsed, Request};

fn env_text(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Output settings for errors raised before the command line is parsed.
fn early_output(args: &[OsString]) -> Output {
    let text: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let flag = text
        .iter()
        .enumerate()
        .find_map(|(i, a)| match a.strip_prefix("--lang=") {
            Some(v) => Some(v.to_owned()),
            None if a == "--lang" => text.get(i + 1).cloned(),
            None => None,
        });
    Output {
        json: text.iter().any(|a| a == "--json"),
        quiet: false,
        lang: Lang::detect(flag.as_deref(), &env_text),
    }
}

fn help(out: &Output, p: &Parsed) -> Result<()> {
    if p.has("man") {
        out.write_raw(spec::man_page().as_bytes());
        return Ok(());
    }
    let command = match p.args.first() {
        Some(name) => Some(
            spec::find(name)
                .filter(|c| !c.hidden)
                .ok_or_else(|| CliError::usage(Msg::ErrUnknownCommand).with("name", "#2"))?,
        ),
        None => None,
    };
    out.write_raw(spec::help_text(command).as_bytes());
    Ok(())
}

fn run(parsed: &Parsed) -> Result<u8> {
    let out = Output {
        json: parsed.has("json"),
        quiet: parsed.has("quiet"),
        lang: Lang::detect(parsed.value("lang"), &env_text),
    };
    for name in ["MOSAIC_PASSWORD", "MOSAIC_SECRET_KEY"] {
        if std::env::var_os(name).is_some() {
            out.warn(
                "envSecretIgnored",
                Msg::ErrEnvSecretIgnored,
                &[("name", name.to_owned())],
            );
        }
    }
    match parsed.cmd.name {
        "help" => help(&out, parsed)?,
        "completions" => {
            let shell = parsed.args.first().map_or("", String::as_str);
            out.write_raw(spec::completion(shell).as_bytes());
        }
        "__clipboard-hold" => clipboard::hold(parsed)?,
        _ => {
            input::install_interrupt_handler();
            let mut app = App::new(parsed, out)?;
            app.sweep();
            return cmd::dispatch(&mut app, parsed);
        }
    }
    Ok(0)
}

fn real_main() -> u8 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let out = early_output(&args);
    match spec::parse(args) {
        Err(e) => {
            out.error(&e);
            e.exit
        }
        Ok(Request::Version) => {
            let version = env!("CARGO_PKG_VERSION");
            out.data(&serde_json::json!({ "version": version }), |o| {
                o.answer(&format!("mosaic {version}"));
            });
            0
        }
        Ok(Request::Help(command)) => {
            out.write_raw(spec::help_text(command).as_bytes());
            0
        }
        Ok(Request::Run(parsed)) => {
            let shown = Output {
                json: parsed.has("json"),
                quiet: parsed.has("quiet"),
                lang: Lang::detect(parsed.value("lang"), &env_text),
            };
            match run(&parsed) {
                Ok(code) => code,
                Err(e) => {
                    shown.error(&e);
                    if e.exit == 0 { EXIT_OTHER } else { e.exit }
                }
            }
        }
    }
}

fn main() -> ExitCode {
    ExitCode::from(real_main())
}

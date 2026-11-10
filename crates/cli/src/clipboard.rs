//! `--copy` and the clipboard helper (spec §3.2). The value goes to the helper process through its
//! stdin pipe (never argv or the environment). The helper sets the clipboard with the
//! history-exclusion hints, waits, and clears it only if it still holds our value (compared in
//! constant time by `mosaic-platform`).

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::Duration;

use zeroize::Zeroizing;

use crate::backends;
use crate::error::{CliError, Result};
use crate::i18n::Msg;
use crate::output::Output;
use crate::spec::Parsed;
use crate::util::parse_duration;

/// The default time before the clipboard is cleared.
pub const DEFAULT_CLEAR: Duration = Duration::from_secs(30);

#[cfg(feature = "test-util")]
const MIN_CLEAR_SECS: u64 = 1;
#[cfg(not(feature = "test-util"))]
const MIN_CLEAR_SECS: u64 = 5;
const MAX_CLEAR_SECS: u64 = 300;
/// The longest value the helper holds.
const MAX_VALUE: usize = 64 * 1024;

/// The `--clear-after` of a command (5 to 300 seconds, default 30).
///
/// # Errors
/// A usage error for an unreadable or out-of-range value.
pub fn clear_after(parsed: &Parsed) -> Result<Duration> {
    let Some(text) = parsed.value("clear-after") else {
        return Ok(DEFAULT_CLEAR);
    };
    parse_duration(text, 1)
        .filter(|d| (MIN_CLEAR_SECS..=MAX_CLEAR_SECS).contains(&d.as_secs()))
        .ok_or_else(|| CliError::usage(Msg::ErrInvalidValue).with("name", "--clear-after"))
}

/// Puts `value` on the clipboard through a detached helper that clears it after `clear_after`.
///
/// # Errors
/// `internal` when the helper cannot be started.
pub fn copy(out: &Output, value: &str, clear_after: Duration) -> Result<()> {
    let exe = std::env::current_exe().map_err(|_| CliError::internal())?;
    let mut command = Command::new(exe);
    command
        .arg("__clipboard-hold")
        .arg("--clear-after")
        .arg(clear_after.as_secs().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        // Its own process group: a Ctrl-C in the caller's terminal does not reach the helper.
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // DETACHED_PROCESS | CREATE_NO_WINDOW: the helper outlives the console. (It still inherits
        // handles the parent got from its caller: a caller that waits for end-of-file on a pipe
        // of `mosaic ... --copy` waits for the helper too; see docs/CLI.md.)
        command.creation_flags(0x0000_0008 | 0x0800_0000);
    }
    let mut child = command.spawn().map_err(|_| CliError::internal())?;
    let mut stdin = child.stdin.take().ok_or_else(CliError::internal)?;
    stdin
        .write_all(value.as_bytes())
        .map_err(|_| CliError::internal())?;
    drop(stdin);
    out.note(
        Msg::OutCopied,
        &[("seconds", clear_after.as_secs().to_string())],
    );
    Ok(())
}

/// The helper: reads the value from stdin, sets the clipboard, waits, then clears it if unchanged.
///
/// # Errors
/// `internal` when there is no clipboard or no value.
pub fn hold(parsed: &Parsed) -> Result<()> {
    let wait = clear_after(parsed)?;
    #[cfg(feature = "test-util")]
    if let Some(dir) = std::env::var_os("MOSAIC_TEST_DIR") {
        // Lets a test check that the value is not on the helper's command line.
        let argv: Vec<String> = std::env::args().collect();
        let _ = std::fs::write(
            std::path::Path::new(&dir).join("helper-argv.txt"),
            argv.join("\n"),
        );
    }
    // Reserved once and read through `take`, so the value is never copied by a reallocation.
    let mut value = Zeroizing::new(String::with_capacity(MAX_VALUE + 1));
    let read = std::io::Read::read_to_string(
        &mut std::io::Read::take(std::io::stdin().lock(), (MAX_VALUE + 1) as u64),
        &mut value,
    );
    if read.is_err() || value.is_empty() || value.len() > MAX_VALUE {
        return Err(CliError::internal());
    }
    let mut clipboard = backends::clipboard().map_err(|_| CliError::internal())?;
    clipboard.set(&value).map_err(|_| CliError::internal())?;
    std::thread::sleep(wait);
    clipboard
        .clear_if_matches(&value)
        .map(|_| ())
        .map_err(|_| CliError::internal())
}

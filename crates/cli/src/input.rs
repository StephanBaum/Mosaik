//! Secret input (spec §3.2). Secrets come from a no-echo prompt on the controlling terminal, one
//! line of stdin (`--password-stdin`) or a file descriptor (`--password-fd`, Unix): never from argv
//! or the environment. Every buffer is a [`Zeroizing`] string; the terminal driver's and `stdin`'s
//! own buffers are not wiped (THREAT_MODEL R109).

use std::io::{BufRead, IsTerminal};
use std::sync::mpsc;
use std::time::Duration;

use zeroize::Zeroizing;

use crate::error::{CliError, EXIT_AUTH, Result};
use crate::i18n::{Lang, Msg, text};
use crate::output;

/// How long a prompt waits.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

/// A terminal, so tests can script one.
pub trait Terminal {
    /// Reads a line without echo.
    ///
    /// # Errors
    /// `inputRequired` when there is no terminal, `promptTimeout` after 120 s.
    fn secret(&mut self, prompt: &str) -> Result<Zeroizing<String>>;

    /// Reads a line with echo (confirmations, the typed word).
    ///
    /// # Errors
    /// As [`Terminal::secret`].
    fn line(&mut self, prompt: &str) -> Result<String>;
}

fn timeout() -> CliError {
    CliError::new("promptTimeout", EXIT_AUTH, Msg::ErrPromptTimeout)
        .with("seconds", PROMPT_TIMEOUT.as_secs().to_string())
}

/// Runs a blocking read on its own thread so it can time out.
fn with_timeout<T: Send + 'static>(
    read: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> Result<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read());
    });
    match rx.recv_timeout(PROMPT_TIMEOUT) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => Err(CliError::input_required()),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(timeout()),
    }
}

/// The terminal mode (echo) saved before a no-echo prompt, so a timeout, a Ctrl-C or an early
/// return can put it back. Unix only: the Windows console resets its input mode for the shell.
#[cfg(unix)]
mod tty {
    use std::sync::Mutex;

    use rustix::fs::{Mode, OFlags, open};
    use rustix::termios::{OptionalActions, Termios, tcgetattr, tcsetattr};

    static SAVED: Mutex<Option<Termios>> = Mutex::new(None);

    pub fn save() {
        let Ok(fd) = open("/dev/tty", OFlags::RDWR | OFlags::NOCTTY, Mode::empty()) else {
            return;
        };
        if let (Ok(mode), Ok(mut slot)) = (tcgetattr(&fd), SAVED.lock()) {
            *slot = Some(mode);
        }
    }

    pub fn restore() {
        let Ok(slot) = SAVED.lock() else { return };
        let Some(mode) = slot.as_ref() else { return };
        if let Ok(fd) = open("/dev/tty", OFlags::RDWR | OFlags::NOCTTY, Mode::empty()) {
            let _ = tcsetattr(&fd, OptionalActions::Now, mode);
        }
    }

    pub fn forget() {
        if let Ok(mut slot) = SAVED.lock() {
            *slot = None;
        }
    }
}

/// Restores the terminal mode when dropped (also on a timeout or an error return).
struct ModeGuard;

impl ModeGuard {
    fn new() -> Self {
        #[cfg(unix)]
        tty::save();
        Self
    }
}

impl Drop for ModeGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            tty::restore();
            tty::forget();
        }
    }
}

/// Ctrl-C: put the terminal mode back, then exit 130.
pub fn install_interrupt_handler() {
    let _ = ctrlc::set_handler(|| {
        #[cfg(unix)]
        tty::restore();
        std::process::exit(130);
    });
}

/// The controlling terminal (`/dev/tty`, `CONIN$`) through `rpassword`.
pub struct RealTerminal;

impl Terminal for RealTerminal {
    fn secret(&mut self, prompt: &str) -> Result<Zeroizing<String>> {
        output::prompt(prompt);
        let guard = ModeGuard::new();
        let read = with_timeout(rpassword::read_password);
        drop(guard);
        output::end_prompt_line();
        read.map(Zeroizing::new)
    }

    fn line(&mut self, prompt: &str) -> Result<String> {
        if !std::io::stdin().is_terminal() {
            return Err(CliError::input_required());
        }
        output::prompt(prompt);
        with_timeout(|| {
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            Ok(line)
        })
    }
}

/// Asks through a [`Terminal`], honoring `--no-input`.
pub struct Prompter {
    term: Box<dyn Terminal>,
    no_input: bool,
    lang: Lang,
}

impl Prompter {
    /// A prompter over `term`.
    #[must_use]
    pub fn new(term: Box<dyn Terminal>, no_input: bool, lang: Lang) -> Self {
        Self {
            term,
            no_input,
            lang,
        }
    }

    /// A secret line.
    ///
    /// # Errors
    /// `inputRequired` (exit 3) under `--no-input`, or as [`Terminal::secret`].
    pub fn secret(&mut self, msg: Msg, args: &[(&str, String)]) -> Result<Zeroizing<String>> {
        if self.no_input {
            return Err(CliError::input_required());
        }
        let line = self.term.secret(&text(self.lang, msg, args))?;
        Ok(trimmed(line))
    }

    /// A visible line, trimmed.
    ///
    /// # Errors
    /// As [`Prompter::secret`].
    pub fn line(&mut self, msg: Msg, args: &[(&str, String)]) -> Result<String> {
        if self.no_input {
            return Err(CliError::input_required());
        }
        Ok(self
            .term
            .line(&text(self.lang, msg, args))?
            .trim()
            .to_owned())
    }
}

/// Removes a trailing line break in place (no copy is made).
fn trimmed(mut line: Zeroizing<String>) -> Zeroizing<String> {
    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }
    line
}

/// One line from `reader` (stdin or a file descriptor), without its line break.
///
/// # Errors
/// `inputRequired` when the source is empty or unreadable.
pub fn read_secret_line(reader: &mut dyn BufRead) -> Result<Zeroizing<String>> {
    let mut line = Zeroizing::new(String::new());
    match reader.read_line(&mut line) {
        Ok(n) if n > 0 => Ok(trimmed(line)),
        _ => Err(CliError::input_required()),
    }
}

/// One line from file descriptor `fd` (Unix: opened through `/dev/fd`, which keeps the code free
/// of `unsafe`).
///
/// # Errors
/// `fdUnsupported` outside Unix, `inputRequired` when the descriptor is unreadable.
pub fn read_secret_fd(fd: i32) -> Result<Zeroizing<String>> {
    // `cfg!` rather than `#[cfg]`: both arms compile everywhere, so no platform sees the message
    // as unused.
    if !cfg!(unix) {
        return Err(CliError::usage(Msg::ErrFdUnsupported));
    }
    let file =
        std::fs::File::open(format!("/dev/fd/{fd}")).map_err(|_| CliError::input_required())?;
    read_secret_line(&mut std::io::BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_loses_its_line_break_only() {
        let mut src: &[u8] = b" pass word \r\nnext";
        let line = read_secret_line(&mut src).unwrap_or_default();
        assert_eq!(line.as_str(), " pass word ");
    }

    #[test]
    fn an_empty_source_is_input_required() {
        let mut src: &[u8] = b"";
        assert!(read_secret_line(&mut src).is_err());
    }
}

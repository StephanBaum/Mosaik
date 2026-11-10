//! The only module that prints (spec §3.2). Text goes to stdout, warnings, errors and prompts to
//! stderr. With `--json` stdout carries exactly one `{"v":1,"data":...}` document and errors are
//! `{"v":1,"error":{"code","message"}}` on stderr (ADR-0075). Secrets go through
//! [`Output::write_secret`] and the serializers, which write straight to the locked stdout.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the output layer is the one place a CLI talks to the terminal"
)]

use std::fmt::Write as _;
use std::io::{self, IsTerminal as _, Write};

use zeroize::Zeroizing;

use serde::Serialize;

use crate::error::CliError;
use crate::i18n::{Lang, Msg, text};

/// How results and messages are shown.
#[derive(Clone, Debug)]
pub struct Output {
    /// `--json`.
    pub json: bool,
    /// `--quiet`.
    pub quiet: bool,
    /// The message language.
    pub lang: Lang,
}

#[derive(Serialize)]
struct Data<'a, T: Serialize + ?Sized> {
    v: u8,
    data: &'a T,
}

#[derive(Serialize)]
struct Failure<'a> {
    v: u8,
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(flatten)]
    extra: Option<&'a serde_json::Value>,
}

#[derive(Serialize)]
struct WarningDoc<'a> {
    v: u8,
    warning: WarningBody<'a>,
}

#[derive(Serialize)]
struct WarningBody<'a> {
    code: &'a str,
    message: &'a str,
}

impl Output {
    /// The localized text of a message.
    #[must_use]
    pub fn t(&self, msg: Msg, args: &[(&str, String)]) -> String {
        text(self.lang, msg, args)
    }

    /// A line of text on stdout (text mode only; `--quiet` hides it).
    pub fn say(&self, line: &str) {
        if !self.json && !self.quiet {
            println!("{line}");
        }
    }

    /// A line of text that is the answer itself: shown even with `--quiet`.
    pub fn answer(&self, line: &str) {
        if self.json {
            return;
        }
        match escape_for_terminal(line).filter(|_| io::stdout().is_terminal()) {
            Some(safe) => println!("{}", safe.as_str()),
            None => println!("{line}"),
        }
    }

    /// Prints a result: the JSON document in JSON mode, otherwise `render` produces the text.
    pub fn data<T: Serialize + ?Sized>(&self, value: &T, render: impl FnOnce(&Self)) {
        if self.json {
            let mut out = io::stdout().lock();
            let _ = serde_json::to_writer(&mut out, &Data { v: 1, data: value });
            let _ = out.write_all(b"\n");
            let _ = out.flush();
        } else {
            render(self);
        }
    }

    /// A warning on stderr (a JSON line in JSON mode).
    pub fn warn(&self, code: &str, msg: Msg, args: &[(&str, String)]) {
        let message = self.t(msg, args);
        if self.json {
            let doc = WarningDoc {
                v: 1,
                warning: WarningBody {
                    code,
                    message: &message,
                },
            };
            if let Ok(line) = serde_json::to_string(&doc) {
                eprintln!("{line}");
            }
        } else {
            eprintln!("mosaic: {message}");
        }
    }

    /// An informational line on stderr (text mode and `--quiet` off).
    pub fn note(&self, msg: Msg, args: &[(&str, String)]) {
        if !self.json && !self.quiet {
            eprintln!("{}", self.t(msg, args));
        }
    }

    /// Reports an error on stderr.
    pub fn error(&self, err: &CliError) {
        let args: Vec<(&str, String)> = err.args.iter().map(|(k, v)| (*k, v.clone())).collect();
        let message = self.t(err.msg, &args);
        if self.json {
            let doc = Failure {
                v: 1,
                error: ErrorBody {
                    code: err.code,
                    message: &message,
                    extra: err.extra.as_ref(),
                },
            };
            if let Ok(line) = serde_json::to_string(&doc) {
                eprintln!("{line}");
            }
        } else {
            eprintln!("mosaic: {message}");
            if let Some(list) = err
                .extra
                .as_ref()
                .and_then(|v| v.get("candidates"))
                .and_then(serde_json::Value::as_array)
            {
                for c in list {
                    let field =
                        |k: &str| c.get(k).and_then(serde_json::Value::as_str).unwrap_or("");
                    eprintln!(
                        "  {}",
                        self.t(
                            Msg::OutAmbiguousCandidate,
                            &[
                                ("vault", escape(field("vault"))),
                                ("title", escape(field("title"))),
                                ("id", field("id").to_owned()),
                            ],
                        )
                    );
                }
            }
        }
    }

    /// Writes a secret value and a newline straight to stdout, with no intermediate copy. In
    /// JSON mode `json_key` names the member of `{"v":1,"data":{key:value}}`.
    pub fn write_secret(&self, secret: &str, json_key: &str) {
        let mut out = io::stdout().lock();
        if self.json {
            let _ = out.write_all(b"{\"v\":1,\"data\":{");
            let _ = serde_json::to_writer(&mut out, json_key);
            let _ = out.write_all(b":");
            let _ = serde_json::to_writer(&mut out, secret);
            let _ = out.write_all(b"}}\n");
        } else if let Some(safe) =
            escape_for_terminal(secret).filter(|_| io::stdout().is_terminal())
        {
            // A terminal would act on control sequences in a value: show them escaped.
            let _ = out.write_all(safe.as_bytes());
            let _ = out.write_all(b"\n");
        } else {
            let _ = out.write_all(secret.as_bytes());
            let _ = out.write_all(b"\n");
        }
        let _ = out.flush();
    }

    /// Writes raw bytes to stdout (the result of `inject`).
    #[expect(
        clippy::unused_self,
        reason = "kept a method so every write goes through the output layer"
    )]
    pub fn write_raw(&self, bytes: &[u8]) {
        let mut out = io::stdout().lock();
        let _ = out.write_all(bytes);
        let _ = out.flush();
    }
}

/// A prompt on stderr, without a newline.
pub fn prompt(text: &str) {
    eprint!("{text}");
    let _ = io::stderr().flush();
}

/// Ends the line after a no-echo prompt.
pub fn end_prompt_line() {
    eprintln!();
}

/// Shows control and bidirectional characters of untrusted text (titles) as `\u{...}` escapes.
#[must_use]
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control() || mosaic_core::is_spoofing_char(c) {
            let _ = write!(out, "\\u{{{:04x}}}", u32::from(c));
        } else {
            out.push(c);
        }
    }
    out
}

/// For single-value output to a terminal: C0 and C1 control characters (except line feed and tab)
/// and bidirectional controls become `\u{...}` escapes, so a stored value cannot move the cursor,
/// retitle the window or reorder the text. `None` when nothing needs escaping. The copy is
/// zeroized (the value may be a secret). Output to a pipe or file stays raw.
#[must_use]
pub fn escape_for_terminal(s: &str) -> Option<Zeroizing<String>> {
    let bad =
        |c: char| (c.is_control() && c != '\n' && c != '\t') || mosaic_core::is_spoofing_char(c);
    if !s.chars().any(bad) {
        return None;
    }
    let mut out = Zeroizing::new(String::with_capacity(s.len() + 16));
    for c in s.chars() {
        if bad(c) {
            let _ = write!(out, "\\u{{{:04x}}}", u32::from(c));
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn terminal_escaping_keeps_line_feed_and_tab() {
        assert!(super::escape_for_terminal("plain\ttext\nmore").is_none());
        let safe = super::escape_for_terminal("a\u{1b}[2Jb\u{9b}c\u{202e}d\r").unwrap();
        assert_eq!(safe.as_str(), "a\\u{001b}[2Jb\\u{009b}c\\u{202e}d\\u{000d}");
    }

    #[test]
    fn bidi_and_control_characters_are_escaped() {
        assert_eq!(escape("a\u{202e}b"), "a\\u{202e}b");
        assert_eq!(escape("x\ny"), "x\\u{000a}y");
        assert_eq!(escape("plain äöü"), "plain äöü");
    }
}

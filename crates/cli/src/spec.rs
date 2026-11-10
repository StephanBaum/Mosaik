//! The single command table (spec §3.1). It drives the parser, `help`, the man page and the shell
//! completions, so they cannot drift apart. Help text and the man page are English only (OD-M8-3).

use std::ffi::OsString;
use std::fmt::Write as _;

use crate::error::{CliError, Result};
use crate::i18n::Msg;

/// An option of a command (or a global one).
pub struct Flag {
    /// The long name without `--`.
    pub name: &'static str,
    /// The metavariable of the value, if the option takes one.
    pub value: Option<&'static str>,
    /// Whether it may repeat.
    pub repeat: bool,
    /// One help line.
    pub help: &'static str,
}

/// A positional argument.
pub struct Arg {
    /// The name shown in help.
    pub name: &'static str,
    /// Whether it must be given.
    pub required: bool,
    /// The allowed values, if it is a fixed set.
    pub choices: &'static [&'static str],
}

/// A command.
pub struct Command {
    /// The name typed on the command line.
    pub name: &'static str,
    /// One help line.
    pub summary: &'static str,
    /// The positional arguments.
    pub args: &'static [Arg],
    /// The command's own options.
    pub flags: &'static [Flag],
    /// Whether everything after the first positional belongs to a child command (`run`).
    pub trailing: bool,
    /// Left out of help, the man page and the completions.
    pub hidden: bool,
}

const fn flag(name: &'static str, help: &'static str) -> Flag {
    Flag {
        name,
        value: None,
        repeat: false,
        help,
    }
}

const fn opt(name: &'static str, value: &'static str, help: &'static str) -> Flag {
    Flag {
        name,
        value: Some(value),
        repeat: false,
        help,
    }
}

const fn multi(name: &'static str, value: &'static str, help: &'static str) -> Flag {
    Flag {
        name,
        value: Some(value),
        repeat: true,
        help,
    }
}

const fn arg(name: &'static str, required: bool) -> Arg {
    Arg {
        name,
        required,
        choices: &[],
    }
}

const fn cmd(
    name: &'static str,
    summary: &'static str,
    args: &'static [Arg],
    flags: &'static [Flag],
) -> Command {
    Command {
        name,
        summary,
        args,
        flags,
        trailing: false,
        hidden: false,
    }
}

/// Options every command accepts.
pub const GLOBAL: &[Flag] = &[
    opt(
        "store",
        "PATH",
        "Use the vault store at PATH (also MOSAIC_STORE).",
    ),
    opt(
        "profile",
        "shared|own",
        "shared: the desktop app's store (default). own: the CLI's own device.",
    ),
    flag(
        "json",
        "Print one JSON document ({\"v\":1,...}) instead of text.",
    ),
    opt(
        "lang",
        "en|de",
        "Message language (also MOSAIC_LANG, LANG).",
    ),
    flag("no-input", "Never prompt; a needed prompt is exit 3."),
    flag("quiet", "Print only the requested value."),
];

/// Option names that would put a secret on the command line. They are refused with a usage error.
const SECRET_FLAGS: &[&str] = &[
    "password",
    "master-password",
    "secret-key",
    "passphrase",
    "file-password",
    "export-password",
    "token",
];

const UNLOCK_FLAGS: &[Flag] = &[
    opt("ttl", "5m", "Session length: 1 to 30 minutes (default 5m)."),
    flag(
        "password-stdin",
        "Read the master password from one line of stdin.",
    ),
    opt(
        "password-fd",
        "N",
        "Read the master password from file descriptor N (Unix).",
    ),
    flag(
        "secret-key-prompt",
        "Ask for the Secret Key instead of using the keystore.",
    ),
];
const LIST_FLAGS: &[Flag] = &[
    opt("vault", "V", "Only this vault (name or ID)."),
    opt("type", "T", "Only this item type (login, note, card, ...)."),
    flag("favorites", "Only favorites."),
    flag("trash", "Only items in the trash."),
];
const SEARCH_FLAGS: &[Flag] = &[opt("vault", "V", "Only this vault (name or ID).")];
const GET_FLAGS: &[Flag] = &[
    multi("field", "F", "Print this field only (repeatable)."),
    flag(
        "reveal",
        "Print secret fields (they are concealed by default).",
    ),
    flag("copy", "Copy the requested field to the clipboard."),
    opt(
        "clear-after",
        "30s",
        "Clear the clipboard after this long (5s to 300s).",
    ),
];
const TOTP_FLAGS: &[Flag] = &[
    flag("copy", "Copy the code to the clipboard."),
    opt(
        "clear-after",
        "30s",
        "Clear the clipboard after this long (5s to 300s).",
    ),
];
const GENERATE_FLAGS: &[Flag] = &[
    opt("length", "N", "Password length."),
    opt("words", "N", "Make a passphrase of N words."),
    opt("separator", "S", "Separator between passphrase words."),
    flag("no-upper", "No uppercase letters."),
    flag("no-lower", "No lowercase letters."),
    flag("no-digits", "No digits."),
    flag("no-symbols", "No symbols."),
    opt(
        "site",
        "DOMAIN",
        "Follow the known password rules of DOMAIN.",
    ),
    flag("copy", "Copy the result to the clipboard."),
    opt(
        "clear-after",
        "30s",
        "Clear the clipboard after this long (5s to 300s).",
    ),
];
const RUN_FLAGS: &[Flag] = &[
    multi("env", "NAME=REF", "Set NAME from a reference (repeatable)."),
    opt("env-file", "FILE", "Read NAME=REF lines from FILE."),
];
const INJECT_FLAGS: &[Flag] = &[
    opt("in", "FILE", "Template to read (default: stdin)."),
    opt(
        "out",
        "FILE",
        "Write the result to FILE (mode 0600, never overwrites).",
    ),
    flag("reveal", "Allow secrets on a terminal."),
];
const IMPORT_FLAGS: &[Flag] = &[
    opt("vault", "V", "Import into this vault (name or ID)."),
    flag("dry-run", "Show the preview and import nothing."),
    flag(
        "file-password-prompt",
        "Ask for the file's password (KeePass).",
    ),
];
const EXPORT_FLAGS: &[Flag] = &[
    multi("vault", "V", "Export only this vault (repeatable)."),
    flag("include-attachments", "Include attachment contents."),
];
const SYNC_FLAGS: &[Flag] = &[
    flag("status", "Show the sync state and findings; do not sync."),
    flag(
        "acknowledge",
        "Show the findings and clear them after you type the word.",
    ),
];
const SETUP_FLAGS: &[Flag] = &[
    opt("folder", "PATH", "Join over this sync folder."),
    opt("server", "URL", "Join over this server."),
    opt("email", "E", "The sync account's email."),
];
const HELP_FLAGS: &[Flag] = &[flag("man", "Print the manual page (roff).")];
const HOLD_FLAGS: &[Flag] = &[opt("clear-after", "30", "Seconds to hold the clipboard.")];

/// Every command, in help order.
pub const COMMANDS: &[Command] = &[
    cmd(
        "unlock",
        "Unlock with the master password and keep a short session.",
        &[],
        UNLOCK_FLAGS,
    ),
    cmd("lock", "End the session.", &[], &[]),
    cmd(
        "status",
        "Show the store, the session and the sync state.",
        &[],
        &[],
    ),
    cmd("list", "List items.", &[], LIST_FLAGS),
    cmd(
        "search",
        "Search items.",
        &[arg("QUERY", true)],
        SEARCH_FLAGS,
    ),
    cmd(
        "get",
        "Show an item or one field of it.",
        &[arg("REF", true)],
        GET_FLAGS,
    ),
    cmd(
        "totp",
        "Print the current one-time code.",
        &[arg("REF", true)],
        TOTP_FLAGS,
    ),
    cmd(
        "generate",
        "Generate a password or passphrase.",
        &[],
        GENERATE_FLAGS,
    ),
    Command {
        name: "run",
        summary: "Run a command with secrets in its environment.",
        args: &[],
        flags: RUN_FLAGS,
        trailing: true,
        hidden: false,
    },
    cmd(
        "inject",
        "Fill {{ mosaic://... }} placeholders of a template.",
        &[],
        INJECT_FLAGS,
    ),
    cmd(
        "ssh",
        "List SSH keys, print a public key, or explain the agent.",
        &[
            Arg {
                name: "ACTION",
                required: true,
                choices: &["list", "public-key", "agent-status"],
            },
            arg("REF", false),
        ],
        &[],
    ),
    cmd(
        "import",
        "Import a file from another password manager or a Mosaic export.",
        &[
            Arg {
                name: "FORMAT",
                required: true,
                choices: &[
                    "enpass",
                    "bitwarden",
                    "keepass",
                    "1password",
                    "cxf",
                    "csv",
                    "mosaicx",
                ],
            },
            arg("FILE", true),
        ],
        IMPORT_FLAGS,
    ),
    cmd(
        "export",
        "Write an encrypted export file.",
        &[arg("FILE", true)],
        EXPORT_FLAGS,
    ),
    cmd(
        "sync",
        "Sync once with the store's backend.",
        &[],
        SYNC_FLAGS,
    ),
    cmd(
        "setup",
        "Give the CLI its own device store.",
        &[Arg {
            name: "WHAT",
            required: true,
            choices: &["own-store"],
        }],
        SETUP_FLAGS,
    ),
    cmd(
        "recover",
        "Recover after a password or key change made elsewhere.",
        &[],
        &[],
    ),
    cmd(
        "completions",
        "Print a shell completion script.",
        &[Arg {
            name: "SHELL",
            required: true,
            choices: &["bash", "zsh", "fish", "powershell"],
        }],
        &[],
    ),
    cmd(
        "help",
        "Show help or the manual page.",
        &[arg("COMMAND", false)],
        HELP_FLAGS,
    ),
    Command {
        name: "__clipboard-hold",
        summary: "Internal: hold a value on the clipboard, then clear it if unchanged.",
        args: &[],
        flags: HOLD_FLAGS,
        trailing: false,
        hidden: true,
    },
];

/// What the command line asked for.
pub enum Request {
    /// Run a command.
    Run(Parsed),
    /// Print help (for one command, or the overview).
    Help(Option<&'static Command>),
    /// Print the version.
    Version,
}

/// A parsed command line.
pub struct Parsed {
    /// The command.
    pub cmd: &'static Command,
    opts: Vec<(&'static str, Option<String>)>,
    /// Positional arguments.
    pub args: Vec<String>,
    /// The child command of `run`.
    pub trailing: Vec<OsString>,
}

impl Parsed {
    /// Whether the option was given.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.opts.iter().any(|(n, _)| *n == name)
    }

    /// The last value of an option.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.opts
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| v.as_deref())
    }

    /// Every value of a repeatable option, in order.
    #[must_use]
    pub fn values(&self, name: &str) -> Vec<&str> {
        self.opts
            .iter()
            .filter(|(n, _)| *n == name)
            .filter_map(|(_, v)| v.as_deref())
            .collect()
    }
}

/// Finds a command by name.
#[must_use]
pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

fn usage(msg: Msg, key: &'static str, value: &str) -> CliError {
    CliError::usage(msg).with(key, value)
}

fn lexopt_error(e: &lexopt::Error) -> CliError {
    match e {
        lexopt::Error::MissingValue { option } => usage(
            Msg::ErrMissingValue,
            "name",
            option.as_deref().unwrap_or("option"),
        ),
        lexopt::Error::UnexpectedOption(o) => usage(Msg::ErrUnknownOption, "name", o),
        lexopt::Error::UnexpectedArgument(_) => {
            usage(Msg::ErrUnexpectedArgument, "value", "(argument)")
        }
        lexopt::Error::UnexpectedValue { option, .. } => {
            usage(Msg::ErrInvalidValue, "name", option)
        }
        _ => CliError::usage(Msg::ErrNonUtf8),
    }
}

fn utf8(value: OsString) -> Result<String> {
    value
        .into_string()
        .map_err(|_| CliError::usage(Msg::ErrNonUtf8))
}

/// Parses a command line (without the program name).
///
/// # Errors
/// A usage error (exit 2); a secret option is `secretInArgv`.
pub fn parse(args: Vec<OsString>) -> Result<Request> {
    use lexopt::Arg as A;
    let mut parser = lexopt::Parser::from_args(args);
    let mut command: Option<&'static Command> = None;
    let mut opts: Vec<(&'static str, Option<String>)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut trailing: Vec<OsString> = Vec::new();
    let mut want_help = false;
    let mut want_version = false;
    loop {
        let next = parser.next().map_err(|e| lexopt_error(&e))?;
        let Some(item) = next else { break };
        match item {
            A::Long(name) => {
                if SECRET_FLAGS.contains(&name) {
                    return Err(CliError::secret_in_argv(&format!("--{name}")));
                }
                match name {
                    "help" => {
                        want_help = true;
                        continue;
                    }
                    "version" => {
                        want_version = true;
                        continue;
                    }
                    _ => {}
                }
                let spec = command
                    .and_then(|c| c.flags.iter().find(|f| f.name == name))
                    .or_else(|| GLOBAL.iter().find(|f| f.name == name))
                    .ok_or_else(|| usage(Msg::ErrUnknownOption, "name", &format!("--{name}")))?;
                if spec.value.is_some() {
                    let value = utf8(parser.value().map_err(|e| lexopt_error(&e))?)?;
                    if !spec.repeat && opts.iter().any(|(n, _)| *n == spec.name) {
                        opts.retain(|(n, _)| *n != spec.name);
                    }
                    opts.push((spec.name, Some(value)));
                } else {
                    opts.push((spec.name, None));
                }
            }
            A::Short(c) => match c {
                'h' => want_help = true,
                'V' => want_version = true,
                other => return Err(usage(Msg::ErrUnknownOption, "name", &format!("-{other}"))),
            },
            A::Value(value) => match command {
                None => {
                    let name = utf8(value)?;
                    command = Some(
                        find(&name)
                            .filter(|c| !c.hidden || name == "__clipboard-hold")
                            .ok_or_else(|| usage(Msg::ErrUnknownCommand, "name", "#1"))?,
                    );
                }
                Some(c) if c.trailing => {
                    trailing.push(value);
                    trailing.extend(parser.raw_args().map_err(|e| lexopt_error(&e))?);
                    break;
                }
                Some(_) => positional.push(utf8(value)?),
            },
        }
    }
    if want_version {
        return Ok(Request::Version);
    }
    if want_help {
        return Ok(Request::Help(command));
    }
    let Some(command) = command else {
        return Ok(Request::Help(None));
    };
    check_args(command, &positional)?;
    if command.trailing && trailing.is_empty() {
        return Err(usage(Msg::ErrMissingArgument, "name", "COMMAND"));
    }
    Ok(Request::Run(Parsed {
        cmd: command,
        opts,
        args: positional,
        trailing,
    }))
}

fn check_args(command: &Command, given: &[String]) -> Result<()> {
    if given.len() > command.args.len() {
        // Name the position, never the value (it could be a secret typed in the wrong place).
        let position = command.args.len() + 2;
        return Err(usage(
            Msg::ErrUnexpectedArgument,
            "value",
            &format!("#{position}"),
        ));
    }
    for (i, spec) in command.args.iter().enumerate() {
        match given.get(i) {
            None if spec.required => {
                return Err(usage(Msg::ErrMissingArgument, "name", spec.name));
            }
            Some(value) if !spec.choices.is_empty() && !spec.choices.contains(&value.as_str()) => {
                return Err(usage(Msg::ErrInvalidValue, "name", spec.name));
            }
            _ => {}
        }
    }
    Ok(())
}

fn visible() -> impl Iterator<Item = &'static Command> {
    COMMANDS.iter().filter(|c| !c.hidden)
}

fn usage_line(c: &Command) -> String {
    let mut line = format!("mosaic {}", c.name);
    for a in c.args {
        if a.choices.is_empty() {
            let _ = write!(
                line,
                " {}",
                if a.required {
                    a.name.to_owned()
                } else {
                    format!("[{}]", a.name)
                }
            );
        } else {
            let _ = write!(line, " {}", a.choices.join("|"));
        }
    }
    if c.trailing {
        line.push_str(" -- COMMAND [ARGS...]");
    }
    line
}

fn flag_line(f: &Flag) -> String {
    match f.value {
        Some(v) => format!("--{} {v}", f.name),
        None => format!("--{}", f.name),
    }
}

fn flags_help(flags: &[Flag], out: &mut String) {
    for f in flags {
        let _ = writeln!(out, "  {:<26}{}", flag_line(f), f.help);
    }
}

/// The help text of one command, or the overview.
#[must_use]
pub fn help_text(command: Option<&Command>) -> String {
    let mut out = String::new();
    if let Some(c) = command {
        let _ = writeln!(out, "{}\n\n{}\n", usage_line(c), c.summary);
        if !c.flags.is_empty() {
            out.push_str("Options:\n");
            flags_help(c.flags, &mut out);
            out.push('\n');
        }
    } else {
        out.push_str("mosaic: the Mosaic password manager on the command line (pre-audit).\n\n");
        out.push_str("Usage: mosaic [GLOBAL OPTIONS] <command> [ARGS]\n\nCommands:\n");
        for c in visible() {
            let _ = writeln!(out, "  {:<13}{}", c.name, c.summary);
        }
        out.push('\n');
    }
    out.push_str("Global options:\n");
    flags_help(GLOBAL, &mut out);
    out.push_str("  --help, -h                Show this help.\n  --version, -V             Show the version.\n");
    out
}

fn roff(s: &str) -> String {
    s.replace('\\', "\\\\").replace('-', "\\-")
}

/// The manual page in roff.
#[must_use]
pub fn man_page() -> String {
    let mut out = String::new();
    out.push_str(
        ".TH MOSAIC 1\n.SH NAME\nmosaic \\- the Mosaic password manager on the command line\n",
    );
    out.push_str(".SH SYNOPSIS\n.B mosaic\n[GLOBAL OPTIONS] COMMAND [ARGS]\n");
    out.push_str(".SH DESCRIPTION\nMosaic is a zero\\-knowledge password manager (pre\\-audit). This client opens a vault store directly, never takes a secret from the command line or the environment, and prints a secret only when asked with \\fB\\-\\-reveal\\fR.\n");
    out.push_str(".SH COMMANDS\n");
    for c in visible() {
        let _ = writeln!(out, ".TP\n.B {}\n{}", roff(&usage_line(c)), roff(c.summary));
        for f in c.flags {
            let _ = writeln!(
                out,
                ".RS\n.TP\n.B {}\n{}\n.RE",
                roff(&flag_line(f)),
                roff(f.help)
            );
        }
    }
    out.push_str(".SH GLOBAL OPTIONS\n");
    for f in GLOBAL {
        let _ = writeln!(out, ".TP\n.B {}\n{}", roff(&flag_line(f)), roff(f.help));
    }
    out.push_str(".SH EXIT STATUS\n0 ok; 1 other error; 2 usage; 3 authentication failed or input required; 4 not found; 5 store in use; 6 sync halted or sync error; 7 ambiguous reference; 8 vault part locked; 9 reveal required; 10 rotation in progress; 125 the command of \\fBrun\\fR could not start; 130 interrupted.\n");
    out.push_str(".SH SEE ALSO\ndocs/CLI.md\n");
    out
}

/// A completion script for `shell` (`bash`, `zsh`, `fish` or `powershell`).
#[must_use]
pub fn completion(shell: &str) -> String {
    let names: Vec<&str> = visible().map(|c| c.name).collect();
    let global: Vec<String> = GLOBAL.iter().map(|f| format!("--{}", f.name)).collect();
    let mut out = String::new();
    match shell {
        "bash" => {
            out.push_str("_mosaic() {\n  local cur=\"${COMP_WORDS[COMP_CWORD]}\" cmd=\"${COMP_WORDS[1]}\"\n  if [ \"$COMP_CWORD\" -le 1 ]; then\n");
            let _ = writeln!(
                out,
                "    COMPREPLY=($(compgen -W \"{}\" -- \"$cur\"))\n    return\n  fi\n  case \"$cmd\" in",
                names.join(" ")
            );
            for c in visible() {
                let mut words: Vec<String> =
                    c.flags.iter().map(|f| format!("--{}", f.name)).collect();
                words.extend(global.iter().cloned());
                for a in c.args {
                    words.extend(a.choices.iter().map(|s| (*s).to_owned()));
                }
                let _ = writeln!(
                    out,
                    "    {}) COMPREPLY=($(compgen -W \"{}\" -- \"$cur\")) ;;",
                    c.name,
                    words.join(" ")
                );
            }
            out.push_str("  esac\n}\ncomplete -F _mosaic mosaic\n");
        }
        "zsh" => {
            out.push_str("#compdef mosaic\n_mosaic() {\n  local -a commands\n  commands=(\n");
            for c in visible() {
                let _ = writeln!(out, "    '{}:{}'", c.name, c.summary.replace('\'', ""));
            }
            out.push_str("  )\n  if (( CURRENT == 2 )); then\n    _describe 'command' commands\n  else\n    _files\n  fi\n}\n_mosaic \"$@\"\n");
        }
        "fish" => {
            out.push_str("complete -c mosaic -f\n");
            for c in visible() {
                let _ = writeln!(
                    out,
                    "complete -c mosaic -n __fish_use_subcommand -a {} -d '{}'",
                    c.name,
                    c.summary.replace('\'', "")
                );
                for f in c.flags {
                    let _ = writeln!(
                        out,
                        "complete -c mosaic -n '__fish_seen_subcommand_from {}' -l {}",
                        c.name, f.name
                    );
                }
            }
            for f in GLOBAL {
                let _ = writeln!(out, "complete -c mosaic -l {}", f.name);
            }
        }
        _ => {
            out.push_str("Register-ArgumentCompleter -Native -CommandName mosaic -ScriptBlock {\n  param($wordToComplete, $commandAst, $cursorPosition)\n");
            let quoted: Vec<String> = names.iter().map(|n| format!("'{n}'")).collect();
            let _ = writeln!(out, "  $words = @({})", quoted.join(","));
            let flags: Vec<String> = GLOBAL.iter().map(|f| format!("'--{}'", f.name)).collect();
            let _ = writeln!(out, "  $words += @({})", flags.join(","));
            out.push_str("  $words | Where-Object { $_ -like \"$wordToComplete*\" } | ForEach-Object {\n    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)\n  }\n}\n");
        }
    }
    out
}

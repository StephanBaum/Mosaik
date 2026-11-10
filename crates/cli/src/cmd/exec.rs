//! `generate`, `run` and `inject`.

use std::io::{IsTerminal as _, Read as _, Write as _};
use std::process::Command;

use mosaic_core::SecretString;
use mosaic_generator::{
    GeneratedPassword, PassphraseOptions, PasswordOptions, generate_passphrase, generate_password,
    rules_for_domain,
};
use mosaic_vault::session::UnlockedSession;
use zeroize::Zeroizing;

use crate::app::App;
use crate::clipboard;
use crate::error::{CliError, EXIT_OTHER, EXIT_REVEAL, EXIT_SPAWN, Result};
use crate::i18n::Msg;
use crate::model::{self, Value};
use crate::refs;
use crate::spec::Parsed;
use crate::util::{create_private, finish_private};

fn number(p: &Parsed, name: &'static str) -> Result<Option<u32>> {
    p.value(name)
        .map(|v| {
            v.parse::<u32>().map_err(|_| {
                CliError::usage(Msg::ErrInvalidValue).with("name", format!("--{name}"))
            })
        })
        .transpose()
}

fn bad_options() -> CliError {
    CliError::usage(Msg::ErrInvalidValue).with("name", "generate")
}

fn make(p: &Parsed) -> Result<GeneratedPassword> {
    if let Some(words) = number(p, "words")? {
        let mut options = PassphraseOptions::default();
        if let Some(sep) = p.value("separator") {
            sep.clone_into(&mut options.separator);
        }
        options.words = words;
        return generate_passphrase(&options).map_err(|_| bad_options());
    }
    let length = number(p, "length")?;
    if let Some(site) = p.value("site").and_then(rules_for_domain) {
        let rules = site.into_rules();
        let made = match length {
            Some(n) => rules.generate(n),
            None => rules.generate_default(),
        };
        return made.map_err(|_| bad_options());
    }
    let mut options = PasswordOptions::default();
    if let Some(n) = length {
        options.length = n;
    }
    if p.has("no-lower") {
        options.lowercase = None;
    }
    if p.has("no-upper") {
        options.uppercase = None;
    }
    if p.has("no-digits") {
        options.digits = None;
    }
    if p.has("no-symbols") {
        options.symbols = None;
    }
    generate_password(&options).map_err(|_| bad_options())
}

/// `mosaic generate`.
pub fn generate(app: &mut App, p: &Parsed) -> Result<()> {
    let made = make(p)?;
    if p.has("copy") {
        clipboard::copy(&app.out, made.expose_secret(), clipboard::clear_after(p)?)?;
        return Ok(());
    }
    app.out.write_secret(made.expose_secret(), "value");
    Ok(())
}

/// The value a reference names: its field (default `password`), as a secret.
fn secret_of(
    session: &mut UnlockedSession,
    reference: &str,
    now_secs: u64,
) -> Result<SecretString> {
    let target = refs::resolve(session, reference)?;
    let field = target.field.as_deref().unwrap_or("password");
    if field == "otp" {
        let code = session.otp_code(target.vault, target.item, now_secs)?;
        return SecretString::from_str_copy(code.code.expose_secret())
            .map_err(|_| CliError::internal());
    }
    let item = session.item(target.vault, target.item)?;
    let entry = model::lookup(item, field).ok_or_else(|| refs::no_such_field(field))?;
    match entry.value {
        Value::Secret(s) => Ok(s),
        Value::Text(t) => SecretString::from_str_copy(&t).map_err(|_| CliError::internal()),
    }
}

fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn env_specs(p: &Parsed) -> Result<Vec<(String, String)>> {
    let mut lines: Vec<String> = p.values("env").into_iter().map(str::to_owned).collect();
    if let Some(file) = p.value("env-file") {
        let path = std::path::Path::new(file);
        let text = std::fs::read_to_string(path).map_err(|_| CliError::file_read(path))?;
        lines.extend(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_owned),
        );
    }
    lines
        .into_iter()
        .map(|line| {
            let (name, reference) = line
                .split_once('=')
                .ok_or_else(|| CliError::usage(Msg::ErrInvalidValue).with("name", "--env"))?;
            if !valid_env_name(name) {
                return Err(CliError::usage(Msg::ErrBadEnvName).with("name", name));
            }
            Ok((name.to_owned(), reference.to_owned()))
        })
        .collect()
}

/// `mosaic run`: returns the child's exit code.
pub fn run(app: &mut App, p: &Parsed) -> Result<u8> {
    let specs = env_specs(p)?;
    let resolved: Vec<(String, SecretString)> = {
        // The unlocked session and the store lock live only inside this block (R106).
        let mut u = app.unlock()?;
        let now_secs = app.now / 1000;
        specs
            .iter()
            .map(|(name, reference)| {
                secret_of(&mut u.session, reference, now_secs).map(|s| (name.clone(), s))
            })
            .collect::<Result<_>>()?
    };
    let mut parts = p.trailing.iter();
    let program = parts.next().ok_or_else(CliError::internal)?;
    let mut command = Command::new(program);
    command.args(parts);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("MOSAIC_") {
            command.env_remove(&key);
        }
    }
    for (name, value) in &resolved {
        command.env(name, value.expose_secret());
    }
    let status = command.status().map_err(|_| {
        CliError::new("spawnFailed", EXIT_SPAWN, Msg::ErrSpawnFailed)
            .with("command", program.to_string_lossy().into_owned())
    })?;
    drop(command);
    drop(resolved);
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        if let Some(signal) = status.signal() {
            return Ok(u8::try_from(128 + signal).unwrap_or(EXIT_OTHER));
        }
    }
    Ok(status
        .code()
        .map_or(EXIT_OTHER, |c| u8::try_from(c & 0xff).unwrap_or(EXIT_OTHER)))
}

/// `mosaic inject`.
pub fn inject(app: &mut App, p: &Parsed) -> Result<()> {
    let mut template = Zeroizing::new(String::new());
    match p.value("in") {
        Some(file) => {
            let path = std::path::Path::new(file);
            let mut f = std::fs::File::open(path).map_err(|_| CliError::file_read(path))?;
            f.read_to_string(&mut template)
                .map_err(|_| CliError::file_read(path))?;
        }
        None => {
            std::io::stdin()
                .lock()
                .read_to_string(&mut template)
                .map_err(|_| CliError::internal())?;
        }
    }
    let to_file = p.value("out");
    if to_file.is_none() && std::io::stdout().is_terminal() && !p.has("reveal") {
        return Err(
            CliError::new("revealRequired", EXIT_REVEAL, Msg::ErrRevealRequired)
                .with("field", "inject"),
        );
    }
    let mut result = Zeroizing::new(Vec::<u8>::with_capacity(template.len()));
    let mut u = app.unlock()?;
    let now_secs = app.now / 1000;
    let mut rest = template.as_str();
    while let Some((before, after)) = rest.split_once("{{") {
        result.extend_from_slice(before.as_bytes());
        let Some((inner, tail)) = after.split_once("}}") else {
            result.extend_from_slice(b"{{");
            rest = after;
            continue;
        };
        let reference = inner.trim();
        if reference.starts_with("mosaic://") {
            let secret = secret_of(&mut u.session, reference, now_secs)?;
            result.extend_from_slice(secret.expose_secret().as_bytes());
        } else {
            result.extend_from_slice(b"{{");
            result.extend_from_slice(inner.as_bytes());
            result.extend_from_slice(b"}}");
        }
        rest = tail;
    }
    result.extend_from_slice(rest.as_bytes());
    drop(u);
    if let Some(dest) = to_file {
        let dest = std::path::Path::new(dest);
        let (mut file, tmp) = create_private(dest)?;
        if file.write_all(&result).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return Err(CliError::file_write(dest));
        }
        finish_private(file, &tmp, dest)?;
    } else {
        app.out.write_raw(&result);
    }
    Ok(())
}

//! The command line (M6 spec §6.5), hand-parsed:
//!
//! ```text
//! mosaic-server init         --config <p>
//! mosaic-server serve        --config <p>
//! mosaic-server check-config --config <p>
//! mosaic-server invite create --config <p>
//! mosaic-server backup       --config <p> --out <file> [--blobs]
//! mosaic-server account delete --config <p> --email <e>
//! ```
//!
//! [`run`] writes its output to the given writer; `main` owns stdout and the
//! exit code.

use std::io::Write;
use std::path::{Path, PathBuf};

use mosaic_sync::wire::email;

use crate::db::{accounts, invites};
use crate::{
    Clock, Config, ConfigError, DB_FILE, Db, DbError, SecretFileError, ServerKeys, StartError,
    SystemClock, blobs,
};

/// Why a command failed. The message never contains a secret.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CliError {
    /// Wrong arguments; the text is the usage.
    #[error("{0}")]
    Usage(&'static str),
    /// The configuration is unusable.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The secret file could not be created or read.
    #[error(transparent)]
    Secret(#[from] SecretFileError),
    /// The database could not be opened.
    #[error(transparent)]
    Db(#[from] DbError),
    /// The server could not start or crashed.
    #[error(transparent)]
    Start(#[from] StartError),
    /// A file or directory could not be handled.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The server is already initialised (`init` never overwrites).
    #[error("already initialised: {0} exists")]
    Exists(&'static str),
    /// The target of a backup already exists.
    #[error("the backup target already exists")]
    BackupTargetExists,
    /// No such account.
    #[error("no such account")]
    NoSuchAccount,
    /// The email is not a valid address.
    #[error("not a valid email address")]
    BadEmail,
    /// A database operation failed.
    #[error("operation failed")]
    Failed,
}

const USAGE: &str = "usage: mosaic-server <init|serve|check-config|invite create|backup|account delete> --config <file> [options]";

struct Args {
    config: PathBuf,
    out: Option<PathBuf>,
    email: Option<String>,
    blobs: bool,
}

fn parse_flags(args: &[String]) -> Result<Args, CliError> {
    let mut parsed = Args {
        config: PathBuf::new(),
        out: None,
        email: None,
        blobs: false,
    };
    let mut have_config = false;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--config" => {
                parsed.config = PathBuf::from(it.next().ok_or(CliError::Usage(USAGE))?);
                have_config = true;
            }
            "--out" => parsed.out = Some(PathBuf::from(it.next().ok_or(CliError::Usage(USAGE))?)),
            "--email" => parsed.email = Some(it.next().ok_or(CliError::Usage(USAGE))?.clone()),
            "--blobs" => parsed.blobs = true,
            _ => return Err(CliError::Usage(USAGE)),
        }
    }
    if have_config {
        Ok(parsed)
    } else {
        Err(CliError::Usage(USAGE))
    }
}

/// Runs a command.
///
/// # Errors
/// [`CliError`].
pub fn run(args: &[String], out: &mut dyn Write) -> Result<(), CliError> {
    let (command, rest) = match args.split_first() {
        Some((c, rest)) if matches!(c.as_str(), "invite" | "account") => {
            let (sub, rest) = rest.split_first().ok_or(CliError::Usage(USAGE))?;
            (format!("{c} {sub}"), rest)
        }
        Some((c, rest)) => (c.clone(), rest),
        None => return Err(CliError::Usage(USAGE)),
    };
    let flags = parse_flags(rest)?;
    let config = Config::load(&flags.config)?;
    match command.as_str() {
        "init" => init(&config, out),
        "serve" => serve(config),
        "check-config" => check_config(&config, out),
        "invite create" => invite_create(&config, out),
        "backup" => backup(
            &config,
            flags.out.as_deref().ok_or(CliError::Usage(USAGE))?,
            flags.blobs,
            out,
        ),
        "account delete" => account_delete(
            &config,
            flags.email.as_deref().ok_or(CliError::Usage(USAGE))?,
            out,
        ),
        _ => Err(CliError::Usage(USAGE)),
    }
}

fn open_db(config: &Config) -> Result<Db, CliError> {
    let path = config.data_dir.join(DB_FILE);
    if !path.is_file() {
        return Err(CliError::Usage("not initialised: run `init` first"));
    }
    Ok(Db::open(&path)?)
}

/// Creates the secret file and the database; refuses if either exists.
fn init(config: &Config, out: &mut dyn Write) -> Result<(), CliError> {
    let db_path = config.data_dir.join(DB_FILE);
    if config.secret_file.exists() {
        return Err(CliError::Exists("the secret file"));
    }
    if db_path.exists() {
        return Err(CliError::Exists("the database"));
    }
    std::fs::create_dir_all(config.data_dir.join("blobs"))?;
    crate::fsperm::restrict_dir(&config.data_dir)?;
    if let Some(parent) = config.secret_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    ServerKeys::generate(SystemClock.now_ms())?.create(&config.secret_file)?;
    Db::open(&db_path)?;
    writeln!(
        out,
        "initialised; back up the secret file and the data directory"
    )?;
    Ok(())
}

fn serve(config: Config) -> Result<(), CliError> {
    // Logs go to stderr: method, route, status and duration, never user data.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .try_init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(crate::serve(config))?;
    Ok(())
}

fn check_config(config: &Config, out: &mut dyn Write) -> Result<(), CliError> {
    writeln!(
        out,
        "config ok: origin {}, registration {:?}, listen {}",
        config.canonical_origin, config.registration, config.listen
    )?;
    let state = if config.secret_file.is_file() && config.data_dir.join(DB_FILE).is_file() {
        "initialised"
    } else {
        "not initialised (run `init`)"
    };
    writeln!(out, "{state}")?;
    Ok(())
}

fn invite_create(config: &Config, out: &mut dyn Write) -> Result<(), CliError> {
    let db = open_db(config)?;
    let code = invites::generate().ok_or(CliError::Failed)?;
    let hash = invites::hash(&code);
    let now = SystemClock.now_ms();
    db.run(|c| invites::create(c, &hash, now))
        .map_err(|_| CliError::Failed)?;
    // The code is shown once; only its hash is stored.
    writeln!(out, "{code}")?;
    Ok(())
}

/// A consistent copy of the database (`VACUUM INTO`), and with `--blobs` the
/// attachment files next to it in `<out>.blobs`.
fn backup(
    config: &Config,
    dest: &Path,
    with_blobs: bool,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    let db = open_db(config)?;
    if dest.exists() {
        return Err(CliError::BackupTargetExists);
    }
    db.backup_to(dest)?;
    if with_blobs {
        let mut target = dest.as_os_str().to_owned();
        target.push(".blobs");
        copy_dir(&config.data_dir.join("blobs"), Path::new(&target))?;
    }
    writeln!(out, "backup written")?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn account_delete(config: &Config, address: &str, out: &mut dyn Write) -> Result<(), CliError> {
    let normalized = email::normalize(address).map_err(|_| CliError::BadEmail)?;
    let db = open_db(config)?;
    let blob_store = blobs::Blobs::new(&config.data_dir);
    let removed = db
        .run_tx(|tx| {
            let Some(a) = accounts::by_email(tx, &normalized)? else {
                return Ok(None);
            };
            accounts::delete(tx, &a.account_id).map(Some)
        })
        .map_err(|_| CliError::Failed)?
        .ok_or(CliError::NoSuchAccount)?;
    for id in removed {
        blob_store.remove(&id)?;
    }
    writeln!(out, "account deleted")?;
    Ok(())
}

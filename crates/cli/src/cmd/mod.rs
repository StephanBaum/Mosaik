//! The commands.

mod exec;
mod read;
mod session;
mod sync;
mod transfer;

use crate::app::App;
use crate::error::Result;
use crate::spec::Parsed;

/// Runs a command that needs the run context. Returns the process exit code.
///
/// # Errors
/// The command's error.
pub fn dispatch(app: &mut App, p: &Parsed) -> Result<u8> {
    match p.cmd.name {
        "unlock" => session::unlock(app, p)?,
        "lock" => session::lock(app),
        "status" => session::status(app)?,
        "list" => read::list(app, p)?,
        "search" => read::search(app, p)?,
        "get" => read::get(app, p)?,
        "totp" => read::totp(app, p)?,
        "ssh" => read::ssh(app, p)?,
        "generate" => exec::generate(app, p)?,
        "inject" => exec::inject(app, p)?,
        "run" => return exec::run(app, p),
        "import" => transfer::import(app, p)?,
        "export" => transfer::export(app, p)?,
        "sync" => return sync::sync(app, p),
        "setup" => sync::setup(app, p)?,
        "recover" => sync::recover()?,
        _ => return Err(crate::error::CliError::internal()),
    }
    Ok(0)
}

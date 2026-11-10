//! `unlock`, `lock` and `status`.

use std::time::Duration;

use mosaic_vault::storage::SyncKind;
use serde::Serialize;

use crate::app::{App, PasswordSource};
use crate::error::{CliError, EXIT_OTHER, Result};
use crate::i18n::Msg;
use crate::spec::Parsed;
use crate::util::parse_duration;

const DEFAULT_TTL: Duration = Duration::from_mins(5);
const MAX_TTL: Duration = Duration::from_mins(30);

fn not_cached() -> CliError {
    CliError::new("sessionNotCached", EXIT_OTHER, Msg::ErrSessionNotCached)
}

/// `mosaic unlock`: a master-password unlock, then a session (decision L3).
pub fn unlock(app: &mut App, p: &Parsed) -> Result<()> {
    let ttl = match p.value("ttl") {
        None => DEFAULT_TTL,
        Some(text) => parse_duration(text, 60)
            .filter(|d| *d >= Duration::from_secs(60) && *d <= MAX_TTL)
            .ok_or_else(|| CliError::usage(Msg::ErrInvalidValue).with("name", "--ttl"))?,
    };
    let source = match (p.has("password-stdin"), p.value("password-fd")) {
        (false, None) => PasswordSource::Prompt,
        (true, None) => PasswordSource::Stdin,
        (false, Some(fd)) => PasswordSource::Fd(
            fd.parse()
                .map_err(|_| CliError::usage(Msg::ErrInvalidValue).with("name", "--password-fd"))?,
        ),
        (true, Some(_)) => {
            return Err(CliError::usage(Msg::ErrInvalidValue).with("name", "--password-stdin"));
        }
    };
    // No session store (no keystore, no runtime folder) means nothing to create: say so first.
    if app.be.sessions.is_err() {
        return Err(not_cached());
    }
    let locked = app.open_store()?;
    let unlocked = app.unlock_with_password(locked, source, p.has("secret-key-prompt"))?;
    let (key, file) = unlocked.session.create_cli_session(ttl, app.now)?;
    let saved = match &app.be.sessions {
        Ok(sessions) => sessions.save(unlocked.account, &key, &file).is_ok(),
        Err(_) => false,
    };
    if !saved {
        return Err(not_cached());
    }
    let expires = app
        .now
        .saturating_add(u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX));
    app.out
        .data(&serde_json::json!({ "expiresAt": expires }), |o| {
            o.say(&o.t(
                Msg::OutUnlocked,
                &[("minutes", (ttl.as_secs() / 60).to_string())],
            ));
        });
    Ok(())
}

/// `mosaic lock`: deletes every session (no store access, so it works while the desktop runs).
pub fn lock(app: &mut App) {
    if let Ok(sessions) = &app.be.sessions {
        let _ = sessions.delete_all();
    }
    app.out.data(&serde_json::json!({ "locked": true }), |o| {
        o.say(&o.t(Msg::OutLocked, &[]));
    });
}

#[derive(Serialize)]
struct SessionOut {
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

#[derive(Serialize)]
struct SyncOut {
    backend: &'static str,
    halted: bool,
    findings: usize,
}

#[derive(Serialize)]
struct StatusOut {
    store: String,
    profile: &'static str,
    account: Option<String>,
    locked: bool,
    session: Option<SessionOut>,
    sync: Option<SyncOut>,
}

/// `mosaic status`.
pub fn status(app: &mut App) -> Result<()> {
    let locked = app.open_store()?;
    let account = locked.account_id().ok();
    let session = account.and_then(|a| {
        let (_, file) = app.be.sessions.as_ref().ok()?.load(a).ok().flatten()?;
        file.claimed_expires_at_ms().filter(|e| *e > app.now)
    });
    let store = locked.store();
    let sync = match store.sync_config()? {
        None => None,
        Some(config) => {
            let state = store.engine_state()?;
            let flags = store.vault_flags()?;
            Some(SyncOut {
                backend: match config.kind {
                    SyncKind::Hosted => "hosted",
                    SyncKind::Folder => "folder",
                },
                halted: state.halt.is_some(),
                findings: flags.len() + usize::from(state.halt.is_some()),
            })
        }
    };
    let doc = StatusOut {
        store: app.store_path.display().to_string(),
        profile: app.profile.name(),
        account: account.map(|a| a.to_string()),
        locked: session.is_none(),
        session: session.map(|expires_at| SessionOut { expires_at }),
        sync,
    };
    app.out.data(&doc, |o| {
        let line = |label: Msg, value: String| o.say(&format!("{}: {value}", o.t(label, &[])));
        line(Msg::StatusStore, doc.store.clone());
        line(Msg::StatusProfile, doc.profile.to_owned());
        line(Msg::StatusAccount, doc.account.clone().unwrap_or_default());
        line(
            Msg::StatusSession,
            doc.session.as_ref().map_or_else(
                || o.t(Msg::StatusSessionNone, &[]),
                |s| {
                    o.t(
                        Msg::StatusSessionUntil,
                        &[("time", s.expires_at.to_string())],
                    )
                },
            ),
        );
        line(
            Msg::StatusSync,
            doc.sync.as_ref().map_or_else(
                || o.t(Msg::StatusSyncOff, &[]),
                |s| {
                    o.t(
                        Msg::StatusSyncOn,
                        &[
                            ("backend", s.backend.to_owned()),
                            ("halted", s.halted.to_string()),
                            ("findings", s.findings.to_string()),
                        ],
                    )
                },
            ),
        );
    });
    Ok(())
}

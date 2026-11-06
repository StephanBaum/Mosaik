//! The allowlist of the browser endpoint (ADR-0035, ADR-0037).
//!
//! [`dispatch`] has one `match` on [`Op`], the protocol's closed list of
//! operations, and each arm calls one `Backend::browser_*` method. There is
//! no other way in: nothing here can unlock, lock, reveal a secret, list the
//! vault, add or change an item directly, or touch settings. Work on the
//! vault runs on the blocking pool.
//!
//! Before an op runs: the protocol version, that the pairing still exists
//! (`notPaired`, and the connection closes), the per-pairing rate limit (and, for the
//! passkey ops, the smaller per-origin one), and
//! the lock (every op except `status`, `focus` and `unpair` answers `locked`
//! while the vault is locked).

use std::sync::Arc;

use mosaic_core::SecretString;
use mosaic_native_messaging::bodies::{
    CopyBody, Empty, FillBody, GenerateBody, ListForOriginBody, PasskeyCancelBody,
    PasskeyCreateBody, PasskeyGetBody, SaveBody, SearchBody, StatusResult,
};
use mosaic_native_messaging::protocol::{
    ErrorCode as WireError, MAX_FRAME_BYTES, Op, PROTOCOL_VERSION, RequestEnvelope, error_json,
    ok_json,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use super::connection::Ctx;
use super::fill::parse_origin;
use super::passkey::PasskeyPlan;
use super::passkey_prompt::PasskeyReply;
use super::prompts::cancelled;
use super::registry::RegistryError;
use super::save::MAX_ORIGIN_BYTES;
use super::{Bucket, Shared, unix_millis};
use crate::contract::browser::BrowserCloseReason;
use crate::state::Backend;

/// A finished request: the response to send, and whether to close the
/// connection after it.
pub(super) struct Done {
    pub(super) response: Zeroizing<Vec<u8>>,
    pub(super) close: bool,
}

/// What an arm produces: the response body's JSON, and whether to close.
struct Reply {
    response: Zeroizing<Vec<u8>>,
    close: bool,
}

/// Runs one request (`plain` is the decrypted envelope) and builds its
/// response.
pub(super) async fn run(ctx: &Ctx, plain: &[u8]) -> Done {
    let (id, result) = match serde_json::from_slice::<RequestEnvelope<'_>>(plain) {
        Ok(request) if request.v == PROTOCOL_VERSION => (request.id, dispatch(ctx, &request).await),
        Ok(request) => (request.id, Err(WireError::ProtocolVersion)),
        Err(_) => (0, Err(WireError::InvalidRequest)),
    };
    match result {
        Ok(reply) if reply.response.len() <= MAX_FRAME_BYTES => Done {
            response: reply.response,
            close: reply.close,
        },
        Ok(_) => failure(id, WireError::FrameTooLarge),
        Err(code) => failure(id, code),
    }
}

/// An error response. A pairing that is gone closes the connection.
fn failure(id: u32, code: WireError) -> Done {
    Done {
        response: error_json(id, code).unwrap_or_default(),
        close: code == WireError::NotPaired,
    }
}

fn body<T: Serialize>(id: u32, value: &T) -> Result<Reply, WireError> {
    Ok(Reply {
        response: ok_json(id, value).map_err(|_| WireError::Internal)?,
        close: false,
    })
}

fn parse<'a, T: Deserialize<'a>>(raw: &'a RawValue) -> Result<T, WireError> {
    serde_json::from_str(raw.get()).map_err(|_| WireError::InvalidRequest)
}

/// The rate-limit buckets an op counts against. `proposeSave` also counts
/// against the `checkSave` bucket: it answers `busy` and `cancelled` from the
/// same comparison with the vault that `checkSave` makes, so it must not be a
/// cheaper way to test candidate passwords.
const fn buckets(op: Op) -> &'static [Bucket] {
    match op {
        Op::Fill => &[Bucket::Fill],
        Op::Copy => &[Bucket::Copy],
        Op::ListForOrigin | Op::Search => &[Bucket::Lookup],
        Op::Generate => &[Bucket::Generate],
        Op::CheckSave => &[Bucket::CheckSave],
        Op::ProposeSave => &[Bucket::ProposeSave, Bucket::CheckSave],
        Op::PasskeyCreate => &[Bucket::PasskeyCreate],
        Op::PasskeyGet => &[Bucket::PasskeyGet],
        Op::PasskeyCancel => &[Bucket::PasskeyCancel],
        Op::Focus => &[Bucket::Focus],
        Op::Hello | Op::Pair | Op::Status | Op::Unpair => &[],
    }
}

/// The canonical origin of a passkey request, for its per-origin rate window.
/// `None` when the body has no origin that parses: the handler refuses it
/// (`invalidRequest`), and only the pairing's window has counted it.
fn passkey_origin(raw: &RawValue) -> Option<String> {
    #[derive(Deserialize)]
    struct Origin {
        origin: String,
    }
    let body: Origin = serde_json::from_str(raw.get()).ok()?;
    if body.origin.len() > MAX_ORIGIN_BYTES {
        return None;
    }
    parse_origin(&body.origin).ok().map(|page| page.serialize())
}

/// Counts a request against its rate-limit windows: `rateLimited` when one is full. A site
/// spends its own, smaller passkey windows before the pairing's.
fn count_request(ctx: &Ctx, op: Op, raw: &RawValue) -> Result<(), WireError> {
    let origin = matches!(op, Op::PasskeyCreate | Op::PasskeyGet)
        .then(|| passkey_origin(raw))
        .flatten();
    for &bucket in buckets(op) {
        let allowed = super::lock(&ctx.shared.limiter)?.allow_for_origin(
            bucket,
            ctx.pairing,
            origin.as_deref(),
            ctx.shared.now(),
        );
        if !allowed {
            return Err(WireError::RateLimited);
        }
    }
    Ok(())
}

/// Ops that need the vault unlocked. `passkeyCancel` does not: a page that
/// aborts while the vault locks must still be able to say so.
const fn needs_unlock(op: Op) -> bool {
    !matches!(op, Op::Status | Op::Focus | Op::Unpair | Op::PasskeyCancel)
}

/// Runs `work` on the blocking pool with the backend.
async fn blocking<T, F>(ctx: &Ctx, work: F) -> Result<T, WireError>
where
    F: FnOnce(&Backend) -> Result<T, WireError> + Send + 'static,
    T: Send + 'static,
{
    let backend = ctx.shared.backend().ok_or(WireError::Internal)?;
    tauri::async_runtime::spawn_blocking(move || work(&backend))
        .await
        .map_err(|_| WireError::Internal)?
}

/// The allowlist: every op the extension may call, and nothing else.
async fn dispatch(ctx: &Ctx, request: &RequestEnvelope<'_>) -> Result<Reply, WireError> {
    let op = Op::from_wire(&request.op).ok_or(WireError::InvalidRequest)?;
    let (id, raw) = (request.id, request.body);
    let shared = &ctx.shared;
    // A pairing removed while this session was being set up, or between two
    // requests, ends the session.
    if !shared.is_paired(&ctx.pairing) {
        return Err(WireError::NotPaired);
    }
    count_request(ctx, op, raw)?;
    if needs_unlock(op) && !shared.is_unlocked() {
        return Err(WireError::Locked);
    }
    match op {
        Op::Status => body(
            id,
            &StatusResult {
                state: shared.lock_state(),
            },
        ),
        Op::Focus => {
            parse::<Empty>(raw)?;
            if let Some(backend) = shared.backend() {
                backend.focus_window();
            }
            body(id, &Empty {})
        }
        Op::Unpair => {
            parse::<Empty>(raw)?;
            shared
                .remove_pairing_blocking(ctx.pairing, Some(ctx.conn))
                .await?;
            let mut reply = body(id, &Empty {})?;
            reply.close = true;
            Ok(reply)
        }
        Op::ListForOrigin => {
            let b: ListForOriginBody = parse(raw)?;
            body(
                id,
                &blocking(ctx, move |be| be.browser_list_for_origin(&b)).await?,
            )
        }
        Op::Search => {
            let b: SearchBody = parse(raw)?;
            body(id, &blocking(ctx, move |be| be.browser_search(&b)).await?)
        }
        Op::Fill => {
            let b: FillBody = parse(raw)?;
            let filled = blocking(ctx, move |be| be.browser_fill(&b)).await?;
            shared.note_used_blocking(ctx.pairing).await;
            body(id, &filled)
        }
        Op::Copy => {
            let b: CopyBody = parse(raw)?;
            let copied = blocking(ctx, move |be| be.browser_copy(&b)).await?;
            shared.note_used_blocking(ctx.pairing).await;
            body(id, &copied)
        }
        Op::Generate => {
            let b: GenerateBody<'_> = parse(raw)?;
            // The request holds options and page rules, no secret.
            let request = b.request.to_owned();
            body(
                id,
                &blocking(ctx, move |be| be.browser_generate(&request)).await?,
            )
        }
        Op::CheckSave => {
            let b: SaveBody = parse(raw)?;
            body(
                id,
                &blocking(ctx, move |be| be.browser_check_save(&b)).await?,
            )
        }
        Op::ProposeSave => propose_save(ctx, id, raw).await,
        Op::PasskeyCreate => passkey_create(ctx, id, raw).await,
        Op::PasskeyGet => passkey_get(ctx, id, raw).await,
        Op::PasskeyCancel => {
            let b: PasskeyCancelBody = parse(raw)?;
            // Always `ok`: whether a request was open is not told to the
            // page. Only this pairing's own prompt closes.
            shared.close_passkey_by_request_id(&ctx.pairing, &b.request_id);
            body(id, &Empty {})
        }
        // Host-local ops never reach the desktop.
        Op::Hello | Op::Pair => Err(WireError::InvalidRequest),
    }
}

/// `proposeSave`: classify, show the desktop dialog, wait for the answer.
async fn propose_save(ctx: &Ctx, id: u32, raw: &RawValue) -> Result<Reply, WireError> {
    let captured: SaveBody = parse(raw)?;
    // `busy` before any comparison with the vault: while a prompt is open the
    // answer must not depend on what is saved.
    if ctx.shared.save_is_open() {
        return Err(WireError::Busy);
    }
    let (plan, captured) = blocking(ctx, move |be| {
        let plan = be.browser_plan_save(&captured)?;
        Ok((plan, captured))
    })
    .await?;
    let Some(plan) = plan else {
        // Nothing would change: nothing to confirm.
        return body(id, &cancelled());
    };
    // The prompt keeps the password in Rust; the wire copy goes now, not after the wait.
    let SaveBody {
        username,
        password: wire_password,
        ..
    } = captured;
    let password = SecretString::from_str_copy(wire_password.expose_secret())
        .map_err(|_| WireError::Internal)?;
    drop(wire_password);
    let (request_id, rx) =
        ctx.shared
            .open_save(&ctx.pairing, ctx.conn, plan, username, password)?;
    let result = ctx.shared.await_save(&request_id, rx).await?;
    body(id, &result)
}

/// `passkeyCreate`: plan, show the desktop dialog, wait for the answer. The
/// answer writes the new `passkey` item before it returns.
async fn passkey_create(ctx: &Ctx, id: u32, raw: &RawValue) -> Result<Reply, WireError> {
    let request: PasskeyCreateBody = parse(raw)?;
    // `busy` before any comparison with the vault: while a prompt is open the
    // answer must not depend on what is saved.
    if ctx.shared.passkey_is_open() {
        return Err(WireError::Busy);
    }
    let label = ctx.shared.pairing_label(&ctx.pairing)?;
    let plan = blocking(ctx, move |be| {
        be.browser_plan_passkey_create(&request, &label)
    })
    .await?;
    let (handle, rx) =
        ctx.shared
            .open_passkey(&ctx.pairing, ctx.conn, PasskeyPlan::Create(plan))?;
    finish_passkey(ctx, id, &handle, rx).await
}

/// `passkeyGet`: plan (no matching passkey answers `passkeyNotFound` without
/// a dialog), show the desktop dialog, wait, sign.
async fn passkey_get(ctx: &Ctx, id: u32, raw: &RawValue) -> Result<Reply, WireError> {
    let request: PasskeyGetBody = parse(raw)?;
    if ctx.shared.passkey_is_open() {
        return Err(WireError::Busy);
    }
    let label = ctx.shared.pairing_label(&ctx.pairing)?;
    let plan = blocking(ctx, move |be| be.browser_plan_passkey_get(&request, &label)).await?;
    let (handle, rx) = ctx
        .shared
        .open_passkey(&ctx.pairing, ctx.conn, PasskeyPlan::Get(plan))?;
    finish_passkey(ctx, id, &handle, rx).await
}

async fn finish_passkey(
    ctx: &Ctx,
    id: u32,
    handle: &str,
    rx: tokio::sync::oneshot::Receiver<super::passkey_prompt::PasskeyResult>,
) -> Result<Reply, WireError> {
    let reply = ctx.shared.await_passkey(handle, rx).await?;
    if reply.completed() {
        ctx.shared.note_used_blocking(ctx.pairing).await;
    }
    match reply {
        PasskeyReply::Create(result) => body(id, &result),
        PasskeyReply::Get(result) => body(id, &result),
    }
}

impl Shared {
    /// Removes a pairing: the file, the rate-limit windows, its save prompt
    /// and every live session (told `unpaired`, then closed), except
    /// `except`, which the caller closes itself. Writes the file: not for an
    /// async thread (see `remove_pairing_blocking`).
    pub(crate) fn remove_pairing(
        &self,
        pairing: &[u8; 16],
        except: Option<u64>,
    ) -> Result<bool, WireError> {
        let removed =
            super::lock(&self.registry)?
                .remove(pairing)
                .map_err(|e: RegistryError| match e {
                    RegistryError::Full | RegistryError::Storage | RegistryError::Unreadable => {
                        WireError::Internal
                    }
                })?;
        if let Ok(mut limiter) = self.limiter.lock() {
            limiter.forget(pairing);
        }
        self.close_pairing(pairing, except);
        // The prompt of a removed pairing has nobody to answer to; its
        // password is dropped.
        self.close_saves_where(BrowserCloseReason::Disconnected, |save| {
            save.pairing == *pairing
        });
        self.close_passkeys_where(BrowserCloseReason::Disconnected, |passkey| {
            passkey.pairing == *pairing
        });
        self.pairings_changed();
        Ok(removed)
    }

    /// [`Shared::remove_pairing`] on the blocking pool.
    pub(crate) async fn remove_pairing_blocking(
        self: &Arc<Self>,
        pairing: [u8; 16],
        except: Option<u64>,
    ) -> Result<bool, WireError> {
        let shared = Arc::clone(self);
        tauri::async_runtime::spawn_blocking(move || shared.remove_pairing(&pairing, except))
            .await
            .map_err(|_| WireError::Internal)?
    }

    /// Records that a pairing filled or copied (the file is written at most
    /// every ten minutes, on the blocking pool).
    async fn note_used_blocking(self: &Arc<Self>, pairing: [u8; 16]) {
        let shared = Arc::clone(self);
        let _ = tauri::async_runtime::spawn_blocking(move || {
            if let Ok(mut registry) = shared.registry.lock() {
                registry.note_used(&pairing, unix_millis());
            }
        })
        .await;
    }
}

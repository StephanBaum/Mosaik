//! The allowlist of the browser endpoint (ADR-0035, ADR-0037).
//!
//! [`dispatch`] has one `match` on [`Op`], the protocol's closed list of
//! operations, and each arm calls one `Backend::browser_*` method. There is
//! no other way in: nothing here can unlock, lock, reveal a secret, list the
//! vault, add or change an item directly, or touch settings. Work on the
//! vault runs on the blocking pool.
//!
//! Before an op runs: the protocol version, the per-pairing rate limit, and
//! the lock (every op except `status`, `focus` and `unpair` answers `locked`
//! while the vault is locked).

use mosaic_core::SecretString;
use mosaic_native_messaging::bodies::{
    CopyBody, Empty, FillBody, GenerateBody, ListForOriginBody, SaveBody, SearchBody, StatusResult,
};
use mosaic_native_messaging::protocol::{
    ErrorCode as WireError, MAX_FRAME_BYTES, Op, PROTOCOL_VERSION, RequestEnvelope, error_json,
    ok_json,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use super::connection::Ctx;
use super::prompts::cancelled;
use super::registry::RegistryError;
use super::{Bucket, Shared, unix_millis};
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

fn failure(id: u32, code: WireError) -> Done {
    Done {
        response: error_json(id, code).unwrap_or_default(),
        close: false,
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

/// The rate-limit bucket of an op.
const fn bucket(op: Op) -> Option<Bucket> {
    match op {
        Op::Fill => Some(Bucket::Fill),
        Op::Copy => Some(Bucket::Copy),
        Op::ListForOrigin | Op::Search => Some(Bucket::Lookup),
        Op::Generate => Some(Bucket::Generate),
        Op::CheckSave => Some(Bucket::CheckSave),
        Op::ProposeSave => Some(Bucket::ProposeSave),
        Op::Focus => Some(Bucket::Focus),
        Op::Hello | Op::Pair | Op::Status | Op::Unpair => None,
    }
}

/// Ops that need the vault unlocked.
const fn needs_unlock(op: Op) -> bool {
    !matches!(op, Op::Status | Op::Focus | Op::Unpair)
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
    if let Some(bucket) = bucket(op) {
        let allowed = super::lock(&shared.limiter)?.allow(bucket, Some(ctx.pairing), shared.now());
        if !allowed {
            return Err(WireError::RateLimited);
        }
    }
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
            shared.remove_pairing(&ctx.pairing, Some(ctx.conn))?;
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
            shared.note_used(&ctx.pairing);
            body(id, &filled)
        }
        Op::Copy => {
            let b: CopyBody = parse(raw)?;
            let copied = blocking(ctx, move |be| be.browser_copy(&b)).await?;
            shared.note_used(&ctx.pairing);
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
        // Host-local ops never reach the desktop.
        Op::Hello | Op::Pair => Err(WireError::InvalidRequest),
    }
}

/// `proposeSave`: classify, show the desktop dialog, wait for the answer.
async fn propose_save(ctx: &Ctx, id: u32, raw: &RawValue) -> Result<Reply, WireError> {
    let captured: SaveBody = parse(raw)?;
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
    let (request_id, rx) = ctx
        .shared
        .open_save(&ctx.pairing, plan, username, password)?;
    let result = ctx.shared.await_save(&request_id, rx).await?;
    body(id, &result)
}

impl Shared {
    /// Removes a pairing: the file, the rate-limit windows and every live
    /// session (told `unpaired`, then closed), except `except`, which the
    /// caller closes itself.
    pub(crate) fn remove_pairing(
        &self,
        pairing: &[u8; 16],
        except: Option<u64>,
    ) -> Result<bool, WireError> {
        let removed =
            super::lock(&self.registry)?
                .remove(pairing)
                .map_err(|e: RegistryError| match e {
                    RegistryError::Full | RegistryError::Storage => WireError::Internal,
                })?;
        if let Ok(mut limiter) = self.limiter.lock() {
            limiter.forget(pairing);
        }
        self.close_pairing(pairing, except);
        self.pairings_changed();
        Ok(removed)
    }

    /// Records that a pairing filled or copied.
    fn note_used(&self, pairing: &[u8; 16]) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.note_used(pairing, unix_millis());
        }
    }
}

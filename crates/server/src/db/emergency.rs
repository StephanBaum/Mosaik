//! Emergency access (CRYPTO_SPEC §8.4, M6b spec §4): grants, the sealed vault
//! keys the grantor keeps fresh, and the waiting-period state machine that the
//! server enforces. The server never opens a seal.

use axum::http::StatusCode;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::sharing::{DAY_MS, bump_inbox};
use super::{audit, id16, int};
use crate::api::codes;
use crate::error::ApiError;

/// The waiting periods the grantor may choose (days).
pub const WAIT_DAYS: [u8; 5] = [1, 2, 7, 14, 30];
/// The pause after a refusal before the grantee may ask again.
pub const COOLDOWN_MS: u64 = DAY_MS;
/// Most grants one account gives, and most it receives.
pub const MAX_GRANTS: i64 = 20;
/// Most vault seals in one grant.
pub const MAX_SEALS: usize = 1000;

/// No request is open.
pub const IDLE: u8 = 0;
/// The grantee asked; the waiting period runs.
pub const WAITING: u8 = 1;
/// The grantor approved before the wait ended.
pub const APPROVED: u8 = 2;
/// The seals were handed over.
pub const RELEASED: u8 = 3;

/// A grant as stored.
struct Grant {
    wait_days: u8,
    state: u8,
    requested_at: Option<u64>,
    cooldown_until: Option<u64>,
}

fn small(r: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<u8> {
    let v: i64 = r.get(idx)?;
    u8::try_from(v).map_err(|_| rusqlite::Error::InvalidQuery)
}

fn opt_u64(r: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<Option<u64>> {
    let v: Option<i64> = r.get(idx)?;
    Ok(v.and_then(|v| u64::try_from(v).ok()))
}

fn grant(
    conn: &Connection,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
) -> Result<Option<Grant>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT wait_days, state, requested_at, cooldown_until FROM emergency_grants \
             WHERE grantor = ?1 AND grantee = ?2",
            params![grantor.as_slice(), grantee.as_slice()],
            |r| {
                Ok(Grant {
                    wait_days: small(r, 0)?,
                    state: small(r, 1)?,
                    requested_at: opt_u64(r, 2)?,
                    cooldown_until: opt_u64(r, 3)?,
                })
            },
        )
        .optional()?)
}

fn release_at(g: &Grant) -> Option<u64> {
    g.requested_at
        .map(|t| t.saturating_add(u64::from(g.wait_days).saturating_mul(DAY_MS)))
}

fn state_error(code: &'static str) -> ApiError {
    ApiError::conflict(code)
}

/// A sealed vault key of a grant.
pub struct Seal {
    pub vault_id: [u8; 16],
    pub generation: u32,
    pub blob: Vec<u8>,
}

/// The vaults of `seals` must be owned by `grantor` (vaults shared to the
/// grantor are excluded, §4.1) and the generations known.
fn check_seals(tx: &Connection, grantor: &[u8; 16], seals: &[Seal]) -> Result<(), ApiError> {
    for s in seals {
        let row: Option<(Vec<u8>, i64)> = tx
            .query_row(
                "SELECT owner_account_id, current_generation FROM vaults WHERE vault_id = ?1",
                [s.vault_id.as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match row {
            Some((owner, current)) if owner == grantor && i64::from(s.generation) <= current => {}
            _ => {
                return Err(ApiError::new(
                    StatusCode::FORBIDDEN,
                    codes::EMERGENCY_OWNER_ONLY,
                ));
            }
        }
    }
    Ok(())
}

fn store_seals(
    tx: &Transaction<'_>,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
    seals: &[Seal],
) -> Result<(), ApiError> {
    for s in seals {
        // A seal is replaced only by one of the same or a newer generation.
        tx.execute(
            "INSERT INTO emergency_seals (grantor, grantee, vault_id, generation, seal) \
             VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (grantor, grantee, vault_id) DO UPDATE SET \
             generation = excluded.generation, seal = excluded.seal \
             WHERE excluded.generation >= emergency_seals.generation",
            params![
                grantor.as_slice(),
                grantee.as_slice(),
                s.vault_id.as_slice(),
                s.generation,
                s.blob
            ],
        )?;
    }
    Ok(())
}

fn count(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> Result<i64, ApiError> {
    Ok(conn.query_row(sql, p, |r| r.get(0))?)
}

/// What a grant upload sets.
pub struct NewGrant<'a> {
    pub wait_days: u8,
    /// 1 all vaults, 2 selected.
    pub scope: u8,
    pub seals: &'a [Seal],
}

/// Creates or updates a grant. The wait can't change while a request is
/// open (`emergency_waiting`), and a released grant is final until deleted.
pub fn put_grant(
    tx: &Transaction<'_>,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
    g: &NewGrant<'_>,
    now: u64,
) -> Result<(), ApiError> {
    if grantor == grantee || !WAIT_DAYS.contains(&g.wait_days) || g.seals.len() > MAX_SEALS {
        return Err(ApiError::bad_request());
    }
    let mut seen = std::collections::BTreeSet::new();
    if !g.seals.iter().all(|s| seen.insert(s.vault_id)) {
        return Err(ApiError::bad_request());
    }
    if !super::sharing::account_exists(tx, grantee)? {
        return Err(ApiError::not_found());
    }
    let existing = grant(tx, grantor, grantee)?;
    match existing.as_ref().map(|e| e.state) {
        Some(WAITING) => return Err(state_error(codes::EMERGENCY_WAITING)),
        Some(APPROVED | RELEASED) => return Err(state_error(codes::EMERGENCY_RELEASED)),
        _ => {}
    }
    if existing.is_none() {
        let given = count(
            tx,
            "SELECT COUNT(*) FROM emergency_grants WHERE grantor = ?1",
            [grantor.as_slice()],
        )?;
        let received = count(
            tx,
            "SELECT COUNT(*) FROM emergency_grants WHERE grantee = ?1",
            [grantee.as_slice()],
        )?;
        if given >= MAX_GRANTS || received >= MAX_GRANTS {
            return Err(super::devices::quota("grants"));
        }
    }
    check_seals(tx, grantor, g.seals)?;
    tx.execute(
        "INSERT INTO emergency_grants (grantor, grantee, wait_days, scope, state, created_at) \
         VALUES (?1, ?2, ?3, ?4, 0, ?5) ON CONFLICT (grantor, grantee) DO UPDATE SET \
         wait_days = excluded.wait_days, scope = excluded.scope",
        params![
            grantor.as_slice(),
            grantee.as_slice(),
            g.wait_days,
            g.scope,
            int(now)
        ],
    )?;
    // The upload is the whole set: vaults not named leave the grant.
    tx.execute(
        "DELETE FROM emergency_seals WHERE grantor = ?1 AND grantee = ?2",
        params![grantor.as_slice(), grantee.as_slice()],
    )?;
    store_seals(tx, grantor, grantee, g.seals)?;
    audit::record(tx, grantor, now, audit::Kind::EmergencyGranted, None)?;
    bump_inbox(tx, grantee)?;
    bump_inbox(tx, grantor)?;
    Ok(())
}

/// Replaces seals of an existing grant (keeping them fresh, §4.3).
pub fn put_seals(
    tx: &Transaction<'_>,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
    seals: &[Seal],
) -> Result<(), ApiError> {
    if seals.len() > MAX_SEALS {
        return Err(ApiError::bad_request());
    }
    grant(tx, grantor, grantee)?.ok_or_else(ApiError::not_found)?;
    check_seals(tx, grantor, seals)?;
    store_seals(tx, grantor, grantee, seals)
}

/// Deletes a grant in any state, with its seals. Memberships a release
/// created stay: the owner's next rotation names them in `revoke`.
pub fn delete_grant(
    tx: &Transaction<'_>,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
) -> Result<(), ApiError> {
    let n = tx.execute(
        "DELETE FROM emergency_grants WHERE grantor = ?1 AND grantee = ?2",
        params![grantor.as_slice(), grantee.as_slice()],
    )?;
    if n > 0 {
        bump_inbox(tx, grantee)?;
        bump_inbox(tx, grantor)?;
    }
    Ok(())
}

/// The grantee asks. Returns when the wait ends. `emergency_cooldown`
/// within 24 h of a refusal.
pub fn request(
    tx: &Transaction<'_>,
    grantee: &[u8; 16],
    grantor: &[u8; 16],
    now: u64,
) -> Result<u64, ApiError> {
    let g = grant(tx, grantor, grantee)?.ok_or_else(ApiError::not_found)?;
    match g.state {
        WAITING => return release_at(&g).ok_or_else(ApiError::internal),
        APPROVED | RELEASED => return Err(state_error(codes::EMERGENCY_RELEASED)),
        _ => {}
    }
    if let Some(until) = g.cooldown_until.filter(|u| *u > now) {
        return Err(ApiError::conflict(codes::EMERGENCY_COOLDOWN)
            .with_detail(&serde_json::json!({ "until": until })));
    }
    tx.execute(
        "UPDATE emergency_grants SET state = 1, requested_at = ?3, decided_at = NULL \
         WHERE grantor = ?1 AND grantee = ?2",
        params![grantor.as_slice(), grantee.as_slice(), int(now)],
    )?;
    audit::record(tx, grantee, now, audit::Kind::EmergencyRequested, None)?;
    audit::record(tx, grantor, now, audit::Kind::EmergencyRequested, None)?;
    bump_inbox(tx, grantee)?;
    bump_inbox(tx, grantor)?;
    Ok(now.saturating_add(u64::from(g.wait_days).saturating_mul(DAY_MS)))
}

/// The grantor refuses (back to idle with a cooldown) or approves.
pub fn decide(
    tx: &Transaction<'_>,
    grantor: &[u8; 16],
    grantee: &[u8; 16],
    approve: bool,
    now: u64,
) -> Result<(), ApiError> {
    let g = grant(tx, grantor, grantee)?.ok_or_else(ApiError::not_found)?;
    match (g.state, approve) {
        (WAITING, false) => {
            tx.execute(
                "UPDATE emergency_grants SET state = 0, requested_at = NULL, decided_at = ?3, \
                 cooldown_until = ?4 WHERE grantor = ?1 AND grantee = ?2",
                params![
                    grantor.as_slice(),
                    grantee.as_slice(),
                    int(now),
                    int(now.saturating_add(COOLDOWN_MS))
                ],
            )?;
        }
        (WAITING, true) => {
            tx.execute(
                "UPDATE emergency_grants SET state = 2, decided_at = ?3 WHERE grantor = ?1 AND grantee = ?2",
                params![grantor.as_slice(), grantee.as_slice(), int(now)],
            )?;
        }
        // Repeating the decision is fine.
        (IDLE, false) | (APPROVED | RELEASED, true) => return Ok(()),
        (IDLE, true) => return Err(state_error(codes::EMERGENCY_NOT_REQUESTED)),
        (APPROVED | RELEASED, false) => return Err(state_error(codes::EMERGENCY_RELEASED)),
        _ => return Err(ApiError::internal()),
    }
    audit::record(tx, grantor, now, audit::Kind::EmergencyDecided, None)?;
    bump_inbox(tx, grantee)?;
    bump_inbox(tx, grantor)?;
    Ok(())
}

/// The accounts a release changed: the vaults it opened (with their
/// members, for the poll notification).
pub struct Released {
    pub seals: Vec<Seal>,
    pub grantor: [u8; 16],
}

/// Hands the seals to the grantee once the wait is over (or the grantor
/// approved) and adds a `read` membership with origin `emergency` to every
/// vault in the grant.
pub fn release(
    tx: &Transaction<'_>,
    grantee: &[u8; 16],
    grantor: &[u8; 16],
    now: u64,
) -> Result<Released, ApiError> {
    let g = grant(tx, grantor, grantee)?.ok_or_else(ApiError::not_found)?;
    match g.state {
        WAITING => {
            let at = release_at(&g).ok_or_else(ApiError::internal)?;
            if now < at {
                return Err(ApiError::conflict(codes::EMERGENCY_WAITING)
                    .with_detail(&serde_json::json!({ "releaseAt": at })));
            }
        }
        APPROVED | RELEASED => {}
        _ => return Err(state_error(codes::EMERGENCY_NOT_REQUESTED)),
    }
    let mut stmt = tx.prepare(
        "SELECT vault_id, generation, seal FROM emergency_seals WHERE grantor = ?1 AND grantee = ?2 \
         ORDER BY vault_id",
    )?;
    let seals = stmt
        .query_map(params![grantor.as_slice(), grantee.as_slice()], |r| {
            Ok(Seal {
                vault_id: id16(r, 0)?,
                generation: u32::try_from(r.get::<_, i64>(1)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                blob: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for s in &seals {
        tx.execute(
            "INSERT INTO memberships (account_id, vault_id, role, origin, added_at) \
             VALUES (?1, ?2, 4, 2, ?3) ON CONFLICT (account_id, vault_id) DO NOTHING",
            params![grantee.as_slice(), s.vault_id.as_slice(), int(now)],
        )?;
    }
    if g.state != RELEASED {
        tx.execute(
            "UPDATE emergency_grants SET state = 3, decided_at = ?3 WHERE grantor = ?1 AND grantee = ?2",
            params![grantor.as_slice(), grantee.as_slice(), int(now)],
        )?;
        audit::record(tx, grantee, now, audit::Kind::EmergencyReleased, None)?;
        audit::record(tx, grantor, now, audit::Kind::EmergencyReleased, None)?;
        bump_inbox(tx, grantee)?;
        bump_inbox(tx, grantor)?;
    }
    Ok(Released {
        seals,
        grantor: *grantor,
    })
}

/// A grant the caller gave.
pub struct Given {
    pub grantee: [u8; 16],
    pub email: String,
    pub wait_days: u8,
    pub scope: u8,
    pub vaults: Vec<([u8; 16], u32)>,
    pub state: u8,
    pub requested_at: Option<u64>,
    pub release_at: Option<u64>,
}

/// A grant the caller received.
pub struct Received {
    pub grantor: [u8; 16],
    pub email: String,
    pub wait_days: u8,
    pub state: u8,
    pub requested_at: Option<u64>,
    pub release_at: Option<u64>,
    pub cooldown_until: Option<u64>,
}

/// The caller's grants, both directions.
pub fn overview(
    conn: &Connection,
    account: &[u8; 16],
) -> Result<(Vec<Given>, Vec<Received>), ApiError> {
    let mut stmt = conn.prepare(
        "SELECT g.grantee, a.email, g.wait_days, g.scope, g.state, g.requested_at FROM emergency_grants g \
         JOIN accounts a ON a.account_id = g.grantee WHERE g.grantor = ?1 ORDER BY g.created_at, g.grantee",
    )?;
    let mut given = stmt
        .query_map([account.as_slice()], |r| {
            let wait_days = small(r, 2)?;
            let requested_at = opt_u64(r, 5)?;
            Ok(Given {
                grantee: id16(r, 0)?,
                email: r.get(1)?,
                wait_days,
                scope: small(r, 3)?,
                vaults: Vec::new(),
                state: small(r, 4)?,
                requested_at,
                release_at: requested_at
                    .map(|t| t.saturating_add(u64::from(wait_days).saturating_mul(DAY_MS))),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for g in &mut given {
        let mut stmt = conn.prepare(
            "SELECT vault_id, generation FROM emergency_seals WHERE grantor = ?1 AND grantee = ?2 \
             ORDER BY vault_id",
        )?;
        g.vaults = stmt
            .query_map(params![account.as_slice(), g.grantee.as_slice()], |r| {
                Ok((
                    id16(r, 0)?,
                    u32::try_from(r.get::<_, i64>(1)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
    }
    let mut stmt = conn.prepare(
        "SELECT g.grantor, a.email, g.wait_days, g.state, g.requested_at, g.cooldown_until \
         FROM emergency_grants g JOIN accounts a ON a.account_id = g.grantor WHERE g.grantee = ?1 \
         ORDER BY g.created_at, g.grantor",
    )?;
    let received = stmt
        .query_map([account.as_slice()], |r| {
            let wait_days = small(r, 2)?;
            let requested_at = opt_u64(r, 4)?;
            Ok(Received {
                grantor: id16(r, 0)?,
                email: r.get(1)?,
                wait_days,
                state: small(r, 3)?,
                requested_at,
                release_at: requested_at
                    .map(|t| t.saturating_add(u64::from(wait_days).saturating_mul(DAY_MS))),
                cooldown_until: opt_u64(r, 5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((given, received))
}

/// Whether the two accounts are on one grant (either direction).
pub fn related(conn: &Connection, a: &[u8; 16], b: &[u8; 16]) -> Result<bool, ApiError> {
    Ok(count(
        conn,
        "SELECT COUNT(*) FROM emergency_grants WHERE (grantor = ?1 AND grantee = ?2) \
         OR (grantor = ?2 AND grantee = ?1)",
        params![a.as_slice(), b.as_slice()],
    )? > 0)
}

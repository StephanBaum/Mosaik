//! Identity keys, the contact book, blocks, invites, key shares, rosters,
//! roles and shared-vault rotation (M6b spec §5). The server stores and
//! serves ciphertexts and seals; it never opens one.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use rusqlite::{Connection, ErrorCode, OptionalExtension, Transaction, params};

use super::vaults::{self, role};
use super::{audit, id16, int, u64_col};
use crate::api::codes;
use crate::error::ApiError;
use mosaic_sync::wire::json::GenerationDetail;

/// One day in milliseconds.
pub const DAY_MS: u64 = 86_400_000;
/// How long an invite stays open.
pub const INVITE_TTL_MS: u64 = 14 * DAY_MS;
/// Most members of a vault (pending invites count).
pub const MAX_MEMBERS: i64 = 50;
/// Most open invites one account has sent.
pub const MAX_OUTGOING_INVITES: i64 = 200;
/// Most open invites one account holds (more are dropped silently).
pub const MAX_INCOMING_INVITES: i64 = 100;
/// Most blocked accounts.
pub const MAX_BLOCKS: i64 = 1000;
/// How long a finished invite stays listed for its sender.
const OUTGOING_KEEP_MS: u64 = 30 * DAY_MS;

/// Raises the account's inbox counter (the poll change token's source).
pub fn bump_inbox(conn: &Connection, account: &[u8; 16]) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE accounts SET inbox_seq = inbox_seq + 1 WHERE account_id = ?1",
        [account.as_slice()],
    )?;
    Ok(())
}

fn conflict_with(code: &'static str, key: &str, value: u64) -> ApiError {
    ApiError::conflict(code).with_detail(&serde_json::json!({ key: value }))
}

fn generation_error(code: &'static str, current: u32) -> ApiError {
    ApiError::conflict(code).with_detail(&GenerationDetail {
        current_generation: current,
    })
}

// ---- identity -----------------------------------------------------------

/// The result of publishing an identity key.
pub enum Identity {
    /// Stored now.
    Set,
    /// The same pair was already stored.
    Same,
    /// A different pair is stored: `(ipk, wrapped_isk)`.
    Exists(Vec<u8>, Vec<u8>),
}

/// An identity key pair as stored: `(ipk, wrapped_isk)`.
pub type IdentityPair = (Vec<u8>, Vec<u8>);

/// The account's published identity.
pub fn identity(conn: &Connection, account: &[u8; 16]) -> Result<Option<IdentityPair>, ApiError> {
    let row = conn
        .query_row(
            "SELECT ipk, wrapped_isk FROM accounts WHERE account_id = ?1",
            [account.as_slice()],
            |r| {
                Ok((
                    r.get::<_, Option<Vec<u8>>>(0)?,
                    r.get::<_, Option<Vec<u8>>>(1)?,
                ))
            },
        )
        .optional()?;
    Ok(row.and_then(|(a, b)| a.zip(b)))
}

/// Stores the pair if none is set (first write wins; CRYPTO_SPEC §8.1).
pub fn set_identity(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    ipk: &[u8; 32],
    wrapped: &[u8; 74],
    now: u64,
) -> Result<Identity, ApiError> {
    let n = tx.execute(
        "UPDATE accounts SET ipk = ?1, wrapped_isk = ?2 WHERE account_id = ?3 AND ipk IS NULL",
        params![ipk.as_slice(), wrapped.as_slice(), account.as_slice()],
    )?;
    if n == 1 {
        audit::record(tx, account, now, audit::Kind::IdentityPublished, None)?;
        return Ok(Identity::Set);
    }
    match identity(tx, account)? {
        Some((a, b)) if a == ipk && b == wrapped => Ok(Identity::Same),
        Some((a, b)) => Ok(Identity::Exists(a, b)),
        None => Err(ApiError::unauthorized()),
    }
}

// ---- contact book -------------------------------------------------------

/// The contact book ciphertext and its sequence (0 when none).
pub fn contacts(conn: &Connection, account: &[u8; 16]) -> Result<(Option<Vec<u8>>, u64), ApiError> {
    Ok(conn.query_row(
        "SELECT contacts_ct, contacts_seq FROM accounts WHERE account_id = ?1",
        [account.as_slice()],
        |r| Ok((r.get(0)?, u64_col(r, 1)?)),
    )?)
}

/// Replaces the contact book if `base_seq` is the stored sequence. `Err` is
/// the stored sequence.
pub fn put_contacts(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    ciphertext: &[u8],
    base_seq: u64,
) -> Result<Result<u64, u64>, ApiError> {
    let n = tx.execute(
        "UPDATE accounts SET contacts_ct = ?1, contacts_seq = contacts_seq + 1 \
         WHERE account_id = ?2 AND contacts_seq = ?3",
        params![ciphertext, account.as_slice(), int(base_seq)],
    )?;
    if n == 1 {
        return Ok(Ok(base_seq.saturating_add(1)));
    }
    let (_, current) = contacts(tx, account)?;
    Ok(Err(current))
}

// ---- blocks -------------------------------------------------------------

/// Whether `account` blocked `other`.
pub fn is_blocked(
    conn: &Connection,
    account: &[u8; 16],
    other: &[u8; 16],
) -> Result<bool, ApiError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM blocks WHERE account_id = ?1 AND blocked = ?2",
            params![account.as_slice(), other.as_slice()],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// Blocks an account ID (no check that it exists: no oracle).
pub fn block(conn: &Connection, account: &[u8; 16], other: &[u8; 16]) -> Result<(), ApiError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blocks WHERE account_id = ?1",
        [account.as_slice()],
        |r| r.get(0),
    )?;
    if n >= MAX_BLOCKS && !is_blocked(conn, account, other)? {
        return Err(super::devices::quota("blocks"));
    }
    conn.execute(
        "INSERT OR IGNORE INTO blocks (account_id, blocked) VALUES (?1, ?2)",
        params![account.as_slice(), other.as_slice()],
    )?;
    Ok(())
}

/// Removes a block.
pub fn unblock(conn: &Connection, account: &[u8; 16], other: &[u8; 16]) -> Result<(), ApiError> {
    conn.execute(
        "DELETE FROM blocks WHERE account_id = ?1 AND blocked = ?2",
        params![account.as_slice(), other.as_slice()],
    )?;
    Ok(())
}

// ---- invites ------------------------------------------------------------

/// The IDs of the vault's open invites (hidden and fake ones too).
pub fn pending_invite_ids(
    conn: &Connection,
    vault: &[u8; 16],
    now: u64,
) -> Result<Vec<[u8; 16]>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT invite_id FROM share_invites WHERE vault_id = ?1 AND state = 0 AND expires_at > ?2 \
         ORDER BY invite_id",
    )?;
    let rows = stmt.query_map(params![vault.as_slice(), int(now)], |r| id16(r, 0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// What an invite upload carries.
pub struct NewInvite<'a> {
    pub invite_id: &'a [u8; 16],
    pub recipient: &'a [u8; 16],
    pub role: u8,
    pub generation: u32,
    pub seal: &'a [u8],
}

/// The result of storing an invite.
pub struct Sent {
    pub expires_at: u64,
    /// The recipient is a real account that will see the invite (poll it).
    pub notify_recipient: bool,
}

fn count(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> Result<i64, ApiError> {
    Ok(conn.query_row(sql, p, |r| r.get(0))?)
}

/// The rules of an invite that do not depend on the caller's role.
fn check_invite(
    tx: &Transaction<'_>,
    sender: &[u8; 16],
    vault: &[u8; 16],
    inv: &NewInvite<'_>,
    current_generation: u32,
    now: u64,
) -> Result<(), ApiError> {
    tx.execute(
        "UPDATE share_invites SET state = 4 WHERE vault_id = ?1 AND state = 0 AND expires_at <= ?2",
        params![vault.as_slice(), int(now)],
    )?;
    let ids = params![vault.as_slice(), inv.recipient.as_slice()];
    if count(
        tx,
        "SELECT COUNT(*) FROM memberships WHERE vault_id = ?1 AND account_id = ?2",
        ids,
    )? > 0
    {
        return Err(ApiError::conflict(codes::ALREADY_MEMBER));
    }
    // No path around the roster's tie rule: a removal at generation g blocks
    // a re-invite until the vault has rotated past g.
    let blocked_at: Option<i64> = tx
        .query_row(
            "SELECT generation FROM vault_removals WHERE vault_id = ?1 AND account_id = ?2",
            params![vault.as_slice(), inv.recipient.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    if blocked_at.is_some_and(|g| g >= i64::from(current_generation)) {
        return Err(generation_error(
            codes::REINVITE_BLOCKED,
            current_generation,
        ));
    }
    if count(
        tx,
        "SELECT COUNT(*) FROM share_invites WHERE vault_id = ?1 AND recipient = ?2 AND state = 0",
        params![vault.as_slice(), inv.recipient.as_slice()],
    )? > 0
    {
        return Err(ApiError::conflict(codes::INVITE_EXISTS));
    }
    let used = count(
        tx,
        "SELECT (SELECT COUNT(*) FROM memberships WHERE vault_id = ?1) \
         + (SELECT COUNT(*) FROM share_invites WHERE vault_id = ?1 AND state = 0)",
        [vault.as_slice()],
    )?;
    if used >= MAX_MEMBERS {
        return Err(ApiError::conflict(codes::MEMBERS_LIMIT));
    }
    if count(
        tx,
        "SELECT COUNT(*) FROM share_invites WHERE sender = ?1 AND state = 0",
        [sender.as_slice()],
    )? >= MAX_OUTGOING_INVITES
    {
        return Err(ApiError::conflict(codes::INVITE_LIMIT));
    }
    Ok(())
}

/// Stores an invite (CRYPTO_SPEC §8.2, M6b spec §5.2, §5.4). Every outcome
/// that depends on the recipient (no account, blocked sender, full inbox)
/// answers like a stored invite.
pub fn create_invite(
    tx: &Transaction<'_>,
    sender: &[u8; 16],
    vault: &[u8; 16],
    inv: &NewInvite<'_>,
    now: u64,
) -> Result<Sent, ApiError> {
    let v = vaults::access_role(tx, sender, vault, role::MANAGE)?;
    // An invite never grants more than the inviter holds, and never `owner`.
    if inv.role < v.role || inv.role <= role::OWNER || inv.role > role::READ {
        return Err(vaults::role_forbidden());
    }
    if inv.generation != v.current_generation {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    if inv.recipient == sender {
        return Err(ApiError::bad_request());
    }
    check_invite(tx, sender, vault, inv, v.current_generation, now)?;
    let hidden = is_blocked(tx, inv.recipient, sender)?
        || count(
            tx,
            "SELECT COUNT(*) FROM share_invites WHERE recipient = ?1 AND state = 0 AND hidden = 0 \
             AND expires_at > ?2",
            params![inv.recipient.as_slice(), int(now)],
        )? >= MAX_INCOMING_INVITES;
    let expires_at = now.saturating_add(INVITE_TTL_MS);
    let inserted = tx.execute(
        "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation, seal, \
         created_at, expires_at, state, hidden) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10)",
        params![
            inv.invite_id.as_slice(),
            vault.as_slice(),
            sender.as_slice(),
            inv.recipient.as_slice(),
            inv.role,
            inv.generation,
            inv.seal,
            int(now),
            int(expires_at),
            i64::from(hidden)
        ],
    );
    match inserted {
        Ok(_) => {}
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            return Err(ApiError::conflict(codes::INVITE_EXISTS));
        }
        Err(e) => return Err(e.into()),
    }
    audit::record(tx, sender, now, audit::Kind::InviteSent, None)?;
    let real = !hidden && account_exists(tx, inv.recipient)?;
    if real {
        bump_inbox(tx, inv.recipient)?;
    }
    Ok(Sent {
        expires_at,
        notify_recipient: real,
    })
}

/// Whether an account with this ID exists.
pub fn account_exists(conn: &Connection, id: &[u8; 16]) -> Result<bool, ApiError> {
    Ok(count(
        conn,
        "SELECT COUNT(*) FROM accounts WHERE account_id = ?1",
        [id.as_slice()],
    )? > 0)
}

/// An invite addressed to the caller.
pub struct Incoming {
    pub invite_id: [u8; 16],
    pub vault_id: [u8; 16],
    pub sender: [u8; 16],
    pub sender_email: String,
    pub role: u8,
    pub generation: u32,
    pub seal: Vec<u8>,
    pub expires_at: u64,
}

/// An invite the caller sent.
pub struct Outgoing {
    pub invite_id: [u8; 16],
    pub vault_id: [u8; 16],
    pub recipient: [u8; 16],
    pub role: u8,
    /// 0 open, 1 accepted, 2 declined, 3 cancelled, 4 expired.
    pub state: u8,
    pub delivered: bool,
    pub expires_at: u64,
}

fn small(r: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<u8> {
    let v: i64 = r.get(idx)?;
    u8::try_from(v).map_err(|_| rusqlite::Error::InvalidQuery)
}

/// The caller's inbox and outbox. Listing marks the incoming invites
/// delivered (a seal that was delivered may be known to the recipient).
pub fn list_invites(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    now: u64,
) -> Result<(Vec<Incoming>, Vec<Outgoing>), ApiError> {
    let mut stmt = tx.prepare(
        "SELECT i.invite_id, i.vault_id, i.sender, a.email, i.role, i.generation, i.seal, i.expires_at \
         FROM share_invites i JOIN accounts a ON a.account_id = i.sender \
         WHERE i.recipient = ?1 AND i.state = 0 AND i.hidden = 0 AND i.expires_at > ?2 \
         ORDER BY i.created_at, i.invite_id",
    )?;
    let incoming = stmt
        .query_map(params![account.as_slice(), int(now)], |r| {
            Ok(Incoming {
                invite_id: id16(r, 0)?,
                vault_id: id16(r, 1)?,
                sender: id16(r, 2)?,
                sender_email: r.get(3)?,
                role: small(r, 4)?,
                generation: u32::try_from(r.get::<_, i64>(5)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                seal: r.get(6)?,
                expires_at: u64_col(r, 7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    tx.execute(
        "UPDATE share_invites SET delivered_at = ?2 WHERE recipient = ?1 AND state = 0 \
         AND hidden = 0 AND delivered_at IS NULL AND expires_at > ?2",
        params![account.as_slice(), int(now)],
    )?;
    let mut stmt = tx.prepare(
        "SELECT invite_id, vault_id, recipient, role, \
                CASE WHEN state = 0 AND expires_at <= ?2 THEN 4 ELSE state END, \
                delivered_at IS NOT NULL, expires_at \
         FROM share_invites WHERE sender = ?1 AND expires_at > ?3 ORDER BY created_at, invite_id",
    )?;
    let outgoing = stmt
        .query_map(
            params![
                account.as_slice(),
                int(now),
                int(now.saturating_sub(OUTGOING_KEEP_MS))
            ],
            |r| {
                Ok(Outgoing {
                    invite_id: id16(r, 0)?,
                    vault_id: id16(r, 1)?,
                    recipient: id16(r, 2)?,
                    role: small(r, 3)?,
                    state: small(r, 4)?,
                    delivered: r.get::<_, i64>(5)? != 0,
                    expires_at: u64_col(r, 6)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((incoming, outgoing))
}

struct InviteRow {
    vault: [u8; 16],
    sender: [u8; 16],
    role: u8,
    generation: u32,
    expires_at: u64,
}

/// The open, visible invite `id` addressed to `account`.
fn open_invite(
    conn: &Connection,
    account: &[u8; 16],
    id: &[u8; 16],
) -> Result<InviteRow, ApiError> {
    conn.query_row(
        "SELECT vault_id, sender, role, generation, expires_at FROM share_invites \
         WHERE invite_id = ?1 AND recipient = ?2 AND state = 0 AND hidden = 0",
        params![id.as_slice(), account.as_slice()],
        |r| {
            Ok(InviteRow {
                vault: id16(r, 0)?,
                sender: id16(r, 1)?,
                role: small(r, 2)?,
                generation: u32::try_from(r.get::<_, i64>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                expires_at: u64_col(r, 4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(ApiError::not_found)
}

fn expired() -> ApiError {
    ApiError::new(StatusCode::GONE, codes::INVITE_EXPIRED)
}

/// The metadata and the current roster ciphertexts of the vault of a
/// pending invite (the recipient opens them with the sealed key).
pub fn invite_vault(
    conn: &Connection,
    account: &[u8; 16],
    id: &[u8; 16],
    now: u64,
) -> Result<(Vec<u8>, Option<Vec<u8>>), ApiError> {
    let inv = open_invite(conn, account, id)?;
    if inv.expires_at <= now {
        return Err(expired());
    }
    let meta: Vec<u8> = conn
        .query_row(
            "SELECT ciphertext FROM vault_meta WHERE vault_id = ?1",
            [inv.vault.as_slice()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(ApiError::not_found)?;
    Ok((meta, current_roster(conn, &inv.vault)?.map(|(_, ct, _)| ct)))
}

/// The roster of the highest generation: `(generation, ciphertext, seq)`.
pub fn current_roster(
    conn: &Connection,
    vault: &[u8; 16],
) -> Result<Option<(u32, Vec<u8>, u64)>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT key_generation, ciphertext, server_seq FROM vault_roster WHERE vault_id = ?1 \
             ORDER BY key_generation DESC LIMIT 1",
            [vault.as_slice()],
            |r| {
                Ok((
                    u32::try_from(r.get::<_, i64>(0)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    r.get(1)?,
                    u64_col(r, 2)?,
                ))
            },
        )
        .optional()?)
}

/// What an accepted invite changed.
pub struct Accepted {
    pub vault: [u8; 16],
    pub generation: u32,
    pub role: u8,
}

/// Accepts the invite: membership (origin `invite`), the wrapped key row
/// and a head bump.
pub fn accept_invite(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    id: &[u8; 16],
    wrapped: &[u8; 74],
    device: &[u8; 16],
    now: u64,
) -> Result<Accepted, ApiError> {
    let inv = open_invite(tx, account, id)?;
    if inv.expires_at <= now {
        return Err(expired());
    }
    let v = vaults::access_role(tx, &inv.sender, &inv.vault, role::MANAGE)
        // The sender lost the right to invite: the invite is void.
        .map_err(|_| ApiError::not_found())?;
    if inv.role < v.role {
        return Err(ApiError::not_found());
    }
    if inv.generation != v.current_generation {
        return Err(generation_error(codes::INVITE_STALE, v.current_generation));
    }
    let members = count(
        tx,
        "SELECT COUNT(*) FROM memberships WHERE vault_id = ?1",
        [inv.vault.as_slice()],
    )?;
    if members >= MAX_MEMBERS {
        return Err(ApiError::conflict(codes::MEMBERS_LIMIT));
    }
    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO memberships (account_id, vault_id, role, origin, added_at) VALUES (?1, ?2, ?3, 1, ?4) \
         ON CONFLICT (account_id, vault_id) DO UPDATE SET role = excluded.role, origin = 1",
        params![
            account.as_slice(),
            inv.vault.as_slice(),
            inv.role,
            int(now)
        ],
    )?;
    tx.execute(
        "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, server_seq, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT DO NOTHING",
        params![
            inv.vault.as_slice(),
            account.as_slice(),
            inv.generation,
            wrapped.as_slice(),
            device.as_slice(),
            int(seq),
            int(now)
        ],
    )?;
    tx.execute(
        "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
        params![int(seq), inv.vault.as_slice()],
    )?;
    tx.execute(
        "UPDATE share_invites SET state = 1 WHERE invite_id = ?1",
        [id.as_slice()],
    )?;
    tx.execute(
        "DELETE FROM vault_removals WHERE vault_id = ?1 AND account_id = ?2",
        params![inv.vault.as_slice(), account.as_slice()],
    )?;
    audit::record(tx, account, now, audit::Kind::InviteAccepted, Some(device))?;
    bump_inbox(tx, &inv.sender)?;
    Ok(Accepted {
        vault: inv.vault,
        generation: inv.generation,
        role: inv.role,
    })
}

/// Declines an open invite, optionally blocking the sender. Returns the
/// sender.
pub fn decline_invite(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    id: &[u8; 16],
    block_sender: bool,
) -> Result<[u8; 16], ApiError> {
    let inv = open_invite(tx, account, id)?;
    tx.execute(
        "UPDATE share_invites SET state = 2 WHERE invite_id = ?1",
        [id.as_slice()],
    )?;
    if block_sender {
        block(tx, account, &inv.sender)?;
    }
    bump_inbox(tx, &inv.sender)?;
    Ok(inv.sender)
}

/// Cancels an open invite; a finished one is removed from the list. By the
/// sender or a manager of the vault. Returns the recipient.
pub fn cancel_invite(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    id: &[u8; 16],
) -> Result<[u8; 16], ApiError> {
    let row = tx
        .query_row(
            "SELECT vault_id, sender, recipient, state FROM share_invites WHERE invite_id = ?1",
            [id.as_slice()],
            |r| Ok((id16(r, 0)?, id16(r, 1)?, id16(r, 2)?, small(r, 3)?)),
        )
        .optional()?;
    let (vault, sender, recipient, state) = row.ok_or_else(ApiError::not_found)?;
    let allowed = sender == *account
        || tx
            .query_row(
                "SELECT role FROM memberships WHERE vault_id = ?1 AND account_id = ?2",
                params![vault.as_slice(), account.as_slice()],
                |r| small(r, 0),
            )
            .optional()?
            .is_some_and(|r| r <= role::MANAGE);
    if !allowed {
        return Err(ApiError::not_found());
    }
    match state {
        0 => {
            tx.execute(
                "UPDATE share_invites SET state = 3 WHERE invite_id = ?1",
                [id.as_slice()],
            )?;
            bump_inbox(tx, &recipient)?;
        }
        1 => {}
        _ => {
            tx.execute(
                "DELETE FROM share_invites WHERE invite_id = ?1",
                [id.as_slice()],
            )?;
        }
    }
    Ok(recipient)
}

// ---- members and roles --------------------------------------------------

/// A member of a vault.
pub struct Member {
    pub account_id: [u8; 16],
    pub email: String,
    pub role: u8,
    pub origin: u8,
    pub added_at: u64,
}

/// An open invite of a vault.
pub struct OpenInvite {
    pub invite_id: [u8; 16],
    pub recipient: [u8; 16],
    pub role: u8,
    pub delivered: bool,
}

/// The members and the open invites of a vault the caller belongs to.
pub fn members(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
    now: u64,
) -> Result<(Vec<Member>, Vec<OpenInvite>), ApiError> {
    vaults::access(conn, account, vault)?;
    let mut stmt = conn.prepare(
        "SELECT m.account_id, a.email, m.role, m.origin, m.added_at FROM memberships m \
         JOIN accounts a ON a.account_id = m.account_id WHERE m.vault_id = ?1 \
         ORDER BY m.role, m.added_at, m.account_id",
    )?;
    let members = stmt
        .query_map([vault.as_slice()], |r| {
            Ok(Member {
                account_id: id16(r, 0)?,
                email: r.get(1)?,
                role: small(r, 2)?,
                origin: small(r, 3)?,
                added_at: u64_col(r, 4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut stmt = conn.prepare(
        "SELECT invite_id, recipient, role, delivered_at IS NOT NULL FROM share_invites \
         WHERE vault_id = ?1 AND state = 0 AND expires_at > ?2 ORDER BY created_at, invite_id",
    )?;
    let invites = stmt
        .query_map(params![vault.as_slice(), int(now)], |r| {
            Ok(OpenInvite {
                invite_id: id16(r, 0)?,
                recipient: id16(r, 1)?,
                role: small(r, 2)?,
                delivered: r.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((members, invites))
}

/// Changes a member's role (never to or from `owner`, never above the
/// caller's own role).
pub fn set_role(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    target: &[u8; 16],
    new_role: u8,
) -> Result<(), ApiError> {
    let v = vaults::access_role(tx, account, vault, role::MANAGE)?;
    if new_role <= role::OWNER || new_role > role::READ || new_role < v.role {
        return Err(vaults::role_forbidden());
    }
    let current: u8 = tx
        .query_row(
            "SELECT role FROM memberships WHERE vault_id = ?1 AND account_id = ?2",
            params![vault.as_slice(), target.as_slice()],
            |r| small(r, 0),
        )
        .optional()?
        .ok_or_else(ApiError::not_found)?;
    if current == role::OWNER {
        return Err(vaults::role_forbidden());
    }
    tx.execute(
        "UPDATE memberships SET role = ?1 WHERE vault_id = ?2 AND account_id = ?3",
        params![new_role, vault.as_slice(), target.as_slice()],
    )?;
    Ok(())
}

// ---- roster -------------------------------------------------------------

/// `conflict` with the stored sequence.
pub fn conflict(current_seq: u64) -> ApiError {
    conflict_with(codes::CONFLICT, "currentSeq", current_seq)
}

/// Replaces the roster of the current generation if `base_seq` is its
/// stored sequence. Returns the new sequence.
pub fn put_roster(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    generation: u32,
    ciphertext: &[u8],
    base_seq: Option<u64>,
) -> Result<u64, ApiError> {
    let v = vaults::access_role(tx, account, vault, role::MANAGE)?;
    if generation != v.current_generation {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    let stored: Option<u64> = tx
        .query_row(
            "SELECT server_seq FROM vault_roster WHERE vault_id = ?1 AND key_generation = ?2",
            params![vault.as_slice(), generation],
            |r| u64_col(r, 0),
        )
        .optional()?;
    let ok = match (base_seq, stored) {
        (None, None) => true,
        (Some(b), Some(s)) => b == s,
        _ => false,
    };
    if !ok {
        return Err(conflict(stored.unwrap_or(0)));
    }
    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO vault_roster (vault_id, key_generation, ciphertext, server_seq) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (vault_id, key_generation) DO UPDATE SET ciphertext = excluded.ciphertext, \
         server_seq = excluded.server_seq",
        params![vault.as_slice(), generation, ciphertext, int(seq)],
    )?;
    tx.execute(
        "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
        params![int(seq), vault.as_slice()],
    )?;
    Ok(seq)
}

// ---- rotation -----------------------------------------------------------

/// A shared-vault rotation request.
pub struct Rotation<'a> {
    pub generation: u32,
    pub wrapped: &'a [u8; 74],
    pub device: &'a [u8; 16],
    pub expected_head: u64,
    pub roster: &'a [u8],
    /// The generation in the roster ciphertext header.
    pub roster_generation: u32,
    pub seals: &'a [([u8; 16], Vec<u8>)],
    pub invite_seals: &'a [([u8; 16], Vec<u8>)],
    pub revoke: &'a [[u8; 16]],
}

fn distinct(
    ids: impl Iterator<Item = [u8; 16]>,
    len: usize,
) -> Result<BTreeSet<[u8; 16]>, ApiError> {
    let set: BTreeSet<[u8; 16]> = ids.collect();
    if set.len() == len {
        Ok(set)
    } else {
        Err(ApiError::bad_request())
    }
}

/// The accounts a rotation touched, for the poll notifications.
pub struct Rotated {
    pub seq: u64,
    pub accounts: Vec<[u8; 16]>,
}

/// A set of account or invite IDs.
type IdSet = BTreeSet<[u8; 16]>;

/// The request must seal to or revoke every member but the owner, and cover
/// every open invite, exactly. Returns the members and the revoked set.
fn check_covered(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    r: &Rotation<'_>,
    now: u64,
) -> Result<(IdSet, IdSet), ApiError> {
    let revoke = distinct(r.revoke.iter().copied(), r.revoke.len())?;
    let sealed = distinct(r.seals.iter().map(|(a, _)| *a), r.seals.len())?;
    let invite_ids = distinct(r.invite_seals.iter().map(|(a, _)| *a), r.invite_seals.len())?;
    if revoke.contains(account) {
        return Err(vaults::role_forbidden());
    }
    let members: BTreeSet<[u8; 16]> = vaults::member_ids(tx, vault)?
        .into_iter()
        .filter(|m| m != account)
        .collect();
    let open: BTreeSet<[u8; 16]> = pending_invite_ids(tx, vault, now)?.into_iter().collect();
    let covered: BTreeSet<[u8; 16]> = sealed.union(&revoke).copied().collect();
    if !sealed.is_disjoint(&revoke) || covered != members || invite_ids != open {
        return Err(vaults::members_mismatch(tx, vault, account, now)?);
    }
    Ok((members, revoke))
}

/// Rotates a shared vault to `generation` atomically (CRYPTO_SPEC §8.3):
/// the owner's wrapped key, a seal for every other member (or a revocation),
/// refreshed seals for every open invite, and the roster of the new
/// generation. Only the owner rotates. A mismatch between the request and
/// the member and invite sets is `members_mismatch`.
pub fn rotate(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    r: &Rotation<'_>,
    now: u64,
) -> Result<Rotated, ApiError> {
    let v = vaults::access_role(tx, account, vault, role::OWNER)?;
    let taken = count(
        tx,
        "SELECT COUNT(*) FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2 AND generation = ?3",
        params![vault.as_slice(), account.as_slice(), r.generation],
    )?;
    if taken > 0 {
        return Err(generation_error(
            codes::GENERATION_TAKEN,
            v.current_generation,
        ));
    }
    if r.generation != v.current_generation.saturating_add(1) {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    if r.roster_generation != r.generation {
        return Err(ApiError::bad_request());
    }
    if r.expected_head != v.head_seq {
        return Err(
            ApiError::conflict(codes::HEAD_MOVED).with_detail(&serde_json::json!({
                "headSeq": v.head_seq
            })),
        );
    }
    let (members, revoke) = check_covered(tx, account, vault, r, now)?;

    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, server_seq, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            vault.as_slice(),
            account.as_slice(),
            r.generation,
            r.wrapped.as_slice(),
            r.device.as_slice(),
            int(seq),
            int(now)
        ],
    )?;
    for (recipient, seal) in r.seals {
        tx.execute(
            "INSERT OR REPLACE INTO key_shares (vault_id, recipient, generation, sender, seal, server_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                vault.as_slice(),
                recipient.as_slice(),
                r.generation,
                account.as_slice(),
                seal,
                int(seq)
            ],
        )?;
    }
    for (invite, seal) in r.invite_seals {
        tx.execute(
            "UPDATE share_invites SET seal = ?1, generation = ?2 WHERE invite_id = ?3 AND vault_id = ?4 AND state = 0",
            params![seal, r.generation, invite.as_slice(), vault.as_slice()],
        )?;
    }
    tx.execute(
        "INSERT INTO vault_roster (vault_id, key_generation, ciphertext, server_seq) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (vault_id, key_generation) DO UPDATE SET ciphertext = excluded.ciphertext, \
         server_seq = excluded.server_seq",
        params![vault.as_slice(), r.generation, r.roster, int(seq)],
    )?;
    for gone in &revoke {
        remove_member(tx, vault, gone, r.generation)?;
        audit::record(tx, account, now, audit::Kind::MemberRemoved, Some(r.device))?;
    }
    tx.execute(
        "UPDATE vaults SET current_generation = ?1, head_seq = ?2 WHERE vault_id = ?3",
        params![r.generation, int(seq), vault.as_slice()],
    )?;
    let mut accounts: Vec<[u8; 16]> = members.into_iter().collect();
    accounts.push(*account);
    Ok(Rotated { seq, accounts })
}

/// Removes `member` from the vault: membership, keys, shares and open
/// invites; a re-invite stays blocked until the vault passes `generation`.
fn remove_member(
    tx: &Transaction<'_>,
    vault: &[u8; 16],
    member: &[u8; 16],
    generation: u32,
) -> Result<(), ApiError> {
    let p = params![vault.as_slice(), member.as_slice()];
    tx.execute(
        "DELETE FROM memberships WHERE vault_id = ?1 AND account_id = ?2",
        p,
    )?;
    tx.execute(
        "DELETE FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2",
        p,
    )?;
    tx.execute(
        "DELETE FROM key_shares WHERE vault_id = ?1 AND recipient = ?2",
        p,
    )?;
    tx.execute(
        "UPDATE share_invites SET state = 3 WHERE vault_id = ?1 AND recipient = ?2 AND state = 0",
        p,
    )?;
    tx.execute(
        "INSERT INTO vault_removals (vault_id, account_id, generation) VALUES (?1, ?2, ?3) \
         ON CONFLICT (vault_id, account_id) DO UPDATE SET generation = excluded.generation",
        params![vault.as_slice(), member.as_slice(), generation],
    )?;
    Ok(())
}

/// A member leaves. The owner can't. The leaver's keys and shares go; the
/// owner's next rotation revokes the key they held.
pub fn leave(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    now: u64,
) -> Result<Vec<[u8; 16]>, ApiError> {
    let v = vaults::access(tx, account, vault)?;
    if v.role == role::OWNER {
        return Err(ApiError::conflict(codes::OWNER_CANNOT_LEAVE));
    }
    let others: Vec<[u8; 16]> = vaults::member_ids(tx, vault)?;
    // The revocation rotation creates the next generation; a re-invite
    // waits until the vault is past it.
    remove_member(tx, vault, account, v.current_generation.saturating_add(1))?;
    audit::record(tx, account, now, audit::Kind::MemberRemoved, None)?;
    Ok(others)
}

/// Claims the key of the current generation from a share (or, for an
/// emergency membership, from a released emergency seal): stores the
/// caller's wrapped key and deletes the shares up to it.
pub fn claim(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    generation: u32,
    wrapped: &[u8; 74],
    device: &[u8; 16],
    now: u64,
) -> Result<u64, ApiError> {
    let v = vaults::access(tx, account, vault)?;
    if generation != v.current_generation {
        return Err(vaults::generation_mismatch(v.current_generation));
    }
    let share = count(
        tx,
        "SELECT COUNT(*) FROM key_shares WHERE vault_id = ?1 AND recipient = ?2 AND generation = ?3",
        params![vault.as_slice(), account.as_slice(), generation],
    )? > 0;
    let emergency = count(
        tx,
        "SELECT COUNT(*) FROM memberships m \
         JOIN emergency_seals s ON s.vault_id = m.vault_id AND s.grantee = m.account_id \
         JOIN emergency_grants g ON g.grantor = s.grantor AND g.grantee = s.grantee \
         WHERE m.vault_id = ?1 AND m.account_id = ?2 AND m.origin = 2 AND s.generation = ?3 AND g.state = 3",
        params![vault.as_slice(), account.as_slice(), generation],
    )? > 0;
    if !share && !emergency {
        return Err(ApiError::new(StatusCode::FORBIDDEN, codes::NO_SHARE));
    }
    let exists = count(
        tx,
        "SELECT COUNT(*) FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2 AND generation = ?3",
        params![vault.as_slice(), account.as_slice(), generation],
    )?;
    if exists > 0 {
        return Err(generation_error(
            codes::GENERATION_TAKEN,
            v.current_generation,
        ));
    }
    let seq = v.head_seq.saturating_add(1);
    tx.execute(
        "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, server_seq, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            vault.as_slice(),
            account.as_slice(),
            generation,
            wrapped.as_slice(),
            device.as_slice(),
            int(seq),
            int(now)
        ],
    )?;
    tx.execute(
        "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
        params![int(seq), vault.as_slice()],
    )?;
    tx.execute(
        "DELETE FROM key_shares WHERE vault_id = ?1 AND recipient = ?2 AND generation <= ?3",
        params![vault.as_slice(), account.as_slice(), generation],
    )?;
    Ok(seq)
}

// ---- the sharing part of the change feed --------------------------------

/// A share addressed to the caller.
pub struct Share {
    pub generation: u32,
    pub sender: [u8; 16],
    pub seal: Vec<u8>,
    pub server_seq: u64,
}

/// A roster of one generation.
pub struct RosterRow {
    pub generation: u32,
    pub ciphertext: Vec<u8>,
    pub server_seq: u64,
}

/// What a sharing-aware change feed adds: the caller's role, every
/// unclaimed share addressed to the caller, and the rosters newer than
/// `since`.
pub struct Extras {
    pub role: u8,
    pub shares: Vec<Share>,
    pub rosters: Vec<RosterRow>,
}

/// Reads the sharing part of a page.
pub fn changes_extras(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
    since: u64,
) -> Result<Extras, ApiError> {
    let v = vaults::access(conn, account, vault)?;
    let mut stmt = conn.prepare(
        "SELECT generation, sender, seal, server_seq FROM key_shares \
         WHERE vault_id = ?1 AND recipient = ?2 ORDER BY generation",
    )?;
    let shares = stmt
        .query_map(params![vault.as_slice(), account.as_slice()], |r| {
            Ok(Share {
                generation: u32::try_from(r.get::<_, i64>(0)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                sender: id16(r, 1)?,
                seal: r.get(2)?,
                server_seq: u64_col(r, 3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut stmt = conn.prepare(
        "SELECT key_generation, ciphertext, server_seq FROM vault_roster \
         WHERE vault_id = ?1 AND server_seq > ?2 ORDER BY key_generation",
    )?;
    let rosters = stmt
        .query_map(params![vault.as_slice(), int(since)], |r| {
            Ok(RosterRow {
                generation: u32::try_from(r.get::<_, i64>(0)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                ciphertext: r.get(1)?,
                server_seq: u64_col(r, 2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Extras {
        role: v.role,
        shares,
        rosters,
    })
}

/// Drops finished invites a long time after they ended.
pub fn purge_invites(conn: &Connection, before: u64) -> Result<usize, ApiError> {
    Ok(conn.execute(
        "DELETE FROM share_invites WHERE expires_at < ?1",
        [int(before)],
    )?)
}

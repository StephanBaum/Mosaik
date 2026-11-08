//! The key directory (M6b spec §5.4): who may look up whose identity key.

use rusqlite::{Connection, OptionalExtension, params};

use super::{emergency, id16};
use crate::error::ApiError;

/// The published identity key of an account.
pub fn ipk_of(conn: &Connection, id: &[u8; 16]) -> Result<Option<[u8; 32]>, ApiError> {
    let ipk: Option<Option<Vec<u8>>> = conn
        .query_row(
            "SELECT ipk FROM accounts WHERE account_id = ?1",
            [id.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(ipk.flatten().and_then(|v| <[u8; 32]>::try_from(v).ok()))
}

/// An account's ID and its published identity key, if any.
pub type Found = ([u8; 16], Option<[u8; 32]>);

/// The ID and identity key of the account with this (normalized) email.
pub fn by_email(conn: &Connection, email: &str) -> Result<Option<Found>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT account_id, ipk FROM accounts WHERE email = ?1",
            [email],
            |r| {
                let ipk: Option<Vec<u8>> = r.get(1)?;
                Ok((id16(r, 0)?, ipk.and_then(|v| <[u8; 32]>::try_from(v).ok())))
            },
        )
        .optional()?)
}

/// Whether `caller` may look `target` up by ID: they share a vault, the
/// target invited the caller, or they are on one emergency grant. An invite
/// to the target is never a reason, whoever sent it: the inviter has the key
/// from the email lookup (the roster holds the address), and an answer here
/// would tell a real recipient from a fake one (M6b review L3).
pub fn may_look_up(
    conn: &Connection,
    caller: &[u8; 16],
    target: &[u8; 16],
) -> Result<bool, ApiError> {
    let n: i64 = conn.query_row(
        "SELECT \
           EXISTS (SELECT 1 FROM memberships a JOIN memberships b ON a.vault_id = b.vault_id \
                   WHERE a.account_id = ?1 AND b.account_id = ?2) \
        OR EXISTS (SELECT 1 FROM share_invites i WHERE i.recipient = ?1 AND i.sender = ?2 \
                   AND i.state = 0 AND i.hidden = 0)",
        params![caller.as_slice(), target.as_slice()],
        |r| r.get(0),
    )?;
    Ok(n != 0 || emergency::related(conn, caller, target)?)
}

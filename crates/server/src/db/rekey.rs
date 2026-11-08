//! The atomic account re-key (M7 spec §4, CRYPTO_SPEC §6.6): one transaction
//! replaces the OPAQUE record, the wrapped account key, the identity, the key
//! rows of every vault and the seals that were made under the old keys. The
//! server stores and checks the shape of what the client uploads; it never
//! opens a wrapping or a seal.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::sharing::{self as dbs, bump_inbox};
use super::{accounts, audit, id16, int, sessions, u64_col, vaults};
use crate::api::codes;
use crate::error::ApiError;
use mosaic_sync::wire::json::AccountVersionDetail;

/// A set of IDs.
type IdSet = BTreeSet<[u8; 16]>;
/// `(grantee, vault)` of an emergency seal.
type SealKey = ([u8; 16], [u8; 16]);

/// A vault the account owns, as the request rotates it.
pub struct OwnedVault<'a> {
    pub vault: [u8; 16],
    pub generation: u32,
    pub wrapped: &'a [u8; 74],
    pub expected_head: u64,
    /// The roster ciphertext and the generation in its header.
    pub roster: Option<(&'a [u8], u32)>,
    pub seals: &'a [([u8; 16], Vec<u8>)],
    pub invite_seals: &'a [([u8; 16], Vec<u8>)],
}

/// The re-wrapped current key of a vault shared to the account.
pub struct MemberRewrap<'a> {
    pub vault: [u8; 16],
    pub generation: u32,
    pub wrapped: &'a [u8; 74],
}

/// A fresh emergency seal of a grant the account gave.
pub struct EmergencySeal {
    pub grantee: [u8; 16],
    pub vault: [u8; 16],
    pub generation: u32,
    pub seal: Vec<u8>,
}

/// The new identity key pair.
pub struct Identity<'a> {
    pub ipk: &'a [u8; 32],
    pub wrapped_isk: &'a [u8; 74],
}

/// What `rekey/finish` carries.
pub struct Rekey<'a> {
    pub record: &'a [u8],
    pub wrapped_ak: &'a [u8; 74],
    pub salt: &'a [u8; 16],
    pub profile: u8,
    pub epoch: u32,
    pub identity: Option<Identity<'a>>,
    pub vaults: &'a [OwnedVault<'a>],
    pub member_rewraps: &'a [MemberRewrap<'a>],
    pub emergency_seals: &'a [EmergencySeal],
    pub device: &'a [u8; 16],
    /// The account version at the session's login.
    pub session_version: u64,
    /// The changing session, which stays.
    pub keep_session: &'a [u8; 32],
}

/// What a re-key changed.
pub struct Rekeyed {
    pub account_version: u64,
    pub epoch: u32,
    /// The accounts whose poll token must move.
    pub notify: IdSet,
}

fn version_mismatch(current: u64) -> ApiError {
    ApiError::conflict(codes::ACCOUNT_VERSION_MISMATCH).with_detail(&AccountVersionDetail {
        account_version: current,
    })
}

fn epoch_mismatch(current: u32) -> ApiError {
    ApiError::conflict(codes::EPOCH_MISMATCH)
        .with_detail(&serde_json::json!({ "currentEpoch": current }))
}

fn mismatch(set: &str) -> ApiError {
    ApiError::conflict(codes::REKEY_MISMATCH).with_detail(&serde_json::json!({ "set": set }))
}

/// The stored epoch, profile and version of the account.
struct Stored {
    epoch: u32,
    profile: u8,
    version: u64,
}

fn stored(conn: &Connection, account: &[u8; 16]) -> Result<Stored, ApiError> {
    conn.query_row(
        "SELECT ak_epoch, profile, account_version FROM accounts WHERE account_id = ?1",
        [account.as_slice()],
        |r| {
            Ok(Stored {
                epoch: u32::try_from(r.get::<_, i64>(0)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                profile: r.get(1)?,
                version: u64_col(r, 2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(ApiError::unauthorized)
}

/// The account's epoch.
pub fn epoch(conn: &Connection, account: &[u8; 16]) -> Result<u32, ApiError> {
    Ok(stored(conn, account)?.epoch)
}

/// `rekey/start`: the session's account version is the stored one and
/// `target` is the stored epoch plus one.
pub fn check_start(
    conn: &Connection,
    account: &[u8; 16],
    session_version: u64,
    target: u32,
) -> Result<(), ApiError> {
    let s = stored(conn, account)?;
    if s.version != session_version {
        return Err(version_mismatch(s.version));
    }
    if s.epoch.checked_add(1) != Some(target) {
        return Err(epoch_mismatch(s.epoch));
    }
    Ok(())
}

/// The vaults the account owns: `vault -> (generation, head)`.
fn owned(
    conn: &Connection,
    account: &[u8; 16],
) -> Result<BTreeMap<[u8; 16], (u32, u64)>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT vault_id, current_generation, head_seq FROM vaults WHERE owner_account_id = ?1",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| {
        Ok((
            id16(r, 0)?,
            (
                u32::try_from(r.get::<_, i64>(1)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                u64_col(r, 2)?,
            ),
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A vault shared to the account.
struct Member {
    vault: [u8; 16],
    generation: u32,
    owner: [u8; 16],
    /// The account holds a key row of the current generation.
    holds: bool,
}

/// The vaults the account is a member of but does not own.
fn memberships(conn: &Connection, account: &[u8; 16]) -> Result<Vec<Member>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT v.vault_id, v.current_generation, v.owner_account_id, \
                EXISTS (SELECT 1 FROM vault_keys k WHERE k.vault_id = v.vault_id \
                        AND k.account_id = m.account_id AND k.generation = v.current_generation) \
         FROM memberships m JOIN vaults v ON v.vault_id = m.vault_id \
         WHERE m.account_id = ?1 AND v.owner_account_id <> ?1 ORDER BY v.vault_id",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| {
        Ok(Member {
            vault: id16(r, 0)?,
            generation: u32::try_from(r.get::<_, i64>(1)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            owner: id16(r, 2)?,
            holds: r.get::<_, i64>(3)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The `(grantee, vault)` pairs of the seals the account gave.
fn given_seals(conn: &Connection, account: &[u8; 16]) -> Result<BTreeSet<SealKey>, ApiError> {
    let mut stmt =
        conn.prepare("SELECT grantee, vault_id FROM emergency_seals WHERE grantor = ?1")?;
    let rows = stmt.query_map([account.as_slice()], |r| Ok((id16(r, 0)?, id16(r, 1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn ids(conn: &Connection, sql: &str, account: &[u8; 16]) -> Result<Vec<[u8; 16]>, ApiError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([account.as_slice()], |r| id16(r, 0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Checks the request against what the server holds.
fn check(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    r: &Rekey<'_>,
    s: &Stored,
    members: &[Member],
    now: u64,
) -> Result<(), ApiError> {
    if s.version != r.session_version {
        return Err(version_mismatch(s.version));
    }
    if s.epoch.checked_add(1) != Some(r.epoch) {
        return Err(epoch_mismatch(s.epoch));
    }
    if r.profile < s.profile {
        return Err(ApiError::conflict(codes::PROFILE_DOWNGRADE));
    }

    // The identity is replaced here and nowhere else. An account that has
    // one sends a different one: its wrapping is under the old AK.
    match (dbs::identity(tx, account)?, &r.identity) {
        (Some(_), None) => return Err(ApiError::bad_request()),
        (Some((ipk, _)), Some(new)) if ipk.as_slice() == new.ipk.as_slice() => {
            return Err(ApiError::bad_request());
        }
        (_, Some(new)) if *new.ipk == [0u8; 32] => return Err(ApiError::bad_request()),
        _ => {}
    }

    // Owned vaults: exactly the set, each one generation up, head unchanged.
    let owned = owned(tx, account)?;
    let requested: IdSet = r.vaults.iter().map(|v| v.vault).collect();
    if requested.len() != r.vaults.len() {
        return Err(ApiError::bad_request());
    }
    if !owned.keys().eq(requested.iter()) {
        return Err(mismatch("vaults"));
    }
    for v in r.vaults {
        let &(current, head) = owned.get(&v.vault).ok_or_else(ApiError::internal)?;
        let id = super::uuid_text(&v.vault);
        if v.generation <= current {
            return Err(ApiError::conflict(codes::GENERATION_TAKEN).with_detail(
                &serde_json::json!({
                    "currentGeneration": current, "vaultId": id,
                }),
            ));
        }
        if current.checked_add(1) != Some(v.generation) {
            return Err(vaults::generation_mismatch(current));
        }
        if v.expected_head != head {
            return Err(
                ApiError::conflict(codes::HEAD_MOVED).with_detail(&serde_json::json!({
                    "headSeq": head, "vaultId": id,
                })),
            );
        }
        if v.roster.is_some_and(|(_, g)| g != v.generation) {
            return Err(ApiError::bad_request());
        }
        if vaults::is_shared(tx, &v.vault, now)? {
            if v.roster.is_none() {
                return Err(ApiError::bad_request());
            }
        } else if !v.seals.is_empty() || !v.invite_seals.is_empty() {
            return Err(vaults::members_mismatch(tx, &v.vault, account, now)?);
        }
    }

    // Shared to the account: exactly the vaults whose current key it holds.
    let expected: IdSet = members
        .iter()
        .filter(|m| m.holds)
        .map(|m| m.vault)
        .collect();
    let rewraps: IdSet = r.member_rewraps.iter().map(|m| m.vault).collect();
    if rewraps.len() != r.member_rewraps.len() || rewraps != expected {
        return Err(mismatch("memberRewraps"));
    }
    let current_of: BTreeMap<[u8; 16], u32> =
        members.iter().map(|m| (m.vault, m.generation)).collect();
    if r.member_rewraps
        .iter()
        .any(|m| current_of.get(&m.vault) != Some(&m.generation))
    {
        return Err(mismatch("memberRewraps"));
    }

    // Emergency seals: a fresh seal for each seal the account gave, at the
    // vault's new generation.
    let target: BTreeMap<[u8; 16], u32> =
        r.vaults.iter().map(|v| (v.vault, v.generation)).collect();
    let sent: BTreeSet<SealKey> = r
        .emergency_seals
        .iter()
        .map(|e| (e.grantee, e.vault))
        .collect();
    if sent.len() != r.emergency_seals.len()
        || sent != given_seals(tx, account)?
        || r.emergency_seals
            .iter()
            .any(|e| target.get(&e.vault) != Some(&e.generation))
    {
        return Err(mismatch("emergencySeals"));
    }
    Ok(())
}

/// Rotates one owned vault to its next generation and deletes the account's
/// older key rows of it. Returns the accounts to wake.
fn rotate_owned(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    r: &Rekey<'_>,
    v: &OwnedVault<'_>,
    now: u64,
) -> Result<Vec<[u8; 16]>, ApiError> {
    let mut wake = Vec::new();
    if vaults::is_shared(tx, &v.vault, now)? {
        let (roster, roster_generation) = v.roster.ok_or_else(ApiError::bad_request)?;
        let rotated = dbs::rotate(
            tx,
            account,
            &v.vault,
            &dbs::Rotation {
                generation: v.generation,
                wrapped: v.wrapped,
                device: r.device,
                expected_head: v.expected_head,
                roster,
                roster_generation,
                seals: v.seals,
                invite_seals: v.invite_seals,
                revoke: &[],
            },
            now,
        )?;
        wake = rotated.accounts;
    } else {
        let seq = v.expected_head.saturating_add(1);
        tx.execute(
            "INSERT INTO vault_keys (vault_id, account_id, generation, wrapped, device_id, \
             server_seq, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                v.vault.as_slice(),
                account.as_slice(),
                v.generation,
                v.wrapped.as_slice(),
                r.device.as_slice(),
                int(seq),
                int(now)
            ],
        )?;
        if let Some((roster, _)) = v.roster {
            tx.execute(
                "INSERT INTO vault_roster (vault_id, key_generation, ciphertext, server_seq) \
                 VALUES (?1, ?2, ?3, ?4) ON CONFLICT (vault_id, key_generation) DO UPDATE SET \
                 ciphertext = excluded.ciphertext, server_seq = excluded.server_seq",
                params![v.vault.as_slice(), v.generation, roster, int(seq)],
            )?;
        }
        tx.execute(
            "UPDATE vaults SET current_generation = ?1, head_seq = ?2 WHERE vault_id = ?3",
            params![v.generation, int(seq), v.vault.as_slice()],
        )?;
        tx.execute(
            "DELETE FROM vault_rotation_requests WHERE vault_id = ?1",
            [v.vault.as_slice()],
        )?;
    }
    // Wrappings under the old AK go.
    tx.execute(
        "DELETE FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2 AND generation < ?3",
        params![v.vault.as_slice(), account.as_slice(), v.generation],
    )?;
    Ok(wake)
}

/// A vault shared to the account: the current row re-wrapped, older rows
/// gone, and a rotation request to the owner. The head moves so that the
/// account's other devices fetch the row.
fn rewrap_member(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    r: &Rekey<'_>,
    m: &Member,
    now: u64,
) -> Result<(), ApiError> {
    if m.holds {
        let rewrap = r
            .member_rewraps
            .iter()
            .find(|w| w.vault == m.vault)
            .ok_or_else(ApiError::internal)?;
        let head: i64 = tx.query_row(
            "SELECT head_seq FROM vaults WHERE vault_id = ?1",
            [m.vault.as_slice()],
            |row| row.get(0),
        )?;
        let seq = head.saturating_add(1);
        tx.execute(
            "UPDATE vault_keys SET wrapped = ?1, device_id = ?2, server_seq = ?3, created_at = ?4 \
             WHERE vault_id = ?5 AND account_id = ?6 AND generation = ?7",
            params![
                rewrap.wrapped.as_slice(),
                r.device.as_slice(),
                seq,
                int(now),
                m.vault.as_slice(),
                account.as_slice(),
                m.generation
            ],
        )?;
        tx.execute(
            "UPDATE vaults SET head_seq = ?1 WHERE vault_id = ?2",
            params![seq, m.vault.as_slice()],
        )?;
    }
    tx.execute(
        "DELETE FROM vault_keys WHERE vault_id = ?1 AND account_id = ?2 AND generation < ?3",
        params![m.vault.as_slice(), account.as_slice(), m.generation],
    )?;
    tx.execute(
        "INSERT INTO vault_rotation_requests (vault_id, requester, reason, requested_at) \
         VALUES (?1, ?2, 1, ?3) ON CONFLICT (vault_id, requester) DO UPDATE SET \
         requested_at = excluded.requested_at",
        params![m.vault.as_slice(), account.as_slice(), int(now)],
    )?;
    audit::record(tx, &m.owner, now, audit::Kind::RotationRequested, None)?;
    Ok(())
}

/// Seals addressed to the old identity key are dead: unclaimed key shares,
/// the seals of grants the account received (each grantor re-seals after
/// verifying the new code) and invites it received. Invites it sent for
/// vaults it does not own carry a seal from the old identity key too.
fn drop_dead_seals(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    notify: &mut IdSet,
) -> Result<(), ApiError> {
    tx.execute(
        "DELETE FROM key_shares WHERE recipient = ?1",
        [account.as_slice()],
    )?;
    notify.extend(ids(
        tx,
        "SELECT DISTINCT grantor FROM emergency_seals WHERE grantee = ?1",
        account,
    )?);
    tx.execute(
        "DELETE FROM emergency_seals WHERE grantee = ?1",
        [account.as_slice()],
    )?;
    notify.extend(ids(
        tx,
        "SELECT DISTINCT sender FROM share_invites WHERE recipient = ?1 AND state = 0",
        account,
    )?);
    tx.execute(
        "UPDATE share_invites SET state = 5 WHERE recipient = ?1 AND state = 0",
        [account.as_slice()],
    )?;
    // Invites it sent for vaults it does not own carry a seal from the old
    // identity key: they are cancelled. (Its owned vaults' invites were
    // refreshed by the rotation.)
    notify.extend(ids(
        tx,
        "SELECT recipient FROM share_invites i WHERE sender = ?1 AND state = 0 \
         AND NOT EXISTS (SELECT 1 FROM vaults v WHERE v.vault_id = i.vault_id \
                         AND v.owner_account_id = ?1)",
        account,
    )?);
    tx.execute(
        "UPDATE share_invites SET state = 3 WHERE sender = ?1 AND state = 0 \
         AND NOT EXISTS (SELECT 1 FROM vaults v WHERE v.vault_id = share_invites.vault_id \
                         AND v.owner_account_id = ?1)",
        [account.as_slice()],
    )?;
    Ok(())
}

/// Checks everything, then applies everything. The caller's transaction
/// rolls back on an error, so nothing is applied then.
///
/// # Errors
/// `account_version_mismatch`, `epoch_mismatch`, `profile_downgrade`,
/// `rekey_mismatch`, `generation_taken`, `generation_mismatch`, `head_moved`,
/// `members_mismatch` or `bad_request`.
pub fn apply(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    r: &Rekey<'_>,
    now: u64,
) -> Result<Rekeyed, ApiError> {
    let s = stored(tx, account)?;
    let members = memberships(tx, account)?;
    check(tx, account, r, &s, &members, now)?;

    let mut notify = IdSet::new();
    let version = accounts::update_password(
        tx,
        account,
        s.version,
        r.record,
        r.wrapped_ak,
        r.salt,
        r.profile,
    )?;
    tx.execute(
        "UPDATE accounts SET ak_epoch = ?1 WHERE account_id = ?2 AND ak_epoch = ?3",
        params![r.epoch, account.as_slice(), s.epoch],
    )?;
    if let Some(id) = &r.identity {
        tx.execute(
            "UPDATE accounts SET ipk = ?1, wrapped_isk = ?2 WHERE account_id = ?3",
            params![
                id.ipk.as_slice(),
                id.wrapped_isk.as_slice(),
                account.as_slice()
            ],
        )?;
    }

    for v in r.vaults {
        notify.extend(rotate_owned(tx, account, r, v, now)?);
    }
    for e in r.emergency_seals {
        tx.execute(
            "UPDATE emergency_seals SET generation = ?1, seal = ?2 \
             WHERE grantor = ?3 AND grantee = ?4 AND vault_id = ?5",
            params![
                e.generation,
                e.seal,
                account.as_slice(),
                e.grantee.as_slice(),
                e.vault.as_slice()
            ],
        )?;
        notify.insert(e.grantee);
    }
    for m in &members {
        rewrap_member(tx, account, r, m, now)?;
        notify.insert(m.owner);
    }

    drop_dead_seals(tx, account, &mut notify)?;

    audit::record(
        tx,
        account,
        now,
        audit::Kind::AccountRekeyed,
        Some(r.device),
    )?;
    // Every other session ends: a token of the old epoch, a pending second
    // factor and a half-finished login go with it. The changing session
    // moves to the new account version.
    sessions::after_password_change(tx, account, r.keep_session, version)?;

    notify.remove(account);
    for a in &notify {
        bump_inbox(tx, a)?;
    }
    Ok(Rekeyed {
        account_version: version,
        epoch: r.epoch,
        notify,
    })
}

/// Whether a member asked the owner to rotate the vault.
pub fn rotation_requested(conn: &Connection, vault: &[u8; 16]) -> Result<bool, ApiError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM vault_rotation_requests WHERE vault_id = ?1 LIMIT 1",
            [vault.as_slice()],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

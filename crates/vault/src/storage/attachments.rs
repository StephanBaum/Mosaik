//! Attachment ciphertexts (CRYPTO_SPEC §7.3), stored as files in the
//! attachment directory next to the database and named by attachment ID
//! (32 lowercase hex digits). The `attachments` table holds their routing
//! metadata.
//!
//! Writes go to a uniquely named temporary file that is synced and then
//! renamed over the final name, so a crash leaves either no file or the
//! complete one. The file is written before its row is committed and
//! deleted after its row is gone, so a crash can only leave an orphan file
//! (ciphertext without a row), never a row without its file;
//! [`Store::sweep_orphan_attachments`] removes orphans.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use mosaic_core::attachment::{
    ALG_STREAM_BE32_XCHACHA20_POLY1305, CHUNK_LEN, CHUNK_LOG2, NONCE_PREFIX_LEN, ciphertext_len,
};
use mosaic_core::envelope::FORMAT_VERSION;
use mosaic_core::{AttachmentId, ItemId, MAX_ATTACHMENT_LEN, VaultId};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
use super::{Store, now_ms, vault_id_from_sql};

/// Suffix of temporary files in the attachment directory.
const TMP_SUFFIX: &str = ".tmp";

/// `version || alg || chunk_log2`.
const HEADER: [u8; 3] = [
    FORMAT_VERSION,
    ALG_STREAM_BE32_XCHACHA20_POLY1305,
    CHUNK_LOG2,
];
/// Header plus nonce prefix.
const PREFIX_LEN: usize = HEADER.len() + NONCE_PREFIX_LEN;

/// Metadata of a stored attachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentInfo {
    /// The attachment.
    pub attachment_id: AttachmentId,
    /// The vault of the owning item.
    pub vault_id: VaultId,
    /// The owning item's record.
    pub record_id: ItemId,
    /// The ciphertext size in bytes.
    pub size: u64,
    /// Whether the ciphertext still has to be uploaded.
    pub upload_pending: bool,
    /// When it was stored (ms since the Unix epoch).
    pub created_at_ms: i64,
}

/// Checks that `bytes` has the layout of a CRYPTO_SPEC §7.3 attachment
/// ciphertext: the version-1 STREAM header with `chunk_log2 = 16`, a nonce
/// prefix, canonical chunking, and at most the ciphertext size of a
/// `mosaic_core::MAX_ATTACHMENT_LEN` plaintext. This is the same
/// structural check `mosaic_core::open_attachment` makes before any
/// decryption; it cannot check authenticity (that needs the key).
///
/// # Errors
/// [`Error::AttachmentTooLarge`] above the size limit,
/// [`Error::AttachmentMalformed`] for any other layout error.
pub fn validate_attachment_ciphertext(bytes: &[u8]) -> Result<()> {
    if bytes.first_chunk::<3>() != Some(&HEADER) || bytes.len() < PREFIX_LEN {
        return Err(Error::AttachmentMalformed);
    }
    let max = ciphertext_len(MAX_ATTACHMENT_LEN).map_err(|_| Error::AttachmentTooLarge)?;
    if bytes.len() > max {
        return Err(Error::AttachmentTooLarge);
    }
    // The empty file's ciphertext is the prefix plus one tag.
    let empty = ciphertext_len(0).map_err(|_| Error::AttachmentMalformed)?;
    let tag_len = empty.saturating_sub(PREFIX_LEN);
    let body = bytes.len().saturating_sub(PREFIX_LEN);
    if body < tag_len {
        return Err(Error::AttachmentMalformed);
    }
    let chunks = body.div_ceil(CHUNK_LEN.saturating_add(tag_len));
    let plaintext_len = chunks
        .checked_mul(tag_len)
        .and_then(|tags| body.checked_sub(tags))
        .ok_or(Error::AttachmentMalformed)?;
    // Canonical iff the encoder would produce exactly this length.
    match ciphertext_len(plaintext_len) {
        Ok(len) if len == bytes.len() => Ok(()),
        _ => Err(Error::AttachmentMalformed),
    }
}

fn file_name(id: &AttachmentId) -> String {
    use core::fmt::Write as _;
    let mut s = String::with_capacity(32);
    for b in id.as_bytes() {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Parses a final attachment file name (32 lowercase hex digits).
fn parse_file_name(name: &str) -> Option<[u8; 16]> {
    if name.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    let (pairs, _) = name.as_bytes().as_chunks::<2>();
    for (byte, pair) in out.iter_mut().zip(pairs) {
        let pair = core::str::from_utf8(pair).ok()?;
        if pair.bytes().any(|c| c.is_ascii_uppercase()) {
            return None;
        }
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(out)
}

/// Creates the database file with owner-only permissions on Unix before
/// SQLite opens it. SQLite would otherwise create it with the umask
/// default. A no-op elsewhere and for an existing file.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
pub(super) fn prepare_db_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Makes an existing database and its `-wal` / `-shm` files owner-only on
/// Unix (`0600`), for a store created by an older build or copied in with
/// looser permissions. SQLite creates the WAL and shared-memory files with
/// the database file's permissions. Runs only after the file was accepted
/// as a Mosaic store, so a foreign file keeps its permissions. A no-op
/// elsewhere.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
pub(super) fn restrict_db_files(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut paths = vec![path.to_path_buf()];
        for suffix in ["-wal", "-shm"] {
            let mut name = path.as_os_str().to_os_string();
            name.push(suffix);
            paths.push(name.into());
        }
        for p in paths {
            match fs::set_permissions(&p, fs::Permissions::from_mode(0o600)) {
                Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Creates the attachment directory (owner-only on Unix).
pub(super) fn create_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Makes a rename in `dir` durable. Directories cannot be synced through
/// the standard library on Windows; NTFS journals the rename itself.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(dir)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Writes `bytes` to `dir/name` through a synced temporary file and a
/// rename.
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!("{name}.{}.{n}{TMP_SUFFIX}", std::process::id()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, dir.join(name))?;
        sync_dir(dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(result?)
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Deletes the files of attachments whose rows are already gone.
pub(super) fn remove_files(dir: Option<&Path>, ids: &[AttachmentId]) -> Result<()> {
    let Some(dir) = dir else {
        return Ok(());
    };
    for id in ids {
        remove_file_if_present(&dir.join(file_name(id)))?;
    }
    Ok(())
}

fn collect_ids(
    conn: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<AttachmentId>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(args, |r| r.get::<_, Vec<u8>>(0))?;
    rows.map(|id| AttachmentId::from_slice(&id?).map_err(|_| Error::Corrupt))
        .collect()
}

pub(super) fn ids_in_vault(conn: &Connection, vault_id: &VaultId) -> Result<Vec<AttachmentId>> {
    collect_ids(
        conn,
        "SELECT attachment_id FROM attachments WHERE vault_id = ?1",
        [vault_id.as_bytes()],
    )
}

pub(super) fn ids_for_record(
    conn: &Connection,
    vault_id: &VaultId,
    record_id: &ItemId,
) -> Result<Vec<AttachmentId>> {
    collect_ids(
        conn,
        "SELECT attachment_id FROM attachments WHERE vault_id = ?1 AND record_id = ?2",
        params![vault_id.as_bytes(), record_id.as_bytes()],
    )
}

const SELECT_INFO: &str = "SELECT attachment_id, vault_id, record_id, size, upload_pending,
    created_at FROM attachments";

type InfoRow = (Vec<u8>, Vec<u8>, Vec<u8>, i64, bool, i64);

fn info_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<InfoRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
    ))
}

fn decode_info(row: InfoRow) -> Result<AttachmentInfo> {
    let (id, vault, record, size, upload_pending, created_at_ms) = row;
    Ok(AttachmentInfo {
        attachment_id: AttachmentId::from_slice(&id).map_err(|_| Error::Corrupt)?,
        vault_id: vault_id_from_sql(&vault)?,
        record_id: ItemId::from_slice(&record).map_err(|_| Error::Corrupt)?,
        size: u64::try_from(size).map_err(|_| Error::Corrupt)?,
        upload_pending,
        created_at_ms,
    })
}

impl Store {
    fn attachment_dir_or_err(&self) -> Result<&Path> {
        self.attachment_dir
            .as_deref()
            .ok_or(Error::AttachmentsUnavailable)
    }

    /// Stores an attachment ciphertext for an existing record. The bytes
    /// must pass [`validate_attachment_ciphertext`]. Set `upload_pending`
    /// for an attachment created on this device, and clear it for one
    /// downloaded from the server.
    ///
    /// Attachment IDs are unique across the whole store (and the server,
    /// §12), and a stored ciphertext is immutable. Moving an item to
    /// another vault (§7.2) therefore re-seals each attachment under a
    /// **fresh** attachment ID and FK (CRYPTO_SPEC §7.3, ADR-0010): the new
    /// copy is stored first, and the old one is deleted only after the
    /// moved item is safely stored, so there is never a moment without a
    /// copy.
    ///
    /// # Errors
    /// [`Error::AttachmentsUnavailable`] for an in-memory store,
    /// [`Error::AttachmentMalformed`], [`Error::AttachmentTooLarge`],
    /// [`Error::UnknownRecord`], [`Error::AttachmentExists`], [`Error::Io`].
    pub fn put_attachment(
        &mut self,
        vault_id: &VaultId,
        record_id: &ItemId,
        attachment_id: &AttachmentId,
        ciphertext: &[u8],
        upload_pending: bool,
    ) -> Result<()> {
        validate_attachment_ciphertext(ciphertext)?;
        let dir = self.attachment_dir_or_err()?.to_path_buf();
        let size = i64::try_from(ciphertext.len()).map_err(|_| Error::AttachmentTooLarge)?;
        let tx = self.write()?;
        let record_exists: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM records WHERE vault_id = ?1 AND record_id = ?2)",
            params![vault_id.as_bytes(), record_id.as_bytes()],
            |r| r.get(0),
        )?;
        if !record_exists {
            return Err(Error::UnknownRecord);
        }
        let exists: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM attachments WHERE attachment_id = ?1)",
            [attachment_id.as_bytes()],
            |r| r.get(0),
        )?;
        if exists {
            return Err(Error::AttachmentExists);
        }
        let name = file_name(attachment_id);
        // The write lock is held, so no other connection writes this ID.
        write_atomic(&dir, &name, ciphertext)?;
        let committed = tx
            .execute(
                "INSERT INTO attachments (attachment_id, vault_id, record_id, size,
                                          upload_pending, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    attachment_id.as_bytes(),
                    vault_id.as_bytes(),
                    record_id.as_bytes(),
                    size,
                    upload_pending,
                    now_ms()
                ],
            )
            .and_then(|_| tx.commit());
        if let Err(e) = committed {
            remove_file_if_present(&dir.join(&name))?;
            return Err(e.into());
        }
        Ok(())
    }

    /// Reads an attachment ciphertext. Open it with
    /// `mosaic_core::open_attachment`.
    ///
    /// # Errors
    /// [`Error::AttachmentsUnavailable`], [`Error::UnknownAttachment`],
    /// [`Error::Corrupt`] if the file is missing, has another size than
    /// recorded or is not an attachment ciphertext, [`Error::Io`].
    pub fn attachment(&self, attachment_id: &AttachmentId) -> Result<Vec<u8>> {
        let dir = self.attachment_dir_or_err()?;
        let info = self
            .attachment_info(attachment_id)?
            .ok_or(Error::UnknownAttachment)?;
        let path = dir.join(file_name(attachment_id));
        let not_found_is_corrupt = |e: std::io::Error| {
            if e.kind() == ErrorKind::NotFound {
                Error::Corrupt
            } else {
                Error::Io(e)
            }
        };
        let len = fs::metadata(&path).map_err(not_found_is_corrupt)?.len();
        if len != info.size {
            return Err(Error::Corrupt);
        }
        let bytes = fs::read(&path).map_err(not_found_is_corrupt)?;
        if u64::try_from(bytes.len()).ok() != Some(info.size)
            || validate_attachment_ciphertext(&bytes).is_err()
        {
            return Err(Error::Corrupt);
        }
        Ok(bytes)
    }

    /// An attachment's metadata, if stored.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn attachment_info(&self, attachment_id: &AttachmentId) -> Result<Option<AttachmentInfo>> {
        let row = self
            .conn
            .query_row(
                &format!("{SELECT_INFO} WHERE attachment_id = ?1"),
                [attachment_id.as_bytes()],
                info_row,
            )
            .optional()?;
        row.map(decode_info).transpose()
    }

    /// The attachments of one record, ordered by attachment ID.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn attachments_for_record(
        &self,
        vault_id: &VaultId,
        record_id: &ItemId,
    ) -> Result<Vec<AttachmentInfo>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT_INFO} WHERE vault_id = ?1 AND record_id = ?2 ORDER BY attachment_id"
        ))?;
        let rows = stmt.query_map(params![vault_id.as_bytes(), record_id.as_bytes()], info_row)?;
        rows.map(|row| decode_info(row?)).collect()
    }

    /// Attachments still waiting for upload, oldest first.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn pending_attachment_uploads(&self) -> Result<Vec<AttachmentInfo>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT_INFO} WHERE upload_pending = 1 ORDER BY created_at, attachment_id"
        ))?;
        let rows = stmt.query_map([], info_row)?;
        rows.map(|row| decode_info(row?)).collect()
    }

    /// Clears an attachment's upload flag. Returns whether it was pending.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_attachment_uploaded(&mut self, attachment_id: &AttachmentId) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE attachments SET upload_pending = 0
             WHERE attachment_id = ?1 AND upload_pending = 1",
            [attachment_id.as_bytes()],
        )?;
        Ok(n > 0)
    }

    /// Deletes an attachment's row, then its file. Returns whether it
    /// existed.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Io`].
    pub fn delete_attachment(&mut self, attachment_id: &AttachmentId) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM attachments WHERE attachment_id = ?1",
            [attachment_id.as_bytes()],
        )?;
        remove_files(self.attachment_dir.as_deref(), &[*attachment_id])?;
        Ok(n > 0)
    }

    /// Deletes orphan files from the attachment directory: temporary files
    /// left by an interrupted write, and attachment files without a row.
    /// Other files are left alone. Returns the number of files deleted.
    ///
    /// Run it at startup, while no other process has the store open: a
    /// write in progress elsewhere would lose its temporary file.
    ///
    /// # Errors
    /// [`Error::Io`], [`Error::Database`]; `Ok(0)` for an in-memory store.
    pub fn sweep_orphan_attachments(&mut self) -> Result<usize> {
        let Some(dir) = self.attachment_dir.as_deref() else {
            return Ok(0);
        };
        let known: HashSet<[u8; 16]> = {
            let mut stmt = self.conn.prepare("SELECT attachment_id FROM attachments")?;
            let rows = stmt.query_map([], |r| r.get::<_, [u8; 16]>(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut removed = 0usize;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let orphan = name.ends_with(TMP_SUFFIX)
                || parse_file_name(name).is_some_and(|id| !known.contains(&id));
            if orphan && entry.file_type()?.is_file() {
                remove_file_if_present(&entry.path())?;
                removed = removed.saturating_add(1);
            }
        }
        Ok(removed)
    }

    /// Adds a problem for every attachment row whose file is missing or
    /// has another size.
    pub(super) fn check_attachment_files(&self, problems: &mut Vec<String>) -> Result<()> {
        let Some(dir) = self.attachment_dir.as_deref() else {
            return Ok(());
        };
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT_INFO} ORDER BY attachment_id"))?;
        let rows = stmt.query_map([], info_row)?;
        for row in rows {
            let info = decode_info(row?)?;
            let name = file_name(&info.attachment_id);
            match fs::metadata(dir.join(&name)) {
                Ok(m) if m.len() == info.size => {}
                Ok(_) => problems.push(format!("attachment {name}: size mismatch")),
                Err(_) => problems.push(format!("attachment {name}: file missing")),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::PendingPolicy;
    use super::super::records::Origin;
    use super::super::records::tests::{item, store_with_vault};
    use super::super::vaults::tests::{fixture, g};
    use super::*;
    use mosaic_core::{AttachmentKey, open_attachment, seal_attachment};

    fn sealed(len: usize) -> Vec<u8> {
        let fk = AttachmentKey::generate().unwrap();
        seal_attachment(
            &fk,
            &VaultId::new_v7().unwrap(),
            &ItemId::new_v7().unwrap(),
            &AttachmentId::new_v7().unwrap(),
            &vec![0x42; len],
        )
        .unwrap()
    }

    /// Brute-force oracle: is `len` the ciphertext length of some
    /// plaintext length?
    fn canonical(len: usize) -> bool {
        (1..=len / CHUNK_LEN + 2).any(|chunks| {
            len.checked_sub(PREFIX_LEN + 16 * chunks)
                .is_some_and(|p| ciphertext_len(p).ok() == Some(len))
        })
    }

    #[test]
    fn validation_accepts_exactly_the_canonical_lengths() {
        let mut buf = HEADER.to_vec();
        buf.resize(PREFIX_LEN + 2 * (CHUNK_LEN + 16) + 64, 0);
        for len in 0..=buf.len() {
            let ok = validate_attachment_ciphertext(&buf[..len]).is_ok();
            assert_eq!(ok, len >= HEADER.len() && canonical(len), "len {len}");
        }
    }

    #[test]
    fn validation_accepts_real_ciphertexts_only() {
        for len in [
            0,
            1,
            100,
            CHUNK_LEN - 1,
            CHUNK_LEN,
            CHUNK_LEN + 1,
            3 * CHUNK_LEN,
        ] {
            let ct = sealed(len);
            validate_attachment_ciphertext(&ct).unwrap();
            for (i, bad) in [(0, 2), (1, 1), (2, 15)] {
                let mut wrong = ct.clone();
                wrong[i] = bad;
                assert!(matches!(
                    validate_attachment_ciphertext(&wrong),
                    Err(Error::AttachmentMalformed)
                ));
            }
        }
        assert!(matches!(
            validate_attachment_ciphertext(&[]),
            Err(Error::AttachmentMalformed)
        ));
        assert!(matches!(
            validate_attachment_ciphertext(&HEADER),
            Err(Error::AttachmentMalformed)
        ));
        let max = ciphertext_len(MAX_ATTACHMENT_LEN).unwrap();
        let mut huge = HEADER.to_vec();
        huge.resize(max, 0);
        validate_attachment_ciphertext(&huge).unwrap();
        huge.push(0);
        assert!(matches!(
            validate_attachment_ciphertext(&huge),
            Err(Error::AttachmentTooLarge)
        ));
    }

    #[test]
    fn file_names() {
        let id = AttachmentId::from_bytes([0xAB; 16]).unwrap();
        let name = file_name(&id);
        assert_eq!(name, "ab".repeat(16));
        assert_eq!(parse_file_name(&name), Some([0xAB; 16]));
        assert_eq!(parse_file_name(&name.to_uppercase()), None);
        assert_eq!(parse_file_name("ab"), None);
        assert_eq!(parse_file_name(&"zz".repeat(16)), None);
        assert_eq!(parse_file_name(&"é".repeat(16)), None);
    }

    #[test]
    fn in_memory_store_has_no_attachments() {
        let (mut s, f, vk) = store_with_vault();
        let rid = ItemId::new_v7().unwrap();
        s.upsert_record(&f.vault, &rid, &item(&f, &vk, &rid, b"x"), Origin::Local)
            .unwrap();
        let id = AttachmentId::new_v7().unwrap();
        assert!(matches!(
            s.put_attachment(&f.vault, &rid, &id, &sealed(1), true),
            Err(Error::AttachmentsUnavailable)
        ));
        assert!(matches!(
            s.attachment(&id),
            Err(Error::AttachmentsUnavailable)
        ));
        assert_eq!(s.sweep_orphan_attachments().unwrap(), 0);
        assert!(!s.delete_attachment(&id).unwrap());
    }

    #[test]
    fn attachment_lifecycle_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("v.db")).unwrap();
        let f = fixture();
        let (vk, w) = f.key(1);
        s.create_vault(&f.vault, g(1), &w).unwrap();
        let rid = ItemId::new_v7().unwrap();
        let aid = AttachmentId::new_v7().unwrap();
        let fk = AttachmentKey::generate().unwrap();
        let ct = seal_attachment(&fk, &f.vault, &rid, &aid, &vec![7; CHUNK_LEN + 5]).unwrap();

        assert!(matches!(
            s.put_attachment(&f.vault, &rid, &aid, &ct, true),
            Err(Error::UnknownRecord)
        ));
        s.upsert_record(&f.vault, &rid, &item(&f, &vk, &rid, b"x"), Origin::Local)
            .unwrap();
        assert!(matches!(
            s.put_attachment(&f.vault, &rid, &aid, b"plaintext", true),
            Err(Error::AttachmentMalformed)
        ));
        assert!(matches!(s.attachment(&aid), Err(Error::UnknownAttachment)));
        s.put_attachment(&f.vault, &rid, &aid, &ct, true).unwrap();
        assert!(matches!(
            s.put_attachment(&f.vault, &rid, &aid, &ct, true),
            Err(Error::AttachmentExists)
        ));
        let back = s.attachment(&aid).unwrap();
        assert_eq!(back, ct);
        let pt = open_attachment(&fk, &f.vault, &rid, &aid, &back).unwrap();
        assert_eq!(pt.len(), CHUNK_LEN + 5);

        let info = s.attachment_info(&aid).unwrap().unwrap();
        assert_eq!(info.size, ct.len() as u64);
        assert!(info.upload_pending);
        assert_eq!((info.vault_id, info.record_id), (f.vault, rid));
        assert_eq!(
            s.attachments_for_record(&f.vault, &rid).unwrap(),
            vec![info]
        );
        assert_eq!(s.pending_attachment_uploads().unwrap(), vec![info]);
        assert!(s.mark_attachment_uploaded(&aid).unwrap());
        assert!(!s.mark_attachment_uploaded(&aid).unwrap());
        assert_eq!(s.pending_attachment_uploads().unwrap().len(), 0);
        assert_eq!(s.integrity_check().unwrap().len(), 0);

        // No temporary files remain after a write.
        let att_dir = s.attachment_dir().unwrap().to_path_buf();
        let names: Vec<_> = fs::read_dir(&att_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec![file_name(&aid)]);

        // Damage is detected on read and by the integrity check.
        let path = att_dir.join(file_name(&aid));
        fs::write(&path, &ct[..ct.len() - 1]).unwrap();
        assert!(matches!(s.attachment(&aid), Err(Error::Corrupt)));
        assert!(s.integrity_check().unwrap()[0].contains("size mismatch"));
        fs::write(&path, vec![0; ct.len()]).unwrap();
        assert!(matches!(s.attachment(&aid), Err(Error::Corrupt)));
        fs::remove_file(&path).unwrap();
        assert!(matches!(s.attachment(&aid), Err(Error::Corrupt)));
        assert!(s.integrity_check().unwrap()[0].contains("file missing"));
        fs::write(&path, &ct).unwrap();

        // The record is still in the outbox: no silent delete.
        assert!(matches!(
            s.delete_record(&f.vault, &rid, PendingPolicy::Refuse),
            Err(Error::PendingLocalChange)
        ));
        assert!(path.exists());
        // Deleting the record removes its attachments (row and file).
        assert!(
            s.delete_record(&f.vault, &rid, PendingPolicy::Discard)
                .unwrap()
        );
        assert_eq!(s.attachment_info(&aid).unwrap(), None);
        assert!(!path.exists());
    }

    /// An attachment not yet uploaded is a pending local change too.
    #[test]
    fn pending_attachment_blocks_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("v.db")).unwrap();
        let f = fixture();
        let (vk, w) = f.key(1);
        s.create_vault(&f.vault, g(1), &w).unwrap();
        let rid = ItemId::new_v7().unwrap();
        s.upsert_record(
            &f.vault,
            &rid,
            &item(&f, &vk, &rid, b"x"),
            Origin::Remote { server_seq: 1 },
        )
        .unwrap();
        let aid = AttachmentId::new_v7().unwrap();
        s.put_attachment(&f.vault, &rid, &aid, &sealed(3), true)
            .unwrap();
        for r in [
            s.delete_record(&f.vault, &rid, PendingPolicy::Refuse),
            s.delete_vault(&f.vault, PendingPolicy::Refuse),
        ] {
            assert!(matches!(r, Err(Error::PendingLocalChange)), "{r:?}");
        }
        assert!(s.attachment_info(&aid).unwrap().is_some());
        assert!(s.mark_attachment_uploaded(&aid).unwrap());
        assert!(
            s.delete_record(&f.vault, &rid, PendingPolicy::Refuse)
                .unwrap()
        );
    }

    /// Moving an item to another vault (§7.2) re-seals its attachments
    /// under fresh attachment IDs: the new copy is stored while the old one
    /// still exists, so there is never a moment without a copy. Reusing
    /// the old ID is refused instead of silently replacing it.
    #[test]
    fn moving_an_item_uses_fresh_attachment_ids() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("v.db")).unwrap();
        let a = fixture();
        let (vk_a, wa) = a.key(1);
        s.create_vault(&a.vault, g(1), &wa).unwrap();
        let b = fixture();
        let (vk_b, wb) = b.key(1);
        s.create_vault(&b.vault, g(1), &wb).unwrap();

        let rid = ItemId::new_v7().unwrap();
        s.upsert_record(
            &a.vault,
            &rid,
            &item(&a, &vk_a, &rid, b"x"),
            Origin::Remote { server_seq: 1 },
        )
        .unwrap();
        let old_id = AttachmentId::new_v7().unwrap();
        let fk = AttachmentKey::generate().unwrap();
        let content = vec![9u8; 1000];
        let old_ct = seal_attachment(&fk, &a.vault, &rid, &old_id, &content).unwrap();
        s.put_attachment(&a.vault, &rid, &old_id, &old_ct, false)
            .unwrap();

        // The moved item in vault B, with its attachment re-sealed under
        // the new vault ID.
        s.upsert_record(&b.vault, &rid, &item(&b, &vk_b, &rid, b"x"), Origin::Local)
            .unwrap();
        let reused = seal_attachment(&fk, &b.vault, &rid, &old_id, &content).unwrap();
        assert!(matches!(
            s.put_attachment(&b.vault, &rid, &old_id, &reused, true),
            Err(Error::AttachmentExists)
        ));
        let new_id = AttachmentId::new_v7().unwrap();
        let fk2 = AttachmentKey::generate().unwrap();
        let new_ct = seal_attachment(&fk2, &b.vault, &rid, &new_id, &content).unwrap();
        s.put_attachment(&b.vault, &rid, &new_id, &new_ct, true)
            .unwrap();
        // Both copies exist until the old item is purged.
        assert_eq!(s.attachment(&old_id).unwrap(), old_ct);
        assert!(
            s.delete_record(&a.vault, &rid, PendingPolicy::Refuse)
                .unwrap()
        );
        assert!(matches!(
            s.attachment(&old_id),
            Err(Error::UnknownAttachment)
        ));
        let back = s.attachment(&new_id).unwrap();
        let pt = open_attachment(&fk2, &b.vault, &rid, &new_id, &back).unwrap();
        assert_eq!(pt.expose_secret(), content.as_slice());
    }

    #[test]
    fn delete_paths_and_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("v.db")).unwrap();
        let f = fixture();
        let (vk, w) = f.key(1);
        s.create_vault(&f.vault, g(1), &w).unwrap();
        let rid = ItemId::new_v7().unwrap();
        s.upsert_record(
            &f.vault,
            &rid,
            &item(&f, &vk, &rid, b"x"),
            Origin::Remote { server_seq: 1 },
        )
        .unwrap();
        let a1 = AttachmentId::new_v7().unwrap();
        let a2 = AttachmentId::new_v7().unwrap();
        s.put_attachment(&f.vault, &rid, &a1, &sealed(10), false)
            .unwrap();
        s.put_attachment(&f.vault, &rid, &a2, &sealed(0), false)
            .unwrap();
        let att_dir = s.attachment_dir().unwrap().to_path_buf();

        assert!(s.delete_attachment(&a1).unwrap());
        assert!(!s.delete_attachment(&a1).unwrap());
        assert!(!att_dir.join(file_name(&a1)).exists());

        // Orphans: a stale temp file, a file without a row; unrelated files
        // and directories stay.
        let orphan = AttachmentId::new_v7().unwrap();
        fs::write(att_dir.join(file_name(&orphan)), b"x").unwrap();
        fs::write(att_dir.join("abc.1.2.tmp"), b"x").unwrap();
        fs::write(att_dir.join("README"), b"x").unwrap();
        fs::create_dir(att_dir.join(format!("{}{TMP_SUFFIX}", "d"))).unwrap();
        assert_eq!(s.sweep_orphan_attachments().unwrap(), 2);
        assert!(att_dir.join(file_name(&a2)).exists());
        assert!(att_dir.join("README").exists());
        assert_eq!(s.sweep_orphan_attachments().unwrap(), 0);

        // Deleting the vault removes the remaining attachment.
        assert!(s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert!(!att_dir.join(file_name(&a2)).exists());
        assert_eq!(s.attachment_info(&a2).unwrap(), None);
    }

    #[test]
    fn atomic_write_cleans_up_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        // The target is a non-empty directory, so the rename fails.
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inner"), b"x").unwrap();
        assert!(matches!(
            write_atomic(dir.path(), "target", b"data"),
            Err(Error::Io(_))
        ));
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("target")]);
        assert!(remove_files(None, &[AttachmentId::new_v7().unwrap()]).is_ok());
    }
}

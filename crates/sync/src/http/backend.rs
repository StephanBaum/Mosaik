//! [`HttpTransport`]: the [`Transport`] of the hosted backend (M6 spec §2.1,
//! §6.2). Every response is parsed strictly (unknown keys, wrong types and
//! out-of-order sequence numbers are protocol errors) and every size is
//! capped before it is read.

use core::time::Duration;

use mosaic_core::attachment::{MAX_ATTACHMENT_LEN, ciphertext_len};
use mosaic_core::model::Role;
use mosaic_core::{
    AttachmentId, InstallId, KeyGeneration, ShareSeal, VaultId, VaultMetaCiphertext,
    VaultRosterCiphertext, WrappedKey,
};
use mosaic_vault::storage::{RecordBody, RecordKind as StoredKind};
use reqwest::Method;

use super::client::{
    ATTACHMENT_TIMEOUT, Bearer, Body, Http, POLL_TIMEOUT, REQUEST_TIMEOUT, Token, canonical_origin,
    json_body,
};
use crate::sharing::SharingTransport;
use crate::transport::{
    ChangeToken, Cursor, KeyPushResult, PullPage, PulledKey, PulledMeta, PulledRecord,
    PulledRoster, PulledShare, PushRecord, PushResult, RemoteAttachment, RemoteDevice, RemoteVault,
    Transport, TransportError, TransportKind, VaultCreateResult,
};
use crate::wire::json::sharing::{ChangeRoster, ChangeShare, SharingChangesResponse};
use crate::wire::json::{
    self, AttachmentsResponse, B64, CheckpointsResponse, DevicesResponse, Empty, HeadMovedDetail,
    HeadResponse, InfoResponse, MAX_CHECKPOINT_UPLOAD_LEN, MAX_RESPONSE_LEN, MetaResult,
    PollResponse, PushMetaRequest, PushRecordsRequest, PushRecordsResponse, PutKeyRequest,
    PutKeyResponse, RecordKind, RecordResult, RestoreResponse, VaultsResponse, codes,
};

/// The most records the server returns per page.
const MAX_PAGE: usize = 500;
/// The least request size a server may have (its config minimum): a batch is
/// split to fit the limit `GET /v1/info` reported, at least this much.
const MIN_REQUEST_BYTES: usize = 2 * 1024 * 1024;
/// JSON bytes of a pushed record besides its ciphertext (IDs, keys, the
/// base64 expansion is counted separately).
const RECORD_OVERHEAD: usize = 160;
/// Slack for the envelope of a records request.
const REQUEST_SLACK: usize = 1024;

/// The hosted backend over HTTPS.
pub struct HttpTransport {
    pub(super) http: Http,
    /// The `Authorization` value, built once (sensitive, zeroizing source).
    pub(super) bearer: Bearer,
    max_request: usize,
    /// This install's ID, from the engine (`connect`): the checkpoint path
    /// and the own-checkpoint logic use it, not the server's `current` flag.
    own_device: InstallId,
}

impl core::fmt::Debug for HttpTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HttpTransport { .. }")
    }
}

impl HttpTransport {
    /// Connects to `base` (`https://host[:port]`, or `http` for a loopback
    /// address) with a session token from [`super::hosted::login`]: checks
    /// that the server speaks protocol 1 and that it names the origin the
    /// client asked for (an account is bound to one server identity).
    ///
    /// # Errors
    /// `install` is this device's own install ID: the engine's, which also
    /// names the own checkpoint (the server's `current` device flag is not
    /// consulted).
    ///
    /// # Errors
    /// [`TransportError::InsecureUrl`], [`TransportError::InvalidUrl`],
    /// [`TransportError::Incompatible`], [`TransportError::ServerMismatch`]
    /// and the errors of a request.
    pub fn connect(
        base: &str,
        token: &impl Token,
        install: InstallId,
    ) -> Result<Self, TransportError> {
        let http = Http::new(&canonical_origin(base)?)?;
        let info = http.info()?;
        let max_request = usize::try_from(info.limits.max_request_bytes)
            .unwrap_or(usize::MAX)
            .max(MIN_REQUEST_BYTES);
        Ok(Self {
            http,
            bearer: Bearer::new(token)?,
            max_request,
            own_device: install,
        })
    }

    /// `GET /v1/info` of `base`, checked like [`HttpTransport::connect`] does
    /// (no session is needed).
    ///
    /// # Errors
    /// As [`HttpTransport::connect`].
    pub fn info(base: &str) -> Result<InfoResponse, TransportError> {
        Http::new(&canonical_origin(base)?)?.info()
    }

    pub(super) const fn http(&self) -> &Http {
        &self.http
    }

    pub(super) const fn bearer(&self) -> &Bearer {
        &self.bearer
    }

    pub(super) fn call<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        request: Option<&Req>,
        ok: &[u16],
    ) -> Result<(u16, Resp), TransportError> {
        self.http
            .call(method, path, Some(&self.bearer), request, ok)
    }

    pub(super) fn get<Resp: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<Resp, TransportError> {
        Ok(self.call(Method::GET, path, None::<&Empty>, &[200])?.1)
    }
}

/// The server sequence a cursor holds: empty is 0, otherwise 8 bytes
/// big-endian.
fn cursor_seq(cursor: &Cursor) -> Result<u64, TransportError> {
    match <[u8; 8]>::try_from(cursor.0.as_slice()) {
        Ok(b) => Ok(u64::from_be_bytes(b)),
        Err(_) if cursor.0.is_empty() => Ok(0),
        Err(_) => Err(TransportError::Protocol),
    }
}

fn seq_cursor(seq: u64) -> Cursor {
    Cursor(seq.to_be_bytes().to_vec())
}

const fn stored_kind(kind: RecordKind) -> StoredKind {
    match kind {
        RecordKind::Item => StoredKind::Item,
        RecordKind::Tombstone => StoredKind::Tombstone,
    }
}

const fn wire_kind(body: &RecordBody) -> RecordKind {
    match body {
        RecordBody::Item(_) => RecordKind::Item,
        RecordBody::Tombstone(_) => RecordKind::Tombstone,
    }
}

/// A checked page of `changes`.
fn convert_page(
    page: SharingChangesResponse,
    since: u64,
    limit: usize,
) -> Result<PullPage, TransportError> {
    let head = page.head_seq;
    let within = |seq: u64| seq > since && seq <= head;
    // A head below what this device already read: the vault went back in
    // time on the server (a restored backup). Not a protocol error: the
    // engine reports it for this vault and goes on with the others.
    if head < since && page.records.is_empty() && !page.more {
        return Err(TransportError::HeadBelowCursor);
    }
    if page.next_since < since || page.next_since > head || page.records.len() > limit {
        return Err(TransportError::Protocol);
    }
    let mut keys = Vec::with_capacity(page.keys.len());
    for k in page.keys {
        if !within(k.server_seq) {
            return Err(TransportError::Protocol);
        }
        keys.push(PulledKey {
            generation: k.generation,
            wrapped: k.wrapped_key,
            device_id: k.device_id,
            seq: Some(k.server_seq),
        });
    }
    let mut metas = Vec::new();
    if let Some(m) = page.meta {
        if !within(m.server_seq) {
            return Err(TransportError::Protocol);
        }
        metas.push(PulledMeta {
            ciphertext: VaultMetaCiphertext::from_bytes(&m.ciphertext.0)
                .map_err(|_| TransportError::Protocol)?,
            seq: Some(m.server_seq),
        });
    }
    let mut records = Vec::with_capacity(page.records.len());
    let mut previous = since;
    for r in page.records {
        // Strictly ascending: a server cannot repeat or reorder records.
        if r.server_seq <= previous || r.server_seq > head {
            return Err(TransportError::Protocol);
        }
        previous = r.server_seq;
        records.push(PulledRecord {
            record_id: r.record_id,
            body: RecordBody::decode(stored_kind(r.kind), &r.ciphertext.0)
                .map_err(|_| TransportError::Protocol)?,
            seq: Some(r.server_seq),
        });
    }
    if page.more && (records.is_empty() || page.next_since != previous) {
        return Err(TransportError::Protocol);
    }
    let role = Role::from_number(page.role).map_err(|_| TransportError::Protocol)?;
    let (shares, rosters) = convert_sharing(page.shares, page.rosters, since, head)?;
    Ok(PullPage {
        keys,
        metas,
        records,
        next: seq_cursor(page.next_since),
        more: page.more,
        units: Vec::new(),
        shares,
        rosters,
        role: Some(role),
        rotation_requested: page.rotation_requested.unwrap_or(false),
    })
}

/// The shares and rosters of a sharing-aware page: a share may be any age (it
/// stays until claimed) but not beyond the head; a roster is newer than the
/// cursor.
fn convert_sharing(
    shares: Vec<ChangeShare>,
    rosters: Vec<ChangeRoster>,
    since: u64,
    head: u64,
) -> Result<(Vec<PulledShare>, Vec<PulledRoster>), TransportError> {
    let shares = shares
        .into_iter()
        .map(|sh| {
            if sh.seq > head {
                return Err(TransportError::Protocol);
            }
            Ok(PulledShare {
                generation: KeyGeneration::new(sh.generation)
                    .map_err(|_| TransportError::Protocol)?,
                sender: sh.sender,
                seal: ShareSeal::from_bytes(&sh.seal.0).map_err(|_| TransportError::Protocol)?,
                seq: Some(sh.seq),
            })
        })
        .collect::<Result<Vec<_>, TransportError>>()?;
    let rosters = rosters
        .into_iter()
        .map(|r| {
            if r.seq <= since || r.seq > head {
                return Err(TransportError::Protocol);
            }
            Ok(PulledRoster {
                generation: KeyGeneration::new(r.generation)
                    .map_err(|_| TransportError::Protocol)?,
                ciphertext: VaultRosterCiphertext::from_bytes(&r.ciphertext.0)
                    .map_err(|_| TransportError::Protocol)?,
                seq: Some(r.seq),
            })
        })
        .collect::<Result<Vec<_>, TransportError>>()?;
    Ok((shares, rosters))
}

impl Transport for HttpTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Hosted
    }

    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        let r: VaultsResponse = self.get("/v1/vaults")?;
        Ok(r.vaults
            .into_iter()
            .map(|v| RemoteVault {
                vault_id: v.vault_id,
                generation: v.generation,
            })
            .collect())
    }

    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        device: InstallId,
    ) -> Result<VaultCreateResult, TransportError> {
        let request = json::CreateVaultRequest {
            generation,
            wrapped_key: wrapped.clone(),
            device_id: device,
            meta: B64(meta.as_bytes().to_vec()),
        };
        let (status, _): (u16, HeadResponse) = self.call(
            Method::PUT,
            &format!("/v1/vaults/{vault}"),
            Some(&request),
            &[200, 201],
        )?;
        Ok(if status == 201 {
            VaultCreateResult::Created
        } else {
            VaultCreateResult::Exists
        })
    }

    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        let since = cursor_seq(cursor)?;
        let limit = limit.clamp(1, MAX_PAGE);
        // `sharing=1`: the page also carries the role, the key shares for
        // this account and the rosters (the M6 page stays strict for older
        // clients); `rotation=1` adds `rotationRequested` to the owner's page.
        let page: SharingChangesResponse = self.get(&format!(
            "/v1/vaults/{vault}/changes?since={since}&limit={limit}&sharing=1&rotation=1"
        ))?;
        convert_page(page, since, limit)
    }

    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        let budget = self.max_request.saturating_sub(REQUEST_SLACK);
        let mut results = Vec::with_capacity(records.len());
        let mut start = 0;
        while start < records.len() {
            // As many records as fit the server's request limit (the engine
            // batches by ciphertext bytes; base64 adds a third).
            let mut end = start;
            let mut size = 0usize;
            while let Some(r) = records.get(end) {
                let n = r.body.as_bytes().len().div_ceil(3) * 4 + RECORD_OVERHEAD;
                if end > start && size.saturating_add(n) > budget {
                    break;
                }
                size = size.saturating_add(n);
                end += 1;
            }
            let chunk = records.get(start..end).ok_or(TransportError::Protocol)?;
            results.extend(self.push_chunk(vault, generation, chunk)?);
            start = end;
        }
        Ok(results)
    }

    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError> {
        let request = PushMetaRequest {
            ciphertext: B64(meta.as_bytes().to_vec()),
            base_seq,
        };
        let (_, result): (u16, MetaResult) = self.call(
            Method::PUT,
            &format!("/v1/vaults/{vault}/meta"),
            Some(&request),
            &[200],
        )?;
        Ok(match result {
            MetaResult::Accepted { server_seq } => PushResult::Accepted {
                seq: Some(server_seq),
            },
            MetaResult::Conflict { current_seq } => PushResult::Conflict { current_seq },
        })
    }

    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
        expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError> {
        let request = PutKeyRequest {
            wrapped_key: wrapped.clone(),
            device_id: device,
            expected_head_seq: expected_head.map(cursor_seq).transpose()?,
        };
        let path = format!("/v1/vaults/{vault}/keys/{}", generation.get());
        let reply = self.http.send(
            Method::PUT,
            &path,
            Some(&self.bearer),
            json_body(Some(&request))?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        if reply.status == 200 {
            let (_, ok): (u16, PutKeyResponse) = reply.json(&[200])?;
            return Ok(KeyPushResult::Accepted {
                seq: Some(ok.server_seq),
            });
        }
        let body = reply.error_body();
        match body.as_ref().map(|b| b.error.as_str()) {
            Some(codes::GENERATION_TAKEN) => body
                .and_then(|b| b.detail_as::<json::GenerationDetail>())
                .and_then(|d| KeyGeneration::new(d.current_generation).ok())
                .map(|current| KeyPushResult::GenerationTaken { current })
                .ok_or(TransportError::Protocol),
            Some(codes::HEAD_MOVED) => body
                .and_then(|b| b.detail_as::<HeadMovedDetail>())
                .map(|d| KeyPushResult::HeadMoved {
                    head: seq_cursor(d.head_seq),
                })
                .ok_or(TransportError::Protocol),
            _ => Err(reply.failure()),
        }
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        let reply = self.http.send(
            Method::PUT,
            &format!("/v1/vaults/{vault}/attachments/{id}"),
            Some(&self.bearer),
            Body::Bytes(ciphertext),
            ATTACHMENT_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        reply.json::<Empty>(&[200, 201]).map(|_| ())
    }

    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        let cap = ciphertext_len(MAX_ATTACHMENT_LEN).map_err(|_| TransportError::Protocol)?;
        let reply = self.http.send(
            Method::GET,
            &format!("/v1/vaults/{vault}/attachments/{id}"),
            Some(&self.bearer),
            Body::None,
            ATTACHMENT_TIMEOUT,
            cap,
        )?;
        match reply.status {
            200 => Ok(Some(reply.body)),
            404 => Ok(None),
            _ => Err(reply.failure()),
        }
    }

    fn delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        self.call::<Empty, Empty>(
            Method::DELETE,
            &format!("/v1/vaults/{vault}/attachments/{id}"),
            None,
            &[200],
        )
        .map(|_| ())
    }

    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        let (_, r): (u16, RestoreResponse) = self.call(
            Method::POST,
            &format!("/v1/vaults/{vault}/attachments/{id}/restore"),
            None::<&Empty>,
            &[200],
        )?;
        Ok(r.restored)
    }

    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError> {
        let r: AttachmentsResponse = self.get(&format!("/v1/vaults/{vault}/attachments"))?;
        Ok(r.attachments
            .into_iter()
            .map(|a| RemoteAttachment {
                attachment_id: a.attachment_id,
                size: a.size,
                deleted_at_ms: a.deleted_at,
            })
            .collect())
    }

    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        if bytes.len() > MAX_CHECKPOINT_UPLOAD_LEN {
            return Err(TransportError::Protocol);
        }
        let device = self.own_device;
        let reply = self.http.send(
            Method::PUT,
            &format!("/v1/checkpoints/{device}"),
            Some(&self.bearer),
            Body::Bytes(bytes),
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        // `checkpoint_stale` is passed on: the server holds a checkpoint
        // with this counter or a higher one, which the engine does not count
        // as published (CRYPTO_SPEC §10).
        reply.json::<Empty>(&[200]).map(drop)
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        let r: CheckpointsResponse = self.get("/v1/checkpoints")?;
        Ok(r.checkpoints
            .into_iter()
            .map(|c| (c.device_id, c.body.0))
            .collect())
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        let r: DevicesResponse = self.get("/v1/devices")?;
        Ok(r.devices
            .into_iter()
            .map(|d| RemoteDevice {
                device_id: d.device_id,
                name: Some(d.name),
                created_at_ms: Some(d.created_at),
                last_seen_ms: Some(d.last_seen_at),
            })
            .collect())
    }

    fn sharing(&mut self) -> Option<&mut dyn SharingTransport> {
        Some(self)
    }

    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        // No token yet: a value the counter never has, so the first call
        // answers at once with the current counter.
        let since_counter = match since {
            None => u64::MAX,
            Some(t) => match <[u8; 8]>::try_from(t.0.as_slice()) {
                Ok(b) => u64::from_be_bytes(b),
                Err(_) => return Err(TransportError::Protocol),
            },
        };
        let wait = timeout.min(POLL_TIMEOUT);
        let reply = self.http.send(
            Method::GET,
            &format!("/v1/poll?since={since_counter}"),
            Some(&self.bearer),
            Body::None,
            wait,
            MAX_RESPONSE_LEN,
        );
        match reply {
            Ok(r) => {
                let (_, p): (u16, PollResponse) = r.json(&[200])?;
                Ok(ChangeToken(p.change_counter.to_be_bytes().to_vec()))
            }
            // Nothing changed within the caller's time.
            Err(TransportError::Timeout) => Ok(ChangeToken(since_counter.to_be_bytes().to_vec())),
            Err(e) => Err(e),
        }
    }
}

impl HttpTransport {
    fn push_chunk(
        &self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        let request = PushRecordsRequest {
            generation,
            records: records
                .iter()
                .map(|r| json::PushRecord {
                    record_id: r.record_id,
                    kind: wire_kind(r.body),
                    ciphertext: B64(r.body.as_bytes().to_vec()),
                    base_seq: r.base_seq,
                })
                .collect(),
        };
        let (_, response): (u16, PushRecordsResponse) = self.call(
            Method::POST,
            &format!("/v1/vaults/{vault}/records"),
            Some(&request),
            &[200],
        )?;
        if response.results.len() != records.len() {
            return Err(TransportError::Protocol);
        }
        response
            .results
            .into_iter()
            .zip(records)
            .map(|(result, sent)| match result {
                RecordResult::Accepted {
                    record_id,
                    server_seq,
                } if record_id == sent.record_id => Ok(PushResult::Accepted {
                    seq: Some(server_seq),
                }),
                RecordResult::Conflict {
                    record_id,
                    current_seq,
                } if record_id == sent.record_id => Ok(PushResult::Conflict { current_seq }),
                _ => Err(TransportError::Protocol),
            })
            .collect()
    }
}

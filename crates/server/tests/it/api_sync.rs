//! Vaults, records, keys, metadata, attachments, checkpoints and the
//! long poll (CRYPTO_SPEC §10.2).

use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use mosaic_core::{AccountKey, AttachmentId, InstallId, ItemId, KeyGeneration, VaultId, VaultKey};
use mosaic_server::ApiError;
use mosaic_sync::wire::KeyedCrypto;
use mosaic_sync::wire::checkpoint::{Checkpoint, CheckpointEntry};
use mosaic_sync::wire::json::{
    self, AttachmentsResponse, B64, ChangesResponse, CheckpointsResponse, GenerationDetail,
    HeadMovedDetail, HeadResponse, MetaResult, PollResponse, PushMetaRequest, PushRecordsResponse,
    PutKeyRequest, PutKeyResponse, RecordResult, RegistrationMode, RestoreResponse, VaultsResponse,
};

use crate::common::{IP, T0, TestApp, TestVault, wrapped_key};

fn app() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

fn changes_uri(v: &TestVault, since: u64, limit: usize) -> String {
    format!("/v1/vaults/{}/changes?since={since}&limit={limit}", v.id)
}

async fn put_key(
    app: &TestApp,
    token: &[u8; 32],
    v: &TestVault,
    generation: u32,
    expected_head: Option<u64>,
) -> crate::common::Resp {
    app.put(
        &format!("/v1/vaults/{}/keys/{generation}", v.id),
        Some(token),
        &PutKeyRequest {
            wrapped_key: wrapped_key(u8::try_from(generation).unwrap()),
            device_id: v.device,
            expected_head_seq: expected_head,
        },
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn create_vault_idempotent_and_owner_checked() {
    let app = app();
    let (_, a) = app.user("alice").await;
    let (_, b) = app.user("bob").await;
    let v = TestVault::new();

    let r = app.create_vault(&a, &v).await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.json::<HeadResponse>().head_seq, 2);
    // Again: idempotent for the owner.
    let r = app.create_vault(&a, &v).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json::<HeadResponse>().head_seq, 2);
    // Another account cannot take the ID.
    let r = app.create_vault(&b, &v).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "vault_exists");

    let mine: VaultsResponse = app.get("/v1/vaults", Some(&a)).await.json();
    assert_eq!(mine.vaults.len(), 1);
    assert_eq!(mine.vaults[0].vault_id, v.id);
    assert_eq!(mine.vaults[0].generation, KeyGeneration::FIRST);
    assert_eq!(mine.vaults[0].head_seq, 2);
    let theirs: VaultsResponse = app.get("/v1/vaults", Some(&b)).await.json();
    assert_eq!(theirs.vaults.len(), 0);

    // Metadata sealed under another generation than declared is refused.
    let bad = TestVault::new();
    let mut req = bad.create_request();
    req.generation = KeyGeneration::new(2).unwrap();
    let r = app
        .put(&format!("/v1/vaults/{}", bad.id), Some(&a), &req)
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // Garbage metadata too.
    let mut req = bad.create_request();
    req.meta = B64(vec![1, 2, 3]);
    assert_eq!(
        app.put(&format!("/v1/vaults/{}", bad.id), Some(&a), &req)
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
}

async fn rotated_vault(app: &TestApp, token: &[u8; 32]) -> (TestVault, TestVault) {
    // `g1` and `g2` share the vault ID; `g2` holds the generation-2 key.
    let g1 = TestVault::new();
    assert_eq!(
        app.create_vault(token, &g1).await.status,
        StatusCode::CREATED
    );
    let g2 = TestVault {
        id: g1.id,
        vk: VaultKey::generate(KeyGeneration::new(2).unwrap()).unwrap(),
        device: g1.device,
    };
    (g1, g2)
}

#[tokio::test(start_paused = true)]
async fn upload_below_or_above_generation_refused() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let (g1, g2) = rotated_vault(&app, &t).await;
    let id = ItemId::new_v7().unwrap();

    let mismatch = |r: crate::common::Resp, current: u32| {
        assert_eq!(r.status, StatusCode::CONFLICT);
        assert_eq!(r.code(), "generation_mismatch");
        assert_eq!(
            r.error()
                .detail_as::<GenerationDetail>()
                .unwrap()
                .current_generation,
            current
        );
    };
    // Above: sealed under generation 2 while the vault is at 1.
    mismatch(
        app.push(&t, &g2, vec![g2.item_record(id, "x", None)]).await,
        1,
    );
    // The declared generation must match too.
    let mut lie = app
        .post(
            &format!("/v1/vaults/{}/records", g1.id),
            Some(&t),
            &json::PushRecordsRequest {
                generation: KeyGeneration::new(2).unwrap(),
                records: vec![g1.item_record(id, "x", None)],
            },
        )
        .await;
    mismatch(
        std::mem::replace(
            &mut lie,
            crate::common::Resp {
                status: StatusCode::OK,
                headers: axum::http::HeaderMap::default(),
                body: vec![],
            },
        ),
        1,
    );
    // Fine at generation 1; then rotate.
    assert_eq!(
        app.push(&t, &g1, vec![g1.item_record(id, "x", None)])
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(put_key(&app, &t, &g1, 2, None).await.status, StatusCode::OK);
    // Below: a generation-1 record after the rotation.
    let other = ItemId::new_v7().unwrap();
    mismatch(
        app.push(&t, &g1, vec![g1.item_record(other, "y", None)])
            .await,
        2,
    );
    // One bad record fails the whole batch: the good one is not stored.
    let good = ItemId::new_v7().unwrap();
    mismatch(
        app.post(
            &format!("/v1/vaults/{}/records", g1.id),
            Some(&t),
            &json::PushRecordsRequest {
                generation: KeyGeneration::new(2).unwrap(),
                records: vec![
                    g2.item_record(good, "ok", None),
                    g1.item_record(other, "old", None),
                ],
            },
        )
        .await,
        2,
    );
    let page: ChangesResponse = app.get(&changes_uri(&g1, 0, 500), Some(&t)).await.json();
    assert_eq!(page.records.len(), 1, "only the first item");
    // Generation 2 works.
    assert_eq!(
        app.push(&t, &g2, vec![g2.item_record(good, "ok", None)])
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test(start_paused = true)]
async fn base_seq_conflict_and_new_record_conflict() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    let id = ItemId::new_v7().unwrap();

    let results = |r: crate::common::Resp| -> Vec<RecordResult> {
        assert_eq!(
            r.status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&r.body)
        );
        r.json::<PushRecordsResponse>().results
    };
    // New record: accepted at the next sequence after the key (1) and meta (2).
    let r = results(app.push(&t, &v, vec![v.item_record(id, "v1", None)]).await);
    assert_eq!(
        r,
        vec![RecordResult::Accepted {
            record_id: id,
            server_seq: 3
        }]
    );
    // A second "new" record with the same ID conflicts and reports the stored sequence.
    let r = results(app.push(&t, &v, vec![v.item_record(id, "dup", None)]).await);
    assert_eq!(
        r,
        vec![RecordResult::Conflict {
            record_id: id,
            current_seq: 3
        }]
    );
    // The right base sequence is accepted, a stale one conflicts.
    let r = results(
        app.push(&t, &v, vec![v.item_record(id, "v2", Some(3))])
            .await,
    );
    assert_eq!(
        r,
        vec![RecordResult::Accepted {
            record_id: id,
            server_seq: 4
        }]
    );
    let r = results(
        app.push(&t, &v, vec![v.item_record(id, "v3", Some(3))])
            .await,
    );
    assert_eq!(
        r,
        vec![RecordResult::Conflict {
            record_id: id,
            current_seq: 4
        }]
    );
    // A base sequence for an absent record conflicts with 0.
    let ghost = ItemId::new_v7().unwrap();
    let r = results(
        app.push(&t, &v, vec![v.item_record(ghost, "g", Some(2))])
            .await,
    );
    assert_eq!(
        r,
        vec![RecordResult::Conflict {
            record_id: ghost,
            current_seq: 0
        }]
    );
    // A tombstone replaces the item under the same CAS; a delete race loses.
    let r = results(
        app.push(&t, &v, vec![v.tombstone_record(id, Some(4))])
            .await,
    );
    assert_eq!(
        r,
        vec![RecordResult::Accepted {
            record_id: id,
            server_seq: 5
        }]
    );
    let r = results(
        app.push(&t, &v, vec![v.item_record(id, "late edit", Some(4))])
            .await,
    );
    assert_eq!(
        r,
        vec![RecordResult::Conflict {
            record_id: id,
            current_seq: 5
        }]
    );
    // Mixed batch: results in request order, sequence numbers only for accepted ones.
    let fresh = ItemId::new_v7().unwrap();
    let r = results(
        app.push(
            &t,
            &v,
            vec![
                v.item_record(id, "x", Some(1)),
                v.item_record(fresh, "n", None),
            ],
        )
        .await,
    );
    assert_eq!(
        r,
        vec![
            RecordResult::Conflict {
                record_id: id,
                current_seq: 5
            },
            RecordResult::Accepted {
                record_id: fresh,
                server_seq: 6
            },
        ]
    );
    // Duplicate IDs in one batch, garbage and a wrong kind are refused.
    let dup = app
        .push(
            &t,
            &v,
            vec![
                v.item_record(fresh, "a", Some(6)),
                v.item_record(fresh, "b", Some(6)),
            ],
        )
        .await;
    assert_eq!(dup.status, StatusCode::BAD_REQUEST);
    let mut junk = v.item_record(ghost, "g", None);
    junk.ciphertext = B64(vec![0; 40]);
    assert_eq!(
        app.push(&t, &v, vec![junk]).await.status,
        StatusCode::BAD_REQUEST
    );
    let mut wrong_kind = v.item_record(ghost, "g", None);
    wrong_kind.kind = json::RecordKind::Tombstone;
    // An item sealed as a tombstone: the headers differ, so the generation read from it is wrong.
    let r = app.push(&t, &v, vec![wrong_kind]).await;
    assert_eq!(r.code(), "generation_mismatch");
}

#[tokio::test(start_paused = true)]
async fn one_key_per_generation() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let (g1, _) = rotated_vault(&app, &t).await;

    let ok = put_key(&app, &t, &g1, 2, None).await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.json::<PutKeyResponse>().server_seq, 3);
    // A second key for generation 2 (even the same bytes) is "taken".
    let r = put_key(&app, &t, &g1, 2, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "generation_taken");
    assert_eq!(
        r.error()
            .detail_as::<GenerationDetail>()
            .unwrap()
            .current_generation,
        2
    );
    // Generation 1 is taken too; skipping a generation is a mismatch.
    assert_eq!(
        put_key(&app, &t, &g1, 1, None).await.code(),
        "generation_taken"
    );
    let skip = put_key(&app, &t, &g1, 4, None).await;
    assert_eq!(skip.code(), "generation_mismatch");
    assert_eq!(
        put_key(&app, &t, &g1, 0, None).await.status,
        StatusCode::BAD_REQUEST
    );

    // The keys come back in the feed, with the device that made them.
    let page: ChangesResponse = app.get(&changes_uri(&g1, 0, 500), Some(&t)).await.json();
    assert_eq!(page.generation.get(), 2);
    let gens: Vec<u32> = page.keys.iter().map(|k| k.generation.get()).collect();
    assert_eq!(gens, vec![1, 2]);
    assert!(page.keys.iter().all(|k| k.device_id == g1.device));
    // The database holds exactly one row per (vault, account, generation).
    let rows: i64 = app
        .state
        .db()
        .run(|c| {
            Ok::<_, ApiError>(c.query_row("SELECT COUNT(*) FROM vault_keys", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(rows, 2);
}

#[tokio::test(start_paused = true)]
async fn key_cas_head_moved_and_generation_taken() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let (g1, _) = rotated_vault(&app, &t).await;
    app.push_new_items(&t, &g1, 3).await; // head 5

    // The head moved since the caller last looked.
    let r = put_key(&app, &t, &g1, 2, Some(4)).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "head_moved");
    assert_eq!(
        r.error().detail_as::<HeadMovedDetail>().unwrap().head_seq,
        5
    );
    // Conditional on the right head: accepted.
    assert_eq!(
        put_key(&app, &t, &g1, 2, Some(5)).await.status,
        StatusCode::OK
    );
    // Another device that rotated from the same state loses: taken.
    let mut other = TestVault {
        id: g1.id,
        vk: VaultKey::generate(KeyGeneration::new(2).unwrap()).unwrap(),
        device: InstallId::new_v7().unwrap(),
    };
    other.device = InstallId::new_v7().unwrap();
    let r = put_key(&app, &t, &other, 2, Some(5)).await;
    assert_eq!(r.code(), "generation_taken");
    // After the rotation the head is 6 and a stale condition for the next one moves.
    let r = put_key(&app, &t, &g1, 3, Some(5)).await;
    assert_eq!(r.code(), "head_moved");
    assert_eq!(
        r.error().detail_as::<HeadMovedDetail>().unwrap().head_seq,
        6
    );
}

#[tokio::test(start_paused = true)]
async fn changes_paging_ordered_complete_and_keys_in_every_page() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let (g1, g2) = rotated_vault(&app, &t).await;
    let mut all = Vec::new();
    all.extend(app.push_new_items(&t, &g1, 7).await);
    // Rotate: the key lands after 7 records (sequence 10).
    assert_eq!(
        put_key(&app, &t, &g1, 2, None)
            .await
            .json::<PutKeyResponse>()
            .server_seq,
        10
    );
    all.extend(app.push_new_items(&t, &g2, 8).await);

    let mut since = 0u64;
    let mut seen = Vec::new();
    let mut pages = 0;
    loop {
        let page: ChangesResponse = app.get(&changes_uri(&g1, since, 4), Some(&t)).await.json();
        pages += 1;
        assert!(page.records.len() <= 4);
        // Keys newer than `since` come with every page, not only the first.
        if since < 10 {
            assert!(
                page.keys.iter().any(|k| k.generation.get() == 2),
                "page at {since}"
            );
        } else {
            assert!(page.keys.is_empty(), "no old keys at {since}");
        }
        assert_eq!(page.meta.is_some(), since < 2);
        for r in &page.records {
            assert!(r.server_seq > since);
            seen.push((r.record_id, r.server_seq));
        }
        if page.more {
            assert_eq!(page.next_since, page.records.last().unwrap().server_seq);
            since = page.next_since;
        } else {
            assert_eq!(page.next_since, page.head_seq);
            break;
        }
    }
    assert_eq!(pages, 4);
    assert_eq!(seen, all, "every record once, in sequence order");
    assert!(seen.windows(2).all(|w| w[0].1 < w[1].1));

    // Argument checks.
    for q in ["limit=0", "limit=501", "limit=x", "since=-1", "bogus=1"] {
        let r = app
            .get(&format!("/v1/vaults/{}/changes?{q}", g1.id), Some(&t))
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{q}");
    }
    // Nothing new: an empty page at the head.
    let head = seen.last().unwrap().1;
    let page: ChangesResponse = app.get(&changes_uri(&g1, head, 500), Some(&t)).await.json();
    assert!(page.records.is_empty() && !page.more && page.keys.is_empty() && page.meta.is_none());
}

#[tokio::test(start_paused = true)]
async fn meta_cas() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    let uri = format!("/v1/vaults/{}/meta", v.id);
    let req = |base: Option<u64>| PushMetaRequest {
        ciphertext: B64(v.meta()),
        base_seq: base,
    };

    // Created at sequence 2: the right base is accepted.
    let r: MetaResult = app.put(&uri, Some(&t), &req(Some(2))).await.json();
    assert_eq!(r, MetaResult::Accepted { server_seq: 3 });
    // The old base and "new" both conflict with the stored sequence.
    let r: MetaResult = app.put(&uri, Some(&t), &req(Some(2))).await.json();
    assert_eq!(r, MetaResult::Conflict { current_seq: 3 });
    let r: MetaResult = app.put(&uri, Some(&t), &req(None)).await.json();
    assert_eq!(r, MetaResult::Conflict { current_seq: 3 });
    // The feed shows the newest metadata, only when it is newer than `since`.
    let page: ChangesResponse = app.get(&changes_uri(&v, 2, 10), Some(&t)).await.json();
    assert_eq!(page.meta.unwrap().server_seq, 3);
    // After a rotation, metadata under the old generation is refused.
    put_key(&app, &t, &v, 2, None).await;
    let r = app.put(&uri, Some(&t), &req(Some(3))).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "generation_mismatch");
    let g2 = TestVault {
        id: v.id,
        vk: VaultKey::generate(KeyGeneration::new(2).unwrap()).unwrap(),
        device: v.device,
    };
    let r: MetaResult = app
        .put(
            &uri,
            Some(&t),
            &PushMetaRequest {
                ciphertext: B64(g2.meta()),
                base_seq: Some(3),
            },
        )
        .await
        .json();
    assert!(matches!(r, MetaResult::Accepted { .. }));
    // Garbage is refused.
    let r = app
        .put(
            &uri,
            Some(&t),
            &PushMetaRequest {
                ciphertext: B64(vec![1; 10]),
                base_seq: None,
            },
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

fn checkpoint_bytes(
    crypto: &KeyedCrypto,
    device: InstallId,
    counter: u64,
    vault: VaultId,
) -> Vec<u8> {
    Checkpoint {
        device_id: device,
        counter,
        issued_at_ms: T0,
        entries: vec![CheckpointEntry {
            vault_id: vault,
            generation: KeyGeneration::FIRST,
            high_seq: 3,
        }],
    }
    .encode(crypto)
    .unwrap()
}

#[tokio::test(start_paused = true)]
async fn checkpoint_monotonic_and_path_bound() {
    let app = app();
    let (u, t) = app.user("alice").await;
    let ak = AccountKey::generate().unwrap();
    let crypto = KeyedCrypto::new(&ak, u.account_id, u.device).unwrap();
    let vault = VaultId::new_v7().unwrap();
    let uri = format!("/v1/checkpoints/{}", u.device);
    let put = |bytes: Vec<u8>| app.send(Method::PUT, &uri, Some(&t), IP, bytes);

    assert_eq!(
        put(checkpoint_bytes(&crypto, u.device, 5, vault))
            .await
            .status,
        StatusCode::OK
    );
    // Same or lower counter: stale. Higher: accepted.
    for c in [5, 4, 0] {
        let r = put(checkpoint_bytes(&crypto, u.device, c, vault)).await;
        assert_eq!(
            (r.status, r.code().as_str()),
            (StatusCode::CONFLICT, "checkpoint_stale"),
            "{c}"
        );
    }
    let newest = checkpoint_bytes(&crypto, u.device, 6, vault);
    assert_eq!(put(newest.clone()).await.status, StatusCode::OK);

    // The body must name the device of the path.
    let other = InstallId::new_v7().unwrap();
    let other_crypto = KeyedCrypto::new(&ak, u.account_id, other).unwrap();
    let foreign = checkpoint_bytes(&other_crypto, other, 1, vault);
    assert_eq!(put(foreign.clone()).await.status, StatusCode::BAD_REQUEST);
    // A device cannot upload another device's checkpoint even if the path matches.
    let r = app
        .send(
            Method::PUT,
            &format!("/v1/checkpoints/{other}"),
            Some(&t),
            IP,
            foreign.clone(),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    // Shape: garbage, truncation and a flipped length are refused; oversized is 413.
    assert_eq!(put(vec![0; 10]).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        put(newest[..newest.len() - 1].to_vec()).await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        put(vec![0; json::MAX_CHECKPOINT_UPLOAD_LEN + 10])
            .await
            .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );

    // The list returns the stored bytes, unchanged.
    let list: CheckpointsResponse = app.get("/v1/checkpoints", Some(&t)).await.json();
    assert_eq!(list.checkpoints.len(), 1);
    assert_eq!(list.checkpoints[0].device_id, u.device);
    assert_eq!(list.checkpoints[0].body.0, newest);
    // A removed device's checkpoint is gone from the list.
    app.clock.advance_ms(13_000);
    let t2 = app.login_device(&u, other).await;
    let r = app
        .send(
            Method::PUT,
            &format!("/v1/checkpoints/{other}"),
            Some(&t2),
            IP,
            foreign,
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let list: CheckpointsResponse = app.get("/v1/checkpoints", Some(&t)).await.json();
    assert_eq!(list.checkpoints.len(), 2);
    assert_eq!(
        app.delete(&format!("/v1/devices/{other}"), Some(&t))
            .await
            .status,
        StatusCode::OK
    );
    let list: CheckpointsResponse = app.get("/v1/checkpoints", Some(&t)).await.json();
    assert_eq!(list.checkpoints.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn non_member_forbidden() {
    let app = app();
    let (_, a) = app.user("alice").await;
    let (_, b) = app.user("bob").await;
    let v = TestVault::new();
    app.create_vault(&a, &v).await;
    let att = AttachmentId::new_v7().unwrap();
    let item = ItemId::new_v7().unwrap();
    let base = format!("/v1/vaults/{}", v.id);
    let ct = v.attachment(item, att, 10);

    let forbidden = |what: &str, r: crate::common::Resp| {
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{what}");
        assert_eq!(r.code(), "forbidden", "{what}");
    };
    forbidden("changes", app.get(&changes_uri(&v, 0, 10), Some(&b)).await);
    forbidden(
        "records",
        app.push(&b, &v, vec![v.item_record(item, "x", None)]).await,
    );
    forbidden(
        "meta",
        app.put(
            &format!("{base}/meta"),
            Some(&b),
            &PushMetaRequest {
                ciphertext: B64(v.meta()),
                base_seq: Some(2),
            },
        )
        .await,
    );
    forbidden("key", put_key(&app, &b, &v, 2, None).await);
    forbidden(
        "attachment list",
        app.get(&format!("{base}/attachments"), Some(&b)).await,
    );
    forbidden(
        "attachment put",
        app.send(
            Method::PUT,
            &format!("{base}/attachments/{att}"),
            Some(&b),
            IP,
            ct.clone(),
        )
        .await,
    );
    forbidden(
        "attachment get",
        app.get(&format!("{base}/attachments/{att}"), Some(&b))
            .await,
    );
    forbidden(
        "attachment delete",
        app.delete(&format!("{base}/attachments/{att}"), Some(&b))
            .await,
    );
    forbidden(
        "attachment restore",
        app.post(
            &format!("{base}/attachments/{att}/restore"),
            Some(&b),
            &json::Empty {},
        )
        .await,
    );

    // The owner is unaffected, and an unknown vault is not found.
    assert_eq!(
        app.get(&changes_uri(&v, 0, 10), Some(&a)).await.status,
        StatusCode::OK
    );
    let unknown = TestVault::new();
    assert_eq!(
        app.get(&changes_uri(&unknown, 0, 10), Some(&a))
            .await
            .code(),
        "not_found"
    );
    assert_eq!(
        app.get("/v1/vaults/not-a-uuid/changes", Some(&a))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
}

fn att_uri(v: &TestVault, a: AttachmentId) -> String {
    format!("/v1/vaults/{}/attachments/{a}", v.id)
}

#[tokio::test(start_paused = true)]
async fn attachment_layout_validated() {
    let app = app();
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    let item = ItemId::new_v7().unwrap();
    let id = AttachmentId::new_v7().unwrap();
    let ct = v.attachment(item, id, 1000);
    let put = |id: AttachmentId, body: Vec<u8>| {
        let uri = att_uri(&v, id);
        let (app, t) = (&app, &t);
        async move { app.send(Method::PUT, &uri, Some(t), IP, body).await }
    };

    // Not a STREAM layout: refused, nothing stored.
    assert_eq!(put(id, vec![7; 100]).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(put(id, Vec::new()).await.status, StatusCode::BAD_REQUEST);
    // Shorter than the header, and a wrong header byte.
    assert_eq!(
        put(id, ct[..20].to_vec()).await.status,
        StatusCode::BAD_REQUEST
    );
    let mut wrong_header = ct.clone();
    wrong_header[0] ^= 0xff;
    assert_eq!(put(id, wrong_header).await.status, StatusCode::BAD_REQUEST);
    let list: AttachmentsResponse = app
        .get(&format!("/v1/vaults/{}/attachments", v.id), Some(&t))
        .await
        .json();
    assert_eq!(list.attachments.len(), 0);

    // A valid one is created once; the same size again is a no-op.
    assert_eq!(put(id, ct.clone()).await.status, StatusCode::CREATED);
    assert_eq!(put(id, ct.clone()).await.status, StatusCode::OK);
    // The same ID with another size is refused.
    let r = put(id, v.attachment(item, id, 2000)).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "attachment_exists");
    let got = app.get(&att_uri(&v, id), Some(&t)).await;
    assert_eq!(got.status, StatusCode::OK);
    assert_eq!(got.body, ct);
    assert_eq!(got.headers["content-type"], "application/octet-stream");
    let list: AttachmentsResponse = app
        .get(&format!("/v1/vaults/{}/attachments", v.id), Some(&t))
        .await
        .json();
    assert_eq!(list.attachments.len(), 1);
    assert_eq!(list.attachments[0].size, ct.len() as u64);
    assert_eq!(list.attachments[0].deleted_at, None);
    // Unknown attachment and bad ID.
    assert_eq!(
        app.get(&att_uri(&v, AttachmentId::new_v7().unwrap()), Some(&t))
            .await
            .code(),
        "not_found"
    );
    let bad = app
        .send(
            Method::PUT,
            &format!("/v1/vaults/{}/attachments/zz", v.id),
            Some(&t),
            IP,
            ct,
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test(start_paused = true)]
async fn attachment_soft_delete_restore_purge_after_30_days() {
    const DAY: u64 = 86_400_000;
    let app = app();
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    let id = AttachmentId::new_v7().unwrap();
    let ct = v.attachment(ItemId::new_v7().unwrap(), id, 500);
    assert_eq!(
        app.send(Method::PUT, &att_uri(&v, id), Some(&t), IP, ct.clone())
            .await
            .status,
        StatusCode::CREATED
    );
    let list = || async {
        app.get(&format!("/v1/vaults/{}/attachments", v.id), Some(&t))
            .await
            .json::<AttachmentsResponse>()
    };
    let hex = id.as_bytes().iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    let blob = app
        .dir
        .path()
        .join("blobs")
        .join(&hex[..2])
        .join(format!("{hex}.matt"));
    assert!(blob.is_file());

    // Soft delete: still listed with the time, still downloadable.
    assert_eq!(
        app.delete(&att_uri(&v, id), Some(&t)).await.status,
        StatusCode::OK
    );
    assert_eq!(list().await.attachments[0].deleted_at, Some(T0));
    assert_eq!(app.get(&att_uri(&v, id), Some(&t)).await.body, ct);
    // Restore.
    let r: RestoreResponse = app
        .post(
            &format!("{}/restore", att_uri(&v, id)),
            Some(&t),
            &json::Empty {},
        )
        .await
        .json();
    assert!(r.restored);
    assert_eq!(list().await.attachments[0].deleted_at, None);

    // Deleted again: kept for 30 days.
    app.clock.advance_ms(DAY);
    assert_eq!(
        app.delete(&att_uri(&v, id), Some(&t)).await.status,
        StatusCode::OK
    );
    app.clock.advance_ms(29 * DAY);
    app.state.maintain().unwrap();
    assert_eq!(
        list().await.attachments.len(),
        1,
        "29 days after the delete"
    );
    assert!(blob.is_file());
    // The re-assert path: an upload of the same bytes restores it.
    app.clock.advance_ms(DAY + 1);
    app.state.maintain().unwrap();
    assert!(list().await.attachments.is_empty(), "purged after 30 days");
    assert!(!blob.exists(), "the file is gone");
    assert_eq!(
        app.get(&att_uri(&v, id), Some(&t)).await.code(),
        "not_found"
    );
    let r: RestoreResponse = app
        .post(
            &format!("{}/restore", att_uri(&v, id)),
            Some(&t),
            &json::Empty {},
        )
        .await
        .json();
    assert!(!r.restored, "a purged attachment cannot be restored");
    // A holder can put it again.
    assert_eq!(
        app.send(Method::PUT, &att_uri(&v, id), Some(&t), IP, ct)
            .await
            .status,
        StatusCode::CREATED
    );
}

async fn poll(app: &TestApp, t: &[u8; 32], since: u64) -> crate::common::Resp {
    app.get(&format!("/v1/poll?since={since}"), Some(t)).await
}

#[tokio::test(start_paused = true)]
async fn poll_returns_on_change_and_after_25s() {
    let app = Arc::new(app());
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;

    // A `since` that differs from the counter returns at once.
    let base = poll(&app, &t, 0)
        .await
        .json::<PollResponse>()
        .change_counter;
    assert_ne!(base, 0);

    // Waiting poll, woken by a write.
    let waiting = tokio::spawn({
        let (app, t) = (app.clone(), t);
        async move { poll(&app, &t, base).await }
    });
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished(), "nothing changed: the poll waits");
    app.push_new_items(&t, &v, 1).await;
    let woken = waiting.await.unwrap();
    assert_eq!(woken.status, StatusCode::OK);
    let counter = woken.json::<PollResponse>().change_counter;
    assert_ne!(counter, base);

    // Waiting poll, no change: answered after 25 s with the same counter.
    let started = tokio::time::Instant::now();
    let idle = tokio::spawn({
        let (app, t) = (app.clone(), t);
        async move { poll(&app, &t, counter).await }
    });
    tokio::time::sleep(Duration::from_secs(24)).await;
    assert!(!idle.is_finished(), "still waiting at 24 s");
    tokio::time::sleep(Duration::from_secs(2)).await;
    let r = idle.await.unwrap();
    assert_eq!(r.json::<PollResponse>().change_counter, counter);
    assert!(started.elapsed() >= Duration::from_secs(25));

    // Another account's writes do not wake this account's poll.
    let (_, other) = app.user("bob").await;
    let waiting = tokio::spawn({
        let (app, t) = (app.clone(), t);
        async move { poll(&app, &t, counter).await }
    });
    tokio::task::yield_now().await;
    let bv = TestVault::new();
    app.create_vault(&other, &bv).await;
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    tokio::time::sleep(Duration::from_secs(26)).await;
    assert_eq!(
        waiting.await.unwrap().json::<PollResponse>().change_counter,
        counter
    );
}

#[tokio::test(start_paused = true)]
async fn poll_one_per_session() {
    let app = Arc::new(app());
    let (_, t) = app.user("alice").await;
    let counter = poll(&app, &t, 0)
        .await
        .json::<PollResponse>()
        .change_counter;

    let first = tokio::spawn({
        let (app, t) = (app.clone(), t);
        async move { poll(&app, &t, counter).await }
    });
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    assert!(!first.is_finished());

    // A second poll on the same session is refused and releases the first.
    let second = poll(&app, &t, counter).await;
    assert_eq!(second.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second.code(), "too_many_polls");
    let released = first.await.unwrap();
    assert_eq!(released.status, StatusCode::OK);
    assert_eq!(released.json::<PollResponse>().change_counter, counter);

    // After that, a new poll works again (another session also works at once).
    let again = tokio::spawn({
        let (app, t) = (app.clone(), t);
        async move { poll(&app, &t, counter).await }
    });
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    assert!(!again.is_finished());
    tokio::time::sleep(Duration::from_secs(26)).await;
    assert_eq!(again.await.unwrap().status, StatusCode::OK);
}

/// The whole stack over a real socket, with the peer address from
/// `ConnectInfo`.
#[tokio::test]
async fn serves_info_over_tcp() {
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let app = TestApp::new(RegistrationMode::Open);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service =
        mosaic_server::app(app.state.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let server = tokio::spawn(async move { axum::serve(listener, service).await });
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /v1/info HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert!(text.starts_with("HTTP/1.1 200"), "{text}");
    assert!(
        text.to_ascii_lowercase()
            .contains("cache-control: no-store")
    );
    assert!(text.contains("\"protocol\":1"));
    server.abort();
}

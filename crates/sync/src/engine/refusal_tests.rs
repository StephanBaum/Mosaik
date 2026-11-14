//! Regression tests of the red-team round 2 sync findings (SYNC-1, SYNC-2).
//! The engine runs against a folder transport that stands in for the hosted
//! backend and refuses what a hostile server refuses.

#![expect(clippy::unwrap_used, reason = "tests")]

use std::collections::BTreeSet;
use std::sync::Arc;

use mosaic_core::{AccountId, Argon2Profile, InstallId, KeyGeneration, MasterPassword, VaultId};
use mosaic_vault::session::{LockedSession, RevocationReason};
use mosaic_vault::storage::{PeerCheckpoint, PeerEntry, Store, VaultFlag};

use super::pull::RunStats;
use super::sharing::RotateOutcome;
use super::{EngineConfig, SyncEngine, SyncReport, SyncWarning};
use crate::folder::{self, FolderConfig, FolderTransport};
use crate::testutil::{
    FixedJitter, LocalSession, MaliciousTransport, ManualClock, RefusedSharing, RotateStub,
};
use crate::transport::{TransportError, TransportKind};
use crate::wire::SessionCrypto;
use crate::wire::device_info::{DeviceInfo, Platform};

type Hostile = MaliciousTransport<FolderTransport<SessionCrypto<LocalSession>>>;

struct Rig {
    _dir: tempfile::TempDir,
    access: LocalSession,
    engine: SyncEngine<Hostile>,
    vault: VaultId,
}

/// A device that owns a shared vault (the owner's own sharing row), over a
/// server that answers `refusal` to everything about the vault.
fn rig(refusal: TransportError) -> Rig {
    rig_with(Some(refusal), RefusedSharing::new(refusal))
}

/// [`rig`] with the vault calls refused only if `vault_refusal` is set, and
/// the sharing backend `sharing`.
fn rig_with(vault_refusal: Option<TransportError>, sharing: RefusedSharing) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("folder");
    std::fs::create_dir_all(&root).unwrap();
    folder::init(&root).unwrap();
    let store = Store::open(dir.path().join("device.db")).unwrap();
    let (session, _) = LockedSession::open(store)
        .create_account(
            &MasterPassword::new("correct horse battery staple".to_owned()),
            Argon2Profile::Baseline,
        )
        .unwrap();
    let access = LocalSession::new(session);
    let clock = ManualClock::new();
    let (account_id, install_id) = {
        let s = access.session().unwrap();
        (s.account_id(), s.install_id())
    };
    let transport = FolderTransport::open(FolderConfig {
        root,
        account_id,
        install_id,
        crypto: SessionCrypto::new(access.clone()).unwrap(),
        clock: Arc::new(clock),
        device: DeviceInfo {
            created_at_ms: 0,
            name: "owner".to_owned(),
            platform: Platform::Linux,
        },
    })
    .unwrap();
    let mut hostile = MaliciousTransport::new(transport);
    hostile.kind_override = Some(TransportKind::Hosted);
    hostile.refuse_vault_calls = vault_refusal;
    hostile.sharing_stub = Some(sharing);
    let engine = SyncEngine::with_jitter(
        hostile,
        EngineConfig::for_kind(TransportKind::Hosted),
        Box::new(FixedJitter(1000)),
    );
    let vault = {
        let mut s = access.session_mut().unwrap();
        let vault = s.create_vault("Shared").unwrap();
        s.ensure_identity_key().unwrap();
        s.ensure_owner_roster(vault).unwrap();
        vault
    };
    Rig {
        _dir: dir,
        access,
        engine,
        vault,
    }
}

impl Rig {
    fn lost(&self) -> bool {
        self.access
            .session()
            .unwrap()
            .vault_access_lost(self.vault)
            .unwrap()
    }

    fn flagged(&self) -> bool {
        self.access
            .session()
            .unwrap()
            .vault_flags()
            .unwrap()
            .contains(&(self.vault, VaultFlag::ServerInconsistent))
    }

    fn queue_revocation(&self) {
        let member = AccountId::new_random().unwrap();
        self.access
            .session_mut()
            .unwrap()
            .request_revocation(self.vault, &member, RevocationReason::Removed)
            .unwrap();
    }
}

const REFUSALS: [TransportError; 2] = [TransportError::Forbidden, TransportError::NotFound];

/// SYNC-1: a 403 or 404 on a pull of the owner's own vault never marks it
/// lost: it is a finding against the server and the vault is tried again.
#[test]
fn a_refused_pull_of_an_owned_vault_is_a_finding_not_a_loss() {
    for refusal in REFUSALS {
        let mut rig = rig(refusal);
        assert!(
            rig.access
                .session()
                .unwrap()
                .vault_shared(rig.vault)
                .unwrap()
        );
        let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
        rig.engine
            .pull_vault(&mut rig.access, rig.vault, &mut report, &mut stats)
            .unwrap();
        assert!(!rig.lost(), "{refusal:?}: the owner never loses the vault");
        assert!(rig.flagged(), "{refusal:?}: a finding against the server");
    }
}

/// SYNC-1: an older version marked the owner's vault lost; the mark is
/// healed and the vault is read again.
#[test]
fn a_loss_mark_on_an_owned_vault_is_healed() {
    let mut rig = rig(TransportError::Forbidden);
    rig.access
        .session_mut()
        .unwrap()
        .mark_access_lost(rig.vault)
        .unwrap();
    assert!(rig.lost());
    let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
    rig.engine
        .pull_phase(&mut rig.access, &mut report, &mut stats, 0)
        .unwrap();
    assert!(!rig.lost());
}

/// SYNC-1 (verifier note): the revocation rotation never returns silently
/// when the server refuses the member list: `RotationBlocked` and a persisted
/// finding, nothing marked lost, the revocation still queued.
#[test]
fn a_refused_rotation_of_an_owned_vault_is_blocked_and_visible() {
    for refusal in REFUSALS {
        let mut rig = rig(refusal);
        rig.queue_revocation();
        let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
        let rotated = rig
            .engine
            .rotate_shared(
                &mut rig.access,
                rig.vault,
                &BTreeSet::new(),
                false,
                &mut report,
                &mut stats,
            )
            .unwrap();
        assert_eq!(rotated, RotateOutcome::Blocked);
        assert!(
            report
                .warnings
                .contains(&SyncWarning::RotationBlocked { vault: rig.vault }),
            "{refusal:?}: {:?}",
            report.warnings
        );
        assert!(!rig.lost());
        assert!(rig.flagged());
        assert_eq!(
            rig.access
                .session()
                .unwrap()
                .pending_revocations()
                .unwrap()
                .len(),
            1,
            "the revocation stays queued"
        );
    }
}

fn rotate_once(rig: &mut Rig) -> (RotateOutcome, SyncReport) {
    let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
    let outcome = rig
        .engine
        .rotate_shared(
            &mut rig.access,
            rig.vault,
            &BTreeSet::new(),
            false,
            &mut report,
            &mut stats,
        )
        .unwrap();
    (outcome, report)
}

fn queued(rig: &Rig) -> usize {
    rig.access
        .session()
        .unwrap()
        .pending_revocations()
        .unwrap()
        .len()
}

/// FIX-2: another install rotated first (`GenerationTaken`) is benign: a
/// retry, no `RotationBlocked`, no finding, the revocation stays queued.
#[test]
fn a_generation_taken_rotation_is_a_retry_not_a_block() {
    let mut rig = rig_with(
        None,
        RefusedSharing::new(TransportError::Forbidden).with_rotate(RotateStub::GenerationTaken),
    );
    rig.queue_revocation();
    let (outcome, report) = rotate_once(&mut rig);
    assert_eq!(outcome, RotateOutcome::Retry, "{:?}", report.warnings);
    assert!(
        !report
            .warnings
            .iter()
            .any(|w| matches!(w, SyncWarning::RotationBlocked { .. })),
        "{:?}",
        report.warnings
    );
    assert!(!rig.flagged() && !rig.lost());
    assert_eq!(queued(&rig), 1, "the revocation stays queued");
}

/// FIX-2: the same through `process_revocations`, which no longer words a
/// retry as a block.
#[test]
fn a_generation_taken_revocation_run_gives_no_warning() {
    let mut rig = rig_with(
        None,
        RefusedSharing::new(TransportError::Forbidden).with_rotate(RotateStub::GenerationTaken),
    );
    rig.queue_revocation();
    let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
    rig.engine
        .process_revocations(&mut rig.access, &mut report, &mut stats, &[])
        .unwrap();
    assert!(
        !report
            .warnings
            .iter()
            .any(|w| matches!(w, SyncWarning::RotationBlocked { .. })),
        "{:?}",
        report.warnings
    );
    assert_eq!(queued(&rig), 1);
}

/// FIX-2 (missing branch): the rotation rejected with `role_forbidden`, which
/// an honest server never answers the owner, is `Blocked` with a persisted
/// finding, nothing marked lost, the revocation queued.
#[test]
fn a_role_forbidden_rotation_of_an_owned_vault_is_blocked() {
    let mut rig = rig_with(
        None,
        RefusedSharing::new(TransportError::Forbidden).with_rotate(RotateStub::RoleForbidden),
    );
    rig.queue_revocation();
    let (outcome, report) = rotate_once(&mut rig);
    assert_eq!(outcome, RotateOutcome::Blocked);
    assert!(
        report
            .warnings
            .contains(&SyncWarning::RotationBlocked { vault: rig.vault }),
        "{:?}",
        report.warnings
    );
    assert!(rig.flagged() && !rig.lost());
    assert_eq!(queued(&rig), 1);
}

/// FIX-2: a refused roster upload of an owned vault is a finding against the
/// server, reported as `ServerInconsistent`, not as a blocked rotation.
#[test]
fn a_refused_roster_upload_is_not_worded_as_a_rotation() {
    for refusal in REFUSALS {
        let mut rig = rig_with(None, RefusedSharing::new(refusal));
        assert!(
            rig.access
                .session()
                .unwrap()
                .pending_roster_upload(rig.vault)
                .unwrap()
                .is_some(),
            "the owner roster waits for its upload"
        );
        let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
        rig.engine
            .push_pending_rosters(&mut rig.access, &mut report, &mut stats)
            .unwrap();
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, SyncWarning::RotationBlocked { .. })),
            "{refusal:?}: {:?}",
            report.warnings
        );
        assert!(
            report
                .warnings
                .contains(&SyncWarning::ServerInconsistent { vault: rig.vault }),
            "{refusal:?}: {:?}",
            report.warnings
        );
        assert!(rig.flagged() && !rig.lost());
    }
}

/// SYNC-1 (verifier note): `process_revocations` does not `continue` silently
/// on a lost member list.
#[test]
fn a_queued_revocation_is_not_skipped_silently() {
    for refusal in REFUSALS {
        let mut rig = rig(refusal);
        rig.queue_revocation();
        let (mut report, mut stats) = (SyncReport::default(), RunStats::default());
        rig.engine
            .process_revocations(&mut rig.access, &mut report, &mut stats, &[])
            .unwrap();
        assert!(
            report
                .warnings
                .contains(&SyncWarning::RotationBlocked { vault: rig.vault }),
            "{refusal:?}: {:?}",
            report.warnings
        );
        assert!(rig.flagged());
        assert!(!rig.lost());
    }
}

/// SYNC-2: a peer checkpoint announcing a newer key generation than this
/// device holds is lag even when the cursors are equal: the warning fires
/// after the grace, not before.
#[test]
fn a_newer_generation_announced_by_a_peer_is_lag_with_the_same_grace() {
    let mut rig = rig(TransportError::Forbidden);
    let peer = InstallId::new_random().unwrap();
    let held = rig
        .access
        .session()
        .unwrap()
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == rig.vault)
        .unwrap()
        .key_generation;
    rig.access
        .session_mut()
        .unwrap()
        .put_peer_checkpoint(&PeerCheckpoint {
            device_id: peer,
            counter: 1,
            issued_at_ms: 1,
            entries: vec![PeerEntry {
                vault_id: rig.vault,
                generation: held.next().unwrap(),
                // Nothing the cursor has not reached: only the generation.
                high_seq: 0,
                view_digest: [0; 32],
            }],
        })
        .unwrap();
    let grace = rig.engine.config.freeze_grace_ms;
    let (warnings, none_lagging) = rig.engine.evaluate_lag(&mut rig.access, 1_000).unwrap();
    assert!(warnings.is_empty() && !none_lagging, "the timer starts");
    let (warnings, _) = rig
        .engine
        .evaluate_lag(&mut rig.access, 1_000 + grace)
        .unwrap();
    assert!(warnings.is_empty(), "not yet: the grace is inclusive");
    let (warnings, _) = rig
        .engine
        .evaluate_lag(&mut rig.access, 1_000 + grace + 1)
        .unwrap();
    assert!(
        matches!(warnings.as_slice(), [SyncWarning::FreezeSuspected { device, .. }] if *device == peer),
        "{warnings:?}"
    );
}

/// SYNC-2: the same generation (or a lower one) is no lag.
#[test]
fn an_equal_or_older_generation_is_no_lag() {
    let mut rig = rig(TransportError::Forbidden);
    let peer = InstallId::new_random().unwrap();
    let held = rig
        .access
        .session()
        .unwrap()
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == rig.vault)
        .unwrap()
        .key_generation;
    assert_eq!(held, KeyGeneration::FIRST);
    rig.access
        .session_mut()
        .unwrap()
        .put_peer_checkpoint(&PeerCheckpoint {
            device_id: peer,
            counter: 1,
            issued_at_ms: 1,
            entries: vec![PeerEntry {
                vault_id: rig.vault,
                generation: held,
                high_seq: 0,
                view_digest: [0; 32],
            }],
        })
        .unwrap();
    let (warnings, none_lagging) = rig.engine.evaluate_lag(&mut rig.access, 1_000).unwrap();
    assert!(warnings.is_empty() && none_lagging);
}

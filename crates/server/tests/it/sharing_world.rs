//! Several accounts, each with devices, on one real server on `127.0.0.1`
//! (M6b): the world of the sharing simulations and the malicious-server
//! tests. Every device has its own store, session and sync engine (over the
//! HTTP transport, wrapped in a chaos transport that is calm unless a test
//! asks). Nothing sleeps: the server clock and the engines' clock are manual
//! clocks the helpers move.

#![allow(dead_code, reason = "each test module uses some of the helpers")]

use mosaic_core::model::{ItemType, Role, SecretField};
use mosaic_core::{
    AccountId, Argon2Profile, Fingerprint, InstallId, ItemId, KeyGeneration, SecretKey, VaultId,
};
use mosaic_sync::engine::{EngineConfig, InviteSentInfo, SyncEngine, SyncError, SyncReport};
use mosaic_sync::http::{HttpTransport, hosted};
use mosaic_sync::testutil::{
    ChaosConfig, ChaosTransport, FixedJitter, LocalSession, ManualClock, SimRng,
};
use mosaic_sync::transport::TransportKind;
use mosaic_vault::session::{InviteId, LockedSession};
use mosaic_vault::storage::Store;

use crate::hosted::{Dev, PASSWORD, connect, pw};
use crate::loopback::LoopbackServer;

/// One account and its devices.
pub struct Acct {
    pub email: String,
    pub id: AccountId,
    pub secret_key: SecretKey,
    pub devs: Vec<Dev>,
}

/// The world: one server, several accounts.
pub struct SWorld {
    pub tmp: tempfile::TempDir,
    pub accts: Vec<Acct>,
    pub clock: ManualClock,
    pub rng: SimRng,
    pub seed: u64,
    pub chaos: ChaosConfig,
    pub server: LoopbackServer,
}

impl SWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            tmp: tempfile::tempdir().unwrap(),
            accts: Vec::new(),
            clock: ManualClock::new(),
            rng: SimRng::new(seed ^ 0x5ca1e),
            seed,
            chaos: ChaosConfig::default(),
            server: LoopbackServer::start(),
        }
    }

    /// A world with accounts named A, B, C... each with one device.
    pub fn with_accounts(seed: u64, n: usize) -> Self {
        let mut w = Self::new(seed);
        for i in 0..n {
            let name = ["a", "b", "c", "d"][i];
            w.account(name);
        }
        w
    }

    /// Registers an account with one device (device 0) and logs in.
    pub fn account(&mut self, name: &str) -> usize {
        let a = self.accts.len();
        let store = Store::open(self.tmp.path().join(format!("acct-{a}-dev-0.db"))).unwrap();
        let device = store.install_id().unwrap();
        let (session, created) = LockedSession::open(store)
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
        let email = format!("{name}{}@example.com", self.seed);
        let boot = session.account_bootstrap().unwrap();
        let opw = session
            .opaque_password(&pw(PASSWORD), &created.secret_key)
            .unwrap();
        hosted::register(
            &self.server.base,
            &email,
            None,
            &boot,
            &opw,
            device,
            "device 0",
        )
        .unwrap();
        let id = session.account_id();
        let transport = self.login_transport(&email, id, device, "device 0", &opw);
        let engine = self.engine_over(transport, a * 16);
        self.accts.push(Acct {
            email,
            id,
            secret_key: created.secret_key,
            devs: vec![Dev {
                access: LocalSession::new(session),
                engine,
                last: SyncReport::default(),
                device,
                name: "device 0".to_owned(),
            }],
        });
        a
    }

    pub fn login_transport(
        &self,
        email: &str,
        id: AccountId,
        device: InstallId,
        name: &str,
        opw: &mosaic_core::OpaquePassword,
    ) -> HttpTransport {
        self.server.advance_ms(60_000);
        let grant = hosted::login(&self.server.base, email, id, opw, device, name).unwrap();
        connect(&self.server, &grant, device)
    }

    pub fn engine_over(
        &self,
        transport: HttpTransport,
        index: usize,
    ) -> SyncEngine<ChaosTransport<HttpTransport>> {
        SyncEngine::with_jitter(
            ChaosTransport::new(
                transport,
                self.seed ^ ((index as u64 + 1) * 0x9e37),
                self.chaos,
            ),
            EngineConfig::for_kind(TransportKind::Hosted),
            Box::new(FixedJitter(1000)),
        )
    }

    /// A new device of account `a` joins over the wire.
    pub fn join(&mut self, a: usize) -> usize {
        let d = self.accts[a].devs.len();
        let store = Store::open(self.tmp.path().join(format!("acct-{a}-dev-{d}.db"))).unwrap();
        let device = store.install_id().unwrap();
        let name = format!("device {d}");
        self.server.advance_ms(60_000);
        let (email, key) = (self.accts[a].email.clone(), &self.accts[a].secret_key);
        let joined =
            hosted::join_login(&self.server.base, &email, &pw(PASSWORD), key, device, &name)
                .unwrap();
        let account =
            hosted::join_fetch_account(&self.server.base, &joined.grant.token, &joined.keys)
                .unwrap();
        let session = LockedSession::open(store)
            .join_account_with_keys(&account.wrapped_ak, joined.keys)
            .map_err(|e| e.error)
            .unwrap();
        let transport = connect(&self.server, &joined.grant, device);
        let engine = self.engine_over(transport, a * 16 + d);
        self.accts[a].devs.push(Dev {
            access: LocalSession::new(session),
            engine,
            last: SyncReport::default(),
            device,
            name,
        });
        d
    }

    pub fn dev(&mut self, a: usize, d: usize) -> &mut Dev {
        &mut self.accts[a].devs[d]
    }

    pub fn id(&self, a: usize) -> AccountId {
        self.accts[a].id
    }

    pub fn email(&self, a: usize) -> String {
        self.accts[a].email.clone()
    }

    /// One sync run that must work.
    pub fn try_sync(&mut self, a: usize, d: usize) -> Result<SyncReport, SyncError> {
        self.server.advance_ms(60_000);
        let clock = self.clock.clone();
        let dev = &mut self.accts[a].devs[d];
        let report = dev.engine.sync_once(&mut dev.access, &clock)?;
        dev.last = report.clone();
        Ok(report)
    }

    pub fn sync(&mut self, a: usize, d: usize) -> SyncReport {
        let faulty = self.chaos.fail_before_permille > 0
            || self.chaos.lost_response_permille > 0
            || self.chaos.duplicate_permille > 0;
        for _ in 0..400 {
            match self.try_sync(a, d) {
                Ok(report) => return report,
                Err(SyncError::Backoff { retry_at_ms }) if faulty => {
                    let now = mosaic_sync::clock::Clock::now_ms(&self.clock);
                    self.clock.advance_ms(retry_at_ms.saturating_sub(now) + 1);
                }
                Err(SyncError::Transport(_)) if faulty => {}
                Err(e) => panic!("account {a} device {d} failed to sync: {e:?}"),
            }
        }
        panic!("account {a} device {d} never got through");
    }

    /// Every device of every account syncs once, in random order.
    pub fn sync_all(&mut self) {
        let mut all: Vec<(usize, usize)> = Vec::new();
        for (a, acct) in self.accts.iter().enumerate() {
            all.extend((0..acct.devs.len()).map(|d| (a, d)));
        }
        self.rng.shuffle(&mut all);
        for (a, d) in all {
            self.sync(a, d);
        }
    }

    /// Passes until two in a row change nothing anywhere.
    pub fn converge(&mut self) {
        let mut idle = 0;
        for _ in 0..40 {
            let mut busy = false;
            let mut all: Vec<(usize, usize)> = Vec::new();
            for (a, acct) in self.accts.iter().enumerate() {
                all.extend((0..acct.devs.len()).map(|d| (a, d)));
            }
            self.rng.shuffle(&mut all);
            for (a, d) in all {
                let r = self.sync(a, d);
                busy |= !crate::hosted::is_idle(&r)
                    || !r.rotated_vaults.is_empty()
                    || !r.claimed_vaults.is_empty();
            }
            if busy {
                idle = 0;
            } else {
                idle += 1;
                if idle == 2 {
                    return;
                }
            }
        }
        panic!("no convergence after 40 passes");
    }

    // ---- sharing helpers ---------------------------------------------------

    /// `a`/`d` looks up `b`, compares codes (always a match here) and
    /// verifies the contact.
    pub fn verify(&mut self, a: usize, d: usize, b: usize) {
        let email = self.email(b);
        let dev = &mut self.accts[a].devs[d];
        let found = dev.engine.lookup_contact(&mut dev.access, &email).unwrap();
        let fpr = Fingerprint::of(&found.account_id, &found.ipk);
        dev.s_mut().verify_contact(&found.account_id, &fpr).unwrap();
    }

    /// Verified invite of `b` into `vault` by `a`/`d`.
    pub fn invite(
        &mut self,
        a: usize,
        d: usize,
        vault: VaultId,
        b: usize,
        role: Role,
    ) -> Result<InviteSentInfo, SyncError> {
        self.verify(a, d, b);
        let (id, email) = (self.id(b), self.email(b));
        let dev = &mut self.accts[a].devs[d];
        dev.engine
            .invite_member(&mut dev.access, vault, &id, &email, role)
    }

    /// The invite for `vault` addressed to `b`, as `b`/`d` lists it.
    pub fn incoming(&mut self, b: usize, d: usize, vault: VaultId) -> InviteId {
        let dev = &mut self.accts[b].devs[d];
        let (incoming, _) = dev.engine.list_invites(&mut dev.access).unwrap();
        incoming
            .into_iter()
            .find(|i| i.invite.vault_id == vault)
            .expect("an invite for the vault")
            .invite
            .invite_id
    }

    /// `b`/`d` previews, verifies the sender and accepts the invite to
    /// `vault`.
    pub fn accept(&mut self, b: usize, d: usize, vault: VaultId) {
        let invite = self.incoming(b, d, vault);
        let dev = &mut self.accts[b].devs[d];
        let preview = dev.engine.preview_invite(&mut dev.access, &invite).unwrap();
        let contact = dev
            .s()
            .contacts()
            .unwrap()
            .into_iter()
            .find(|c| c.account_id == preview.sender)
            .unwrap();
        dev.s_mut()
            .verify_contact(&preview.sender, &contact.fingerprint)
            .unwrap();
        let joined = dev.engine.accept_invite(&mut dev.access, &invite).unwrap();
        assert_eq!(joined, vault);
    }

    /// Signs device `d` of account `a` in again (a fresh login, which the
    /// grant and approve calls of emergency access need) and gives it a new
    /// engine. The old transport's session ends with the login.
    pub fn fresh(&mut self, a: usize, d: usize) {
        let acct = &self.accts[a];
        let opw = acct.devs[d]
            .s()
            .opaque_password(&pw(PASSWORD), &acct.secret_key)
            .unwrap();
        let (email, id) = (acct.email.clone(), acct.id);
        let (device, name) = (acct.devs[d].device, acct.devs[d].name.clone());
        let transport = self.login_transport(&email, id, device, &name, &opw);
        let engine = self.engine_over(transport, a * 16 + d);
        self.accts[a].devs[d].engine = engine;
    }

    /// The owner's explicit rotation of `vault` through the engine.
    pub fn rotate(
        &mut self,
        a: usize,
        d: usize,
        vault: VaultId,
    ) -> Result<mosaic_sync::engine::RotationOutcome, SyncError> {
        self.server.advance_ms(60_000);
        let clock = self.clock.clone();
        let dev = &mut self.accts[a].devs[d];
        dev.engine.rotate_vault(&mut dev.access, &clock, vault)
    }

    /// `a`/`d` removes `b` from the vault.
    pub fn remove(
        &mut self,
        a: usize,
        d: usize,
        vault: VaultId,
        b: usize,
    ) -> Result<Vec<mosaic_core::AttachmentId>, SyncError> {
        self.server.advance_ms(60_000);
        let id = self.id(b);
        let dev = &mut self.accts[a].devs[d];
        dev.engine.remove_member(&mut dev.access, vault, &id)
    }

    pub fn new_vault(&mut self, a: usize, d: usize, name: &str) -> VaultId {
        self.accts[a].devs[d].s_mut().create_vault(name).unwrap()
    }

    pub fn add_login(&mut self, a: usize, d: usize, vault: VaultId, title: &str) -> ItemId {
        self.accts[a].devs[d]
            .s_mut()
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_secret(SecretField::Password, &format!("pw of {title}"))
            })
            .unwrap()
    }

    pub fn set_password(&mut self, a: usize, d: usize, vault: VaultId, item: ItemId, v: &str) {
        self.accts[a].devs[d]
            .s_mut()
            .update_item(vault, item, |e| e.set_secret(SecretField::Password, v))
            .unwrap();
    }

    pub fn has(&self, a: usize, d: usize, vault: VaultId, item: ItemId) -> bool {
        self.accts[a].devs[d].s().item(vault, item).is_ok()
    }

    pub fn password(&self, a: usize, d: usize, vault: VaultId, item: ItemId) -> Option<String> {
        let s = self.accts[a].devs[d].s();
        let it = s.item(vault, item).ok()?;
        it.secret(SecretField::Password)
            .unwrap()
            .map(|p| p.expose_secret().to_owned())
    }

    pub fn generation(&self, a: usize, d: usize, vault: VaultId) -> KeyGeneration {
        self.accts[a].devs[d]
            .s()
            .vaults()
            .iter()
            .find(|v| v.vault_id == vault)
            .unwrap()
            .key_generation
    }

    pub fn has_vault(&self, a: usize, d: usize, vault: VaultId) -> bool {
        self.accts[a].devs[d]
            .s()
            .vaults()
            .iter()
            .any(|v| v.vault_id == vault)
    }

    /// Every record of `vault` on a device, sorted: `(item, ciphertext
    /// generation)`.
    pub fn record_generations(&self, a: usize, d: usize, vault: VaultId) -> Vec<(ItemId, u32)> {
        let s = self.accts[a].devs[d].s();
        let mut v: Vec<(ItemId, u32)> = s
            .store()
            .records(&vault)
            .unwrap()
            .into_iter()
            .filter_map(mosaic_vault::storage::RecordEntry::ok)
            .map(|r| (r.record_id, r.body.key_generation().get()))
            .collect();
        v.sort();
        v
    }
}

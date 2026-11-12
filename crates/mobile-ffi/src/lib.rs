//! The binding shell of the Mosaic mobile core (ADR-0089): UniFFI records,
//! enums and one object, `MosaicCore`, whose every method delegates in one line
//! to `mosaic_mobile::Core`. The generated Kotlin and Swift code is the only
//! consumer; nothing here is a Rust API for other crates.
//!
//! - There is **no logic** here. `scripts/mobile/check-ffi-crate.sh` keeps the
//!   crate to `lib.rs`, `types.rs`, `convert.rs` and `bin/uniffi-bindgen.rs`, and
//!   the `ffi_crate_has_no_logic` test caps `lib.rs`.
//! - There is **no hand-written low-level code**: UniFFI's scaffolding expands
//!   inside this crate and compiles under the workspace lints, which ban such
//!   code (UniFFI 0.32.2, probe a of T1).
//! - Every method blocks; the platform calls it off the main thread.
//! - Secrets come in as `Vec<u8>` and go out only as `SecretValue`.
//! - The global allocator wipes every freed heap block (R125), the
//!   `RustBuffer`s that carry arguments and return values included.
#![forbid(unsafe_code)]
#![expect(
    missing_docs,
    reason = "a binding shell: the mirrors document nothing the mosaic-mobile types do not"
)]
#![expect(
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    reason = "UniFFI hands owned values over and generates the exports"
)]

use std::alloc::System;
use std::sync::Arc;

use mosaic_mobile as m;
use zeroizing_alloc::ZeroAlloc;

pub mod convert;
pub mod types;

use convert::{Conv, ForeignStorage, lift};
pub use types::*;

uniffi::setup_scaffolding!("mosaic");

#[global_allocator]
static ALLOCATOR: ZeroAlloc<System> = ZeroAlloc(System);

/// The platform's secure storage (`docs/MOBILE.md`): implemented in Kotlin and
/// Swift. Rust decides the names and the lifetimes.
#[uniffi::export(with_foreign)]
pub trait SecureStorage: Send + Sync {
    fn get(&self, name: String) -> Result<Option<Vec<u8>>, StorageError>;
    /// Whether a value is stored under `name`, without reading or decrypting
    /// it: the core asks this when it only needs presence (G1-2).
    fn contains(&self, name: String) -> Result<bool, StorageError>;
    fn put(&self, name: String, value: Vec<u8>) -> Result<(), StorageError>;
    fn delete(&self, name: String) -> Result<(), StorageError>;
}

/// A secret value handed to the platform: `expose_bytes()` copies it out at the
/// last moment (the caller wipes the copy), `wipe()` zeroizes it.
#[derive(uniffi::Object)]
pub struct SecretValue {
    inner: Arc<m::SecretValue>,
}

impl SecretValue {
    pub(crate) const fn wrap(inner: Arc<m::SecretValue>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<m::SecretValue> {
        Arc::clone(&self.inner)
    }
}

#[uniffi::export]
impl SecretValue {
    pub fn expose_bytes(&self) -> Vec<u8> {
        self.inner.expose_bytes()
    }

    pub fn len(&self) -> u32 {
        u32::try_from(self.inner.len()).unwrap_or(u32::MAX)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn wipe(&self) {
        self.inner.wipe();
    }
}

impl core::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

#[cfg(test)]
impl PartialEq for SecretValue {
    fn eq(&self, other: &Self) -> bool {
        self.inner.expose_bytes() == other.inner.expose_bytes()
    }
}

/// The codes of every error, for the apps' tests.
#[uniffi::export]
pub fn mobile_error_codes() -> Vec<String> {
    m::error_codes().into_iter().map(str::to_owned).collect()
}

/// The version of the library.
#[uniffi::export]
pub fn core_version() -> String {
    m::core_version().to_owned()
}

/// The mobile core. One per process.
#[derive(uniffi::Object)]
pub struct MosaicCore {
    inner: m::Core,
}

#[uniffi::export]
impl MosaicCore {
    #[uniffi::constructor]
    pub fn open(
        config: CoreConfig,
        storage: Arc<dyn SecureStorage>,
    ) -> Result<Arc<Self>, MobileError> {
        m::Core::open(config.conv(), Arc::new(ForeignStorage(storage)))
            .map(|inner| Arc::new(Self { inner }))
            .map_err(MobileError::from)
    }

    pub fn state(&self) -> SessionState {
        self.inner.state().conv()
    }
    pub fn close_store(&self) {
        self.inner.close_store();
    }
    pub fn reopen_store(&self) -> Result<SessionState, MobileError> {
        lift(self.inner.reopen_store())
    }
    pub fn required_unlock_memory_bytes(&self) -> Option<u64> {
        self.inner.required_unlock_memory_bytes()
    }

    // ---- onboarding ---------------------------------------------------------------------------
    pub fn prelogin_check(&self, server_url: String) -> Result<ServerInfo, MobileError> {
        lift(self.inner.prelogin_check(server_url))
    }
    pub fn join_hosted(&self, req: JoinHostedRequest) -> Result<JoinOutcome, MobileError> {
        lift(self.inner.join_hosted(req.conv()))
    }
    pub fn join_totp(&self, code: Vec<u8>) -> Result<SessionState, MobileError> {
        lift(self.inner.join_totp(code))
    }
    pub fn join_webauthn_begin(&self) -> Result<HandoffInfo, MobileError> {
        lift(self.inner.join_webauthn_begin())
    }
    pub fn join_webauthn_status(&self) -> Result<HandoffStatus, MobileError> {
        lift(self.inner.join_webauthn_status())
    }
    pub fn join_cancel(&self) {
        self.inner.join_cancel();
    }
    pub fn parse_setup_code(&self, text: Vec<u8>) -> Result<SetupCode, MobileError> {
        lift(self.inner.parse_setup_code(text))
    }
    pub fn scan_setup_qr(&self, frame: LumaFrame) -> Result<Option<SetupCode>, MobileError> {
        lift(self.inner.scan_setup_qr(frame.conv()))
    }

    // ---- unlock -------------------------------------------------------------------------------
    pub fn unlock_password(
        &self,
        password: Vec<u8>,
        secret_key: Option<Vec<u8>>,
    ) -> Result<SessionState, MobileError> {
        lift(self.inner.unlock_password(password, secret_key))
    }
    pub fn unlock_quick(&self, w: Vec<u8>) -> Result<SessionState, MobileError> {
        lift(self.inner.unlock_quick(w))
    }
    pub fn lock(&self) {
        self.inner.lock();
    }
    pub fn quick_unlock_enroll_begin(
        &self,
        password: Vec<u8>,
    ) -> Result<QuickUnlockSetup, MobileError> {
        lift(self.inner.quick_unlock_enroll_begin(password))
    }
    pub fn quick_unlock_enroll_finish(
        &self,
        platform: QuickUnlockPlatform,
        body: Vec<u8>,
    ) -> Result<QuickUnlockEnrolled, MobileError> {
        lift(self.inner.quick_unlock_enroll_finish(platform.conv(), body))
    }
    pub fn quick_unlock_remove(&self) -> Result<Option<String>, MobileError> {
        lift(self.inner.quick_unlock_remove())
    }
    pub fn quick_unlock_platform_failed(&self) -> Result<Option<String>, MobileError> {
        lift(self.inner.quick_unlock_platform_failed())
    }

    // ---- vault --------------------------------------------------------------------------------
    pub fn vaults(&self) -> Result<Vec<VaultSummary>, MobileError> {
        lift(self.inner.vaults())
    }
    pub fn items(&self, filter: ItemFilter) -> Result<Vec<ItemSummary>, MobileError> {
        lift(self.inner.items(filter.conv()))
    }
    pub fn search(&self, query: String) -> Result<Vec<ItemSummary>, MobileError> {
        lift(self.inner.search(query))
    }
    pub fn item(&self, vault_id: String, item_id: String) -> Result<ItemDetail, MobileError> {
        lift(self.inner.item(&vault_id, &item_id))
    }
    pub fn reveal(
        &self,
        vault_id: String,
        item_id: String,
        field: String,
    ) -> Result<Arc<SecretValue>, MobileError> {
        lift(self.inner.reveal(&vault_id, &item_id, &field))
    }
    pub fn otp_code(&self, vault_id: String, item_id: String) -> Result<OtpCode, MobileError> {
        lift(self.inner.otp_code(&vault_id, &item_id))
    }
    pub fn save_login(&self, draft: LoginDraft) -> Result<ItemKey, MobileError> {
        lift(self.inner.save_login(draft.conv()))
    }
    pub fn save_note(&self, draft: NoteDraft) -> Result<ItemKey, MobileError> {
        lift(self.inner.save_note(draft.conv()))
    }
    pub fn trash_item(&self, vault_id: String, item_id: String) -> Result<(), MobileError> {
        lift(self.inner.trash_item(&vault_id, &item_id))
    }
    pub fn set_favorite(
        &self,
        vault_id: String,
        item_id: String,
        favorite: bool,
    ) -> Result<(), MobileError> {
        lift(self.inner.set_favorite(&vault_id, &item_id, favorite))
    }
    pub fn generate_password(
        &self,
        options: GeneratorOptions,
    ) -> Result<GeneratedSecret, MobileError> {
        lift(self.inner.generate_password(options.conv()))
    }
    pub fn otp_from_text(&self, text: Vec<u8>) -> Result<OtpPreview, MobileError> {
        lift(self.inner.otp_from_text(text))
    }
    pub fn scan_otp_qr(&self, frame: LumaFrame) -> Result<Option<OtpPreview>, MobileError> {
        lift(self.inner.scan_otp_qr(frame.conv()))
    }
    pub fn otp_pending_discard(&self, pending_id: String) {
        self.inner.otp_pending_discard(pending_id);
    }
    pub fn link_android_app(
        &self,
        vault_id: String,
        item_id: String,
        app: AndroidCaller,
    ) -> Result<(), MobileError> {
        lift(self.inner.link_android_app(&vault_id, &item_id, app.conv()))
    }
    pub fn asset_links_status(
        &self,
        vault_id: String,
        item_id: String,
        app: AndroidCaller,
    ) -> Result<AssetLinksStatus, MobileError> {
        lift(
            self.inner
                .asset_links_status(&vault_id, &item_id, app.conv()),
        )
    }

    // ---- sync ---------------------------------------------------------------------------------
    pub fn sync_overview(&self) -> SyncOverview {
        self.inner.sync_overview().conv()
    }
    pub fn sync_now(&self) -> Result<SyncOverview, MobileError> {
        lift(self.inner.sync_now())
    }
    pub fn recover_password(&self, new_password: Vec<u8>) -> Result<SyncOverview, MobileError> {
        lift(self.inner.recover_password(new_password))
    }
    /// Deviation from the plan: the result names the platform keys to delete.
    pub fn reset_device(&self, confirm: ResetConfirm) -> Result<ResetOutcome, MobileError> {
        lift(self.inner.reset_device(confirm.conv()))
    }

    // ---- autofill -----------------------------------------------------------------------------
    pub fn select_fill_fields(&self, nodes: Vec<FieldNode>) -> FieldSelection {
        self.inner.select_fill_fields(nodes.conv()).conv()
    }
    pub fn fill_target_android(&self, caller: AndroidCaller) -> Result<FillTarget, MobileError> {
        lift(self.inner.fill_target_android(caller.conv()))
    }
    pub fn fill_target_ios(
        &self,
        identifier: ServiceIdentifier,
    ) -> Result<FillTarget, MobileError> {
        lift(self.inner.fill_target_ios(identifier.conv()))
    }
    pub fn fill_warning(&self, target: FillTarget) -> Result<Option<OriginWarning>, MobileError> {
        lift(self.inner.fill_warning(target.conv()))
    }
    pub fn fill_candidates(&self, target: FillTarget) -> Result<FillCandidates, MobileError> {
        lift(self.inner.fill_candidates(target.conv()))
    }
    pub fn fill_values(
        &self,
        session_id: String,
        candidate: ItemKey,
    ) -> Result<FillValues, MobileError> {
        lift(self.inner.fill_values(session_id, candidate.conv()))
    }
    pub fn fill_otp(
        &self,
        session_id: String,
        candidate: ItemKey,
    ) -> Result<Arc<SecretValue>, MobileError> {
        lift(self.inner.fill_otp(session_id, candidate.conv()))
    }
    pub fn privileged_browsers_json(&self) -> String {
        self.inner.privileged_browsers_json()
    }

    // ---- passkeys -----------------------------------------------------------------------------
    pub fn passkey_android_get_begin(
        &self,
        req: AndroidPasskeyRequest,
    ) -> Result<PasskeySession, MobileError> {
        lift(self.inner.passkey_android_get_begin(req.conv()))
    }
    pub fn passkey_android_get_finish(
        &self,
        session_id: String,
        candidate: ItemKey,
        user_verified: bool,
    ) -> Result<String, MobileError> {
        lift(
            self.inner
                .passkey_android_get_finish(session_id, candidate.conv(), user_verified),
        )
    }
    pub fn passkey_android_create_begin(
        &self,
        req: AndroidPasskeyRequest,
    ) -> Result<PasskeyCreateSession, MobileError> {
        lift(self.inner.passkey_android_create_begin(req.conv()))
    }
    pub fn passkey_android_create_finish(
        &self,
        session_id: String,
        vault_id: String,
        user_verified: bool,
    ) -> Result<String, MobileError> {
        lift(
            self.inner
                .passkey_android_create_finish(session_id, vault_id, user_verified),
        )
    }
    pub fn passkey_ios_candidates(
        &self,
        rp_id: String,
        allow: Vec<Vec<u8>>,
    ) -> Result<Vec<PasskeyCandidate>, MobileError> {
        lift(self.inner.passkey_ios_candidates(rp_id, allow))
    }
    pub fn passkey_ios_assert(&self, req: IosAssertRequest) -> Result<IosAssertion, MobileError> {
        lift(self.inner.passkey_ios_assert(req.conv()))
    }
    pub fn passkey_ios_register(
        &self,
        req: IosRegisterRequest,
    ) -> Result<IosRegistration, MobileError> {
        lift(self.inner.passkey_ios_register(req.conv()))
    }
    pub fn passkey_identities(&self) -> Result<Vec<PasskeyIdentity>, MobileError> {
        lift(self.inner.passkey_identities())
    }
    pub fn password_identities(&self) -> Result<Vec<PasswordIdentity>, MobileError> {
        lift(self.inner.password_identities())
    }

    // ---- settings -----------------------------------------------------------------------------
    pub fn settings(&self) -> MobileSettings {
        self.inner.settings().conv()
    }
    pub fn update_settings(&self, update: SettingsUpdate) -> Result<MobileSettings, MobileError> {
        lift(self.inner.update_settings(update.conv()))
    }
}

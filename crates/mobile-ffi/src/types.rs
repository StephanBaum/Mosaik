//! The UniFFI mirrors of the `mosaic-mobile` types (ADR-0089): records and
//! enums that the generated Kotlin and Swift code uses. Field names and meanings
//! are those of `mosaic-mobile`, which documents them. Each declaration also
//! brings its conversions (`convert.rs`).

use std::path::PathBuf;
use std::sync::Arc;

use mosaic_mobile as m;

use crate::SecretValue;
use crate::convert::{Conv, record, record_out, unit_enum};

pub use crate::convert::MobileError;

/// The platform's secure storage failed. A foreign implementation that throws
/// anything else is mapped to this too.
#[derive(Clone, Copy, Debug, uniffi::Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum StorageError {
    /// The platform could not read, write or delete the value.
    Failed,
}

impl core::fmt::Display for StorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("storageUnavailable")
    }
}

impl std::error::Error for StorageError {}

impl From<uniffi::UnexpectedUniFFICallbackError> for StorageError {
    fn from(_: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Failed
    }
}

impl Conv<String> for PathBuf {
    fn conv(self) -> String {
        self.to_string_lossy().into_owned()
    }
}

impl Conv<PathBuf> for String {
    fn conv(self) -> PathBuf {
        PathBuf::from(self)
    }
}

// ---- construction -----------------------------------------------------------------------------

unit_enum!(Platform <=> m::Platform { Android, Ios });
unit_enum!(CoreContext <=> m::CoreContext { App, IosAutofillExtension });

record!(CoreConfig <=> m::CoreConfig {
    store_dir: String,
    platform: Platform,
    context: CoreContext,
    boot_id: Vec<u8>,
    trust_anchors_der: Vec<Vec<u8>>,
});

// ---- state ---------------------------------------------------------------------------------------

/// The platform row of a quick-unlock enrolment.
#[derive(Clone, Copy, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum QuickUnlockPlatform {
    /// iOS Secure Enclave.
    IosSecureEnclave,
    /// Android Keystore.
    AndroidKeystore {
        /// The key lives in `StrongBox`.
        strongbox: bool,
    },
}

/// Whether and how the phone can quick-unlock now.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum QuickUnlockAvailability {
    /// No enrolment.
    NotEnrolled,
    /// The master password is needed first.
    PasswordRequired,
    /// The platform decrypts `body` with `key_alias` (and `aad` on Android).
    Available {
        /// The platform row.
        platform: QuickUnlockPlatform,
        /// The platform key.
        key_alias: String,
        /// The public parameters the platform decrypts.
        body: Vec<u8>,
        /// The GCM AAD (Android only).
        aad: Vec<u8>,
    },
    /// The enrolment ran out; the platform deletes the key.
    Expired {
        /// The key to delete.
        key_alias: String,
    },
}

/// What the core is doing.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum SessionState {
    /// No account on this phone.
    NoAccount,
    /// An account, locked.
    Locked {
        /// The account email.
        email: Option<String>,
        /// The Secret Key is in secure storage.
        secret_key_stored: bool,
        /// Quick unlock.
        quick_unlock: QuickUnlockAvailability,
    },
    /// Unlocked.
    Unlocked {
        /// The account ID.
        account_id: String,
        /// The account email.
        email: Option<String>,
        /// A quick-unlock enrolment exists.
        quick_unlock_enrolled: bool,
    },
    /// The store is closed.
    StoreClosed,
    /// Another process holds the store.
    StoreBusy,
    /// A long call is running.
    Busy,
}

// ---- onboarding ------------------------------------------------------------------------------------

record!(ServerInfo <=> m::ServerInfo { origin: String, webauthn: bool });
record!(JoinHostedRequest <=> m::JoinHostedRequest {
    server_url: String,
    email: String,
    password: Vec<u8>,
    secret_key: Vec<u8>,
    device_name: Option<String>,
});
record!(SecondFactorMethods <=> m::SecondFactorMethods { totp: bool, webauthn: bool });
record!(HandoffInfo <=> m::HandoffInfo {
    url: String,
    match_code: String,
    expires_at_ms: u64,
});
record!(LumaFrame <=> m::LumaFrame {
    width: u32,
    height: u32,
    stride: u32,
    pixels: Vec<u8>,
});
record_out!(SetupCode <=> m::SetupCode {
    server: String,
    email: String,
    secret_key: Arc<SecretValue>,
});

/// How a join went.
#[derive(uniffi::Enum)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum JoinOutcome {
    /// Joined and unlocked.
    Unlocked {
        /// The state afterwards.
        state: SessionState,
    },
    /// The account needs a second factor.
    SecondFactor {
        /// What the account offers.
        methods: SecondFactorMethods,
    },
}

/// Where a security-key handoff stands.
#[derive(uniffi::Enum)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub enum HandoffStatus {
    /// Waiting for the key.
    Waiting,
    /// Done: the phone joined.
    Done {
        /// The state afterwards.
        state: SessionState,
    },
    /// The key was refused.
    Failed,
    /// The handoff ran out.
    Expired,
}

// ---- quick unlock ---------------------------------------------------------------------------------

record_out!(QuickUnlockSetup <=> m::QuickUnlockSetup {
    key_alias: String,
    key_nonce: Vec<u8>,
    aad: Vec<u8>,
    w: Arc<SecretValue>,
    expires_at_ms: u64,
});
record!(QuickUnlockEnrolled <=> m::QuickUnlockEnrolled {
    replaced_key_alias: Option<String>,
});

// ---- vault ------------------------------------------------------------------------------------------

unit_enum!(ItemKind <=> m::ItemKind {
    Login, Passkey, Totp, SecureNote, CreditCard, Identity, SshKey, ApiKey, Wifi, Document, Other
});
unit_enum!(TrashMode <=> m::TrashMode { Exclude, Only, Include });
unit_enum!(OtpScheme <=> m::OtpScheme { Totp, Hotp, Steam });

record!(VaultSummary <=> m::VaultSummary {
    vault_id: String,
    name: Option<String>,
    item_count: u32,
    available: bool,
});
record!(ItemFilter <=> m::ItemFilter {
    vault_id: Option<String>,
    trash: TrashMode,
    favorites_only: bool,
    kind: Option<ItemKind>,
});
record!(ItemKey <=> m::ItemKey { vault_id: String, item_id: String });
record!(ItemSummary <=> m::ItemSummary {
    vault_id: String,
    item_id: String,
    kind: ItemKind,
    title: String,
    subtitle: Option<String>,
    favorite: bool,
    has_otp: bool,
    has_passkey: bool,
    primary_host: Option<String>,
    updated_at_ms: i64,
    protected: bool,
});
record!(FieldView <=> m::FieldView {
    key: String,
    label_key: Option<String>,
    label: Option<String>,
    concealed: bool,
    value: Option<String>,
    copyable: bool,
});
record!(UrlView <=> m::UrlView {
    url: String,
    match_etld1: bool,
    allow_http: bool,
    is_app: bool,
    needs_linking: bool,
});
record!(UrlDraft <=> m::UrlDraft { url: String, match_etld1: bool, allow_http: bool });
record!(PasskeyView <=> m::PasskeyView {
    rp_id: String,
    user_name: String,
    display_name: String,
    created_at_ms: i64,
});
record!(AttachmentView <=> m::AttachmentView {
    attachment_id: String,
    file_name: String,
    size: u64,
});
record!(OtpView <=> m::OtpView {
    scheme: OtpScheme,
    digits: u8,
    period_s: Option<u32>,
    issuer: Option<String>,
    label: Option<String>,
});
record!(ItemDetail <=> m::ItemDetail {
    summary: ItemSummary,
    fields: Vec<FieldView>,
    urls: Vec<UrlView>,
    tags: Vec<String>,
    passkeys: Vec<PasskeyView>,
    attachments: Vec<AttachmentView>,
    otp: Option<OtpView>,
    history_count: u32,
});

/// What a login draft does with the generator.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum OtpEdit {
    /// Leave it.
    Keep,
    /// Remove it.
    Remove,
    /// Set it from a pending setup.
    SetPending {
        /// The pending ID.
        id: String,
    },
}

record!(LoginDraft <=> m::LoginDraft {
    vault_id: String,
    item_id: Option<String>,
    title: String,
    username: String,
    password: Option<Vec<u8>>,
    urls: Vec<UrlDraft>,
    notes: Option<Vec<u8>>,
    otp: OtpEdit,
    favorite: bool,
});
record!(NoteDraft <=> m::NoteDraft {
    vault_id: String,
    item_id: Option<String>,
    title: String,
    notes: Option<Vec<u8>>,
    favorite: bool,
});

// ---- one-time passwords and the generator ---------------------------------------------------------

record!(OtpPreview <=> m::OtpPreview {
    pending_id: String,
    scheme: OtpScheme,
    issuer: Option<String>,
    account_name: Option<String>,
    digits: u8,
    period_s: Option<u32>,
    expires_in_s: u32,
});
record_out!(OtpCode <=> m::OtpCode {
    code: Arc<SecretValue>,
    period_s: u32,
    remaining_s: u32,
    digits: u8,
});
unit_enum!(GeneratorMode <=> m::GeneratorMode { Password, Passphrase });
record!(GeneratorOptions <=> m::GeneratorOptions {
    mode: GeneratorMode,
    length: u32,
    lowercase: bool,
    uppercase: bool,
    digits: bool,
    symbols: bool,
    exclude_ambiguous: bool,
    words: u32,
    separator: String,
    capitalize: bool,
    append_digit: bool,
});
record_out!(GeneratedSecret <=> m::GeneratedSecret {
    value: Arc<SecretValue>,
    entropy_bits: f64,
});

// ---- sync ----------------------------------------------------------------------------------------------

record!(SyncOverview <=> m::SyncOverview {
    configured: bool,
    signed_in: bool,
    server_url: Option<String>,
    pending_uploads: u32,
    last_synced_at_ms: Option<u64>,
    last_error: Option<String>,
    halted: bool,
    warnings: Vec<SyncWarning>,
});
unit_enum!(SyncWarningKind <=> m::SyncWarningKind {
    FreezeSuspected,
    NoCommonView,
    RollbackDetected,
    IdentityDuplicated,
    ServerInconsistent,
    ServerRollback,
    Undecryptable,
    AccountRollback,
    SharingAttention,
});
record!(SyncWarning <=> m::SyncWarning {
    kind: SyncWarningKind,
    since_ms: Option<u64>,
    count: Option<u32>,
});
record!(ResetConfirm <=> m::ResetConfirm { typed_email: String });
record_out!(ResetOutcome <=> m::ResetOutcome {
    state: SessionState,
    key_aliases: Vec<String>,
});

// ---- autofill ---------------------------------------------------------------------------------------------

record!(AndroidCaller <=> m::AndroidCaller {
    package: String,
    current_signers: Vec<Vec<u8>>,
    web_domain: Option<String>,
    web_scheme: Option<String>,
});
unit_enum!(ServiceIdentifierKind <=> m::ServiceIdentifierKind { Domain, Url });
record!(ServiceIdentifier <=> m::ServiceIdentifier {
    kind: ServiceIdentifierKind,
    value: String,
});

/// What a fill is for.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum FillTargetKind {
    /// A web page.
    Web {
        /// The page origin.
        origin: String,
    },
    /// An Android app.
    AndroidApp {
        /// The package name.
        package: String,
        /// SHA-256 of each current signing certificate.
        certs: Vec<Vec<u8>>,
    },
}

/// Why the user is warned about the target.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum OriginWarning {
    /// The host has punycode labels.
    Punycode {
        /// The Unicode form.
        unicode: String,
    },
    /// The host imitates a saved host.
    Lookalike {
        /// The saved host.
        saved_host: String,
    },
}

record!(FillTarget <=> m::FillTarget {
    kind: FillTargetKind,
    display: String,
    warning: Option<OriginWarning>,
});
unit_enum!(FillMatch <=> m::FillMatch { Exact, Etld1 });
record!(FillCandidate <=> m::FillCandidate {
    key: ItemKey,
    title: String,
    username: Option<String>,
    match_kind: FillMatch,
    has_otp: bool,
});
record!(FillCandidates <=> m::FillCandidates {
    session_id: String,
    candidates: Vec<FillCandidate>,
});
record_out!(FillValues <=> m::FillValues {
    username: Option<String>,
    password: Option<Arc<SecretValue>>,
});
record!(Attr <=> m::Attr { name: String, value: String });
record!(FieldNode <=> m::FieldNode {
    id: u32,
    focused: bool,
    visible: bool,
    web_domain: Option<String>,
    input_type: u32,
    autofill_hints: Vec<String>,
    html_tag: Option<String>,
    html_attrs: Vec<Attr>,
    id_entry: Option<String>,
    hint: Option<String>,
});
unit_enum!(FieldRefusal <=> m::FieldRefusal { ForeignFrame, NoFillableField });
record!(FieldSelection <=> m::FieldSelection {
    username: Option<u32>,
    password: Option<u32>,
    new_password: Option<u32>,
    otp: Option<u32>,
    refused: Option<FieldRefusal>,
});
/// What the site's asset-links file says about an app.
#[derive(Clone, uniffi::Enum)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum AssetLinksStatus {
    /// A web domain of the login lists the app and its certificate.
    Verified {
        /// The host whose file lists the app.
        host: String,
    },
    /// The files were read and none lists the app.
    NotListed,
    /// No file could be read.
    Unavailable,
}

// ---- passkeys ------------------------------------------------------------------------------------------------

record!(AndroidPasskeyRequest <=> m::AndroidPasskeyRequest {
    request_json: String,
    caller: AndroidCaller,
    platform_origin: Option<String>,
    client_data_hash: Option<Vec<u8>>,
});
record!(PasskeyCandidate <=> m::PasskeyCandidate {
    key: ItemKey,
    rp_id: String,
    user_name: String,
    display_name: String,
    created_at_ms: i64,
});
record!(PasskeySession <=> m::PasskeySession {
    session_id: String,
    rp_id: String,
    candidates: Vec<PasskeyCandidate>,
});
record!(PasskeyCreateSession <=> m::PasskeyCreateSession {
    session_id: String,
    rp_id: String,
    rp_name: String,
    user_name: String,
    user_display_name: String,
    vaults: Vec<VaultSummary>,
});
record!(IosAssertRequest <=> m::IosAssertRequest {
    rp_id: String,
    client_data_hash: Vec<u8>,
    credential_id: Vec<u8>,
    user_verified: bool,
});
record!(IosAssertion <=> m::IosAssertion {
    credential_id: Vec<u8>,
    authenticator_data: Vec<u8>,
    signature: Vec<u8>,
    user_handle: Vec<u8>,
});
record!(IosRegisterRequest <=> m::IosRegisterRequest {
    rp_id: String,
    user_name: String,
    user_handle: Vec<u8>,
    client_data_hash: Vec<u8>,
    algorithms: Vec<i64>,
    exclude_credential_ids: Vec<Vec<u8>>,
    vault_id: String,
    user_verified: bool,
});
record!(IosRegistration <=> m::IosRegistration {
    credential_id: Vec<u8>,
    attestation_object: Vec<u8>,
});
record!(PasskeyIdentity <=> m::PasskeyIdentity {
    rp_id: String,
    user_name: String,
    credential_id: Vec<u8>,
    user_handle: Vec<u8>,
    record: ItemKey,
});
record!(PasswordIdentity <=> m::PasswordIdentity {
    domain: String,
    user_name: String,
    record: ItemKey,
});

// ---- settings -----------------------------------------------------------------------------------------------

record!(MobileSettings <=> m::MobileSettings {
    auto_lock_background_seconds: u32,
    idle_lock_minutes: u32,
    clipboard_clear_seconds: u32,
    quick_unlock_max_age_days: u32,
    passkey_identities: bool,
    password_identities: bool,
});
record!(SettingsUpdate <=> m::SettingsUpdate {
    auto_lock_background_seconds: Option<u32>,
    idle_lock_minutes: Option<u32>,
    clipboard_clear_seconds: Option<u32>,
    quick_unlock_max_age_days: Option<u32>,
    passkey_identities: Option<bool>,
    password_identities: Option<bool>,
    reauth_password: Option<Vec<u8>>,
});

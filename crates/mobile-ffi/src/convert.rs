//! Conversions between the `mosaic-mobile` types and their UniFFI mirrors
//! (ADR-0089). The mirrors in `types.rs` are declared through the macros below,
//! so every record and every fieldless enum gets both directions from one line
//! per field; the few enums with data and the error are written out.

use std::sync::Arc;

use mosaic_mobile as m;
use zeroize::Zeroizing;

use crate::types as f;

/// A value and its counterpart across the binding.
pub trait Conv<T> {
    /// Converts, moving the value.
    fn conv(self) -> T;
}

macro_rules! identity {
    ($($t:ty),*) => {$(
        impl Conv<$t> for $t {
            fn conv(self) -> $t {
                self
            }
        }
    )*};
}
identity!(String, bool, u8, u32, u64, i32, i64, f64, ());

impl<T: Conv<U>, U> Conv<Vec<U>> for Vec<T> {
    fn conv(self) -> Vec<U> {
        self.into_iter().map(Conv::conv).collect()
    }
}

impl<T: Conv<U>, U> Conv<Option<U>> for Option<T> {
    fn conv(self) -> Option<U> {
        self.map(Conv::conv)
    }
}

/// A record: the mirror struct (with its UniFFI derive) and both conversions.
macro_rules! record {
    ($name:ident <=> $mobile:path { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(uniffi::Record)]
        #[cfg_attr(test, derive(Clone, Debug, PartialEq))]
        pub struct $name {
            $(pub $field: $ty),*
        }

        impl Conv<$name> for $mobile {
            fn conv(self) -> $name {
                $name { $($field: self.$field.conv()),* }
            }
        }

        impl Conv<$mobile> for $name {
            fn conv(self) -> $mobile {
                $mobile { $($field: self.$field.conv()),* }
            }
        }
    };
}

/// A record that only leaves Rust: the mirror and the conversion to it.
macro_rules! record_out {
    ($name:ident <=> $mobile:path { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(uniffi::Record)]
        #[cfg_attr(test, derive(Clone, Debug, PartialEq))]
        pub struct $name {
            $(pub $field: $ty),*
        }

        impl Conv<$name> for $mobile {
            fn conv(self) -> $name {
                $name { $($field: self.$field.conv()),* }
            }
        }
    };
}

/// An enum without data.
macro_rules! unit_enum {
    ($name:ident <=> $mobile:path { $($variant:ident),* $(,)? }) => {
        #[derive(Clone, Copy, uniffi::Enum)]
        #[cfg_attr(test, derive(Debug, PartialEq))]
        pub enum $name {
            $($variant),*
        }

        impl Conv<$name> for $mobile {
            fn conv(self) -> $name {
                match self {
                    $(<$mobile>::$variant => $name::$variant),*
                }
            }
        }

        impl Conv<$mobile> for $name {
            fn conv(self) -> $mobile {
                match self {
                    $($name::$variant => <$mobile>::$variant),*
                }
            }
        }
    };
}

pub(crate) use {record, record_out, unit_enum};

impl Conv<Arc<crate::SecretValue>> for Arc<m::SecretValue> {
    fn conv(self) -> Arc<crate::SecretValue> {
        Arc::new(crate::SecretValue::wrap(self))
    }
}

impl Conv<Arc<m::SecretValue>> for Arc<crate::SecretValue> {
    fn conv(self) -> Arc<m::SecretValue> {
        self.inner()
    }
}

// ---- the error --------------------------------------------------------------------------------

macro_rules! mobile_error_mirror {
    ($($variant:ident),* $(,)?) => {
        /// Why a call failed. Unit variants only: an error carries no message, ID
        /// or value. `code()` of `mosaic-mobile` is the camelCase string.
        #[derive(Clone, Copy, Debug, uniffi::Error)]
        #[cfg_attr(test, derive(PartialEq))]
        pub enum MobileError {
            $($variant),*
        }

        impl From<m::MobileError> for MobileError {
            fn from(e: m::MobileError) -> Self {
                match e {
                    $(m::MobileError::$variant => Self::$variant,)*
                    _ => Self::Internal,
                }
            }
        }

        impl From<MobileError> for m::MobileError {
            fn from(e: MobileError) -> Self {
                match e {
                    $(MobileError::$variant => Self::$variant),*
                }
            }
        }
    };
}

mobile_error_mirror!(
    Internal,
    Busy,
    StoreInUse,
    StoreUnavailable,
    NoAccount,
    AccountExists,
    Locked,
    EmptyPassword,
    WrongCredentials,
    SecretKeyRequired,
    SecretKeyMalformed,
    SecretKeyMistyped,
    PasswordRequired,
    QuickUnlockExpired,
    QuickUnlockInvalidated,
    ReauthRequired,
    NotFound,
    InvalidInput,
    VaultUnavailable,
    ItemTypeNotAllowed,
    OtpQrNotOtp,
    OtpQrMigration,
    OtpQrMultiple,
    OtpQrInvalid,
    OtpUriInvalid,
    OtpKeyInvalid,
    SyncNotConfigured,
    SyncSignedOut,
    SyncTwoFactorRequired,
    SyncTwoFactorInvalid,
    SyncHandoffExpired,
    SyncRateLimited,
    SyncPasswordChanged,
    SyncAccountRekeyed,
    SyncFailed,
    SyncServerUnreachable,
    SyncServerInvalid,
    SyncServerInsecure,
    SyncServerMismatch,
    SyncServerIncompatible,
    PasswordInApp,
    FolderAccountUnsupported,
    SetupCodeInvalid,
    NotEnoughMemory,
    NoMatch,
    InsecureOrigin,
    AppNotLinked,
    ForeignFrame,
    AssetLinksUnavailable,
    AppNotVerified,
    RpIdInvalid,
    PasskeyNotFound,
    PasskeyExcluded,
    PasskeyUnsupportedAlgorithm,
    PasskeySessionExpired,
    QrNotFound,
    StorageUnavailable,
);

impl core::fmt::Display for MobileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(m::MobileError::from(*self).code())
    }
}

impl std::error::Error for MobileError {}

/// `last_error` of a sync overview crosses as its code.
impl Conv<String> for m::MobileError {
    fn conv(self) -> String {
        self.code().to_owned()
    }
}

impl Conv<m::MobileError> for String {
    fn conv(self) -> m::MobileError {
        m::MobileError::ALL
            .iter()
            .copied()
            .find(|e| e.code() == self)
            .unwrap_or(m::MobileError::Internal)
    }
}

/// A call's result across the binding.
pub fn lift<T: Conv<U>, U>(result: m::Result<T>) -> Result<U, MobileError> {
    result.map(Conv::conv).map_err(MobileError::from)
}

/// The password bytes of a test hook as zeroizing text; invalid UTF-8 wipes
/// the bytes and is `invalidInput`.
#[cfg(feature = "test-hooks")]
pub fn password_text(bytes: Vec<u8>) -> Result<Zeroizing<String>, MobileError> {
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Zeroizing::new(text)),
        Err(e) => {
            drop(Zeroizing::new(e.into_bytes()));
            Err(MobileError::InvalidInput)
        }
    }
}

// ---- the platform's secure storage -------------------------------------------------------------

/// The platform's `SecureStorage` as the core's trait. Values come back in
/// zeroizing buffers.
pub struct ForeignStorage(pub Arc<dyn crate::SecureStorage>);

impl m::SecureStorage for ForeignStorage {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, m::StorageError> {
        self.0
            .get(name.to_owned())
            .map(|value| value.map(Zeroizing::new))
            .map_err(|_| m::StorageError)
    }

    fn contains(&self, name: &str) -> Result<bool, m::StorageError> {
        self.0
            .contains(name.to_owned())
            .map_err(|_| m::StorageError)
    }

    fn put(&self, name: &str, value: &[u8]) -> Result<(), m::StorageError> {
        self.0
            .put(name.to_owned(), value.to_vec())
            .map_err(|_| m::StorageError)
    }

    fn delete(&self, name: &str) -> Result<(), m::StorageError> {
        self.0.delete(name.to_owned()).map_err(|_| m::StorageError)
    }
}

// ---- enums with data --------------------------------------------------------------------------

impl Conv<f::QuickUnlockPlatform> for m::QuickUnlockPlatform {
    fn conv(self) -> f::QuickUnlockPlatform {
        match self {
            Self::IosSecureEnclave => f::QuickUnlockPlatform::IosSecureEnclave,
            Self::AndroidKeystore { strongbox } => {
                f::QuickUnlockPlatform::AndroidKeystore { strongbox }
            }
        }
    }
}

impl Conv<m::QuickUnlockPlatform> for f::QuickUnlockPlatform {
    fn conv(self) -> m::QuickUnlockPlatform {
        match self {
            Self::IosSecureEnclave => m::QuickUnlockPlatform::IosSecureEnclave,
            Self::AndroidKeystore { strongbox } => {
                m::QuickUnlockPlatform::AndroidKeystore { strongbox }
            }
        }
    }
}

impl Conv<f::QuickUnlockAvailability> for m::QuickUnlockAvailability {
    fn conv(self) -> f::QuickUnlockAvailability {
        match self {
            Self::NotEnrolled => f::QuickUnlockAvailability::NotEnrolled,
            Self::PasswordRequired => f::QuickUnlockAvailability::PasswordRequired,
            Self::Available {
                platform,
                key_alias,
                body,
                aad,
            } => f::QuickUnlockAvailability::Available {
                platform: platform.conv(),
                key_alias,
                body,
                aad,
            },
            Self::Expired { key_alias } => f::QuickUnlockAvailability::Expired { key_alias },
        }
    }
}

impl Conv<f::SessionState> for m::SessionState {
    fn conv(self) -> f::SessionState {
        match self {
            Self::NoAccount => f::SessionState::NoAccount,
            Self::Locked {
                email,
                secret_key_stored,
                quick_unlock,
            } => f::SessionState::Locked {
                email,
                secret_key_stored,
                quick_unlock: quick_unlock.conv(),
            },
            Self::Unlocked {
                account_id,
                email,
                quick_unlock_enrolled,
            } => f::SessionState::Unlocked {
                account_id,
                email,
                quick_unlock_enrolled,
            },
            Self::StoreClosed => f::SessionState::StoreClosed,
            Self::StoreBusy => f::SessionState::StoreBusy,
            Self::Busy => f::SessionState::Busy,
        }
    }
}

impl Conv<f::JoinOutcome> for m::JoinOutcome {
    fn conv(self) -> f::JoinOutcome {
        match self {
            Self::Unlocked(state) => f::JoinOutcome::Unlocked {
                state: state.conv(),
            },
            Self::SecondFactor { methods } => f::JoinOutcome::SecondFactor {
                methods: methods.conv(),
            },
        }
    }
}

impl Conv<f::HandoffStatus> for m::HandoffStatus {
    fn conv(self) -> f::HandoffStatus {
        match self {
            Self::Waiting => f::HandoffStatus::Waiting,
            Self::Done(state) => f::HandoffStatus::Done {
                state: state.conv(),
            },
            Self::Failed => f::HandoffStatus::Failed,
            Self::Expired => f::HandoffStatus::Expired,
        }
    }
}

impl Conv<f::OtpEdit> for m::OtpEdit {
    fn conv(self) -> f::OtpEdit {
        match self {
            Self::Keep => f::OtpEdit::Keep,
            Self::Remove => f::OtpEdit::Remove,
            Self::SetPending { id } => f::OtpEdit::SetPending { id },
        }
    }
}

impl Conv<m::OtpEdit> for f::OtpEdit {
    fn conv(self) -> m::OtpEdit {
        match self {
            Self::Keep => m::OtpEdit::Keep,
            Self::Remove => m::OtpEdit::Remove,
            Self::SetPending { id } => m::OtpEdit::SetPending { id },
        }
    }
}

impl Conv<f::FillTargetKind> for m::FillTargetKind {
    fn conv(self) -> f::FillTargetKind {
        match self {
            Self::Web { origin } => f::FillTargetKind::Web { origin },
            Self::AndroidApp { package, certs } => f::FillTargetKind::AndroidApp { package, certs },
        }
    }
}

impl Conv<m::FillTargetKind> for f::FillTargetKind {
    fn conv(self) -> m::FillTargetKind {
        match self {
            Self::Web { origin } => m::FillTargetKind::Web { origin },
            Self::AndroidApp { package, certs } => m::FillTargetKind::AndroidApp { package, certs },
        }
    }
}

impl Conv<f::AssetLinksStatus> for m::AssetLinksStatus {
    fn conv(self) -> f::AssetLinksStatus {
        match self {
            Self::Verified { host } => f::AssetLinksStatus::Verified { host },
            Self::NotListed => f::AssetLinksStatus::NotListed,
            Self::Unavailable => f::AssetLinksStatus::Unavailable,
        }
    }
}

impl Conv<m::AssetLinksStatus> for f::AssetLinksStatus {
    fn conv(self) -> m::AssetLinksStatus {
        match self {
            Self::Verified { host } => m::AssetLinksStatus::Verified { host },
            Self::NotListed => m::AssetLinksStatus::NotListed,
            Self::Unavailable => m::AssetLinksStatus::Unavailable,
        }
    }
}

impl Conv<f::OriginWarning> for m::OriginWarning {
    fn conv(self) -> f::OriginWarning {
        match self {
            Self::Punycode { unicode } => f::OriginWarning::Punycode { unicode },
            Self::Lookalike { saved_host } => f::OriginWarning::Lookalike { saved_host },
        }
    }
}

impl Conv<m::OriginWarning> for f::OriginWarning {
    fn conv(self) -> m::OriginWarning {
        match self {
            Self::Punycode { unicode } => m::OriginWarning::Punycode { unicode },
            Self::Lookalike { saved_host } => m::OriginWarning::Lookalike { saved_host },
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    /// A value through the binding and back.
    fn rt<F, M>(value: F)
    where
        F: Conv<M> + Clone + PartialEq + Debug,
        M: Conv<F>,
    {
        let mobile: M = value.clone().conv();
        let back: F = mobile.conv();
        assert_eq!(back, value);
    }

    fn s(text: &str) -> String {
        text.to_owned()
    }

    fn key() -> f::ItemKey {
        f::ItemKey {
            vault_id: s("v"),
            item_id: s("i"),
        }
    }

    fn summary() -> f::ItemSummary {
        f::ItemSummary {
            vault_id: s("v"),
            item_id: s("i"),
            kind: f::ItemKind::Login,
            title: s("t"),
            subtitle: Some(s("u")),
            favorite: true,
            has_otp: true,
            has_passkey: false,
            primary_host: Some(s("example.com")),
            updated_at_ms: -5,
            protected: true,
        }
    }

    fn caller() -> f::AndroidCaller {
        f::AndroidCaller {
            package: s("com.example.app"),
            current_signers: vec![vec![1; 32], vec![2; 32]],
            web_domain: Some(s("example.com")),
            web_scheme: None,
        }
    }

    fn target() -> f::FillTarget {
        f::FillTarget {
            kind: f::FillTargetKind::AndroidApp {
                package: s("p"),
                certs: vec![vec![3; 32]],
            },
            display: s("p"),
            warning: Some(f::OriginWarning::Lookalike { saved_host: s("h") }),
        }
    }

    fn candidate() -> f::PasskeyCandidate {
        f::PasskeyCandidate {
            key: key(),
            rp_id: s("example.com"),
            user_name: s("alice"),
            display_name: s("Alice"),
            created_at_ms: 7,
        }
    }

    fn vault() -> f::VaultSummary {
        f::VaultSummary {
            vault_id: s("v"),
            name: None,
            item_count: 3,
            available: true,
        }
    }

    #[test]
    fn error_codes_unique_camel_case_and_complete() {
        let codes = crate::mobile_error_codes();
        assert_eq!(codes.len(), m::MobileError::ALL.len());
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "codes are unique");
        for code in &codes {
            let mut chars = code.chars();
            assert!(
                chars.next().is_some_and(|c| c.is_ascii_lowercase()),
                "{code}"
            );
            assert!(chars.all(|c| c.is_ascii_alphanumeric()), "{code}");
        }
        // Every error of the core has its mirror, and back; `code()` is the text.
        for error in m::MobileError::ALL {
            let mirror = MobileError::from(*error);
            assert_eq!(m::MobileError::from(mirror), *error);
            assert_eq!(mirror.to_string(), error.code());
            let code: String = (*error).conv();
            let back: m::MobileError = code.conv();
            assert_eq!(back, *error);
        }
        assert_eq!(
            Conv::<m::MobileError>::conv(s("noSuchCode")),
            m::MobileError::Internal
        );
        assert_eq!(f::StorageError::Failed.to_string(), "storageUnavailable");
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "one sample per record")]
    fn conversions_round_trip_for_every_record() {
        rt::<f::CoreConfig, m::CoreConfig>(f::CoreConfig {
            store_dir: s("/data/mosaic"),
            platform: f::Platform::Android,
            context: f::CoreContext::IosAutofillExtension,
            boot_id: vec![0, 0, 0, 7],
            trust_anchors_der: vec![vec![1, 2], vec![3]],
        });
        rt::<f::ServerInfo, m::ServerInfo>(f::ServerInfo {
            origin: s("https://sync.example.com"),
            webauthn: true,
        });
        rt::<f::JoinHostedRequest, m::JoinHostedRequest>(f::JoinHostedRequest {
            server_url: s("https://sync.example.com"),
            email: s("a@example.com"),
            password: vec![1, 2, 3],
            secret_key: vec![4, 5],
            device_name: Some(s("Pixel")),
        });
        rt::<f::SecondFactorMethods, m::SecondFactorMethods>(f::SecondFactorMethods {
            totp: true,
            webauthn: false,
        });
        rt::<f::HandoffInfo, m::HandoffInfo>(f::HandoffInfo {
            url: s("https://x/webauthn/#abc"),
            match_code: s("123456"),
            expires_at_ms: u64::MAX,
        });
        rt::<f::LumaFrame, m::LumaFrame>(f::LumaFrame {
            width: 2,
            height: 2,
            stride: 3,
            pixels: vec![9; 6],
        });
        rt::<f::QuickUnlockEnrolled, m::QuickUnlockEnrolled>(f::QuickUnlockEnrolled {
            replaced_key_alias: Some(s("old")),
        });
        rt::<f::VaultSummary, m::VaultSummary>(vault());
        rt::<f::ItemFilter, m::ItemFilter>(f::ItemFilter {
            vault_id: Some(s("v")),
            trash: f::TrashMode::Only,
            favorites_only: true,
            kind: Some(f::ItemKind::Passkey),
        });
        rt::<f::ItemKey, m::ItemKey>(key());
        rt::<f::ItemSummary, m::ItemSummary>(summary());
        rt::<f::FieldView, m::FieldView>(f::FieldView {
            key: s("login.password"),
            label_key: Some(s("field.login.password")),
            label: None,
            concealed: true,
            value: None,
            copyable: true,
        });
        rt::<f::UrlView, m::UrlView>(f::UrlView {
            url: s("androidapp://p"),
            match_etld1: true,
            allow_http: true,
            is_app: true,
            needs_linking: true,
        });
        rt::<f::UrlDraft, m::UrlDraft>(f::UrlDraft {
            url: s("https://example.com"),
            match_etld1: true,
            allow_http: false,
        });
        rt::<f::PasskeyView, m::PasskeyView>(f::PasskeyView {
            rp_id: s("example.com"),
            user_name: s("alice"),
            display_name: s("Alice"),
            created_at_ms: 9,
        });
        rt::<f::AttachmentView, m::AttachmentView>(f::AttachmentView {
            attachment_id: s("a"),
            file_name: s("f.pdf"),
            size: 1 << 40,
        });
        let otp_view = f::OtpView {
            scheme: f::OtpScheme::Steam,
            digits: 5,
            period_s: Some(30),
            issuer: Some(s("Steam")),
            label: None,
        };
        rt::<f::OtpView, m::OtpView>(otp_view.clone());
        rt::<f::ItemDetail, m::ItemDetail>(f::ItemDetail {
            summary: summary(),
            fields: Vec::new(),
            urls: Vec::new(),
            tags: vec![s("work")],
            passkeys: Vec::new(),
            attachments: Vec::new(),
            otp: Some(otp_view),
            history_count: 4,
        });
        for otp in [
            f::OtpEdit::Keep,
            f::OtpEdit::Remove,
            f::OtpEdit::SetPending { id: s("p") },
        ] {
            rt::<f::OtpEdit, m::OtpEdit>(otp.clone());
            rt::<f::LoginDraft, m::LoginDraft>(f::LoginDraft {
                vault_id: s("v"),
                item_id: None,
                title: s("t"),
                username: s("u"),
                password: Some(vec![1]),
                urls: vec![f::UrlDraft {
                    url: s("u"),
                    match_etld1: false,
                    allow_http: true,
                }],
                notes: Some(vec![]),
                otp,
                favorite: true,
            });
        }
        rt::<f::NoteDraft, m::NoteDraft>(f::NoteDraft {
            vault_id: s("v"),
            item_id: Some(s("i")),
            title: s("t"),
            notes: None,
            favorite: false,
        });
        rt::<f::OtpPreview, m::OtpPreview>(f::OtpPreview {
            pending_id: s("p"),
            scheme: f::OtpScheme::Hotp,
            issuer: None,
            account_name: Some(s("a")),
            digits: 8,
            period_s: None,
            expires_in_s: 600,
        });
        rt::<f::GeneratorOptions, m::GeneratorOptions>(f::GeneratorOptions {
            mode: f::GeneratorMode::Passphrase,
            length: 20,
            lowercase: true,
            uppercase: false,
            digits: true,
            symbols: false,
            exclude_ambiguous: true,
            words: 6,
            separator: s("-"),
            capitalize: true,
            append_digit: true,
        });
        rt::<f::SyncOverview, m::SyncOverview>(f::SyncOverview {
            configured: true,
            signed_in: true,
            server_url: Some(s("https://sync.example.com")),
            pending_uploads: 3,
            last_synced_at_ms: Some(1),
            last_error: Some(s("syncPasswordChanged")),
            halted: true,
        });
        rt::<f::ResetConfirm, m::ResetConfirm>(f::ResetConfirm {
            typed_email: s("a@example.com"),
        });
        rt::<f::AndroidCaller, m::AndroidCaller>(caller());
        rt::<f::ServiceIdentifier, m::ServiceIdentifier>(f::ServiceIdentifier {
            kind: f::ServiceIdentifierKind::Url,
            value: s("https://example.com"),
        });
        rt::<f::FillTarget, m::FillTarget>(target());
        rt::<f::FillCandidate, m::FillCandidate>(f::FillCandidate {
            key: key(),
            title: s("t"),
            username: None,
            match_kind: f::FillMatch::Etld1,
            has_otp: true,
        });
        rt::<f::FillCandidates, m::FillCandidates>(f::FillCandidates {
            session_id: s("0011"),
            candidates: vec![f::FillCandidate {
                key: key(),
                title: s("t"),
                username: Some(s("u")),
                match_kind: f::FillMatch::Exact,
                has_otp: false,
            }],
        });
        rt::<f::Attr, m::Attr>(f::Attr {
            name: s("autocomplete"),
            value: s("username"),
        });
        rt::<f::FieldNode, m::FieldNode>(f::FieldNode {
            id: 7,
            focused: true,
            visible: false,
            web_domain: Some(s("example.com")),
            input_type: 0x81,
            autofill_hints: vec![s("password")],
            html_tag: Some(s("input")),
            html_attrs: vec![f::Attr {
                name: s("a"),
                value: s("b"),
            }],
            id_entry: None,
            hint: Some(s("Password")),
        });
        rt::<f::FieldSelection, m::FieldSelection>(f::FieldSelection {
            username: Some(1),
            password: Some(2),
            new_password: None,
            otp: Some(4),
            refused: Some(f::FieldRefusal::ForeignFrame),
        });
        rt::<f::AndroidPasskeyRequest, m::AndroidPasskeyRequest>(f::AndroidPasskeyRequest {
            request_json: s("{}"),
            caller: caller(),
            platform_origin: Some(s("https://example.com")),
            client_data_hash: Some(vec![5; 32]),
        });
        rt::<f::PasskeyCandidate, m::PasskeyCandidate>(candidate());
        rt::<f::PasskeySession, m::PasskeySession>(f::PasskeySession {
            session_id: s("s"),
            rp_id: s("example.com"),
            candidates: vec![candidate()],
        });
        rt::<f::PasskeyCreateSession, m::PasskeyCreateSession>(f::PasskeyCreateSession {
            session_id: s("s"),
            rp_id: s("example.com"),
            rp_name: s("Example"),
            user_name: s("alice"),
            user_display_name: s("Alice"),
            vaults: vec![vault()],
        });
        rt::<f::IosAssertRequest, m::IosAssertRequest>(f::IosAssertRequest {
            rp_id: s("example.com"),
            client_data_hash: vec![1; 32],
            credential_id: vec![2],
            user_verified: true,
        });
        rt::<f::IosAssertion, m::IosAssertion>(f::IosAssertion {
            credential_id: vec![1],
            authenticator_data: vec![2; 37],
            signature: vec![3; 70],
            user_handle: vec![4],
        });
        rt::<f::IosRegisterRequest, m::IosRegisterRequest>(f::IosRegisterRequest {
            rp_id: s("example.com"),
            user_name: s("alice"),
            user_handle: vec![1],
            client_data_hash: vec![2; 32],
            algorithms: vec![-7, -257],
            exclude_credential_ids: vec![vec![3]],
            vault_id: s("v"),
            user_verified: false,
        });
        rt::<f::IosRegistration, m::IosRegistration>(f::IosRegistration {
            credential_id: vec![1],
            attestation_object: vec![2; 90],
        });
        rt::<f::PasskeyIdentity, m::PasskeyIdentity>(f::PasskeyIdentity {
            rp_id: s("example.com"),
            user_name: s("alice"),
            credential_id: vec![1],
            user_handle: vec![2],
            record: key(),
        });
        rt::<f::PasswordIdentity, m::PasswordIdentity>(f::PasswordIdentity {
            domain: s("example.com"),
            user_name: s("alice"),
            record: key(),
        });
        rt::<f::MobileSettings, m::MobileSettings>(f::MobileSettings {
            auto_lock_background_seconds: 30,
            idle_lock_minutes: 5,
            clipboard_clear_seconds: 30,
            quick_unlock_max_age_days: 14,
            passkey_identities: true,
            password_identities: false,
        });
        rt::<f::SettingsUpdate, m::SettingsUpdate>(f::SettingsUpdate {
            auto_lock_background_seconds: Some(0),
            idle_lock_minutes: None,
            clipboard_clear_seconds: Some(10),
            quick_unlock_max_age_days: None,
            passkey_identities: Some(true),
            password_identities: None,
            reauth_password: Some(vec![1, 2]),
        });
    }

    #[test]
    fn conversions_round_trip_for_every_enum() {
        for v in [f::Platform::Android, f::Platform::Ios] {
            rt::<f::Platform, m::Platform>(v);
        }
        for v in [f::CoreContext::App, f::CoreContext::IosAutofillExtension] {
            rt::<f::CoreContext, m::CoreContext>(v);
        }
        for v in [
            f::ItemKind::Login,
            f::ItemKind::Passkey,
            f::ItemKind::Totp,
            f::ItemKind::SecureNote,
            f::ItemKind::CreditCard,
            f::ItemKind::Identity,
            f::ItemKind::SshKey,
            f::ItemKind::ApiKey,
            f::ItemKind::Wifi,
            f::ItemKind::Document,
            f::ItemKind::Other,
        ] {
            rt::<f::ItemKind, m::ItemKind>(v);
        }
        for v in [
            f::TrashMode::Exclude,
            f::TrashMode::Only,
            f::TrashMode::Include,
        ] {
            rt::<f::TrashMode, m::TrashMode>(v);
        }
        for v in [f::OtpScheme::Totp, f::OtpScheme::Hotp, f::OtpScheme::Steam] {
            rt::<f::OtpScheme, m::OtpScheme>(v);
        }
        for v in [f::GeneratorMode::Password, f::GeneratorMode::Passphrase] {
            rt::<f::GeneratorMode, m::GeneratorMode>(v);
        }
        for v in [
            f::ServiceIdentifierKind::Domain,
            f::ServiceIdentifierKind::Url,
        ] {
            rt::<f::ServiceIdentifierKind, m::ServiceIdentifierKind>(v);
        }
        for v in [f::FillMatch::Exact, f::FillMatch::Etld1] {
            rt::<f::FillMatch, m::FillMatch>(v);
        }
        for v in [
            f::FieldRefusal::ForeignFrame,
            f::FieldRefusal::NoFillableField,
        ] {
            rt::<f::FieldRefusal, m::FieldRefusal>(v);
        }
        for v in [
            f::AssetLinksStatus::Verified { host: s("h") },
            f::AssetLinksStatus::NotListed,
            f::AssetLinksStatus::Unavailable,
        ] {
            rt::<f::AssetLinksStatus, m::AssetLinksStatus>(v);
        }
        for v in [
            f::QuickUnlockPlatform::IosSecureEnclave,
            f::QuickUnlockPlatform::AndroidKeystore { strongbox: true },
            f::QuickUnlockPlatform::AndroidKeystore { strongbox: false },
        ] {
            rt::<f::QuickUnlockPlatform, m::QuickUnlockPlatform>(v);
        }
        for v in [
            f::FillTargetKind::Web {
                origin: s("https://example.com"),
            },
            f::FillTargetKind::AndroidApp {
                package: s("p"),
                certs: vec![vec![1; 32]],
            },
        ] {
            rt::<f::FillTargetKind, m::FillTargetKind>(v);
        }
        for v in [
            f::OriginWarning::Punycode {
                unicode: s("buecher"),
            },
            f::OriginWarning::Lookalike { saved_host: s("h") },
        ] {
            rt::<f::OriginWarning, m::OriginWarning>(v);
        }
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "one sample per record")]
    fn outgoing_records_and_states_convert() {
        let secret = Arc::new(m::SecretValue::from_text("s3cret"));
        let setup: f::SetupCode = m::SetupCode {
            server: s("https://sync.example.com"),
            email: s("a@example.com"),
            secret_key: Arc::clone(&secret),
        }
        .conv();
        assert_eq!(setup.secret_key.expose_bytes(), b"s3cret");
        assert_eq!(format!("{:?}", setup.secret_key), "SecretValue([REDACTED])");
        // The wrapper shares the value: wiping it wipes the core's.
        setup.secret_key.wipe();
        assert_eq!(secret.len(), 0);
        let enrolled: f::QuickUnlockSetup = m::QuickUnlockSetup {
            key_alias: s("alias"),
            key_nonce: vec![1; 16],
            aad: vec![2],
            w: Arc::new(m::SecretValue::from_text("w")),
            expires_at_ms: 5,
        }
        .conv();
        assert_eq!(enrolled.key_nonce, vec![1; 16]);
        assert_eq!(enrolled.w.len(), 1);
        let code: f::OtpCode = m::OtpCode {
            code: Arc::new(m::SecretValue::from_text("123456")),
            period_s: 30,
            remaining_s: 12,
            digits: 6,
        }
        .conv();
        assert_eq!((code.period_s, code.remaining_s, code.digits), (30, 12, 6));
        let generated: f::GeneratedSecret = m::GeneratedSecret {
            value: Arc::new(m::SecretValue::from_text("pw")),
            entropy_bits: 80.5,
        }
        .conv();
        assert!((generated.entropy_bits - 80.5).abs() < f64::EPSILON);
        let values: f::FillValues = m::FillValues {
            username: Some(s("alice")),
            password: Some(Arc::new(m::SecretValue::from_text("pw"))),
        }
        .conv();
        assert_eq!(values.password.map(|p| p.len()), Some(2));
        let state: f::SessionState = m::SessionState::Locked {
            email: Some(s("a@example.com")),
            secret_key_stored: true,
            quick_unlock: m::QuickUnlockAvailability::Available {
                platform: m::QuickUnlockPlatform::AndroidKeystore { strongbox: true },
                key_alias: s("k"),
                body: vec![1],
                aad: vec![2],
            },
        }
        .conv();
        assert_eq!(
            state,
            f::SessionState::Locked {
                email: Some(s("a@example.com")),
                secret_key_stored: true,
                quick_unlock: f::QuickUnlockAvailability::Available {
                    platform: f::QuickUnlockPlatform::AndroidKeystore { strongbox: true },
                    key_alias: s("k"),
                    body: vec![1],
                    aad: vec![2],
                },
            }
        );
        for (mobile, mirror) in [
            (m::SessionState::NoAccount, f::SessionState::NoAccount),
            (m::SessionState::StoreClosed, f::SessionState::StoreClosed),
            (m::SessionState::StoreBusy, f::SessionState::StoreBusy),
            (m::SessionState::Busy, f::SessionState::Busy),
        ] {
            assert_eq!(Conv::<f::SessionState>::conv(mobile), mirror);
        }
        let reset: f::ResetOutcome = m::ResetOutcome {
            state: m::SessionState::NoAccount,
            key_aliases: vec![s("a")],
        }
        .conv();
        assert_eq!(reset.state, f::SessionState::NoAccount);
        let outcome: f::JoinOutcome = m::JoinOutcome::SecondFactor {
            methods: m::SecondFactorMethods {
                totp: true,
                webauthn: true,
            },
        }
        .conv();
        assert_eq!(
            outcome,
            f::JoinOutcome::SecondFactor {
                methods: f::SecondFactorMethods {
                    totp: true,
                    webauthn: true
                }
            }
        );
        let done: f::HandoffStatus = m::HandoffStatus::Done(m::SessionState::StoreBusy).conv();
        assert_eq!(
            done,
            f::HandoffStatus::Done {
                state: f::SessionState::StoreBusy
            }
        );
        assert_eq!(
            Conv::<f::HandoffStatus>::conv(m::HandoffStatus::Waiting),
            f::HandoffStatus::Waiting
        );
    }

    #[test]
    fn ffi_crate_has_no_logic() {
        let lib = include_str!("lib.rs");
        assert!(lib.lines().count() < 700, "lib.rs is a delegating shell");
        // The sources hold no hand-written low-level code: the check script greps
        // them for the keyword, so the test builds it from two halves.
        let keyword = concat!("un", "safe");
        for source in [lib, include_str!("types.rs"), include_str!("convert.rs")] {
            let has_it = source.lines().any(|l| {
                l.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == keyword)
            });
            assert!(!has_it, "no low-level code");
        }
    }
}

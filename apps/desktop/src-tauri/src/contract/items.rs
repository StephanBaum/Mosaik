//! Items: list and search (summaries, never a secret), detail (secrets
//! masked), reveal (one secret, on request), edit, favorite, trash, restore,
//! purge and move.

use mosaic_core::model::{ItemType, SecretField, TextField};
use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireAttachmentId, WireItemId, WireVaultId};
use super::macros::wire_enum;
use super::secret::{SecretIn, SecretOut};

wire_enum!(
    /// The item types of brief §5.
    pub enum ItemKind {
        /// Username, password, URLs, optional one-time password.
        Login,
        /// A WebAuthn credential.
        Passkey,
        /// A one-time-password generator on its own.
        Totp,
        /// Notes only.
        SecureNote,
        /// A payment card.
        CreditCard,
        /// Personal details.
        Identity,
        /// An SSH key pair.
        SshKey,
        /// An API key or token.
        ApiKey,
        /// Wi-Fi credentials.
        Wifi,
        /// Files with notes.
        Document,
        /// A type written by a newer Mosaic; shown read-only.
        Unsupported,
    }
);

impl ItemKind {
    /// The kind of a model type.
    #[must_use]
    pub fn from_model(item_type: &ItemType) -> Self {
        match item_type {
            ItemType::Login => Self::Login,
            ItemType::Passkey => Self::Passkey,
            ItemType::Totp => Self::Totp,
            ItemType::SecureNote => Self::SecureNote,
            ItemType::CreditCard => Self::CreditCard,
            ItemType::Identity => Self::Identity,
            ItemType::SshKey => Self::SshKey,
            ItemType::ApiKey => Self::ApiKey,
            ItemType::Wifi => Self::Wifi,
            ItemType::Document => Self::Document,
            ItemType::Other(_) => Self::Unsupported,
        }
    }

    /// The model type for a new item; `None` for [`ItemKind::Unsupported`].
    #[must_use]
    pub const fn to_model(self) -> Option<ItemType> {
        Some(match self {
            Self::Login => ItemType::Login,
            Self::Passkey => ItemType::Passkey,
            Self::Totp => ItemType::Totp,
            Self::SecureNote => ItemType::SecureNote,
            Self::CreditCard => ItemType::CreditCard,
            Self::Identity => ItemType::Identity,
            Self::SshKey => ItemType::SshKey,
            Self::ApiKey => ItemType::ApiKey,
            Self::Wifi => ItemType::Wifi,
            Self::Document => ItemType::Document,
            Self::Unsupported => return None,
        })
    }
}

macro_rules! field_key {
    ($(#[$meta:meta])* $name:ident => $model:ident { $($variant:ident),* $(,)? }) => {
        wire_enum!(
            $(#[$meta])*
            pub enum $name {
                $(
                    #[doc = concat!("`", stringify!($model), "::", stringify!($variant), "`.")]
                    $variant
                ),*
            }
        );

        impl $name {
            /// The model field.
            #[must_use]
            pub const fn to_model(self) -> $model {
                match self {
                    $(Self::$variant => $model::$variant),*
                }
            }

            /// The wire key of a model field.
            #[must_use]
            pub const fn from_model(field: $model) -> Self {
                match field {
                    $($model::$variant => Self::$variant),*
                }
            }
        }
    };
}

field_key!(
    /// A plain text field of an item (`mosaic_core::model::TextField`).
    TextFieldKey => TextField {
        Title, Notes, Username, CardholderName, CardBrand, CardExpiry, FirstName, MiddleName,
        LastName, Email, Phone, Company, AddressLine1, AddressLine2, City, Region, PostalCode,
        Country, BirthDate, ApiKeyId, ApiKeyHost, WifiSsid, WifiSecurity, SshComment,
    }
);

field_key!(
    /// A secret field of an item (`mosaic_core::model::SecretField`).
    SecretFieldKey => SecretField {
        Password, CardNumber, CardVerificationCode, CardPin, PassportNumber, LicenseNumber,
        NationalId, ApiKeySecret, WifiPassword,
    }
);

wire_enum!(
    /// Which items a listing includes with respect to the trash.
    pub enum TrashFilter {
        /// Items outside the trash.
        Exclude,
        /// Only items in the trash.
        Only,
        /// Both.
        Include,
    }
);

wire_enum!(
    /// How a login URL is matched (brief §4).
    pub enum UrlMatchMode {
        /// Scheme, host and port equal (the default).
        ExactOrigin,
        /// Registrable domain equal (opt-in per URL).
        Etld1,
    }
);

wire_enum!(
    /// The kind of a custom field.
    pub enum CustomFieldKind {
        /// Plain text.
        Text,
        /// A secret, masked like a password.
        Hidden,
        /// A URL (never used for autofill).
        Url,
        /// An email address.
        Email,
        /// A phone number.
        Phone,
        /// A date, `YYYY-MM-DD`.
        Date,
        /// A one-time-password generator.
        Otp,
    }
);

wire_enum!(
    /// One-time-password schemes.
    pub enum OtpScheme {
        /// RFC 6238.
        Totp,
        /// RFC 4226.
        Hotp,
        /// Steam Guard.
        Steam,
    }
);

/// `list_items`: a filter and an optional search query.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListItemsRequest {
    /// Only this vault (`null`: every vault).
    pub vault_id: Option<WireVaultId>,
    /// Trash handling.
    pub trash: TrashFilter,
    /// Only favorites.
    pub favorites_only: bool,
    /// Search over title, user name, email, URLs and tags (`null` or empty:
    /// no search). Results are ranked by the core's search.
    pub query: Option<String>,
}

/// An item in a list. Never contains a secret.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummary {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The type.
    pub kind: ItemKind,
    /// The title (`null` if empty).
    pub title: Option<String>,
    /// The secondary line: user name, cardholder, network name, ... (`null`
    /// if the type has none or it is empty). Never derived from a secret.
    pub subtitle: Option<String>,
    /// Host of the first URL, for the icon and the list (`null` if none).
    pub host: Option<String>,
    /// Starred.
    pub favorite: bool,
    /// Has a one-time password (the item's own or in a custom field).
    pub has_otp: bool,
    /// Tags.
    pub tags: Vec<String>,
    /// Last change (`null` if unknown).
    pub modified_at: Option<UnixMillis>,
    /// When it went to the trash (`null` if it is not in the trash).
    pub trashed_at: Option<UnixMillis>,
    /// Has a sync conflict, a conflict copy or a concurrent deletion to
    /// review (CORE_API "Contracts the apps must keep").
    pub needs_review: bool,
}

/// `get_item`, `trash_item`, `restore_item`, `purge_item`, `totp_code`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
}

/// A plain text field and its value.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFieldEntry {
    /// The field.
    pub field: TextFieldKey,
    /// Its value.
    pub value: String,
}

/// A login URL.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlEntry {
    /// The element (stable across edits and devices).
    pub element_id: String,
    /// The URL as entered.
    pub url: String,
    /// How it is matched.
    pub match_mode: UrlMatchMode,
    /// Filling over plain HTTP was allowed explicitly.
    pub allow_http: bool,
}

/// A custom field. Hidden and OTP values are never included.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldEntry {
    /// The element.
    pub element_id: String,
    /// The label.
    pub label: String,
    /// The kind.
    pub kind: CustomFieldKind,
    /// The value; `null` for `hidden` and `otp` (reveal or copy instead).
    pub value: Option<String>,
}

/// An attachment (metadata only).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentEntry {
    /// The attachment.
    pub attachment_id: WireAttachmentId,
    /// The file name.
    pub file_name: String,
    /// The MIME type.
    pub mime: String,
    /// Size in bytes.
    pub size: u64,
}

/// The item's one-time-password configuration, without the secret.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpSummary {
    /// The scheme.
    pub scheme: OtpScheme,
    /// The issuer (`null` if none).
    pub issuer: Option<String>,
    /// Code length.
    pub digits: u8,
    /// Time step in seconds (`null` for HOTP).
    pub period: Option<u32>,
    /// The secret lives in a hardware-protected vault (CRYPTO_SPEC §7.4).
    pub hardware_protected: bool,
}

/// What a sync left for the user to review.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewState {
    /// Fields with concurrent values to choose from.
    pub conflicting_fields: u32,
    /// Another device deleted the item while this one edited it.
    pub deleted_concurrently: bool,
    /// This item is a conflict copy of that item (`null` if not).
    pub conflict_copy_of: Option<WireItemId>,
}

/// One item in the detail pane. Secret fields are listed by name only.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetail {
    /// List fields (vault, item, kind, title, favorite, tags, times).
    pub summary: ItemSummary,
    /// When it was created (`null` if unknown).
    pub created_at: Option<UnixMillis>,
    /// Plain text fields with a value, in model order (the title is in the
    /// summary).
    pub text: Vec<TextFieldEntry>,
    /// Secret fields that hold a value, masked.
    pub secrets: Vec<SecretFieldKey>,
    /// Login URLs.
    pub urls: Vec<UrlEntry>,
    /// Custom fields.
    pub custom_fields: Vec<CustomFieldEntry>,
    /// Attachments.
    pub attachments: Vec<AttachmentEntry>,
    /// The item's own one-time password (`null` if none).
    pub otp: Option<OtpSummary>,
    /// Entries in the password history.
    pub history_count: u32,
    /// Sync review state.
    pub review: ReviewState,
}

/// A secret to reveal or copy.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretRef {
    /// A secret field.
    Field {
        /// The field.
        field: SecretFieldKey,
    },
    /// A hidden custom field.
    CustomField {
        /// The element.
        element_id: String,
    },
    /// An entry of the password history, newest first.
    History {
        /// Position, 0 = newest.
        index: u32,
    },
}

/// `reveal_secret`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Which secret.
    pub secret: SecretRef,
}

/// The `reveal_secret` response: one secret, shown until the UI hides it
/// again (after `hideAfterSeconds`, on blur or on lock, UX_GUIDELINES §8).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealedSecret {
    /// The secret.
    pub value: SecretOut,
    /// Hide it again after this many seconds.
    pub hide_after_seconds: u32,
}

/// A text field change: a value, or `null` to clear the field.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextFieldChange {
    /// The field.
    pub field: TextFieldKey,
    /// The new value, or `null` to clear.
    pub value: Option<String>,
}

/// A secret field change: a value, or `null` to clear the field. Changing a
/// password moves the old one to the history.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretFieldChange {
    /// The field.
    pub field: SecretFieldKey,
    /// The new value, or `null` to clear.
    pub value: Option<SecretIn>,
}

/// A change to the URL list.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UrlChange {
    /// Append a URL.
    Add {
        /// The URL.
        url: String,
        /// How it is matched.
        match_mode: UrlMatchMode,
        /// Allow filling over plain HTTP.
        allow_http: bool,
    },
    /// Replace a URL.
    Update {
        /// The element.
        element_id: String,
        /// The URL.
        url: String,
        /// How it is matched.
        match_mode: UrlMatchMode,
        /// Allow filling over plain HTTP.
        allow_http: bool,
    },
    /// Remove a URL.
    Remove {
        /// The element.
        element_id: String,
    },
}

/// A change to the custom fields. Values travel as secrets: a hidden field's
/// value is one, and the type decides nothing about the transport.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CustomFieldChange {
    /// Append a field.
    Add {
        /// The label.
        label: String,
        /// The kind (`otp` takes an `otpauth://` URI).
        kind: CustomFieldKind,
        /// The value.
        value: SecretIn,
    },
    /// Replace a field.
    Update {
        /// The element.
        element_id: String,
        /// The label.
        label: String,
        /// The kind.
        kind: CustomFieldKind,
        /// The value.
        value: SecretIn,
    },
    /// Remove a field.
    Remove {
        /// The element.
        element_id: String,
    },
}

/// A change to the item's own one-time password.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OtpChange {
    /// Set it from an `otpauth://` URI (pasted or scanned).
    Set {
        /// The URI (it contains the shared secret).
        uri: SecretIn,
    },
    /// Remove it.
    Remove,
}

/// The edits of `add_item` and `update_item`. Empty lists and `null` leave
/// things unchanged; the whole draft is one edit and one sync event.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDraft {
    /// Plain text fields (including the title).
    pub text: Vec<TextFieldChange>,
    /// Secret fields.
    pub secrets: Vec<SecretFieldChange>,
    /// URL list changes, applied in order.
    pub urls: Vec<UrlChange>,
    /// Custom field changes, applied in order.
    pub custom_fields: Vec<CustomFieldChange>,
    /// The new tag set (`null`: unchanged).
    pub tags: Option<Vec<String>>,
    /// The item's own one-time password (`null`: unchanged).
    pub otp: Option<OtpChange>,
}

/// `add_item`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddItemRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The type (`unsupported` is refused with `invalidInput`).
    pub kind: ItemKind,
    /// The content.
    pub draft: ItemDraft,
}

/// `update_item`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateItemRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The edits.
    pub draft: ItemDraft,
}

/// `set_favorite`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetFavoriteRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Starred or not.
    pub favorite: bool,
}

/// `move_item`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveItemRequest {
    /// The source vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The target vault.
    pub target_vault_id: WireVaultId,
}

/// Where a moved item lives now (a move creates a new item ID).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedItem {
    /// The target vault.
    pub vault_id: WireVaultId,
    /// The item's new ID.
    pub item_id: WireItemId,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn summary() -> ItemSummary {
        ItemSummary {
            vault_id: ids::vault(1),
            item_id: ids::item(1),
            kind: ItemKind::Login,
            title: Some("GitHub".into()),
            subtitle: Some("alex@example.com".into()),
            host: Some("github.com".into()),
            favorite: true,
            has_otp: true,
            tags: vec!["work".into()],
            modified_at: Some(1_761_480_000_000),
            trashed_at: None,
            needs_review: false,
        }
    }

    pub(crate) fn summaries() -> Vec<ItemSummary> {
        vec![
            summary(),
            ItemSummary {
                vault_id: ids::vault(1),
                item_id: ids::item(2),
                kind: ItemKind::SecureNote,
                title: None,
                subtitle: None,
                host: None,
                favorite: false,
                has_otp: false,
                tags: vec![],
                modified_at: None,
                trashed_at: Some(1_761_480_000_000),
                needs_review: true,
            },
        ]
    }

    pub(crate) fn detail() -> ItemDetail {
        ItemDetail {
            summary: summary(),
            created_at: Some(1_761_400_000_000),
            text: vec![TextFieldEntry {
                field: TextFieldKey::Username,
                value: "alex@example.com".into(),
            }],
            secrets: vec![SecretFieldKey::Password],
            urls: vec![UrlEntry {
                element_id: "3-019a2b3c4d5e7000".into(),
                url: "https://github.com/login".into(),
                match_mode: UrlMatchMode::ExactOrigin,
                allow_http: false,
            }],
            custom_fields: vec![
                CustomFieldEntry {
                    element_id: "4-019a2b3c4d5e7000".into(),
                    label: "Recovery email".into(),
                    kind: CustomFieldKind::Email,
                    value: Some("alex@example.org".into()),
                },
                CustomFieldEntry {
                    element_id: "5-019a2b3c4d5e7000".into(),
                    label: "PIN".into(),
                    kind: CustomFieldKind::Hidden,
                    value: None,
                },
            ],
            attachments: vec![AttachmentEntry {
                attachment_id: ids::attachment(1),
                file_name: "recovery-codes.txt".into(),
                mime: "text/plain".into(),
                size: 512,
            }],
            otp: Some(OtpSummary {
                scheme: OtpScheme::Totp,
                issuer: Some("GitHub".into()),
                digits: 6,
                period: Some(30),
                hardware_protected: false,
            }),
            history_count: 2,
            review: ReviewState {
                conflicting_fields: 0,
                deleted_concurrently: false,
                conflict_copy_of: None,
            },
        }
    }

    pub(crate) fn draft() -> ItemDraft {
        ItemDraft {
            text: vec![
                TextFieldChange {
                    field: TextFieldKey::Title,
                    value: Some("GitHub".into()),
                },
                TextFieldChange {
                    field: TextFieldKey::Notes,
                    value: None,
                },
            ],
            secrets: vec![SecretFieldChange {
                field: SecretFieldKey::Password,
                value: Some(SecretIn::test("n3w-pa55word")),
            }],
            urls: vec![
                UrlChange::Add {
                    url: "https://github.com".into(),
                    match_mode: UrlMatchMode::ExactOrigin,
                    allow_http: false,
                },
                UrlChange::Update {
                    element_id: "3-019a2b3c4d5e7000".into(),
                    url: "https://github.com/login".into(),
                    match_mode: UrlMatchMode::Etld1,
                    allow_http: false,
                },
                UrlChange::Remove {
                    element_id: "6-019a2b3c4d5e7000".into(),
                },
            ],
            custom_fields: vec![
                CustomFieldChange::Add {
                    label: "PIN".into(),
                    kind: CustomFieldKind::Hidden,
                    value: SecretIn::test("1234"),
                },
                CustomFieldChange::Update {
                    element_id: "4-019a2b3c4d5e7000".into(),
                    label: "Recovery email".into(),
                    kind: CustomFieldKind::Email,
                    value: SecretIn::test("alex@example.org"),
                },
                CustomFieldChange::Remove {
                    element_id: "5-019a2b3c4d5e7000".into(),
                },
            ],
            tags: Some(vec!["work".into()]),
            otp: Some(OtpChange::Set {
                uri: SecretIn::test(
                    "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP&issuer=GitHub",
                ),
            }),
        }
    }

    pub(crate) fn secret_refs() -> Vec<SecretRef> {
        vec![
            SecretRef::Field {
                field: SecretFieldKey::Password,
            },
            SecretRef::CustomField {
                element_id: "5-019a2b3c4d5e7000".into(),
            },
            SecretRef::History { index: 0 },
        ]
    }

    pub(crate) fn otp_changes() -> Vec<OtpChange> {
        vec![
            OtpChange::Set {
                uri: SecretIn::test("otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP"),
            },
            OtpChange::Remove,
        ]
    }
}

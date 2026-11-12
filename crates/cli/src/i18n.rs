//! CLI messages (spec §3.8, ADR-0075). The EN and DE texts live in `packages/i18n`
//! (`catalog/{en,de}/cli.ts`); `pnpm --filter @mosaic/i18n export:cli` writes the JSON that is
//! embedded here. Placeholders are plain `{name}`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A message language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// English (the default).
    En,
    /// German, informal "du".
    De,
}

impl Lang {
    /// `en` or `de` (case-insensitive, region and encoding suffixes ignored).
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let lower = tag.to_ascii_lowercase();
        if lower.starts_with("de") {
            Some(Self::De)
        } else if lower.starts_with("en") || lower == "c" || lower == "posix" {
            Some(Self::En)
        } else {
            None
        }
    }

    /// `--lang`, then `MOSAIC_LANG`, then `LC_ALL`, `LC_MESSAGES`, `LANG` starting with `de`; else
    /// English.
    pub fn detect(flag: Option<&str>, env: &dyn Fn(&str) -> Option<String>) -> Self {
        if let Some(lang) = flag.and_then(Self::parse) {
            return lang;
        }
        for name in ["MOSAIC_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Some(value) = env(name).filter(|v| !v.is_empty()) {
                return Self::parse(&value).unwrap_or(Self::En);
            }
        }
        Self::En
    }
}

macro_rules! messages {
    ($($name:ident => $key:literal),* $(,)?) => {
        /// A message of the catalog.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Msg {
            $(
                #[doc = $key]
                $name
            ),*
        }

        impl Msg {
            /// Every message.
            #[cfg(test)]
            pub const ALL: &'static [Self] = &[$(Self::$name),*];

            /// The catalog key.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$name => $key),*
                }
            }
        }
    };
}

messages! {
    ErrStoreInUse => "cli.err.storeInUse",
    ErrNoStore => "cli.err.noStore",
    ErrNoAccount => "cli.err.noAccount",
    ErrWrongCredentials => "cli.err.wrongCredentials",
    ErrSecretKeyInvalid => "cli.err.secretKeyInvalid",
    ErrInputRequired => "cli.err.inputRequired",
    ErrPromptTimeout => "cli.err.promptTimeout",
    ErrNotFound => "cli.err.notFound",
    ErrAmbiguousRef => "cli.err.ambiguousRef",
    ErrInvalidRef => "cli.err.invalidRef",
    ErrNoSuchField => "cli.err.noSuchField",
    ErrNoOtp => "cli.err.noOtp",
    ErrRevealRequired => "cli.err.revealRequired",
    ErrVaultUnavailable => "cli.err.vaultUnavailable",
    ErrRotationInProgress => "cli.err.rotationInProgress",
    ErrSecretInArgv => "cli.err.secretInArgv",
    ErrUnknownCommand => "cli.err.unknownCommand",
    ErrUnknownOption => "cli.err.unknownOption",
    ErrMissingValue => "cli.err.missingValue",
    ErrMissingArgument => "cli.err.missingArgument",
    ErrUnexpectedArgument => "cli.err.unexpectedArgument",
    ErrInvalidValue => "cli.err.invalidValue",
    ErrNonUtf8 => "cli.err.nonUtf8",
    ErrFdUnsupported => "cli.err.fdUnsupported",
    ErrSessionExpired => "cli.err.sessionExpired",
    ErrSessionNotCached => "cli.err.sessionNotCached",
    ErrTwoFactorMethodUnsupported => "cli.err.twoFactorMethodUnsupported",
    ErrEnvSecretIgnored => "cli.err.envSecretIgnored",
    ErrExportUnreadable => "cli.err.exportUnreadable",
    ErrImportFailed => "cli.err.importFailed",
    ErrSyncNotConfigured => "cli.err.syncNotConfigured",
    ErrSyncHalted => "cli.err.syncHalted",
    ErrSyncFailed => "cli.err.syncFailed",
    ErrSyncSessionRevoked => "cli.err.syncSessionRevoked",
    ErrSyncSessionPasswordChanged => "cli.err.syncSessionPasswordChanged",
    ErrSyncSessionDeviceRemoved => "cli.err.syncSessionDeviceRemoved",
    ErrSyncSessionOperator => "cli.err.syncSessionOperator",
    ErrFileExists => "cli.err.fileExists",
    ErrFileRead => "cli.err.fileRead",
    ErrFileWrite => "cli.err.fileWrite",
    ErrPassphraseMismatch => "cli.err.passphraseMismatch",
    ErrPassphraseWeak => "cli.err.passphraseWeak",
    ErrBadEnvName => "cli.err.badEnvName",
    ErrSpawnFailed => "cli.err.spawnFailed",
    ErrNoVaultChosen => "cli.err.noVaultChosen",
    ErrNotSupported => "cli.err.notSupported",
    ErrStoreExists => "cli.err.storeExists",
    ErrInternal => "cli.err.internal",
    PromptPassword => "cli.prompt.password",
    PromptSecretKey => "cli.prompt.secretKey",
    PromptFilePassword => "cli.prompt.filePassword",
    PromptExportPassphrase => "cli.prompt.exportPassphrase",
    PromptExportPassphraseAgain => "cli.prompt.exportPassphraseAgain",
    PromptImportPassphrase => "cli.prompt.importPassphrase",
    PromptConfirmImport => "cli.prompt.confirmImport",
    PromptNewPassword => "cli.prompt.newPassword",
    PromptTwoFactor => "cli.prompt.twoFactor",
    OutRecovered => "cli.out.recovered",
    PromptAcknowledge => "cli.prompt.acknowledge",
    WarnRevealToTerminal => "cli.warn.revealToTerminal",
    OutUnlocked => "cli.out.unlocked",
    OutLocked => "cli.out.locked",
    OutConcealed => "cli.out.concealed",
    OutCopied => "cli.out.copied",
    OutNoItems => "cli.out.noItems",
    OutAmbiguousCandidate => "cli.out.ambiguousCandidate",
    OutAttachment => "cli.out.attachment",
    LabelId => "cli.label.id",
    LabelVault => "cli.label.vault",
    LabelType => "cli.label.type",
    LabelTitle => "cli.label.title",
    LabelOtp => "cli.label.otp",
    LabelAttachments => "cli.label.attachments",
    StatusStore => "cli.status.store",
    StatusProfile => "cli.status.profile",
    StatusAccount => "cli.status.account",
    StatusSession => "cli.status.session",
    StatusSessionNone => "cli.status.sessionNone",
    StatusSessionUntil => "cli.status.sessionUntil",
    StatusSync => "cli.status.sync",
    StatusSyncOff => "cli.status.syncOff",
    StatusSyncOn => "cli.status.syncOn",
    ImportPreview => "cli.import.preview",
    ImportDuplicates => "cli.import.duplicates",
    ImportWarnings => "cli.import.warnings",
    ImportDone => "cli.import.done",
    ImportDryRun => "cli.import.dryRun",
    ImportCancelled => "cli.import.cancelled",
    ExportDone => "cli.export.done",
    ExportAttachmentWarn => "cli.export.attachmentWarn",
    ExportAttachmentsLeftOut => "cli.export.attachmentsLeftOut",
    ExportUnavailableVaults => "cli.export.unavailableVaults",
    SyncDone => "cli.sync.done",
    SyncFinding => "cli.sync.finding",
    SyncClean => "cli.sync.clean",
    SyncAcknowledged => "cli.sync.acknowledged",
    SyncAcknowledgeWord => "cli.sync.acknowledgeWord",
    SetupDone => "cli.setup.done",
    SshAgentStatus => "cli.ssh.agentStatus",
    SshNoKeys => "cli.ssh.noKeys",
}

type Catalog = BTreeMap<String, String>;

fn load(json: &str) -> Catalog {
    serde_json::from_str(json).unwrap_or_default()
}

fn catalog(lang: Lang) -> &'static Catalog {
    static EN: OnceLock<Catalog> = OnceLock::new();
    static DE: OnceLock<Catalog> = OnceLock::new();
    match lang {
        Lang::En => EN.get_or_init(|| load(include_str!("../i18n/en.json"))),
        Lang::De => DE.get_or_init(|| load(include_str!("../i18n/de.json"))),
    }
}

/// The text of `msg` in `lang` with `{name}` placeholders filled from `args` (one pass, so a value
/// that looks like a placeholder is not expanded).
#[must_use]
pub fn text(lang: Lang, msg: Msg, args: &[(&str, String)]) -> String {
    let template = catalog(lang)
        .get(msg.key())
        .map_or(msg.key(), String::as_str);
    fill(template, args)
}

fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after)) = rest.split_once('{') {
        out.push_str(before);
        if let Some((name, tail)) = after.split_once('}') {
            if let Some((_, value)) = args.iter().find(|(n, _)| *n == name) {
                out.push_str(value);
            } else {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
            rest = tail;
        } else {
            out.push('{');
            out.push_str(after);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(s: &str) -> Vec<String> {
        let mut found: Vec<String> = s
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(n, _)| n.to_owned()))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn every_message_key_exists_in_both_catalogs_with_the_same_placeholders() {
        let (en, de) = (catalog(Lang::En), catalog(Lang::De));
        assert_eq!(en.keys().collect::<Vec<_>>(), de.keys().collect::<Vec<_>>());
        for msg in Msg::ALL {
            let (e, d) = (en.get(msg.key()), de.get(msg.key()));
            assert!(e.is_some() && d.is_some(), "{} missing", msg.key());
            assert_eq!(
                placeholders(e.map_or("", String::as_str)),
                placeholders(d.map_or("", String::as_str)),
                "{}",
                msg.key()
            );
        }
        assert_eq!(Msg::ALL.len(), en.len(), "a catalog key has no Msg");
    }

    #[test]
    fn placeholders_fill_in_one_pass() {
        let out = text(Lang::En, Msg::ErrNotFound, &[("ref", "{ref}".to_owned())]);
        assert_eq!(out, "Nothing found for {ref}.");
    }

    #[test]
    fn language_detection_order() {
        let env = |name: &str| match name {
            "LANG" => Some("de_DE.UTF-8".to_owned()),
            _ => None,
        };
        assert_eq!(Lang::detect(None, &env), Lang::De);
        assert_eq!(Lang::detect(Some("en"), &env), Lang::En);
        assert_eq!(Lang::detect(None, &|_| None), Lang::En);
    }
}

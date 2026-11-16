//! CLI messages (spec §3.8, ADR-0075, ADR-0168). The texts live in `packages/i18n`
//! (`catalog/<lang>/cli.ts`); `pnpm --filter @mosaic/i18n export:cli` writes the JSON that is
//! embedded here. Placeholders are plain `{name}`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A message language, in the order of `locales` in `packages/i18n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// English (the default).
    En,
    /// German, informal "du".
    De,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Portuguese, Brazilian (also for `pt-PT`).
    Pt,
    /// Dutch.
    Nl,
    /// Polish.
    Pl,
    /// Turkish.
    Tr,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
    /// Chinese, Simplified (`zh`, `zh-CN`, `zh-SG`, `zh-Hans`).
    Zh,
}

impl Lang {
    /// Every language, in declaration order (`lang as usize` is the index).
    pub const ALL: [Self; 12] = [
        Self::En,
        Self::De,
        Self::Es,
        Self::Fr,
        Self::It,
        Self::Pt,
        Self::Nl,
        Self::Pl,
        Self::Tr,
        Self::Ja,
        Self::Ko,
        Self::Zh,
    ];

    /// The catalog's code: `en`, `de`, …
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Pt => "pt",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Tr => "tr",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Zh => "zh",
        }
    }

    /// The language of a BCP 47 tag or POSIX locale name (`de-AT`, `pt_PT.UTF-8`, `zh-Hans-CN`;
    /// case-insensitive). `C` and `POSIX` are English. Chinese in Traditional characters
    /// (`zh-Hant`, `zh-TW`, `zh-HK`, `zh-MO`) is English too, never Simplified. `None`: a
    /// language Mosaic does not have.
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let lower = tag.trim().to_ascii_lowercase();
        let name = lower.split(['.', '@']).next().unwrap_or_default();
        let mut parts = name.split(['-', '_']);
        let base = parts.next().unwrap_or_default();
        if base == "c" || base == "posix" {
            return Some(Self::En);
        }
        if base == "zh" {
            let rest: Vec<&str> = parts.collect();
            let traditional = match rest.iter().find(|part| part.len() == 4) {
                Some(script) => *script != "hans",
                None => rest.iter().any(|part| matches!(*part, "tw" | "hk" | "mo")),
            };
            return Some(if traditional { Self::En } else { Self::Zh });
        }
        Self::ALL.into_iter().find(|lang| lang.code() == base)
    }

    /// `--lang`, then `MOSAIC_LANG`, then the first set of `LC_ALL`, `LC_MESSAGES`, `LANG`; a
    /// language Mosaic does not have is English.
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
    ErrImportTooLarge => "cli.err.importTooLarge",
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
    ErrAgentContext => "cli.err.agentContext",
    ErrAgentAlreadyPaired => "cli.err.agentAlreadyPaired",
    ErrDesktopNotRunning => "cli.err.desktopNotRunning",
    ErrAgentAccessOff => "cli.err.agentAccessOff",
    ErrAgentLocked => "cli.err.agentLocked",
    ErrPairingDenied => "cli.err.pairingDenied",
    ErrPairingExpired => "cli.err.pairingExpired",
    ErrPairingLimit => "cli.err.pairingLimit",
    ErrPairingNameTaken => "cli.err.pairingNameTaken",
    ErrPairingBlocked => "cli.err.pairingBlocked",
    ErrNotPaired => "cli.err.notPaired",
    ErrAgentRefused => "cli.err.agentRefused",
    ErrAgentIo => "cli.err.agentIo",
    AgentPairCode => "cli.agent.pairCode",
    AgentPaired => "cli.agent.paired",
    AgentStatus => "cli.agent.status",
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

/// The embedded JSON of `lang`.
const fn source(lang: Lang) -> &'static str {
    match lang {
        Lang::En => include_str!("../i18n/en.json"),
        Lang::De => include_str!("../i18n/de.json"),
        Lang::Es => include_str!("../i18n/es.json"),
        Lang::Fr => include_str!("../i18n/fr.json"),
        Lang::It => include_str!("../i18n/it.json"),
        Lang::Pt => include_str!("../i18n/pt.json"),
        Lang::Nl => include_str!("../i18n/nl.json"),
        Lang::Pl => include_str!("../i18n/pl.json"),
        Lang::Tr => include_str!("../i18n/tr.json"),
        Lang::Ja => include_str!("../i18n/ja.json"),
        Lang::Ko => include_str!("../i18n/ko.json"),
        Lang::Zh => include_str!("../i18n/zh.json"),
    }
}

fn catalog(lang: Lang) -> &'static Catalog {
    static CATALOGS: [OnceLock<Catalog>; Lang::ALL.len()] =
        [const { OnceLock::new() }; Lang::ALL.len()];
    static EMPTY: Catalog = BTreeMap::new();
    CATALOGS
        .get(lang as usize)
        .map_or(&EMPTY, |cell| cell.get_or_init(|| load(source(lang))))
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
    fn every_message_key_exists_in_every_catalog_with_the_same_placeholders() {
        let en = catalog(Lang::En);
        assert_eq!(Msg::ALL.len(), en.len(), "a catalog key has no Msg");
        for lang in Lang::ALL {
            let other = catalog(lang);
            let code = lang.code();
            assert_eq!(
                en.keys().collect::<Vec<_>>(),
                other.keys().collect::<Vec<_>>(),
                "{code}"
            );
            for msg in Msg::ALL {
                let (e, o) = (en.get(msg.key()), other.get(msg.key()));
                assert!(e.is_some() && o.is_some(), "{code} {} missing", msg.key());
                assert_eq!(
                    placeholders(e.map_or("", String::as_str)),
                    placeholders(o.map_or("", String::as_str)),
                    "{code} {}",
                    msg.key()
                );
            }
        }
    }

    #[test]
    fn every_language_has_its_index_code_and_catalog() {
        for (index, lang) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(lang as usize, index, "{lang:?}");
            assert_eq!(Lang::parse(lang.code()), Some(lang));
            assert_eq!(catalog(lang).len(), Msg::ALL.len(), "{lang:?}");
        }
    }

    #[test]
    fn tags_and_posix_names_map_to_languages() {
        for (tag, lang) in [
            ("de_AT.UTF-8", Lang::De),
            ("EN-gb", Lang::En),
            ("C", Lang::En),
            ("POSIX", Lang::En),
            ("C.UTF-8", Lang::En),
            ("es_MX", Lang::Es),
            ("fr-CA", Lang::Fr),
            ("it_IT@euro", Lang::It),
            ("pt_BR", Lang::Pt),
            ("pt-PT", Lang::Pt),
            ("nl_BE", Lang::Nl),
            ("pl_PL.UTF-8", Lang::Pl),
            ("tr", Lang::Tr),
            ("ja_JP.UTF-8", Lang::Ja),
            ("ko-KR", Lang::Ko),
            ("zh", Lang::Zh),
            ("zh_CN.UTF-8", Lang::Zh),
            ("zh-SG", Lang::Zh),
            ("zh-Hans", Lang::Zh),
            ("zh-Hans-HK", Lang::Zh),
            ("zh_TW.UTF-8", Lang::En),
            ("zh-HK", Lang::En),
            ("zh-MO", Lang::En),
            ("zh-Hant", Lang::En),
            ("zh-Hant-CN", Lang::En),
        ] {
            assert_eq!(Lang::parse(tag), Some(lang), "{tag}");
        }
        for tag in ["ru_RU.UTF-8", "sv", "dev", ""] {
            assert_eq!(Lang::parse(tag), None, "{tag}");
        }
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
        assert_eq!(Lang::detect(Some("ja"), &env), Lang::Ja);
        assert_eq!(Lang::detect(None, &|_| None), Lang::En);
        let taiwan = |name: &str| (name == "LANG").then(|| "zh_TW.UTF-8".to_owned());
        assert_eq!(Lang::detect(None, &taiwan), Lang::En);
        let russian = |name: &str| (name == "LC_ALL").then(|| "ru_RU.UTF-8".to_owned());
        assert_eq!(Lang::detect(None, &russian), Lang::En);
    }
}

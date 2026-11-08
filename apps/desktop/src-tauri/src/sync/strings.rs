//! The texts of the native acknowledgement dialog (C-I1). Rust owns them so
//! the webview cannot reword the question; they equal the `sync.acknowledge.*`
//! entries of `packages/i18n` (a test compares them with the catalog files).
//! The language is the setting; `System` follows the OS locale.

use crate::contract::settings::Language;

/// The texts of one language.
pub(crate) struct AcknowledgeTexts {
    pub(crate) title: &'static str,
    body: &'static str,
    each_device: &'static str,
}

impl AcknowledgeTexts {
    /// Body and the advice for every device, as the dialog shows them.
    pub(crate) fn message(&self) -> String {
        format!("{}\n\n{}", self.body, self.each_device)
    }
}

const EN: AcknowledgeTexts = AcknowledgeTexts {
    title: "Clear the sync warnings that need a check?",
    body: "Only go on after you have checked the server or folder yourself. This clears every warning that needs a check, and syncing continues. If your data really was rolled back or changed, Mosaic can't tell afterwards.",
    each_device: "Do this on each of your devices.",
};

const DE: AcknowledgeTexts = AcknowledgeTexts {
    title: "Sync-Warnungen zum Prüfen ausblenden?",
    body: "Mach erst weiter, wenn du den Server oder Ordner selbst geprüft hast. Damit verschwinden alle Warnungen, die eine Prüfung brauchen, und die Synchronisierung läuft weiter. Wurden deine Daten tatsächlich zurückgesetzt oder verändert, kann Mosaic das danach nicht mehr erkennen.",
    each_device: "Mach das auf jedem deiner Geräte.",
};

/// The texts for `language`; `System` reads the OS locale.
pub(crate) fn acknowledge(language: Language) -> &'static AcknowledgeTexts {
    let german = match language {
        Language::De => true,
        Language::En => false,
        Language::System => os_locale_is_german(),
    };
    if german { &DE } else { &EN }
}

/// The texts of the native confirmations of sharing: removing a member and
/// leaving a vault. Rust owns them for the same reason as the
/// acknowledgement's (the webview cannot reword or skip the question), and
/// they equal the `sharing.members.*` entries of `packages/i18n`. `{email}`
/// is replaced by the address, wrapped in a bidirectional isolate.
pub(crate) struct ShareTexts {
    remove_title: &'static str,
    remove_owner: &'static str,
    remove_manager: &'static str,
    pub(crate) leave_title: &'static str,
    pub(crate) leave_body: &'static str,
    /// The confirming buttons name the action, so no "OK" can be read as another.
    pub(crate) remove_button: &'static str,
    pub(crate) leave_button: &'static str,
    pub(crate) cancel_button: &'static str,
}

impl ShareTexts {
    /// The question for removing `email`; `owner`: this device changes the
    /// vault key at once (a manager's request waits for the owner).
    pub(crate) fn remove(&self, email: &str, owner: bool) -> (String, String) {
        let clean = email.replace(
            |c: char| c.is_control() || crate::commands::is_spoofing_char(c),
            "",
        );
        let shown = format!("\u{2068}{clean}\u{2069}");
        let body = if owner {
            self.remove_owner
        } else {
            self.remove_manager
        };
        (
            self.remove_title.replace("{email}", &shown),
            body.replace("{email}", &shown),
        )
    }
}

const SHARE_EN: ShareTexts = ShareTexts {
    remove_title: "Remove {email}?",
    remove_owner: "{email} keeps whatever they already downloaded. Mosaic changes the vault key now, and they can't read anything new.",
    remove_manager: "{email} keeps whatever they already downloaded. The server stops sending them updates now. The vault owner's device changes the vault key the next time it syncs, and until then only the server keeps {email} out.",
    leave_title: "Leave this vault?",
    leave_body: "The copy on this device stays readable, but it stops updating. To join again you need a new invite.",
    remove_button: "Remove",
    leave_button: "Leave vault",
    cancel_button: "Cancel",
};

const SHARE_DE: ShareTexts = ShareTexts {
    remove_title: "{email} entfernen?",
    remove_owner: "{email} behält, was schon heruntergeladen wurde. Mosaic wechselt jetzt den Tresorschlüssel, damit nichts Neues mehr lesbar ist.",
    remove_manager: "{email} behält, was schon heruntergeladen wurde. Der Server schickt ab sofort keine Updates mehr. Das Gerät des Tresor-Eigentümers wechselt den Tresorschlüssel beim nächsten Sync, und bis dahin hält nur der Server {email} draußen.",
    leave_title: "Diesen Tresor verlassen?",
    leave_body: "Die Kopie auf diesem Gerät bleibt lesbar, wird aber nicht mehr aktualisiert. Um wieder beizutreten, brauchst du eine neue Einladung.",
    remove_button: "Entfernen",
    leave_button: "Tresor verlassen",
    cancel_button: "Abbrechen",
};

/// The sharing texts for `language`.
pub(crate) fn share(language: Language) -> &'static ShareTexts {
    let german = match language {
        Language::De => true,
        Language::En => false,
        Language::System => os_locale_is_german(),
    };
    if german { &SHARE_DE } else { &SHARE_EN }
}

/// Whether a locale name (`de-DE`, `de_AT.UTF-8`) is German.
fn is_german(locale: &str) -> bool {
    let tag = locale.trim().to_ascii_lowercase();
    tag == "de" || tag.starts_with("de-") || tag.starts_with("de_") || tag.starts_with("de.")
}

/// The user's language: the first Windows user language, else `LC_ALL`,
/// `LC_MESSAGES`, `LANG` (Linux, and macOS where no system call is made).
fn os_locale_is_german() -> bool {
    #[cfg(windows)]
    if let Some(tag) = windows_language() {
        return is_german(&tag);
    }
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
        .is_some_and(|value| is_german(&value))
}

/// The first of the user's preferred languages (`WinRT`, safe API).
#[cfg(windows)]
fn windows_language() -> Option<String> {
    use windows::System::UserProfile::GlobalizationPreferences;
    let languages = GlobalizationPreferences::Languages().ok()?;
    languages.GetAt(0).ok().map(|tag| tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of `key` in a catalog file: the string literal after it.
    fn catalog_value(file: &str, key: &str) -> String {
        let at = file.find(&format!("\"{key}\":")).unwrap();
        let rest = &file[at + key.len() + 3..];
        let start = rest.find('"').unwrap() + 1;
        let mut out = String::new();
        let mut chars = rest[start..].chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => return out,
                '\\' => out.push(chars.next().unwrap()),
                c => out.push(c),
            }
        }
        panic!("unterminated string for {key}");
    }

    fn catalog(language: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/i18n/src/catalog")
            .join(language)
            .join("sync.ts");
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
    }

    /// The Rust texts are the catalog's, word for word, in both languages.
    #[test]
    fn the_dialog_texts_equal_the_i18n_catalog() {
        for (language, texts) in [("en", &EN), ("de", &DE)] {
            let file = catalog(language);
            assert_eq!(texts.title, catalog_value(&file, "sync.acknowledge.title"));
            assert_eq!(texts.body, catalog_value(&file, "sync.acknowledge.body"));
            assert_eq!(
                texts.each_device,
                catalog_value(&file, "sync.acknowledge.eachDevice")
            );
        }
    }

    #[test]
    fn the_sharing_texts_equal_the_i18n_catalog() {
        for (language, texts) in [("en", &SHARE_EN), ("de", &SHARE_DE)] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../packages/i18n/src/catalog")
                .join(language)
                .join("sharing.ts");
            let file = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
            assert_eq!(
                texts.remove_title,
                catalog_value(&file, "sharing.members.remove.title")
            );
            assert_eq!(
                texts.remove_owner,
                catalog_value(&file, "sharing.members.remove.confirm")
            );
            assert_eq!(
                texts.remove_manager,
                catalog_value(&file, "sharing.members.remove.confirmManager")
            );
            assert_eq!(
                texts.leave_title,
                catalog_value(&file, "sharing.members.leave.title")
            );
            assert_eq!(
                texts.leave_body,
                catalog_value(&file, "sharing.members.leave.confirm")
            );
            assert_eq!(
                texts.remove_button,
                catalog_value(&file, "sharing.members.remove")
            );
            assert_eq!(
                texts.leave_button,
                catalog_value(&file, "sharing.members.leave")
            );
            assert_eq!(
                texts.cancel_button,
                catalog_value(&file, "sharing.members.cancel")
            );
        }
    }

    #[test]
    fn the_removal_question_names_the_address_in_an_isolate() {
        let (title, body) = SHARE_EN.remove("sam@example.com", true);
        assert_eq!(title, "Remove \u{2068}sam@example.com\u{2069}?");
        assert!(body.starts_with("\u{2068}sam@example.com\u{2069} keeps"));
        let (_, manager) = SHARE_EN.remove("x\u{202e}y@example.com", false);
        assert!(!manager.contains('\u{202e}'));
    }

    #[test]
    fn the_language_setting_picks_the_texts() {
        assert_eq!(acknowledge(Language::De).title, DE.title);
        assert_eq!(acknowledge(Language::En).title, EN.title);
    }

    #[test]
    fn german_locale_names() {
        for yes in ["de", "de-DE", "de_AT.UTF-8", "DE-ch"] {
            assert!(is_german(yes), "{yes}");
        }
        for no in ["en-US", "dec", "fr_DE", "", "C"] {
            assert!(!is_german(no), "{no}");
        }
    }
}

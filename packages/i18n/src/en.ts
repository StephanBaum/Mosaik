/**
 * English UI strings (source locale). ICU MessageFormat syntax.
 * Every key here must exist in every other locale (enforced by the type of `de`).
 */
export const en = {
  "app.name": "Mosaic",
  "app.preAudit": "Pre-audit software: not yet reviewed by an external security auditor.",

  "action.cancel": "Cancel",
  "action.continue": "Continue",
  "action.retry": "Retry",
  "action.copy": "Copy",
  "action.reveal": "Reveal",
  "action.hide": "Hide",
  "action.lock": "Lock",
  "action.details": "Details",

  "lock.unlockTitle": "Unlock Mosaic",
  "lock.masterPassword": "Master password",
  "lock.wrongPassword":
    "That master password didn't work. Check Caps Lock and your keyboard layout, then try again.",
  "lock.locked": "Mosaic is locked.",

  "onboarding.secretKey.title": "Your Secret Key",
  "onboarding.secretKey.body":
    "Your Secret Key and master password together unlock your vault. We can't recover either of them. Save your Emergency Kit now.",
  "onboarding.secretKey.verify": "Type the last group of your Secret Key to confirm you saved it.",
  "onboarding.secretKey.mistyped": "That doesn't match. Check the last group in your Emergency Kit.",

  "clipboard.copied": "Copied. Clears in {seconds, plural, one {# second} other {# seconds}}.",
  "clipboard.cleared": "Clipboard cleared.",

  "sync.offline":
    "Couldn't reach your sync server. Your changes are saved on this device and will sync when you're back online.",

  "vault.itemCount": "{count, plural, =0 {No items} one {# item} other {# items}}",
  "vault.deleteConfirm": "Delete “{title}”? You can restore it from Trash for 30 days.",

  "app.loading": "Opening Mosaic…",
  "app.skipToContent": "Skip to content",

  "onboarding.step": "Step {current} of {total}",
  "onboarding.create.title": "Create your account",
  "onboarding.create.body":
    "Your master password and a Secret Key we generate on this device together unlock your vault. Neither is ever sent to the server.",
  "onboarding.create.email": "Email",
  "onboarding.create.password": "Master password",
  "onboarding.create.passwordHint": "Four unrelated words work well. Use it only for Mosaic.",
  "onboarding.create.confirm": "Confirm master password",
  "onboarding.create.mismatch":
    "The two passwords don't match. Type the same master password in both fields.",
  "onboarding.create.submit": "Create account",
  "onboarding.create.working": "Creating your account. This takes a few seconds.",
  "onboarding.kit.accountId": "Account ID",
  "onboarding.kit.secretKey": "Secret Key",
  "onboarding.kit.lastGroup": "Last group of your Secret Key",
  "onboarding.kit.confirm": "Confirm",

  "lock.secretKey": "Secret Key",
  "lock.secretKeyHint":
    "This device doesn't have your Secret Key yet. Type it from your Emergency Kit, and Mosaic remembers it on this device.",
  "lock.submit": "Unlock",
  "lock.working": "Unlocking…",

  "vault.defaultName": "Personal",
  "vault.unnamed": "Unnamed vault",
  "vault.vaults": "Vaults",
  "vault.allItems": "All items",
  "vault.items": "Items",
  "vault.search": "Search your vault",
  "vault.untitled": "Untitled",
  "vault.empty":
    "Your vault is empty. Add your first login, or import from another password manager.",
  "vault.noResults":
    "Nothing matches “{query}”. Check the spelling, or search for a website instead.",

  "error.notImplemented": "This isn't available yet. It's coming in a later version of Mosaic.",
  "error.noAccount":
    "There's no account on this device yet. Create one, or sign in with your Emergency Kit.",
  "error.accountExists": "This device already has a Mosaic account. Unlock it instead.",
  "error.locked": "Mosaic is locked. Unlock it to continue.",
  "error.alreadyUnlocked": "Mosaic is already unlocked. You can continue where you left off.",
  "error.wrongCredentials":
    "That master password or Secret Key didn't work. Check Caps Lock and your keyboard layout, then try again.",
  "error.secretKeyRequired":
    "This device doesn't have your Secret Key. Type it from your Emergency Kit.",
  "error.secretKeyMalformed":
    "That isn't a Secret Key. Yours starts with “M1-” and is on your Emergency Kit.",
  "error.secretKeyMistyped":
    "That Secret Key has a typo. Compare it with your Emergency Kit, character by character.",
  "error.emptyPassword": "The master password is empty. Enter it to continue.",
  "error.quickUnlockUnavailable":
    "Quick unlock isn't set up on this device. Unlock with your master password.",
  "error.storeInUse":
    "Mosaic is already open on this computer. Close the other window, then try again.",
  "error.storeUnavailable":
    "Mosaic couldn't open your vault file on this device. Check that the disk is available and has free space, then restart Mosaic.",
  "error.vaultNotFound": "That vault no longer exists. It may have been deleted on another device.",
  "error.vaultUnavailable":
    "This vault can't be opened in this session. Unlock it with its hardware key, or sync and try again.",
  "error.itemNotFound":
    "That item no longer exists. It may have been moved or deleted on another device.",
  "error.itemDeleted":
    "That item was deleted for good, so it can't be opened. Choose another item.",
  "error.sameVault": "The item is already in that vault. Choose a different vault.",
  "error.noOtp": "This item has no one-time code. Edit the item to add one.",
  "error.fieldEmpty":
    "That field is empty, so there's nothing to copy or show. Edit the item to fill it in.",
  "error.invalidInput":
    "Something in this form isn't valid. Check the highlighted fields and try again.",
  "error.reauthRequired": "This change needs your master password. Enter it to confirm.",
  "error.attachmentTooLarge": "That file is larger than 50 MiB. Choose a smaller file.",
  "error.attachmentQuota":
    "Your files would use more than the 1 GiB available. Remove some attachments, then try again.",
  "error.clipboardUnavailable":
    "Mosaic couldn't use the clipboard. Close apps that might be holding it, then try again.",
  "error.importFileUnreadable":
    "Mosaic couldn't read that file. Check that it still exists and isn't open in another app, then try again.",
  "error.importFormatInvalid":
    "That file isn't in the format you chose. Choose the matching format, or export the file again.",
  "error.importPreviewExpired": "This import was cancelled. Choose the file again to start over.",
  "error.sshRequestExpired":
    "That SSH request has already ended. Run the command again if you still need it.",
  "error.sshAgentUnavailable":
    "The SSH agent couldn't start. Check that no other SSH agent is running, then turn it on again.",
  "error.busy": "Mosaic is still finishing the last step. Wait a moment, then try again.",
  "error.internal":
    "Something unexpected stopped this action. Try again, and restart Mosaic if it keeps happening.",
  "shell.emergencyKit.title": "Mosaic Emergency Kit",
  "shell.emergencyKit.intro":
    "Keep this page somewhere safe, such as a locked drawer. Your Secret Key and your master password together unlock your vault on a new device. Nobody, not even Mosaic, can recover either of them for you.",
  "shell.emergencyKit.email": "Account email",
  "shell.emergencyKit.accountId": "Account ID",
  "shell.emergencyKit.signInUrl": "Sign-in address",
  "shell.emergencyKit.signInUrlLocal": "This device only (no sync server set up yet)",
  "shell.emergencyKit.secretKey": "Secret Key",
  "shell.emergencyKit.masterPassword": "Master password",
  "shell.emergencyKit.masterPasswordHint":
    "Write it on the line by hand only if this page stays somewhere safe. Otherwise keep it in your head.",
  "shell.emergencyKit.footer":
    "Mosaic is pre-audit software: it has not yet been reviewed by an external security auditor.",
  "shell.shortcut.conflict":
    "Another app already uses {shortcut}. Close that app or change its shortcut, then restart Mosaic to use quick search.",
  "insights.title": "Security",
  "insights.intro":
    "Checks run on this device. Last checked {checkedAt, date, medium} at {checkedAt, time, short} across {count, plural, one {# item} other {# items}}.",
  "insights.checkAgain": "Check again",
  "insights.summary": "Summary",
  "insights.fixFirst": "Fix these first",
  "insights.showAll": "Show all {count, plural, one {# finding} other {# findings}}",
  "insights.empty": "No issues found. Mosaic checks again whenever you open this page.",
  "insights.kind.weak": "Weak",
  "insights.kind.weak.description": "Easy to guess",
  "insights.kind.reused": "Reused",
  "insights.kind.reused.description": "{groups, plural, one {# group} other {# groups}}",
  "insights.kind.old": "Old (over 1 year)",
  "insights.kind.old.description": "Not changed in over a year",
  "insights.kind.missingTotp": "Missing 2FA",
  "insights.kind.missingTotp.description": "The site supports it, but you haven't set it up",
  "insights.kind.insecureUrl": "Unsecured HTTP",
  "insights.kind.insecureUrl.description": "Sends your password unencrypted",
  "insights.finding.weak": "Easy to guess. Change it to a generated password.",
  "insights.finding.reused":
    "Also used by {count, plural, one {# other item} other {# other items}}. Give each site its own password.",
  "insights.finding.old": "Not changed in over a year. Change it if you still use this account.",
  "insights.finding.missingTotp":
    "{site} offers two-factor codes. Turn them on there and save the code in Mosaic.",
  "insights.finding.insecureUrl":
    "Uses plain HTTP: {url}. Switch to https:// if the site supports it.",
  "insights.severity.high": "High",
  "insights.severity.medium": "Medium",
  "insights.severity.low": "Low",
  "insights.action.changePassword": "Change password",
  "insights.action.setUpTotp": "Set up 2FA",
  "insights.action.reviewUrl": "Review",
  "insights.attribution.twoFactor": "2FA data: 2FA Directory (2fa.directory), MIT license.",

  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, unencrypted)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.csv": "CSV file (.csv)",
  "import.keepassPassword": "Password of the KeePass database",
  "import.preview.ready":
    "{count, plural, one {# item is ready to import.} other {# items are ready to import.}}",
  "import.preview.attachments":
    "{count, plural, one {# attachment} other {# attachments}}",
  "import.preview.duplicate":
    "You already have an item like this. It's unticked, so it won't be imported twice.",
  "import.preview.duplicateInFile": "This file has the same item twice. The copy is unticked.",
  "import.warning.unmappedFields":
    "Some fields have no matching field in Mosaic. They were kept as custom fields.",
  "import.warning.attachmentsSkipped":
    "Some attachments are larger than 50 MiB, don't fit your 1 GiB of attachment storage, or are missing from the export. Add them by hand after the import.",
  "import.warning.entriesSkipped":
    "Some entries couldn't be read and will be left out. Check them in your old password manager.",
  "import.warning.otpUnreadable":
    "Some one-time code settings weren't recognized. They were kept as hidden fields, so set up the code again from them.",
  "import.warning.passkeysNotImported":
    "Passkeys can't be imported yet. The rest of those items is imported; keep your old app until you've moved the passkeys.",
  "import.entry.unmappedFields":
    "Some fields have no matching field in Mosaic. They were kept as custom fields.",
  "import.entry.otpUnreadable":
    "The one-time code setting wasn't recognized. It was kept as a hidden field, so set up the code again from it.",
  "import.entry.attachmentTooLarge":
    "An attachment is larger than 50 MiB and was left out. Add it by hand after the import.",
  "import.entry.attachmentMissing":
    "An attachment is missing from the export. Add it by hand after the import.",
  "import.entry.attachmentQuota":
    "An attachment doesn't fit your 1 GiB of attachment storage and was left out.",
  "import.entry.passkeysNotImported":
    "This item's passkey can't be imported yet. Keep your old app until you've moved it.",
  "import.entry.historyTruncated": "Only the 32 most recent old passwords were kept.",
  "import.entry.tagsDropped": "Empty or overlong tags were left out.",
  "import.entry.archived": "Archived in your old app. It gets the tag \"archived\".",
  "import.entry.unknownCategory": "Unknown item type. It's imported as a secure note.",
  "import.entry.fieldsDropped":
    "This item has more fields than Mosaic keeps. The extra fields are left out.",
  "import.skip.malformed": "This entry is damaged and can't be read.",
  "import.skip.empty": "This entry is empty.",
  "import.skip.tooLarge": "This entry is larger than 1 MiB, the most one item can hold.",
  "import.result":
    "{imported, plural, one {# item imported.} other {# items imported.}} {skipped, plural, =0 {} one {# left out.} other {# left out.}}",
} as const satisfies Record<string, string>;

export type MessageKey = keyof typeof en;

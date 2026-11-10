/** English UI strings: settings. ICU MessageFormat. */
export const settings = {
  "settings.title": "Settings",
  "settings.security": "Security",
  "settings.autoLock": "Lock after inactivity",
  "settings.autoLock.never": "Never",
  "settings.autoLock.minutes": "{minutes, plural, one {# minute} other {# minutes}}",
  "settings.lockOnSleep": "Lock when the computer sleeps",
  "settings.lockOnScreenLock": "Lock when the screen locks",
  "settings.screenLock.unsupported":
    "Mosaic can't tell when the screen locks on this system. It still locks after inactivity and when the computer wakes up.",
  "settings.lockTrigger.required":
    "Keep at least one lock on. Mosaic can't detect a locked screen on this system.",
  "settings.clipboard": "Clear the clipboard after",
  "settings.clipboard.never": "Never",
  "settings.clipboard.seconds": "{seconds, plural, one {# second} other {# seconds}}",
  "settings.appearance": "Appearance",
  "settings.theme": "Theme",
  "settings.theme.system": "Follow the system",
  "settings.theme.light": "Light",
  "settings.theme.dark": "Dark",
  "settings.language": "Language",
  "settings.language.system": "Follow the system",
  "settings.language.en": "English",
  "settings.language.de": "Deutsch",
  "settings.ssh": "SSH agent",
  "settings.ssh.enable": "Use my SSH keys from Mosaic",
  "settings.ssh.hint":
    "Off by default. The agent runs only while Mosaic is unlocked, and asks you before every use.",
  "settings.ssh.status":
    "{count, plural, =0 {No keys served} one {# key served} other {# keys served}}",
  "settings.ssh.endpoint": "Set SSH_AUTH_SOCK to",
  "settings.ssh.skipped": "Keys that are not served",
  "settings.kit": "Emergency Kit",
  "settings.kit.save": "Save the Emergency Kit again",
  "settings.saved": "Settings saved.",
  "reauth.title": "Confirm with your master password",
  "reauth.body": "This change makes Mosaic less safe, so it needs your master password.",
  "reauth.bodyKit":
    "Your Secret Key is about to be shown in a file, so Mosaic needs your master password.",
  "reauth.password": "Master password",
  "reauth.submit": "Confirm",
  "settings.intro": "How Mosaic locks, clears secrets and protects your keys on this device.",
  "settings.section.autoLock": "Auto-lock",
  "settings.section.autoLock.body": "When Mosaic locks itself without you asking.",
  "settings.section.clipboard": "Clipboard",
  "settings.section.clipboard.body": "Copied secrets are cleared after this time.",
  "settings.appearance.body": "The theme and the language of Mosaic.",
  "settings.ssh.body": "Let programs sign in with SSH keys from your vault. You approve each use.",
  "settings.kit.body": "Save a fresh copy of your Emergency Kit. This needs your master password.",
  "settings.quick": "Quick unlock",
  "settings.quick.body": "Faster ways back in, on top of your master password.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Unlock with your face, fingerprint or PIN. Your master password is still needed after a restart.",
  "settings.quick.biometric.bodyMac":
    "Unlock with your fingerprint. Your master password is still needed after a restart.",
  "settings.quick.expires": "Mosaic asks for your master password again on {date, date, medium}.",
  "settings.quick.maxAge": "Ask for the master password at least every",
  "settings.quick.maxAgeDays": "{days, plural, one {# day} other {# days}}",
  "settings.quick.biometricLinux": "Biometric unlock isn't available on Linux.",
  "settings.quick.biometricUnavailable": "Not set up on this device.",
  "settings.quick.keys": "Security keys",
  "settings.quick.keysCount":
    "{count, plural, =0 {None registered} one {# registered} other {# registered}}",
  "settings.quick.keysUnsupported": "Security keys aren't supported on this system yet.",
  "settings.quick.keyRow": "Security key · {tag} · added {addedAt, date, medium}",
  "settings.quick.addKey": "Add security key",
  "settings.quick.addKeyBody":
    "Insert the key you want to add and enter its PIN. Then touch it when it blinks.",
  "settings.quick.manageKeys": "Manage",
  "settings.quick.removeKey": "Remove",
  "settings.quick.removeKeyNamed": "Remove security key {tag}",
  "settings.quick.removeKeyConfirm": "Remove security key {tag}?",
  "settings.quick.keyRisk":
    "Anyone with this key and its PIN can open your vault without your master password.",
  "reauth.bodyQuick": "This makes it easier to get into Mosaic, so it needs your master password.",
  "reauth.bodyKey": "Adding a key to this vault needs your master password.",
  "settings.passkeyExport": "Export passkeys",
  "settings.passkeyExport.body":
    "Save the passkeys of a vault in a FIDO CXF file, to move them to another app.",
  "settings.passkeyExport.vault": "Vault",
  "settings.passkeyExport.warning":
    "The file contains your passkeys unencrypted. Anyone with the file can sign in as you. Delete it after importing.",
  "settings.passkeyExport.none": "This vault has no passkeys to export.",
  "settings.passkeyExport.button": "Export passkeys",
  "settings.passkeyExport.reauth":
    "The file will hold your passkeys unencrypted, so exporting needs your master password.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# passkey exported.} other {# passkeys exported.}} Delete the file after importing.",
  "reauth.bodyMove": "Moving an item out of a vault protected by a security key needs your master password.",
  "toast.settingsReset":
    "Your security settings were changed outside Mosaic and were reset to safe defaults.",
  "settings.shortcut.title":
    "Quick search shortcut",
  "settings.shortcut.body":
    "Press this combination in any program to open Mosaic's search.",
  "settings.shortcut.label":
    "Shortcut",
  "settings.shortcut.hint":
    "Ctrl, Alt or Command, plus one key. For example Ctrl+Shift+Space.",
  "settings.shortcut.save":
    "Save shortcut",
  "settings.shortcut.reset":
    "Use the default",
  "settings.shortcut.saved":
    "Shortcut saved: {shortcut}.",
  "settings.shortcut.invalid":
    "That isn't a shortcut Mosaic can use. Use Ctrl, Alt or Command plus one key, for example Ctrl+Shift+Space.",
  "settings.shortcut.unsupported":
    "Your system doesn't allow shortcuts that work in every program, so this can't be changed here.",
} as const satisfies Record<string, string>;

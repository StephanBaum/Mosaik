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
} as const satisfies Record<string, string>;

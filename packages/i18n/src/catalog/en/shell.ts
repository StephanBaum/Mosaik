/** English UI strings: shell. ICU MessageFormat. */
export const shell = {
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
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Space",
} as const satisfies Record<string, string>;

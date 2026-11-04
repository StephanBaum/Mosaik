/** English UI strings: one-time code setup (ADR-0031). ICU MessageFormat. */
export const otp = {
  "otp.section.label": "One-time code (2FA)",
  "otp.section.empty": "Add the code from the site's two-factor setup page.",
  "otp.section.required": "Add a one-time code to save this item.",
  "otp.source.screen": "Scan screen",
  "otp.source.clipboard": "Paste screenshot",
  "otp.source.file": "Choose image…",
  "otp.source.manual": "Can't scan? Enter the key",
  "otp.source.screenHint":
    "Mosaic steps aside for a moment, looks at your screen once and keeps nothing but the code.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Press Cmd+Ctrl+Shift+4, select the QR code, then paste it here.} windows {Press Win+Shift+S, select the QR code, then paste it here.} other {Copy a screenshot of the QR code, then paste it here.}}",
  "otp.dialog.title": "Set up a one-time code",
  "otp.working.screen": "Looking for a QR code on your screen…",
  "otp.working.image": "Reading the QR code…",
  "otp.manual.title": "Enter the setup key",
  "otp.manual.input": "Setup key or otpauth:// link",
  "otp.manual.inputHint":
    "The site shows it next to the QR code, often under \"Can't scan?\" or \"Enter manually\". Spaces don't matter.",
  "otp.manual.issuer": "Service",
  "otp.manual.account": "Account",
  "otp.manual.advanced": "Advanced",
  "otp.manual.algorithm": "Algorithm",
  "otp.manual.digits": "Digits",
  "otp.manual.period": "New code every",
  "otp.manual.periodValue": "{seconds} seconds",
  "otp.manual.advancedHint":
    "Change these only if the site says so. Most sites use SHA-1, 6 digits and 30 seconds.",
  "otp.manual.continue": "Continue",
  "otp.confirm.title": "Is this the right code?",
  "otp.confirm.issuer": "Service",
  "otp.confirm.account": "Account",
  "otp.confirm.details":
    "{scheme, select, hotp {Counter-based, {digits} digits} steam {Steam Guard, 5 characters} other {{digits} digits, new code every {period} seconds}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Current code",
  "otp.confirm.codeHint": "If the site asks for a code to finish setup, enter this one.",
  "otp.confirm.hotpNote":
    "Counter-based codes appear after you save, so the counter stays in step with the site.",
  "otp.confirm.weak":
    "This secret is unusually short. It works, but this site's two-factor protection is weaker than usual.",
  "otp.confirm.use": "Use this code",
  "otp.confirm.back": "Scan again",
  "otp.confirm.copy": "Copy code",
  "otp.summary.noIssuer": "One-time code",
  "otp.summary.pending": "Saved together with this item.",
  "otp.summary.removed": "The one-time code will be removed when you save.",
  "otp.action.replace": "Replace",
  "otp.action.remove": "Remove one-time code",
  "otp.action.keep": "Keep it",
  "otp.action.cancelSetup": "Don't add",
  "otp.storage.label": "Where to keep the code",
  "otp.storage.item": "With this item",
  "otp.storage.itemTradeoff":
    "Convenient, but anyone who opens this vault gets your password and your second factor together.",
  "otp.storage.hardware": "In {vault} (needs your security key)",
  "otp.storage.hardwareClosed": "Open {vault} with your security key first.",
  "otp.toast.clipboardCleared": "Screenshot read. Mosaic cleared the clipboard.",
  "otp.toast.savedInVault": "Code saved in {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {If clipboard history or the Snipping Tool kept a copy of the screenshot, delete it there.} mac {If you also saved the screenshot as a file, delete it.} other {If a clipboard manager kept a copy of the screenshot, delete it there.}}",
} as const satisfies Record<string, string>;

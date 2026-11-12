/**
 * English UI strings: the Android and iOS apps (M10). Namespace `mobile.*`, plus the `error.*`
 * strings of the mobile-only error codes, which the desktop catalogs do not have. ICU
 * MessageFormat, limited to `{name}` and one `{n, plural, one {…} other {…}}` per message so
 * `export:mobile` can write Android `strings.xml` and iOS `Localizable.xcstrings`.
 * Error texts follow "what happened + what to do".
 */
export const mobile = {
  "mobile.app.name": "Mosaic",
  "mobile.nav.vault": "Vault",
  "mobile.nav.search": "Search",
  "mobile.nav.settings": "Settings",
  "mobile.common.cancel": "Cancel",
  "mobile.common.done": "Done",
  "mobile.common.close": "Close",
  "mobile.common.continue": "Continue",
  "mobile.unlock.title": "Unlock Mosaic",
  "mobile.unlock.passwordLabel": "Master password",
  "mobile.unlock.action": "Unlock",
  "mobile.unlock.quick": "Use quick unlock",
  "mobile.unlock.failed":
    "{attempts, plural, one {That didn't work. # attempt so far.} other {That didn't work. # attempts so far.}}",
  "mobile.lock.now": "Lock now",
  "mobile.vault.title": "Your vault",
  "mobile.vault.count": "{count, plural, one {# item} other {# items}}",
  "mobile.vault.emptyTitle": "No items yet",
  "mobile.vault.emptyBody":
    "Add your first login, or sync with your desktop to bring your items here.",
  "mobile.search.placeholder": "Search your vault",
  "mobile.copy.done":
    "Copied. It clears from the clipboard in {seconds, plural, one {# second} other {# seconds}}.",
  "mobile.setup.scanTitle": "Scan your setup code",
  "mobile.setup.scanHint":
    "Hold the camera over the code on your computer. Mosaic reads it on this device only.",
  "mobile.autofill.title": "Fill from Mosaic",
  "mobile.autofill.target": "Fill in {app}?",
  "mobile.settings.title": "Settings",
  "mobile.settings.about": "About Mosaic",
  "mobile.settings.version": "Version {version}",
  // Error texts of the codes the desktop app does not have (docs/MOBILE.md §2).
  "error.notFound": "That item no longer exists. It may have been deleted on another device.",
  "error.syncFailed": "Mosaic couldn't sync. Check your connection, then try again.",
  "error.passwordInApp":
    "Unlocking with the master password isn't possible here. Open the Mosaic app and unlock it there.",
  "error.folderAccountUnsupported":
    "This account syncs through a cloud folder, which Mosaic on phones doesn't support yet. Use a desktop or a sync server account.",
  "error.setupCodeInvalid":
    "That setup code isn't valid or has expired. Create a new one on your computer and scan it again.",
  "error.notEnoughMemory":
    "This device doesn't have enough free memory to unlock your vault. Close other apps and try again.",
  "error.noMatch": "Mosaic has no login for this app or website. Open Mosaic to add one.",
  "error.insecureOrigin":
    "This page isn't using a secure connection, so Mosaic won't fill it. Open the https version of the page.",
  "error.appNotLinked":
    "This app isn't linked to a saved website. Link it in Mosaic first, then try again.",
  "error.foreignFrame":
    "This field belongs to another site embedded in the page, so Mosaic won't fill it.",
  "error.assetLinksUnavailable":
    "Mosaic couldn't check that this app belongs to the website. Check your connection and try again.",
  "error.appNotVerified":
    "This app isn't verified for that website, so Mosaic won't use its passkeys. Use the website in a browser instead.",
  "error.rpIdInvalid": "The website name in this request isn't valid. Cancel and start again.",
  "error.passkeyNotFound": "Mosaic has no passkey for this website. Sign in another way.",
  "error.passkeyExcluded":
    "You already have a passkey for this account in Mosaic. Sign in with it instead.",
  "error.passkeyUnsupportedAlgorithm":
    "This website needs a passkey type Mosaic can't create yet. Choose another way to sign in.",
  "error.passkeySessionExpired": "This passkey request has expired. Start again from the app.",
  "error.qrNotFound": "Mosaic found no QR code in that image. Try again with a sharper picture.",
  "error.storageUnavailable":
    "Mosaic couldn't reach the secure storage of this device. Restart your phone and try again.",
  // iOS (T10)
  "mobile.common.back": "Back",
  "mobile.common.tryAgain": "Try again",
  "mobile.onboarding.welcomeTitle": "Set up Mosaic on this phone",
  "mobile.onboarding.welcomeBody":
    "Sign in with the account you already use on your computer. Your master password and Secret Key never leave your devices.",
  "mobile.onboarding.preAudit": "Pre-audit software: no outside security review yet.",
  "mobile.onboarding.start": "Get started",
  "mobile.onboarding.detailsTitle": "Your account",
  "mobile.onboarding.server": "Server address",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "Email",
  "mobile.onboarding.secretKey": "Secret Key",
  "mobile.onboarding.secretKeyHint":
    "You find it in your Emergency Kit or in Settings on your computer. Capital letters and dashes do not matter.",
  "mobile.onboarding.deviceName": "Name of this phone",
  "mobile.onboarding.passwordTitle": "Your master password",
  "mobile.onboarding.passwordHint":
    "Mosaic checks it on this phone. It is never sent to the server.",
  "mobile.onboarding.signIn": "Sign in",
  "mobile.onboarding.working": "Signing in. This can take a few seconds.",
  "mobile.onboarding.secondFactorTitle": "One more step",
  "mobile.onboarding.totpLabel": "Code from your authenticator app",
  "mobile.onboarding.verify": "Verify",
  "mobile.onboarding.securityKey": "Use your security key",
  "mobile.onboarding.securityKeyWaiting":
    "Finish the sign-in in your browser. Check that it shows the code {code}, then come back here.",
  "mobile.onboarding.autofillTitle": "Fill passwords and passkeys",
  "mobile.onboarding.autofillBody":
    "Turn on Mosaic in the AutoFill settings so it can fill passwords, passkeys and codes in apps and in Safari.",
  "mobile.onboarding.autofillSteps":
    "Open Settings, then General, then AutoFill & Passwords, and turn on Mosaic.",
  "mobile.onboarding.openSettings": "Open settings",
  "mobile.onboarding.notNow": "Not now",
  "mobile.onboarding.doneTitle": "You're all set",
  "mobile.onboarding.doneBody": "Your vault is on this phone and unlocked.",
  "mobile.unlock.account": "Account: {email}",
  "mobile.unlock.secretKeyNeeded":
    "This phone does not have your Secret Key. Enter it once to unlock.",
  "mobile.unlock.quickOptionFaceId": "Unlock with Face ID next time",
  "mobile.unlock.biometricReason": "Unlock your Mosaic vault",
  "mobile.unlock.faceIdReason":
    "Mosaic uses Face ID to unlock your vault quickly. Your master password always works too.",
  "mobile.unlock.usePassword": "Use master password",
  "mobile.unlock.working": "Unlocking. This can take a few seconds.",
  "mobile.unlock.quickInvalidated":
    "Your Face ID settings changed. Unlock with your master password to turn quick unlock on again.",
  "mobile.unlock.storeBusy": "Mosaic is busy. Try again in a moment.",
  "mobile.unlock.rebootNote": "Unlock once with your master password after a restart.",
  "mobile.deviceLock.required.title": "Set a device passcode first",
  "mobile.deviceLock.required.body":
    "Mosaic stores your Secret Key only on phones that are protected by a passcode. Set a passcode in your device settings, then open Mosaic again.",
  "mobile.capture.hidden": "Hidden while your screen is recorded",
} as const satisfies Record<string, string>;

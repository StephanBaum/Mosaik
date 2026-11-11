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
} as const satisfies Record<string, string>;

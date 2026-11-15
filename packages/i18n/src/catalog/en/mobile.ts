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
    "{attempts, plural, one {That didn't work # time. Mosaic can't reset your master password. If you changed it on another device, use the new one.} other {That didn't work # times. Mosaic can't reset your master password. If you changed it on another device, use the new one.}}",
  "mobile.lock.now": "Lock now",
  "mobile.vault.title": "Your vault",
  "mobile.vault.count": "{count, plural, one {# item} other {# items}}",
  "mobile.vault.emptyTitle": "No items yet",
  "mobile.vault.emptyBody":
    "Pull down to sync with your other devices, or add your first login.",
  "mobile.search.placeholder": "Search your vault",
  "mobile.copy.done":
    "Copied. It clears from the clipboard in {seconds, plural, one {# second} other {# seconds}}.",
  "mobile.setup.scanTitle": "Scan your setup code",
  "mobile.setup.scanHint":
    "On your computer, open Mosaic, go to Settings > Sync and choose “Set up a phone”. Hold the camera over the code.",
  "mobile.autofill.title": "Fill from Mosaic",
  "mobile.autofill.target": "Fill in {app}?",
  "mobile.settings.title": "Settings",
  "mobile.settings.about": "About Mosaic",
  "mobile.settings.version": "Version {version}",
  "mobile.common.back": "Back",
  "mobile.common.notNow": "Not now",
  "mobile.onboarding.step": "Step {current} of {total}",
  "mobile.onboarding.welcome.title": "Set up Mosaic on this phone",
  "mobile.onboarding.welcome.body":
    "Sign in to your Mosaic account to bring your passwords here. Everything stays encrypted on this phone.",
  "mobile.onboarding.welcome.start": "Get started",
  "mobile.onboarding.details.title": "Your account",
  "mobile.onboarding.details.server": "Server address",
  "mobile.onboarding.details.serverHint": "https://sync.example.com",
  "mobile.onboarding.details.email": "Email",
  "mobile.onboarding.details.secretKey": "Secret Key",
  "mobile.onboarding.details.secretKeyHelp":
    "From your Emergency Kit: letters and numbers in groups, starting with M1.",
  "mobile.onboarding.details.deviceName": "Name of this phone",
  "mobile.onboarding.details.deviceNameDefault": "Android phone",
  "mobile.onboarding.details.checking": "Checking the server",
  "mobile.onboarding.password.title": "Your master password",
  "mobile.onboarding.password.body":
    "Mosaic never sends it anywhere. It unlocks your vault on this phone.",
  "mobile.onboarding.password.action": "Sign in",
  "mobile.onboarding.password.working": "Signing in. This can take a few seconds.",
  "mobile.onboarding.second.title": "One more step",
  "mobile.onboarding.second.totpLabel": "6-digit code",
  "mobile.onboarding.second.totpAction": "Verify",
  "mobile.onboarding.second.keyAction": "Use your security key",
  "mobile.onboarding.second.keyWaiting":
    "Waiting for your security key. Finish in the browser, then come back. The code there should read {code}.",
  "mobile.onboarding.quick.title": "Unlock faster",
  "mobile.onboarding.quick.body":
    "Use your fingerprint to unlock Mosaic next time. After a restart of the phone you still need your master password once.",
  "mobile.onboarding.quick.enable": "Turn on quick unlock",
  "mobile.onboarding.quick.unavailable":
    "Add a fingerprint in your phone's settings to use quick unlock.",
  "mobile.onboarding.autofill.title": "Fill logins automatically",
  "mobile.onboarding.autofill.body":
    "Let Mosaic offer your logins in other apps and in the browser. You can change this later in the system settings.",
  "mobile.onboarding.autofill.open": "Open autofill settings",
  "mobile.onboarding.done.title": "You're all set",
  "mobile.onboarding.done.body": "Your vault is on this phone and unlocked.",
  "mobile.onboarding.done.action": "Open your vault",
  "mobile.unlock.account": "Unlocking {email}",
  "mobile.unlock.secretKey": "Secret Key",
  "mobile.unlock.secretKeyHelp":
    "This phone does not have your Secret Key stored. Type it from your Emergency Kit.",
  "mobile.unlock.enrollNext": "Unlock with my fingerprint next time",
  "mobile.unlock.working": "Unlocking. This can take a few seconds.",
  "mobile.unlock.biometricTitle": "Unlock Mosaic",
  "mobile.unlock.biometricSubtitle": "Confirm that it is you",
  "mobile.unlock.usePassword": "Use master password",
  "mobile.unlock.busyTitle": "Mosaic is busy",
  "mobile.unlock.retry": "Try again",
  "mobile.home.unlockedAs": "Unlocked as {email}",
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
  "mobile.common.tryAgain": "Try again",
  "mobile.onboarding.preAudit": "Pre-audit software: no outside security review yet.",
  "mobile.onboarding.start": "Get started",
  "mobile.onboarding.server": "Server address",
  "mobile.onboarding.serverPlaceholder": "https://",
  "mobile.onboarding.email": "Email",
  "mobile.onboarding.secretKey": "Secret Key",
  "mobile.onboarding.secretKeyHint":
    "You find it in your Emergency Kit or in Settings on your computer. Capital letters and dashes do not matter.",
  "mobile.onboarding.deviceName": "Name of this phone",
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
  "mobile.onboarding.autofillSteps":
    "Open the Settings app, tap Passwords, then Password Options, and turn on Mosaic.",
  "mobile.onboarding.openSettings": "Open settings",
  "mobile.onboarding.notNow": "Not now",
  "mobile.unlock.secretKeyNeeded":
    "This phone does not have your Secret Key. Enter it once to unlock.",
  "mobile.unlock.quickOptionFaceId": "Unlock with Face ID next time",
  "mobile.unlock.biometricReason": "Unlock your Mosaic vault",
  "mobile.unlock.faceIdReason":
    "Mosaic uses Face ID to unlock your vault quickly. Your master password always works too.",
  "mobile.unlock.quickInvalidated":
    "Your Face ID settings changed. Unlock with your master password to turn quick unlock on again.",
  "mobile.unlock.storeBusy": "Mosaic is busy. Try again in a moment.",
  "mobile.unlock.rebootNote": "Unlock once with your master password after a restart.",
  "mobile.capture.hidden": "Hidden while your screen is recorded",
  // Android (T8): vault, editor, link app, settings, device lock.
  "mobile.deviceLock.required.title": "Set a screen lock first",
  "mobile.deviceLock.required.body":
    "Mosaic keeps your keys behind your phone's screen lock. Set a PIN, pattern or password in the system settings, then come back.",
  "mobile.deviceLock.required.action": "Open security settings",
  "mobile.common.show": "Show",
  "mobile.common.hide": "Hide",
  "mobile.common.copy": "Copy",
  "mobile.common.save": "Save",
  "mobile.common.remove": "Remove",
  "mobile.common.edit": "Edit",
  "mobile.common.working": "Working",
  "mobile.vault.add": "Add item",
  "mobile.vault.addLogin": "Login",
  "mobile.vault.addNote": "Secure note",
  "mobile.vault.untitled": "Untitled",
  "mobile.vault.favorite": "Favorite",
  "mobile.sync.never": "Not synced yet",
  "mobile.sync.last": "Synced {time}",
  "mobile.sync.localOnly": "This vault is only on this phone. Sync is not set up.",
  "mobile.sync.pending":
    "{count, plural, one {# change waiting to upload} other {# changes waiting to upload}}",
  "mobile.sync.now": "Sync now",
  "mobile.sync.working": "Syncing",
  "mobile.sync.warnings": "Sync warnings",
  "mobile.sync.warning.freezeSuspected":
    "Changes from another device have not arrived for a while. Check that the other device is on and online.",
  "mobile.sync.warning.noCommonView":
    "This phone and another device haven't seen the same server data for two days. The server may be showing them different versions. Sync your other device, then sync here.",
  "mobile.sync.warning.rollbackDetected":
    "Your sync data looks older than this phone. Open Mosaic on another device and check that your latest changes are there. Then choose “I've checked”.",
  "mobile.sync.warning.identityDuplicated":
    "Another copy of Mosaic is writing under this phone's identity, for example after a backup was restored. Syncing is paused. Make sure only one copy is in use, then choose “I've checked”.",
  "mobile.sync.warning.serverInconsistent":
    "The server showed this phone different data than another device. Open Mosaic on another device and check that your latest changes are there. Then choose “I've checked”.",
  "mobile.sync.warning.serverRollback":
    "The server's copy of one of your vaults is older than this phone's. Open Mosaic on another device and check that your latest changes are there. Then choose “I've checked”.",
  "mobile.sync.warning.sharingAttention":
    "A shared vault needs attention: a key change is waiting, or a member could not be checked. Open Mosaic on your computer to see which one. Then choose “I've checked”.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# received item} other {# received items}} couldn't be read on this phone. Update Mosaic on all your devices, then sync again.",
  "mobile.sync.warning.accountRollback":
    "The server shows an older account key than this phone has. Nothing was changed. Open Mosaic on another device and check that your latest changes are there. Then choose “I've checked”.",
  "mobile.sync.acknowledge": "I've checked…",
  "mobile.sync.acknowledgeTitle": "Clear the sync warnings?",
  "mobile.sync.acknowledgeBody":
    "Only go on after you have checked the server yourself. This clears every warning, and syncing continues. If your data really was rolled back or changed, Mosaic can't tell afterwards. Do this on each of your devices.",
  "mobile.sync.acknowledgeConfirm": "I've checked",
  "mobile.search.hint": "Search by title, username, website or tag.",
  "mobile.search.none": "No items match {query}.",
  "mobile.detail.favoriteAdd": "Add to favorites",
  "mobile.detail.favoriteRemove": "Remove from favorites",
  "mobile.detail.delete": "Move to trash",
  "mobile.detail.deleteTitle": "Move “{title}” to the trash?",
  "mobile.detail.deleteBody":
    "The change syncs to your other devices.",
  "mobile.detail.otpLeft": "{seconds, plural, one {# second left} other {# seconds left}}",
  "mobile.detail.passkey": "Passkey for {site}",
  "mobile.detail.attachments":
    "{count, plural, one {# file is attached. Open it on your computer.} other {# files are attached. Open them on your computer.}}",
  "mobile.detail.linkedApp": "App: {name}",
  "mobile.detail.needsLinking": "Needs linking before it can fill",
  "mobile.detail.linkApp": "Link an app",
  "mobile.editor.newLogin": "New login",
  "mobile.editor.editLogin": "Edit login",
  "mobile.editor.newNote": "New note",
  "mobile.editor.editNote": "Edit note",
  "mobile.editor.title": "Title",
  "mobile.editor.username": "Username or email",
  "mobile.editor.password": "Password",
  "mobile.editor.passwordKeep": "Leave empty to keep the current password.",
  "mobile.editor.websites": "Websites",
  "mobile.editor.websiteHint": "https://example.com",
  "mobile.editor.websiteAdd": "Add a website",
  "mobile.editor.websiteRemove": "Remove website {number}",
  "mobile.editor.notes": "Notes",
  "mobile.editor.favorite": "Mark as favorite",
  "mobile.editor.saving": "Saving",
  "mobile.editor.otpTitle": "One-time code",
  "mobile.editor.otpHave": "This login has a one-time code.",
  "mobile.editor.otpInput": "Setup key or otpauth link",
  "mobile.editor.otpAdd": "Add the code",
  "mobile.editor.otpReady": "Ready: {name}. It is saved with the login.",
  "mobile.editor.otpRemove": "Remove the code",
  "mobile.editor.otpRemoved": "The code is removed when you save.",
  "mobile.editor.generate": "Generate a password",
  "mobile.generator.title": "Password generator",
  "mobile.generator.password": "Characters",
  "mobile.generator.passphrase": "Words",
  "mobile.generator.length": "Length: {length}",
  "mobile.generator.lowercase": "Lowercase letters",
  "mobile.generator.uppercase": "Uppercase letters",
  "mobile.generator.digits": "Digits",
  "mobile.generator.symbols": "Symbols",
  "mobile.generator.ambiguous": "Skip look-alike characters",
  "mobile.generator.wordCount": "Words: {count}",
  "mobile.generator.separator": "Separator",
  "mobile.generator.capitalize": "Capitalize words",
  "mobile.generator.appendDigit": "Add a digit",
  "mobile.generator.run": "Generate",
  "mobile.generator.use": "Use this password",
  "mobile.generator.strength": "About {bits} bits of strength",
  "mobile.link.title": "Link an app",
  "mobile.link.choose": "Choose the app to link to {login}.",
  "mobile.link.filter": "Find an app",
  "mobile.link.app": "App",
  "mobile.link.package": "App ID",
  "mobile.link.installer": "Installed from",
  "mobile.link.installerUnknown": "Unknown source",
  "mobile.link.certificate": "Developer signature (start)",
  "mobile.link.verified": "Verified by {host}",
  "mobile.link.notVerified": "Not verified",
  "mobile.link.unavailable": "Could not check",
  "mobile.link.checking": "Checking the website",
  "mobile.link.warning":
    "Only link apps you trust. Mosaic can't confirm this app belongs to {host}.",
  "mobile.link.warningNoHost":
    "Only link apps you trust. This login has no website Mosaic could check.",
  "mobile.link.confirm": "Fill {login} in this app from now on",
  "mobile.link.done": "Linked. Mosaic fills {login} in this app.",
  "mobile.link.noApps": "No other apps found.",
  "mobile.settings.security": "Security",
  "mobile.settings.autoLock": "Lock when you leave the app",
  "mobile.settings.autoLock.now": "Right away",
  "mobile.settings.autoLock.s30": "After 30 seconds",
  "mobile.settings.autoLock.s60": "After 1 minute",
  "mobile.settings.autoLock.s300": "After 5 minutes",
  "mobile.settings.idle": "Lock when you stop touching the screen",
  "mobile.settings.idleOption": "{minutes, plural, one {After # minute} other {After # minutes}}",
  "mobile.settings.clipboard": "Clear copied values",
  "mobile.settings.clipboardOption":
    "{seconds, plural, one {After # second} other {After # seconds}}",
  "mobile.settings.quick": "Quick unlock",
  "mobile.settings.quickOn": "Quick unlock is on.",
  "mobile.settings.quickOff": "Quick unlock is off.",
  "mobile.settings.quickEnable": "Turn on",
  "mobile.settings.quickDisable": "Turn off",
  "mobile.settings.quickPassword": "Type your master password to turn quick unlock on.",
  "mobile.settings.sync": "Sync",
  "mobile.about.preAudit":
    "Mosaic is pre-audit software. No independent security audit has happened yet.",
  "mobile.about.privacy": "No analytics, no tracking and no crash reports leave this phone.",
  "mobile.about.licences": "Open-source licences",
  "mobile.licences.title": "Open-source licences",
  "mobile.licences.intro": "Mosaic for Android is built with these open-source parts.",
  "mobile.licences.rust":
    "The Rust core uses open-source crates. Their licences are listed in the project's source repository.",
  // Android (T9): Autofill service and Credential Manager provider.
  "mobile.autofill.unlockEntry": "Unlock Mosaic",
  "mobile.autofill.noLogin": "No login for {app}",
  "mobile.autofill.noLoginHint": "Tap to link a login",
  "mobile.autofill.cannotCheck": "Could not check this app",
  "mobile.autofill.warning": "Check the address first",
  "mobile.autofill.noUsername": "No user name",
  "mobile.autofill.otpEntry": "One-time code",
  "mobile.link.pickLogin": "Choose the login Mosaic should fill in {app}.",
  "mobile.credential.unlockEntry": "Unlock Mosaic",
  "mobile.credential.passwordEntry": "Password",
  "mobile.credential.passkeyEntry": "Passkey",
  "mobile.credential.createEntry": "Save passkey in Mosaic",
  "mobile.credential.createTitle": "Save a passkey?",
  "mobile.credential.createBody": "Mosaic will save a passkey for {site} ({user}).",
  "mobile.credential.createVault": "Save in",
  "mobile.credential.vaultDefault": "Personal vault",
  "mobile.credential.createConfirm": "Save passkey",
  "mobile.credential.verifyTitle": "Confirm that it is you",
  "mobile.credential.verifySignIn": "Sign in to {site}",
  "mobile.credential.verifyCreate": "Save a passkey for {site}",
  "mobile.credential.verifyFailed": "Mosaic could not confirm that it is you. Try again.",
  "mobile.credential.unsupported": "Mosaic can't answer this sign-in request. Cancel and choose another way to sign in.",
  "mobile.settings.fill": "Fill in other apps",
  "mobile.settings.autofillOn": "Mosaic is your Autofill service.",
  "mobile.settings.autofillOff": "Mosaic is not your Autofill service yet.",
  "mobile.settings.autofillEnable": "Use Mosaic for Autofill",
  "mobile.settings.credentialHint":
    "Android can also offer your passkeys and passwords from Mosaic when an app asks you to sign in.",
  "mobile.settings.credentialEnable": "Use Mosaic for passkeys and passwords",
  // Android (T13): the setup code and one-time code scanners (CameraX, no Play services).
  "mobile.scan.setupAction": "Scan setup code",
  "mobile.scan.otpAction": "Scan a QR code",
  "mobile.scan.otpTitle": "Scan the one-time code",
  "mobile.scan.otpHint":
    "Hold the camera over the QR code of the website. Mosaic reads it on this phone only.",
  "mobile.scan.preview": "Camera view",
  "mobile.scan.looking": "Looking for a QR code",
  "mobile.scan.notSetupCode": "Not a Mosaic setup code",
  "mobile.scan.notSetupCodeHelp":
    "Hold the camera over the code that Mosaic shows on your computer, in Settings under Sync.",
  "mobile.scan.rationale":
    "Mosaic uses the camera only to read this QR code. Nothing is saved or sent.",
  "mobile.scan.allow": "Allow the camera",
  "mobile.scan.denied":
    "Mosaic can't use the camera. Allow it in the settings of this phone to scan, or enter the details by hand.",
  "mobile.scan.openSettings": "Open settings",
  "mobile.scan.unavailable": "Mosaic couldn't start the camera. Enter the details by hand.",
  "mobile.onboarding.details.scanned":
    "Server, email and Secret Key are filled in from the setup code. Check them, then continue.",
  "mobile.onboarding.details.scanOr": "Or enter the details by hand",
  "mobile.onboarding.details.keyFromCode": "Secret Key from the setup code",
  // iOS (T11): identity store settings and the licences intro.
  "mobile.settings.passkeyIdentities": "Use Mosaic for passkeys",
  "mobile.settings.passkeyIdentitiesHint":
    "Mosaic tells the system the website and username of each passkey so it can offer it. The keys never leave Mosaic.",
  "mobile.settings.passwordIdentities": "Suggest logins above the keyboard",
  "mobile.settings.passwordIdentitiesHint":
    "Mosaic tells the system the website and username of each login. The system keeps this list on the phone, outside Mosaic's encryption.",
  "mobile.settings.identitiesPassword": "Type your master password to turn this on.",
  "mobile.licences.introIos": "Mosaic for iPhone is built with these open-source parts.",
  // iOS (T12): the AutoFill extension (password, passkey and one-time-code pickers, configuration).
  "mobile.ext.openApp": "Open Mosaic and unlock it once with your master password.",
  "mobile.ext.noAccount":
    "Mosaic has no account on this iPhone yet. Open Mosaic to set it up.",
  "mobile.ext.loading": "Opening your vault",
  "mobile.ext.thisSite": "this site",
  "mobile.ext.matches": "Logins for {site}",
  "mobile.ext.noMatches": "No login for {site}",
  "mobile.ext.noMatchesHint":
    "Mosaic fills only logins that are saved for this site. Open Mosaic to add this site to a login.",
  "mobile.ext.searchAll": "Search all logins",
  "mobile.ext.hideAll": "Hide other logins",
  "mobile.ext.notSavedFor": "Not saved for {site}",
  "mobile.ext.fillHint": "Fills this login",
  "mobile.ext.warnPunycode":
    "This address is written with special characters: {unicode}. Check that it is the site you mean.",
  "mobile.ext.warnLookalike":
    "This address looks like {site}, which you saved in Mosaic. Check that it is the site you mean.",
  "mobile.ext.otpTitle": "Choose a one-time code",
  "mobile.ext.passkeyTitle": "Choose a passkey",
  "mobile.ext.configTitle": "Mosaic AutoFill",
  "mobile.ext.configBody":
    "Mosaic offers your logins, passkeys and one-time codes when you tap a field or a sign-in button. You change what it offers in Settings in the Mosaic app.",
  // iOS (G2): a QuickType suggestion asks first (M3), and the settings text names the limit.
  "mobile.ext.confirmFillTitle": "Fill the login for {site}?",
  "mobile.ext.confirmFillBody":
    "iOS does not tell Mosaic which page you are on, so Mosaic cannot check it. Fill only if this is the site you mean. Mosaic then asks for Face ID or Touch ID.",
  "mobile.ext.confirmFillButton": "Fill this login",
  "mobile.settings.passwordIdentitiesLimit":
    "When you tap a suggestion, iOS does not tell Mosaic which page you are on. So Mosaic shows the saved site and fills only after you confirm it and unlock with Face ID or Touch ID.",
} as const satisfies Record<string, string>;

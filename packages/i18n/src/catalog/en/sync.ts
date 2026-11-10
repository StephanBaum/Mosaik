/** English UI strings: sync settings and warnings (M6). ICU MessageFormat. */
export const sync = {
  "sync.status.signedOut": "Signed out",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",

  "settings.sync.title": "Sync",
  "settings.sync.body":
    "Keep your vaults up to date on all your devices. Everything is encrypted on this device before it leaves.",
  "settings.sync.notSetUp": "Sync is off. Your vaults are stored only on this device.",
  "settings.sync.deviceName": "Name of this device",
  "settings.sync.deviceName.default": "Mosaic on {platform}",
  "settings.sync.password": "Master password",
  "settings.sync.code": "6-digit code",

  "settings.sync.server.title": "Mosaic server",
  "settings.sync.server.body": "Sync through a Mosaic server that you or someone you trust runs.",
  "settings.sync.server.url": "Server address",
  "settings.sync.server.urlHint": "For example https://sync.example.com",
  "settings.sync.server.check": "Check server",
  "settings.sync.server.checking": "Checking the server…",
  "settings.sync.server.registration.open": "This server lets anyone create a sync account.",
  "settings.sync.server.registration.invite":
    "This server needs an invite code to create a sync account.",
  "settings.sync.server.registration.closed":
    "This server doesn't accept new sync accounts. You can still sign in to an existing one.",
  "settings.sync.server.choice.label": "Sync account",
  "settings.sync.server.choice.signIn": "I have a sync account",
  "settings.sync.server.choice.create": "Create one",
  "settings.sync.server.invite": "Invite code",
  "settings.sync.server.create": "Create sync account",
  "settings.sync.server.signIn": "Sign in",
  "settings.sync.server.signInAgain": "Sign in again",
  "settings.sync.server.working": "Connecting…",
  "settings.sync.server.sees":
    "The server sees only encrypted data, your account email and when you sync. It never sees your master password or your Secret Key.",

  "settings.sync.twoFactor.title": "Enter your code",
  "settings.sync.twoFactor.body":
    "This sync account uses two-step verification. Enter the 6-digit code from your authenticator app.",
  "settings.sync.twoFactor.submit": "Verify",

  "settings.sync.folder.title": "Your own cloud folder",
  "settings.sync.folder.body":
    "Sync through a folder that your cloud service (Dropbox, OneDrive, iCloud Drive and others) keeps up to date.",
  "settings.sync.folder.choose": "Choose folder…",
  "settings.sync.folder.chooseOther": "Choose another folder…",
  "settings.sync.folder.path": "Folder",
  "settings.sync.folder.contents.empty": "The folder is empty. Mosaic can use it.",
  "settings.sync.folder.contents.mosaic": "This folder already holds Mosaic sync data.",
  "settings.sync.folder.contents.other":
    "This folder holds other files. Mosaic writes nothing there. Choose an empty folder or create a new one.",
  "settings.sync.folder.connect": "Connect folder",
  "settings.sync.folder.working": "Connecting…",
  "settings.sync.folder.sees":
    "Your cloud provider and anyone with access to the folder see only encrypted data, the account ID and when files change. They never see your master password or your Secret Key.",

  "settings.sync.status.label": "Status",
  "settings.sync.lastSynced": "Last synced {when}",
  "settings.sync.neverSynced": "Not synced yet",
  "settings.sync.offline":
    "You're offline. Your changes are saved on this device and will sync when you're back online.",
  "settings.sync.now": "Sync now",
  "settings.sync.backend.server": "Server",
  "settings.sync.backend.account": "Sync account",
  "settings.sync.backend.folder": "Folder",
  "settings.sync.backend.device": "This device",
  "settings.sync.signedOut.title": "This device is signed out",
  "settings.sync.signedOut.body":
    "The server ended this device's session. Sign in again to keep syncing. Your changes are safe on this device.",

  "settings.sync.recover.title": "Your master password was changed",
  "settings.sync.recover.body":
    "The server or the folder holds a newer master password than this device. Enter the new master password to bring this device up to date. Your vaults and your Secret Key stay the same.",
  "settings.sync.recover.password": "New master password",
  "settings.sync.recover.submit": "Update this device",

  "settings.sync.disconnect": "Disconnect…",
  "settings.sync.disconnect.title": "Disconnect sync?",
  "settings.sync.disconnect.body":
    "This device stops syncing and signs out of the server. Your vaults stay on this device, and your other devices keep their own copies.",
  "settings.sync.disconnect.confirm": "Disconnect",

  "settings.sync.devices.title": "Devices",
  "settings.sync.devices.thisDevice": "This device",
  "settings.sync.devices.unnamed": "Unnamed device",
  "settings.sync.devices.lastSeen": "Last seen {when}",
  "settings.sync.devices.neverSeen": "Not seen yet",
  "settings.sync.devices.rename": "Rename",
  "settings.sync.devices.renameNamed": "Rename {name}",
  "settings.sync.devices.renameLabel": "New name for this device",
  "settings.sync.devices.remove": "Remove",
  "settings.sync.devices.removeNamed": "Remove {name}",
  "settings.sync.devices.removeTitle": "Remove {name}?",
  "settings.sync.devices.removeBody":
    "Removing a device signs it out of sync. It doesn't take away the copy of your vaults it already has. To lock a device out for good, change your master password and Secret Key.",
  "settings.sync.devices.removeConfirm": "Remove device",
  "settings.sync.devices.folderNote":
    "To lock a device out, change your master password and Secret Key.",

  "settings.sync.totp.title": "Two-step verification",
  "settings.sync.totp.body":
    "Asks for a code from an authenticator app when someone signs in to your sync account. It protects the server account, not your vaults: those stay encrypted with your master password and Secret Key.",
  "settings.sync.totp.on": "Two-step verification is on.",
  "settings.sync.totp.off": "Two-step verification is off.",
  "settings.sync.totp.enable": "Turn on…",
  "settings.sync.totp.disable": "Turn off…",
  "settings.sync.totp.beginTitle": "Turn on two-step verification",
  "settings.sync.totp.beginBody": "Enter your master password to start.",
  "settings.sync.totp.begin": "Continue",
  "settings.sync.totp.setupBody":
    "Add this key to your authenticator app, then enter the 6-digit code it shows.",
  "settings.sync.totp.secret": "Setup key",
  "settings.sync.totp.confirm": "Turn on",
  "settings.sync.totp.disableTitle": "Turn off two-step verification",
  "settings.sync.totp.disableBody": "Enter your master password and a current code.",
  "settings.sync.totp.disableConfirm": "Turn off",
  "settings.sync.totp.enabledToast": "Two-step verification is on.",
  "settings.sync.totp.disabledToast": "Two-step verification is off.",

  "settings.sync.key.title": "Security keys",
  "settings.sync.key.body":
    "A security key can be a second step when you sign in to your sync account. It guards the account on the server, not your vaults. Adding or using a key opens your browser.",
  "settings.sync.key.none": "No security keys added.",
  "settings.sync.key.added": "Added {when}",
  "settings.sync.key.lastUsed": "Last used {when}",
  "settings.sync.key.neverUsed": "Not used yet",
  "settings.sync.key.add": "Add a security key…",
  "settings.sync.key.addTitle": "Add a security key",
  "settings.sync.key.addBody":
    "Name the key and enter your master password. Mosaic then opens your browser, where you use the key.",
  "settings.sync.key.label": "Name of the key",
  "settings.sync.key.labelHint": "For example “Desk key”. Up to 64 characters.",
  "settings.sync.key.addConfirm": "Continue",
  "settings.sync.key.addedToast": "Security key added.",
  "settings.sync.key.noBackup":
    "A security key would be your only second step. If you lose it, you may not be able to sign in to your sync account again. We recommend a backup: a second key or a code from an authenticator app.",
  "settings.sync.key.recommend":
    "We recommend two methods, for example two keys, or a key and an authenticator-app code.",
  "settings.sync.key.remove": "Remove",
  "settings.sync.key.removeLabel": "Remove {name}",
  "settings.sync.key.removeTitle": "Remove {name}?",
  "settings.sync.key.removeBody":
    "You confirm once with a security key in your browser. After that, this key no longer works for signing in.",
  "settings.sync.key.removeLast":
    "This is your last second step. Two-step verification turns off, and your sync account is then protected by your master password and Secret Key alone.",
  "settings.sync.key.removeConfirm": "Remove key",
  "settings.sync.key.removedToast": "Security key removed.",
  "settings.sync.key.confirmTitle": "Confirm with a security key",
  "settings.sync.key.confirmBody":
    "Removing a key needs a second step. Enter your master password, then confirm with a key in your browser.",
  "settings.sync.key.step.methods": "Checking how this account signs in…",
  "settings.sync.key.step.useKey": "Use security key (opens your browser)",
  "settings.sync.key.step.keyOnlyTitle": "Use your security key",
  "settings.sync.key.step.keyOnlyBody":
    "This account is protected by a security key. Mosaic opens a page in your browser, where you use it.",
  "settings.sync.key.step.or": "Or use a code",
  "settings.sync.key.wait.login": "Sign in with your security key",
  "settings.sync.key.wait.register": "Add your security key",
  "settings.sync.key.wait.confirm": "Confirm with your security key",
  "settings.sync.key.wait.lead":
    "Your browser opened a Mosaic page from your sync server. Type this code there, then use your security key.",
  "settings.sync.key.wait.codeLabel": "Code to type in your browser",
  "settings.sync.key.wait.codeSpoken": "Code to type in your browser: {digits}",
  "settings.sync.key.wait.expires": "This step runs out at {time}.",
  "settings.sync.key.wait.waiting": "Waiting for your security key…",
  "settings.sync.key.wait.done": "Done.",
  "settings.sync.key.wait.expired": "This step ran out of time. Start it again.",
  "settings.sync.key.wait.failed":
    "Too many wrong codes or failed checks in the browser. Start it again.",
  "settings.sync.key.wait.reopen": "Open the browser page again",
  "settings.sync.key.wait.retry": "Start again",

  "sync.warning.region": "Sync warnings",
  "sync.warning.unknownDevice": "another device",
  "sync.warning.unknownVault": "one of your vaults",
  "sync.warning.freezeSuspected":
    "Changes from {device} have not arrived for over a day. Check that the other device is on and online.",
  "sync.warning.missingFiles":
    "Files from {device} are missing from the folder. Let your cloud service finish syncing, then sync again.",
  "sync.warning.noCommonView":
    "This device and {device} haven't seen the same server data for two days. The server may be showing them different versions.",
  "sync.warning.rollbackDetected":
    "Your sync data looks older than this device. Check the server or folder, then choose “I've checked”.",
  "sync.warning.identityDuplicated":
    "Another copy of Mosaic is writing under this device's identity, for example after a backup was restored. Syncing is paused. Make sure only one copy is in use, then choose “I've checked”.",
  "sync.warning.serverInconsistent":
    "The server showed this device different data than another device. Check the server, then choose “I've checked”.",
  "sync.warning.serverRollback":
    "The server's copy of {vault} is older than this device's. Check the server, then choose “I've checked”.",
  "sync.warning.undecryptable":
    "{count, plural, one {# received item} other {# received items}} couldn't be read on this device. Update Mosaic on all your devices, then sync again.",
  "sync.warning.hardwareVaultsLocal":
    "Hardware-protected vaults stay on this device. {count, plural, one {# vault isn't} other {# vaults aren't}} synced.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# file} other {# files}} on the server or folder couldn't be repaired from this device. Open the item on a device that still has the file and add it again.",
  "sync.warning.acknowledge": "I've checked…",
  "sync.acknowledge.title": "Clear the sync warnings that need a check?",
  "sync.acknowledge.body":
    "Only go on after you have checked the server or folder yourself. This clears every warning that needs a check, and syncing continues. If your data really was rolled back or changed, Mosaic can't tell afterwards.",
  "sync.acknowledge.eachDevice": "Do this on each of your devices.",

  "toast.syncAttention":
    "Sync merged or repaired some items from another device. Have a look at what changed.",
  "sync.revoked.rekey":
    "This account was re-keyed on another device: sign in with the new Secret Key from your newest Emergency Kit.",
  "sync.revoked.passwordChange":
    "Your master password was changed on another device: sign in with the new password.",
  "sync.revoked.deviceRemoved":
    "This device was removed from your account on another device. Sign in again to add it back.",
  "sync.revoked.operator":
    "The operator of the server ended this session. Sign in again to keep syncing.",
} as const satisfies Record<string, string>;

/**
 * English UI strings: password and key changes (M7): change the master password, rotate the
 * Secret Key, upgrade the protection, re-key the account, resume a change that was cut off, and
 * take over keys another device renewed. ICU MessageFormat. No "epoch" in UI text: say that the
 * account's keys were renewed.
 */
export const rotation = {
  "rotation.section.title": "Password and keys",
  "rotation.section.body":
    "Change your master password, get a new Secret Key or move your whole account to new keys. Each change shows you what happens before anything is touched.",

  "rotation.action.password": "Change master password",
  "rotation.action.password.body":
    "Choose a new master password. Your Secret Key and your vaults stay as they are.",
  "rotation.action.password.button": "Change…",
  "rotation.action.secretKey": "Rotate Secret Key",
  "rotation.action.secretKey.body":
    "Get a new Secret Key and a new Emergency Kit. Use this if your kit was lost or someone may have seen it.",
  "rotation.action.secretKey.button": "Rotate…",
  "rotation.action.profile": "Upgrade protection",
  "rotation.action.profile.body":
    "Make guessing your master password harder by using more memory for every try. Unlocking takes a little longer.",
  "rotation.action.profile.button": "Upgrade…",
  "rotation.action.rekey": "Re-key account",
  "rotation.action.rekey.body":
    "Renew every key of your account: a new Secret Key, new vault keys and a new safety code. Use this if someone may have copied your vault data together with your password and Secret Key.",
  "rotation.action.rekey.button": "Re-key…",
  "rotation.action.rekey.needsSync":
    "A re-key renews the copy on your sync server or folder, so it needs sync. Set up sync first.",

  "rotation.title.password": "Change master password",
  "rotation.title.secretKey": "Rotate Secret Key",
  "rotation.title.profile": "Upgrade protection",
  "rotation.title.rekey": "Re-key account",
  "rotation.step": "Step {current} of {total}",
  "rotation.back": "Back",
  "rotation.next": "Continue",
  "rotation.close": "Close",

  // Review: what changes and what happens to everything else
  "rotation.review.title.password": "Before you change your password",
  "rotation.review.title.secretKey": "Before you rotate your Secret Key",
  "rotation.review.title.profile": "Before you upgrade the protection",
  "rotation.review.title.rekey": "Was something compromised?",
  "rotation.review.changes": "What changes",
  "rotation.review.stays": "What stays the same",
  "rotation.review.password.changes":
    "Your master password. Quick unlock on this device is turned off, and you can set it up again afterwards.",
  "rotation.review.password.stays":
    "Your Secret Key, your vaults and items, your safety code and your emergency contacts.",
  "rotation.review.password.devices":
    "Your other devices are signed out of sync. On each one, type the new master password once and sync continues.",
  "rotation.review.secretKey.changes":
    "Your Secret Key and your Emergency Kit. Quick unlock on this device is turned off, and you can set it up again afterwards.",
  "rotation.review.secretKey.stays":
    "Your master password, your vaults and items, your safety code and your emergency contacts. Security keys you registered keep working.",
  "rotation.review.secretKey.devices":
    "Your other devices are signed out of sync. On each one, type your password and the new Secret Key from the new kit once.",
  "rotation.review.secretKey.limit":
    "This stops sign-in with the old Secret Key. Anyone who already copied your vault data with your old password and Secret Key can still read it. Re-key the account to stop that.",
  "rotation.review.secretKey.compromised": "Something was compromised: re-key instead",
  "rotation.review.profile.changes":
    "How much memory every unlock attempt needs, for you and for anyone guessing. Quick unlock on this device is turned off, and you can set it up again afterwards.",
  "rotation.review.profile.stays":
    "Your master password, your Secret Key, your vaults and your safety code. Security keys you registered keep working.",
  "rotation.review.profile.devices":
    "Your other devices are signed out of sync. On each one, type your master password once and sync continues. Slower devices need a little longer to unlock.",
  "rotation.review.rekey.intro":
    "A re-key is for the case that someone may have your vault data and your password and Secret Key. If you only lost a device, rotating the Secret Key or changing your password is enough.",
  "rotation.review.rekey.yes": "Yes, re-key my account",
  "rotation.review.rekey.no": "No, just rotate the Secret Key instead",
  "rotation.review.rekey.limit":
    "A re-key does not take back what someone already copied. Old copies on servers and in backups stay encrypted with the old keys, and the old keys still open them.",

  "rotation.review.rekey.title": "What a re-key does",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# vault only you use gets a new key.} other {# vaults only you use get new keys.}}",
  "rotation.review.rekey.keys":
    "A new Secret Key, a new Emergency Kit, a new safety code and a new account key.",
  "rotation.review.rekey.devices":
    "Your other devices are signed out and can't sync until you type the new Secret Key and your new master password on each one.",
  "rotation.review.rekey.shared":
    "{count, plural, one {You share # vault.} other {You share # vaults.}} Each one gets a new key.",
  "rotation.review.rekey.sharedMembers":
    "Ask each person you share with to compare your new safety code. Until they do, they get no new keys from you and can't open the renewed vault.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# vault belongs to someone else.} other {# vaults belong to other people.}} Their owners are asked to renew them. Until they do, your old keys can still read them.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# person has emergency access to your vaults.} other {# people have emergency access to your vaults.}} Their access is renewed for the new keys. Each of them has to compare your new safety code as well.",
  "rotation.review.rekey.quick":
    "Quick unlock on this device is turned off. You can set it up again afterwards.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# security key you registered for unlocking is turned off.} other {# security keys you registered for unlocking are turned off.}} You can register them again afterwards.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# hardware-protected vault is not affected.} other {# hardware-protected vaults are not affected.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# attachment is not on this device.} other {# attachments are not on this device.}} They are downloaded first so they can be protected again. Any that can't be reached keep the old key.",
  "rotation.review.rekey.folder":
    "Your sync folder gets a new account file. Other devices find a note there that the keys changed.",
  "rotation.review.rekey.unavailable": "Re-keying is not available: sync is not set up.",
  "rotation.review.loading": "Checking what a re-key touches…",
  "rotation.review.rekey.needsClean":
    "A re-key needs sync to be up to date and free of warnings. Sync now, look at any warning, then try again.",

  // The passwords
  "rotation.credentials.title": "Your passwords",
  "rotation.credentials.password.title": "Your master passwords",
  "rotation.credentials.current": "Current master password",
  "rotation.credentials.new": "New master password",
  "rotation.credentials.confirm": "Type the new master password again",
  "rotation.credentials.mismatch": "The two passwords are different.",
  "rotation.credentials.same": "The new master password has to be different from the current one.",
  "rotation.credentials.hint":
    "A long passphrase is better than a short complicated one. You can't recover it: Mosaic does not know it.",
  "rotation.credentials.code": "6-digit code from your authenticator app",
  "rotation.credentials.profile": "Protection level",
  "rotation.credentials.profile.standard": "Standard: 256 MB of memory per try",
  "rotation.credentials.profile.strong": "Strong: 512 MB of memory per try",
  "rotation.credentials.profile.max": "Maximum: 1 GB of memory per try",
  "rotation.credentials.profile.slow":
    "Unlocking with this level takes this device more than two seconds. Other, older devices will be slower still.",
  "rotation.credentials.profile.force": "Upgrade anyway",

  // The new Emergency Kit
  "rotation.kit.title": "Save your new Emergency Kit",
  "rotation.kit.body":
    "Your new Secret Key is in this kit. Save it first: Mosaic changes your account only after you confirm you have it, because without it you can't sign in on a new device.",
  "rotation.kit.save": "Save new Emergency Kit",
  "rotation.kit.saved": "Saved. Keep it somewhere safe, apart from your devices.",
  "rotation.kit.cancelled": "Nothing was saved yet.",
  "rotation.kit.verify": "Type the last group of your new Secret Key from the saved kit.",
  "rotation.kit.confirm": "Confirm kit",
  "rotation.kit.mistyped": "That is not the end of the new Secret Key. Check the saved kit.",
  "rotation.kit.verified": "Kit confirmed.",

  // Confirmation and the run
  "rotation.confirm.title.password": "Change your master password",
  "rotation.confirm.title.secretKey": "Rotate your Secret Key",
  "rotation.confirm.title.profile": "Upgrade the protection",
  "rotation.confirm.title.rekey": "Re-key your account",
  "rotation.confirm.body.rekey":
    "This signs your other devices out and renews every key. Keep Mosaic open until it is done.",
  "rotation.confirm.body.secretKey":
    "This signs your other devices out of sync. Keep Mosaic open until it is done.",
  "rotation.confirm.verb.password": "Change password",
  "rotation.confirm.verb.secretKey": "Rotate Secret Key",
  "rotation.confirm.verb.profile": "Upgrade protection",
  "rotation.confirm.verb.rekey": "Re-key account",

  "rotation.run.title": "Working on it",
  "rotation.run.keepOpen":
    "Keep Mosaic open. With many attachments this can take a few minutes.",
  "rotation.progress.label": "Progress",
  "rotation.progress.count": "{done} of {total}",
  "rotation.phase.prepared": "Getting ready",
  "rotation.phase.backendStarted": "Asking your account to switch to the new keys",
  "rotation.phase.backendAccepted": "Your account accepted the new keys",
  "rotation.phase.folderWritten": "The sync folder has the new account file",
  "rotation.phase.localCommitted": "This device now uses the new keys",
  "rotation.phase.attachments": "Protecting attachments again",
  "rotation.phase.uploading": "Sending the protected data",
  "rotation.phase.done": "Finished",

  // The end
  "rotation.done.title.password": "Master password changed",
  "rotation.done.title.secretKey": "Secret Key rotated",
  "rotation.done.title.profile": "Protection upgraded",
  "rotation.done.title.rekey": "Account re-keyed",
  "rotation.done.body.password":
    "Use the new master password from now on. Your other devices ask for it once.",
  "rotation.done.body.secretKey":
    "Keep the new Emergency Kit safe and delete the old one. Your other devices ask for the new Secret Key once.",
  "rotation.done.body.profile": "Unlocking now needs more memory. Your other devices ask for your password once.",
  "rotation.done.body.rekey":
    "Keep the new Emergency Kit safe and delete the old one. Your other devices ask for the new Secret Key and your new master password once.",
  "rotation.done.members":
    "Ask everyone you share a vault with to compare your new safety code. Until they do, they get no new keys from you.",
  "rotation.done.quick": "Set up quick unlock again",
  "rotation.done.quick.body": "Quick unlock was turned off.",
  "rotation.done.hardware": "Register security keys again",
  "rotation.done.hardware.body":
    "{count, plural, one {# security key was turned off for unlocking.} other {# security keys were turned off for unlocking.}}",
  "rotation.done.unreachable":
    "{count, plural, one {# attachment could not be protected again because it was not available. It still uses the old key.} other {# attachments could not be protected again because they were not available. They still use the old key.}}",
  "rotation.done.unreachable.note":
    "These files couldn't be read and weren't re-encrypted. They keep their old file key until you remove or re-add them, so anyone who holds your old keys and a copy of a file could still open it.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Unknown item",
  "rotation.done.unreachable.unknownFile": "unknown file",
  "rotation.done.unreachable.more": "{count, plural, one {and # more file} other {and # more files}}",
  "rotation.done.unfinished":
    "The last part continues later. If Mosaic closes before it is done, choose Continue key rotation next time.",

  // Continue after a cut-off change
  "rotation.resume.title": "Key change not finished",
  "rotation.resume.body.password":
    "A change of your account's keys was cut off. Nothing is lost. Start it again or discard it.",
  "rotation.resume.body.secretKey":
    "Rotating your Secret Key was cut off. Nothing is lost. Continue to finish it.",
  "rotation.resume.body.rekey":
    "Re-keying your account was cut off. Nothing is lost. Continue to finish it.",
  "rotation.resume.body.profile":
    "Upgrading the protection was cut off. Nothing is lost. Continue to finish it.",
  "rotation.resume.continue": "Continue key rotation",
  "rotation.resume.discard": "Discard…",
  "rotation.resume.running": "Continuing…",
  "rotation.resume.progress":
    "{done} of {total} done. Keep Mosaic open until it finishes.",
  "rotation.resume.dialog.title": "Continue key rotation",
  "rotation.resume.dialog.password": "Master password you have now",
  "rotation.resume.dialog.group": "Last group of the new Secret Key",
  "rotation.resume.dialog.body":
    "Your account's keys may already have changed. To be sure it was you, type your master password and the last group of the new Secret Key from the new Emergency Kit.",
  "rotation.resume.dialog.submit": "Continue",
  "rotation.resume.discard.title": "Discard this key change?",
  "rotation.resume.discard.body":
    "The new Secret Key and the new Emergency Kit from this attempt are deleted. Your account stays as it is.",
  "rotation.resume.discard.confirm": "Discard key change",
  "rotation.resume.abandon": "Give up on the rest…",
  "rotation.resume.stalled": "Continuing didn't finish. If it keeps failing, you can stop here.",
  "rotation.abandon.title": "Stop re-encrypting your files?",
  "rotation.abandon.body":
    "Your new keys are already in use. Mosaic stops re-encrypting the attachments that are still waiting. They keep their old file key, so anyone who holds your old keys and a copy of a file could still open it.",
  "rotation.abandon.stays":
    "Everything already re-encrypted stays as it is. Remove or re-add the waiting files later to protect them with the new key.",
  "rotation.abandon.confirm": "Stop re-encrypting",
  "rotation.abandon.keep": "Keep trying",
  "rotation.marker.title": "Unconfirmed re-key notice",
  "rotation.marker.hint": "A notice in your sync folder says your account was re-keyed, but Mosaic couldn't confirm it.",
  "rotation.marker.open": "Ignore this notice…",
  "rotation.marker.confirm":
    "Mosaic found a re-key notice it couldn't confirm. If you didn't re-key your account on another device, you can ignore it.",
  "rotation.marker.ignore": "Ignore notice",
  "rotation.marker.keep": "Keep",
  "rotation.resume.done": "Key change finished.",

  // Another device renewed the keys
  "rotation.recovery.title": "Your account's keys were renewed on another device",
  "rotation.recovery.body":
    "A device of yours renewed the keys of this account. To keep syncing, type your master password and the Secret Key from the new Emergency Kit.",
  "rotation.recovery.password": "Master password",
  "rotation.recovery.secretKey": "Secret Key from the new Emergency Kit",
  "rotation.recovery.submit": "Use the new keys",
  "rotation.recovery.warning":
    "Did you not do this? Then someone who knows your old password and Secret Key may have. Don't type anything yet and check your other devices.",
  "rotation.recovery.notice": "Sync is paused until you do this on this device.",
  "rotation.recovery.open": "Use the new keys…",
  "rotation.recovery.done": "This device uses the new keys now.",
  "rotation.recovery.optional": "Secret Key from the newest Emergency Kit (only if you renewed it)",

  // A folder file that can't be checked
  "rotation.file.unknown":
    "An unknown account file appeared in your sync folder. It can't be checked with your keys: it may be left over, or someone changed your folder. Your data is not affected.",
  "rotation.file.dismiss": "Dismiss",
  "rotation.file.conflict.title": "Two devices renewed the keys at the same time",
  "rotation.file.conflict":
    "Another device renewed your account's keys at the same moment as this one, so the two no longer agree. Re-key again from this device to bring them together: this device's keys then win and the other one asks for the new Secret Key.",
  "rotation.file.conflict.rekey": "Re-key again from this device",
  "rotation.file.conflict.keep": "Keep this device's keys",
} as const satisfies Record<string, string>;

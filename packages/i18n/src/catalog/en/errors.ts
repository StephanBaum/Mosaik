/** English UI strings: errors. ICU MessageFormat. */
export const errors = {
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
  "error.sshKeyInvalid":
    "This isn't an OpenSSH private key. Paste the whole key, including the BEGIN and END lines.",
  "error.reauthRequired": "This change needs your master password. Enter it to confirm.",
  "error.attachmentTooLarge": "That file is larger than 50 MiB. Choose a smaller file.",
  "error.attachmentQuota":
    "Your files would use more than the 1 GiB available. Remove some attachments, then try again.",
  "error.attachmentNotFound":
    "This file is no longer part of the item. Reload the item.",
  "error.attachmentUnavailable":
    "This file can't be opened on this device. It may not be synced yet or may be damaged.",
  "error.attachmentFileUnreadable":
    "That file can't be read. Choose another file.",
  "error.attachmentSaveFailed":
    "The file couldn't be saved there. Choose another folder.",
  "error.attachmentPreviewUnsupported":
    "There's no preview for this file. Use Save as to open it.",
  "error.attachmentPreviewTooLarge":
    "This image is too large to preview. Use Save as to open it.",
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
  "error.unlockDelayed":
    "Too many wrong attempts in a row. Wait a few seconds, then try again.",
  "error.tooManyAttempts":
    "Too many wrong answers. Mosaic locked. Unlock it with your master password to continue.",
  "error.sshAllowTooSoon":
    "That was too quick to read the request. Check the key and the program, then choose Allow again.",
  "error.browserRequestExpired":
    "That browser request has already ended. Ask again from the extension if you still need it.",
  "error.browserAllowTooSoon":
    "That was too quick to read the request. Check the details, then choose again.",
  "error.browserPairingNotFound":
    "That browser is no longer connected. Reopen Settings to see the current list.",
  "error.kitNotVerified":
    "Save the new Emergency Kit and confirm its last group first. Mosaic changes nothing before that.",
  "error.kitGroupMismatch":
    "That is not the last group of the new Secret Key. Check the new Emergency Kit.",
  "error.rotationInProgress":
    "Another key change is still open. Finish it with Continue key rotation, or discard it first.",
  "error.rotationNone": "There is no key change to continue.",
  "error.rotationNotFound":
    "That password change is no longer open. It may have been closed on another device or in the browser. Reopen the item to see where it stands.",
  "error.rotationLimit":
    "Four password changes of this item are open already. Finish or close one in the item first.",
  "error.rotationOpen":
    "This login has an open password change. Restore it from the trash, then choose It worked or It didn't work on the item. After that you can delete it forever.",
  "error.rotationNotSupported":
    "Mosaic can't guide this change. It needs a login with an https address. Change the password on the site yourself.",
  "error.historyNotFound": "That earlier password isn't in the history any more. Reopen the item to see the current list.",
  "error.originNotRelated":
    "That page doesn't belong to this login's site, so Mosaic didn't add it. Edit the item's addresses if it should.",
  "error.breachConsentRequired":
    "The breach check is off. Turn it on and agree to what it sends first.",
  "error.breachRunning": "A breach check is already running. Wait for it to finish, or stop it first.",
  "error.rotationTooFar":
    "This change already reached your account, so it can't be discarded. Continue it to finish.",
  "error.rotationJumpTooLarge":
    "This device is too far behind the other devices to catch up. Set it up again: remove the account from this device and join again with your Emergency Kit.",
  "error.rotationNotApplied":
    "The change never reached your account, so nothing was changed. Start again or discard it.",
  "error.secretKeyUnchanged":
    "That Secret Key no longer works. Use the one from the Emergency Kit you saved on the device that made the change.",
  "error.passwordUnchanged": "The new master password has to be different from the current one.",
  "error.profileDowngrade":
    "That level is not stronger than what your account uses now. Pick a higher one.",
  "error.profileTooSlow":
    "Unlocking with that level would take this device more than two seconds. Pick a lower level, or upgrade anyway.",
  "error.rekeyNeedsCleanSync":
    "A re-key needs sync to be up to date and free of warnings. Sync now, look at any warning, then try again.",
  "error.rekeyMemberKeyMismatch":
    "You have not compared the safety code of everyone in a shared vault yet, so that vault can't be renewed. Compare their codes in the vault's sharing settings, then try again.",
  "error.rekeyBusy": "Your vaults kept changing while Mosaic worked. Wait a moment and try again.",
  "error.rekeyOutcomeUnknown":
    "Mosaic couldn't confirm whether your account accepted the new keys. Choose Continue key rotation to find out.",
  "error.rekeyNeedsServer":
    "Re-keying needs your sync server or folder, and it can't be reached. Check the connection and try again.",
  "error.rekeyNoAccountFile":
    "Nothing in your sync folder opens with this password and Secret Key. Check the new Emergency Kit.",
  "error.syncAccountRekeyed":
    "Your account's keys were renewed on another device. Use the Secret Key from the new Emergency Kit to continue.",
  "error.syncEpochRollback":
    "Your sync backend shows older account keys than this device has. Nothing was changed.",
  "error.syncRekeyRefused":
    "The keys your sync backend offers are not the next ones after this device's. Nothing was changed. Check your other devices.",
  "error.busy": "Mosaic is still finishing the last step. Wait a moment, then try again.",
  "error.internal":
    "Something unexpected stopped this action. Try again, and restart Mosaic if it keeps happening.",
  "error.importTooLarge":
    "This file is too large or too complex to import. Export fewer items at a time, then try again.",
  "error.importUnsupported":
    "Mosaic can't read this version of the format. Export again in a current format (for KeePass: KDBX 4 with AES or ChaCha20), then try again.",
  "error.importEncrypted":
    "This export is password-protected. Export again without encryption, import it, then delete the file.",
  "error.sshAgentEndpointInUse":
    "Another SSH agent is already running (on Windows, the OpenSSH Authentication Agent service). Stop it, then turn on Mosaic's SSH agent again.",
  "error.cancelled": "Cancelled. Nothing changed.",
  "error.quickUnlockExpired":
    "Quick unlock expired. Unlock with your master password, then turn it on again in Settings.",
  "error.quickUnlockInvalidated":
    "Quick unlock stopped working on this device and was turned off. Unlock with your master password.",
  "error.passwordRequired":
    "Enter your master password this time. Quick unlock works again after that.",
  "error.hardwareKeyUnsupported": "This security key or system can't be used with Mosaic.",
  "error.hardwareKeyNotFound": "No security key found. Insert it and try again.",
  "error.hardwareKeyTimeout": "The key wasn't touched in time. Try again.",
  "error.hardwareKeyPinRequired": "Enter your security key's PIN.",
  "error.hardwareKeyPinInvalid": "Wrong PIN. After several wrong tries the key locks itself.",
  "error.hardwareKeyPinBlocked":
    "This key is locked after too many wrong PINs. Reset it with its maker's tool; its registrations are then lost.",
  "error.hardwareKeyNoPin": "Set a PIN on this key first, with its maker's tool.",
  "error.hardwareKeyNotRecognized":
    "This key isn't registered here. Try another key or your master password.",
  "error.hardwareKeyLimit": "You've registered the most keys allowed (8). Remove one first.",
  "error.hardwareKeyLast":
    "This is the vault's only key. Register another one before you remove it.",
  "error.itemTypeNotAllowed": "This vault holds one-time codes only.",
  "error.otpScanUnsupported":
    "Screen scanning isn't available on this system. Paste a screenshot instead.",
  "error.otpQrNotFound": "No QR code found. Make sure the whole code is visible, then try again.",
  "error.otpQrNotOtp":
    "This QR code isn't a two-factor setup code. Look for the code on the site's 2FA setup page.",
  "error.otpQrMigration":
    "This is a Google Authenticator export code, which Mosaic can't read yet. Set up each account again on its site, or enter its setup key.",
  "error.otpQrMultiple":
    "More than one setup code is visible. Show only the one you want, then try again.",
  "error.otpQrInvalid":
    "This setup code uses settings Mosaic doesn't support. Enter the setup key instead.",
  "error.otpUriInvalid": "This isn't a valid otpauth:// link. Copy the whole link again.",
  "error.otpKeyInvalid":
    "This key isn't valid. It uses only the letters A to Z and the digits 2 to 7; check for typos.",
  "error.otpImageUnreadable": "Mosaic can't read this image. Use a PNG or JPEG file.",
  "error.otpImageTooLarge": "This image is too large. Use a screenshot of just the QR code.",
  "error.otpClipboardNoImage":
    "The clipboard has no image. Take a screenshot of the QR code, then paste it here.",
  "error.otpPendingExpired": "The scanned code is no longer available. Scan it again.",
  "error.syncNotConfigured":
    "Sync isn't set up on this device yet. Set it up in Settings first.",
  "error.syncAlreadyConfigured":
    "Sync is already set up. Disconnect first to use a different server or folder.",
  "error.syncServerUnreachable":
    "Mosaic couldn't reach the server. Check the address and your internet connection, then try again.",
  "error.syncServerInvalid":
    "This doesn't look like a Mosaic server. Check the address; it should look like https://sync.example.com.",
  "error.syncServerInsecure":
    "Mosaic only connects to servers over https, so nobody on the way can read or change your data. Use an https:// address.",
  "error.syncServerMismatch":
    "This isn't the server this account syncs with. Check the address.",
  "error.syncServerIncompatible":
    "This server runs a version that doesn't match your Mosaic. Update Mosaic or ask the server's owner to update it.",
  "error.syncRegistrationClosed":
    "This server isn't accepting new sync accounts. Sign in to an existing one or ask the server's owner.",
  "error.syncInviteInvalid":
    "This invite code isn't valid or was already used. Check it or ask the server's owner for a new one.",
  "error.syncAccountExists":
    "You already have a sync account on this server. Choose Sign in instead.",
  "error.syncAccountMismatch":
    "The sync account on that server belongs to a different Mosaic account. Check the address and your email.",
  "error.syncTwoFactorRequired":
    "This sync account needs a code from your authenticator app. Enter the 6-digit code.",
  "error.syncTwoFactorInvalid":
    "That code isn't right or has expired. Enter the current 6-digit code from your authenticator app.",
  "error.syncSignedOut":
    "The server signed this device out. Sign in again to keep syncing. Your changes are safe on this device.",
  "error.syncRateLimited":
    "Too many attempts. Wait a few minutes, then try again.",
  "error.syncQuotaExceeded":
    "The server or folder has no room left for your data. Free up space or ask the server's owner for more.",
  "error.syncFolderUnavailable":
    "Mosaic can't use this folder. Check that it exists, that your cloud service has finished syncing it and that Mosaic may write to it.",
  "error.syncFolderNotEmpty":
    "This folder holds other files, so Mosaic won't write there. Choose an empty folder or create a new one.",
  "error.syncFolderNoAccount":
    "This folder holds no sync data for this account. Choose the folder your other device syncs to.",
  "error.syncFolderDamaged":
    "Some sync files in this folder are damaged. Let your cloud service finish syncing or restore the folder from a backup, then try again.",
  "error.syncPasswordOutcomeUnknown":
    "We couldn't confirm whether the server changed your master password. Keep both passwords until sync confirms which one applies.",
  "error.syncPasswordNeedsServer":
    "Changing your master password needs your sync server, and it can't be reached. Check your internet connection and try again.",
  "error.syncDeviceIsCurrent":
    "You can't remove the device you're using. Use Disconnect instead.",
  "error.syncChoiceExpired":
    "The folder choice timed out. Choose the folder again.",
  "error.syncFreshLoginExpired":
    "Your password check timed out. Enter your master password again.",
  "error.syncPasswordRollback":
    "The server offered an older master password. Check the server.",
  "error.syncPasswordChanged":
    "Your master password was changed on the server or in the folder. Enter the new one below.",
  "error.sharingNotHosted":
    "Sharing needs a Mosaic server. Connect one in the Sync settings first.",
  "error.sharingContactUnverified":
    "Compare this person's safety code first. Mosaic only shares with people you verified.",
  "error.sharingKeyChanged":
    "This person's key changed, or the code doesn't match. Look them up again and compare the new code before you go on.",
  "error.sharingRoleForbidden":
    "Your role in this vault doesn't allow that. Ask the owner or a manager.",
  "error.sharingInviteExists":
    "An invite for this person and vault is waiting already. Withdraw it first if you want to send a new one.",
  "error.sharingInviteStale":
    "The invite changed in the meantime, for example after a key change. Open it again.",
  "error.sharingInviteExpired":
    "This invite ran out or was withdrawn. Ask the sender to invite you again.",
  "error.sharingInviteUnreadable":
    "This invite can't be opened. It may be damaged or not meant for this account. Ask the sender to invite you again.",
  "error.sharingInviteNotInRoster":
    "The vault's member list doesn't match this invite, so Mosaic didn't open it. Ask the sender to invite you again.",
  "error.sharingOwnerCannotLeave":
    "The owner can't leave a vault. Remove the other people instead.",
  "error.sharingAlreadyMember":
    "This person is a member of the vault already.",
  "error.sharingReinviteBlocked":
    "You removed this person recently. You can invite them again once the vault key has changed. Sync, then try again.",
  "error.sharingInviteLimit":
    "Too many invites are waiting. Wait for answers or withdraw some.",
  "error.sharingVaultKnown":
    "This device has that vault already, so Mosaic didn't accept a key for it. Ask the sender which vault they shared.",
  "error.syncHasSharedVaults":
    "You can't switch servers while shared vaults are on this device. Leave the shared vaults, or have their owners remove you, first.",
  "error.emergencyWaiting":
    "A request is waiting, so this can't change now. Refuse it or wait for it.",
  "error.emergencyCooldown":
    "Your last request was refused. You can ask again after the waiting period of one day.",
  "error.emergencyNotGranted":
    "There is nothing to claim: no request is open, or the person didn't grant you access.",
} as const satisfies Record<string, string>;

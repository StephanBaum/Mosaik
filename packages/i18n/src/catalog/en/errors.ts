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
  "error.reauthRequired": "This change needs your master password. Enter it to confirm.",
  "error.attachmentTooLarge": "That file is larger than 50 MiB. Choose a smaller file.",
  "error.attachmentQuota":
    "Your files would use more than the 1 GiB available. Remove some attachments, then try again.",
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
} as const satisfies Record<string, string>;

// UNTRANSLATED: copy of English, replace every value
/**
 * Japanese UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Not synced yet",
  "mobile.sync.last": "Synced {time}",
  "mobile.sync.localOnly": "This vault is only on this phone. Sync is not set up.",
  "mobile.sync.pending": "{count, plural, other {# changes waiting to upload}}",
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
    "{count, plural, other {# received items}} couldn't be read on this phone. Update Mosaic on all your devices, then sync again.",
  "mobile.sync.warning.accountRollback":
    "The server shows an older account key than this phone has. Nothing was changed. Open Mosaic on another device and check that your latest changes are there. Then choose “I've checked”.",
  "mobile.sync.acknowledge": "I've checked…",
  "mobile.sync.acknowledgeTitle": "Clear the sync warnings?",
  "mobile.sync.acknowledgeBody":
    "Only go on after you have checked the server yourself. This clears every warning, and syncing continues. If your data really was rolled back or changed, Mosaic can't tell afterwards. Do this on each of your devices.",
  "mobile.sync.acknowledgeConfirm": "I've checked",
  "mobile.folder.intro": "This account syncs through your {provider} folder.",
  "mobile.folder.signIn": "Sign in to {provider}",
  "mobile.folder.scope":
    "{provider} will ask for access to all your files. Mosaic only opens the folder you choose and never touches anything else.",
  "mobile.folder.waiting": "Finish signing in in your browser, then come back here.",
  "mobile.folder.serverLabel": "Nextcloud or WebDAV address",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic will sign in at {server}. Only continue if this is your own server.",
  "mobile.folder.locating": "Looking for your Mosaic folder…",
  "mobile.folder.browseTitle": "Choose your Mosaic folder",
  "mobile.folder.browseHint":
    "It's the folder your computer syncs, with a file named mosaic-sync.json.",
  "mobile.folder.currentFolder": "Current folder: {path}",
  "mobile.folder.noSubfolders": "There are no folders here.",
  "mobile.folder.useThis": "Use this folder",
  "mobile.folder.up": "Up one level",
  "mobile.folder.chooseProvider": "Where does your account sync?",
  "mobile.folder.noDrive":
    "Google Drive and iCloud Drive don't work on Android yet. Use Dropbox, OneDrive, Nextcloud or Mosaic sync.",
  "mobile.folder.useFolder": "Sync through a cloud folder instead",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud or WebDAV",
  "mobile.folder.appPasswordToggle": "Use an app password instead",
  "mobile.folder.davLabel": "WebDAV folder address",
  "mobile.folder.loginLabel": "Username",
  "mobile.folder.appPasswordLabel": "App password",
  "mobile.folder.appPasswordHint":
    "Create an app password in your server's security settings. Don't use your main password.",
  "mobile.folder.appPasswordAction": "Sign in with the app password",
  "mobile.folder.pickAccount": "Which account is this phone joining?",
  "mobile.folder.accountOption": "Account {id}",
  "mobile.folder.linked": "Signed in. Syncing again.",
  "mobile.folder.settingsLine": "Syncs through {provider}: {path}",
  "mobile.folder.relink": "Sign in again",
  "mobile.folder.msRevokeHint":
    "To remove Mosaic's access completely, remove it on your Microsoft account page under Apps.",
  "mobile.reset.revokeDropbox":
    "The account was removed. We could not end Mosaic's access at Dropbox: remove it in Dropbox under Settings, Connected apps.",
  "mobile.reset.revokeWebdav":
    "The account was removed. We could not delete the app password on your server: remove it there under Settings, Security, Devices and sessions.",
  "mobile.reset.title": "Remove this account from this phone",
  "mobile.reset.body":
    "This deletes the vault copy, the Secret Key and the cloud sign-in stored on this phone. Your data in the cloud folder or on the server stays. You can join again later with your Secret Key and master password.",
  "mobile.reset.action": "Remove account…",
  "mobile.reset.confirm": "Remove account",
  "mobile.reset.phraseLabel": "Confirmation",
  "mobile.reset.typePhrase": "Type {phrase} to confirm",
} satisfies Record<keyof typeof en, string>;

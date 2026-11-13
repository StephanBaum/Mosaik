/** English UI strings: browser extension (pairing, saving, passkeys, settings). ICU MessageFormat. */
export const browser = {
  "browser.pair.title": "A browser wants to connect to Mosaic",
  "browser.pair.description":
    "Only allow this if you just clicked Connect in the Mosaic extension and it shows this code.",
  "browser.pair.code": "Code",
  "browser.pair.browser": "Browser",
  "browser.pair.label": "Name for this browser",
  "browser.pair.labelHint": "You will see this name in Settings.",
  "browser.pair.deny": "Deny",
  "browser.pair.allow": "Allow",
  "browser.expires": "Closes automatically in {seconds, plural, one {# second} other {# seconds}}",
  "browser.escDenies": "to deny",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Browser",
  "browser.family.chromium": "Chromium-based browser",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "to cancel",
  "browser.save.titleNew": "Save this login in Mosaic?",
  "browser.save.titleUpdate": "Update this login in Mosaic?",
  "browser.save.description":
    "{browser} offers to save a login. Mosaic writes it only if you choose Save.",
  "browser.save.descriptionUpdate":
    "{browser} offers a new password for this login. Mosaic changes it only if you choose Update password.",
  "browser.save.site": "Website",
  "browser.save.username": "User name",
  "browser.save.noUsername": "None",
  "browser.save.password": "Password",
  "browser.save.passwordHidden": "Hidden",
  "browser.save.target": "Save as",
  "browser.save.updateOption": "Update “{title}”",
  "browser.save.newOption": "A new login",
  "browser.save.itemTitle": "Title",
  "browser.save.vault": "Vault",
  "browser.save.allowHttp": "Allow filling on this unencrypted page",
  "browser.save.allowHttpHint":
    "The site doesn't use HTTPS. Anyone on the network could read what you fill in.",
  "browser.save.cancel": "Cancel",
  "browser.save.saveNew": "Save",
  "browser.save.update": "Update password",
  "browser.passkey.secure": "Secure connection (https)",
  "browser.passkey.getTitle": "Sign in to {rpId} with a passkey",
  "browser.passkey.createTitle": "Save a new passkey",
  "browser.passkey.excludedTitle": "Passkey already saved",
  "browser.passkey.createBody": "Create a passkey for {rpId} in Mosaic?",
  "browser.passkey.excludedBody": "You already have a passkey for {rpId} in Mosaic.",
  "browser.passkey.uvNote":
    "The site asked for user verification. Enter your master password to confirm it's you.",
  "browser.passkey.password": "Master password",
  "browser.passkey.account": "Account",
  "browser.passkey.noName": "No name",
  "browser.passkey.vault": "Vault",
  "browser.passkey.signIn": "Sign in with passkey",
  "browser.passkey.save": "Save passkey",
  "browser.passkey.anotherDevice": "Use another device or security key",
  "browser.passkey.cancel": "Cancel",
  "browser.passkey.ok": "Got it",
  "browser.passkey.escCloses": "to close",
  "browser.passkey.passwordNeeded": "Enter your master password to confirm it's you.",
  "browser.settings.title": "Browsers",
  "browser.settings.body":
    "Browser extensions that fill logins from this app. You approve each connection here.",
  "browser.settings.empty":
    "No browser is connected yet. Install the Mosaic extension, then choose Connect in it.",
  "browser.settings.connected": "Connected now",
  "browser.settings.lastUsed": "Last used {when}",
  "browser.settings.neverUsed": "Not used yet",
  "browser.settings.disconnect": "Disconnect",
  "browser.settings.disconnectNamed": "Disconnect {name}",
  "browser.settings.disconnectTitle": "Disconnect “{name}”?",
  "browser.settings.disconnectBody":
    "This browser can't fill logins until you connect it again from the extension.",
  "browser.settings.endpointInUse":
    "Another program, or another Mosaic window, already uses the connection for browsers. Close it, then restart Mosaic.",
  "browser.settings.endpointUnavailable":
    "Mosaic couldn't open the connection for browsers. Restart Mosaic. If it keeps happening, check that your user folder is writable.",
  "browser.settings.registryUnreadable":
    "Saved browser connections can't be read. Mosaic won't change them until this is fixed.",
  "browser.connector.title":
    "A browser connection points somewhere else",
  "browser.connector.body":
    "The connection for {browser} starts a different program instead of Mosaic: {path}. It may be an older Mosaic, or something else that registered itself. Repair points it to this Mosaic again.",
  "browser.connector.item":
    "{browser}: {path}",
  "browser.connector.repair":
    "Repair",
  "browser.connector.repaired":
    "Browser connection repaired.",
} as const satisfies Record<string, string>;

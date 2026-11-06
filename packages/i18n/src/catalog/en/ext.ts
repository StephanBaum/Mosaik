/** English UI strings: browser extension (manifest, menus, popup). ICU MessageFormat. */
export const ext = {
  // Manifest and menu strings (`_locales`, built at build time).
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Fill logins from your Mosaic vault. Your master password stays in the desktop app.",
  "ext.command.fill": "Fill login with Mosaic",
  "ext.command.generate": "Generate a password with Mosaic",
  "ext.save.badgeTitle": "Save login?",

  // Shell
  "ext.loading": "Loading…",
  "ext.site.secure": "Current site, secure connection (https): {host}",
  "ext.site.insecure": "Current site, not encrypted (http): {host}",
  "ext.site.none": "This tab has no website",
  "ext.nav.label": "Views",
  "ext.nav.logins": "Logins and search",
  "ext.nav.generator": "Password generator",
  "ext.footer.open": "Open Mosaic",
  "ext.footer.more": "More options",
  "ext.footer.connected": "Connected to Mosaic",
  "ext.footer.locked": "Mosaic is locked",
  "ext.footer.offline": "Not connected",
  "ext.more.saveOffers": "Offer to save logins",
  "ext.more.saveOffersHint":
    "Needs access to all websites, so the extension can notice a login you submit. Nothing is saved until you confirm in the Mosaic app.",
  "ext.more.disconnect": "Disconnect from Mosaic",
  "ext.more.disconnectTitle": "Disconnect this browser?",
  "ext.more.disconnectBody":
    "This browser can't fill logins until you connect it again. You approve the new connection in the Mosaic app.",
  "ext.action.cancel": "Cancel",

  // Connecting
  "ext.connect.checking": "Connecting to Mosaic…",
  "ext.connect.hostMissing.title": "Mosaic isn't set up for this browser",
  "ext.connect.hostMissing.body":
    "Install the Mosaic app on this computer, then restart the browser.",
  "ext.connect.hostMissing.dev": "Developers: run {command}, then restart the browser.",
  "ext.connect.hostForbidden.title": "This browser isn't allowed to connect",
  "ext.connect.hostForbidden.body":
    "Mosaic talks only to the official Mosaic extension. Install it from your browser's store.",
  "ext.connect.desktopNotRunning.title": "Open Mosaic on this computer",
  "ext.connect.desktopNotRunning.body":
    "The extension works through the Mosaic app. Start the app, then check again.",
  "ext.connect.check": "Check again",
  "ext.connect.notPaired.title": "Connect to Mosaic",
  "ext.connect.notPaired.body":
    "Connect this browser to the Mosaic app on this computer. You approve it in the app.",
  "ext.connect.start": "Connect",
  "ext.connect.pairing.title": "Confirm in Mosaic",
  "ext.connect.pairing.body": "Check that Mosaic shows the same code, then click Allow there.",
  "ext.connect.pairing.code": "Code",
  "ext.connect.pairing.waiting": "Waiting for your answer in the Mosaic app…",
  "ext.connect.protocolMismatch.title": "Update Mosaic",
  "ext.connect.protocolMismatch.body":
    "The extension and the Mosaic app are on different versions. Update both to the latest version.",
  "ext.connect.error.title": "Mosaic can't be reached",
  "ext.connect.error.body": "Check that the Mosaic app is running, then try again.",

  // Locked
  "ext.locked.title": "Mosaic is locked",
  "ext.locked.body": "Unlock it in the Mosaic app to fill on {host}.",
  "ext.locked.bodyNoSite": "Unlock it in the Mosaic app to use it here.",
  "ext.locked.open": "Open Mosaic",
  "ext.locked.note": "Your master password is only ever typed into the Mosaic app.",

  // Logins
  "ext.logins.forSite": "For {host}",
  "ext.logins.count": "{count, plural, one {# login} other {# logins}}",
  "ext.logins.sameSite": "same site",
  "ext.logins.none": "No saved login for {host}.",
  "ext.logins.noneHint":
    "Mosaic compares the full address, so lookalike sites never get a match.",
  "ext.logins.unsupported": "Mosaic can't fill on this page. Search your vault to copy a login.",
  "ext.logins.shortcut": "{shortcut} fills the first match.",
  "ext.logins.frameNote":
    "Mosaic fills only on {host}, never inside embedded frames from other sites.",
  "ext.item.untitled": "Untitled",
  "ext.item.noUsername": "No user name",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Fill",
  "ext.item.fillLabel": "Fill {name}",
  "ext.item.copyMenu": "Copy options for {name}",
  "ext.item.copyUsername": "Copy user name",
  "ext.item.copyPassword": "Copy password",
  "ext.item.copyCode": "Copy code",
  "ext.item.blocked": "Blocked: this page isn't encrypted and this login doesn't allow it.",
  "ext.search.label": "Other items",
  "ext.search.placeholder": "Search your vault",
  "ext.search.note":
    "Results stay in this popup. The page only ever receives the one login you fill.",
  "ext.search.none": "Nothing found.",
  "ext.search.copy": "Copy",
  "ext.search.copyOnly": "Copy only: not saved for this site",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# result} other {# results}}",

  // Frames
  "ext.frame.cross": "The field in focus is in a frame from another site. Mosaic doesn't fill there.",
  "ext.frameConfirm.title": "Fill inside an embedded frame?",
  "ext.frameConfirm.body":
    "The field is in a frame on {host}, not in the page itself. Only fill here if you trust this frame.",
  "ext.frameConfirm.confirm": "Fill in the frame",

  // Address warnings
  "ext.warning.idn.title": "Check the address",
  "ext.warning.idn.body":
    "This address uses special characters and can look like another site. Check the address bar before you sign in.",
  "ext.warning.lookalike.body":
    "This site isn't {saved}. It looks similar, which is a common phishing trick. Close this tab, or type {saved} yourself.",
  "ext.warning.youAreOn": "You're on",
  "ext.warning.writtenAs": "Written as",
  "ext.warning.savedFor": "Saved for",
  "ext.warning.insecure.title": "This page isn't encrypted",
  "ext.warning.insecure.body":
    "The page uses http. Others on the network could read your password. Mosaic fills here only for logins you allowed it for.",

  // One-time codes
  "ext.otp.title": "Fill your one-time code",
  "ext.otp.next": "Password filled for {name}. Next: the two-factor code.",
  "ext.otp.for": "Code for {name}",
  "ext.otp.fill": "Fill code",
  "ext.otp.copy": "Copy code",
  "ext.otp.note":
    "The code goes from Mosaic straight into the field. This popup never shows it.",

  // Saving
  "ext.save.titleNew": "Save this login for {host}?",
  "ext.save.titleUpdate": "Update the saved login for {host}?",
  "ext.save.generated": "A password Mosaic generated for {host}.",
  "ext.save.updates": "Updates “{title}”.",
  "ext.save.username": "User name",
  "ext.save.noUsername": "None",
  "ext.save.password": "Password",
  "ext.save.passwordHidden": "Hidden. You see it in the Mosaic app.",
  "ext.save.insecure": "This page isn't encrypted (http).",
  "ext.save.action": "Save in Mosaic…",
  "ext.save.notNow": "Not now",
  "ext.save.never": "Never for this site",
  "ext.save.waiting": "Confirm in the Mosaic app. Nothing is saved until you do.",
  "ext.save.saved": "Saved in Mosaic.",
  "ext.save.updated": "Updated in Mosaic.",
  "ext.save.cancelled": "Not saved.",
  "ext.save.dismissed": "Dropped. The password is forgotten.",
  "ext.save.neverDone": "Mosaic won't offer to save on {host} again.",
  "ext.save.permissionAsk": "Mosaic asks your browser for access to websites. Allow it to turn this on.",

  // Generator
  "ext.generator.title": "Generate a password for {host}",
  "ext.generator.titleGeneric": "Generate a password",
  "ext.generator.value": "Generated password",
  "ext.generator.regenerate": "Generate another password",
  "ext.generator.type": "Password type",
  "ext.generator.random": "Random",
  "ext.generator.passphrase": "Passphrase",
  "ext.generator.include": "Include",
  "ext.generator.rules.page": "Follows this page's password rules",
  "ext.generator.rules.siteQuirks": "Follows the known rules for this site",
  "ext.generator.fill": "Fill",
  "ext.generator.fillHint":
    "To fill, click into a new-password field on the page, then open this popup again.",
  "ext.generator.copy": "Copy",
  "ext.generator.expired": "That password was no longer held, so a new one was generated.",
  "ext.generator.filled": "Filled. Mosaic will offer to save it.",

  // Announcements
  "ext.copied": "Copied. Clears in {seconds, plural, one {# second} other {# seconds}}.",
  "ext.copiedNoClear": "Copied.",
  "ext.filled": "Filled.",
  "ext.filledCode": "Code filled.",

  // Errors: every code the extension can show (`ErrorCode`), what happened and what to do.
  "ext.error.invalidRequest":
    "Mosaic couldn't understand that request. Update the extension and the Mosaic app.",
  "ext.error.notPaired": "This browser isn't connected to Mosaic. Connect it again.",
  "ext.error.protocolVersion":
    "The extension and the Mosaic app are on different versions. Update both.",
  "ext.error.busy": "Mosaic is busy with another request. Try again in a moment.",
  "ext.error.locked": "Mosaic is locked. Unlock it in the Mosaic app.",
  "ext.error.rateLimited": "That was too many requests in a short time. Wait a moment, then try again.",
  "ext.error.originMismatch":
    "This login isn't saved for this website, so Mosaic won't fill it here.",
  "ext.error.insecureOrigin":
    "This page isn't encrypted and the login doesn't allow filling on it. Nothing was filled.",
  "ext.error.crossOriginFrame":
    "That field is in a frame from another site. Mosaic doesn't fill there.",
  "ext.error.itemNotFound": "That item no longer exists. It may have been moved or deleted.",
  "ext.error.vaultUnavailable": "That vault is closed. Open it in the Mosaic app first.",
  "ext.error.noOtp": "This item has no one-time code.",
  "ext.error.otpNotFillable":
    "This code counts up with every use (HOTP), so it can't be filled or copied from here. Use it in the Mosaic app.",
  "ext.error.fieldEmpty": "That field is empty, so there's nothing to fill or copy.",
  "ext.error.clipboardUnavailable":
    "Mosaic couldn't use the clipboard. Try again, or copy it in the Mosaic app.",
  "ext.error.cancelled": "Cancelled in the Mosaic app. Try again when you're ready.",
  "ext.error.pairingDenied":
    "The connection was denied in the Mosaic app. Choose Connect to try again, then choose Allow.",
  "ext.error.pairingExpired": "The connection request ran out of time. Choose Connect to start again.",
  "ext.error.pairingConflict": "Another connection request was open. Choose Connect to try again.",
  "ext.error.pairingBlocked": "Too many connections were denied. Wait ten minutes, then try again.",
  "ext.error.pairingLimit":
    "This computer has too many connected browsers. Disconnect one in the Mosaic settings.",
  "ext.error.internal":
    "Mosaic hit an unexpected problem. Try again, and if it repeats, restart the Mosaic app.",
  "ext.error.desktopNotRunning": "Mosaic isn't running on this computer. Open it, then try again.",
  "ext.error.desktopNotAuthentic":
    "The app that answered isn't the Mosaic app this browser is connected to. Nothing was sent.",
  "ext.error.timeout":
    "Mosaic didn't answer in time. If a dialog is open in the Mosaic app, answer it there.",
  "ext.error.frameTooLarge": "That was too much data to send to Mosaic.",
  "ext.error.hostMissing":
    "The Mosaic helper isn't installed for this browser. Install the Mosaic app, then restart the browser.",
  "ext.error.hostForbidden": "This browser or extension isn't allowed to talk to Mosaic.",
  "ext.error.hostExited": "The connection to Mosaic closed. Try again.",
  "ext.error.unsupportedPage": "Mosaic can't fill on this kind of page.",
  "ext.error.noFields": "No login fields found here. Click into a field, then try again.",
  "ext.error.noMatch": "Mosaic has no saved login for this site.",
  "ext.error.frameNeedsConfirmation":
    "The field is inside a frame. Open the Mosaic popup and confirm the frame to fill it.",
  "ext.error.frameOpaque": "The field is in a sandboxed frame. Mosaic doesn't fill there.",
  "ext.error.originChanged": "The page changed while Mosaic was filling. Nothing was filled. Try again.",
  "ext.error.pageHidden": "The page isn't visible, so Mosaic didn't fill.",
  "ext.error.permissionDenied":
    "Mosaic needs access to websites to notice logins, and you didn't allow it.",
  "ext.error.generatedExpired":
    "Mosaic no longer holds that password. Generate a new one.",
} as const;

/** English UI strings: sharing, safety codes, members and emergency access (M6b). ICU MessageFormat. */
export const sharing = {
  "sharing.title": "Sharing",
  "sharing.body":
    "Share vaults with people you know. Everything stays encrypted: only the people you invite can read a shared vault, and Mosaic only shares with people whose safety code you compared.",
  "sharing.notHosted.none":
    "Sharing needs a Mosaic server. Turn on sync with a Mosaic server in the Sync settings first.",
  "sharing.notHosted.folder":
    "Sharing isn't available with a sync folder. A folder can't tell who may read a vault, so Mosaic keeps vaults in a folder private to you. To share, disconnect the folder and connect a Mosaic server in the Sync settings.",
  "sharing.notHosted": "Sharing needs a Mosaic server. Connect one in the Sync settings first.",

  "sharing.ownCode.title": "Your safety code",
  "sharing.ownCode.body":
    "Read this code to the person you share with, in person or on the phone. If it matches what they see for you, nobody sits between you.",
  "sharing.ownCode.pending": "Your safety code appears after the first sync with the server.",

  "sharing.contacts.title": "People you looked up",
  "sharing.contacts.empty": "You haven't looked anyone up yet.",
  "sharing.contacts.verified": "Verified",
  "sharing.contacts.notVerified": "Not verified",
  "sharing.contacts.changed": "Needs a new check",
  "sharing.contacts.verify": "Compare code…",
  "sharing.contacts.review": "Review change…",

  "sharing.share.open": "Share…",
  "sharing.share.title": "Share {vault}",
  "sharing.share.dialog": "Sharing: {vault}",
  "sharing.lookup.email": "Their email address",
  "sharing.lookup.find": "Look up",
  "sharing.lookup.searching": "Looking up…",
  "sharing.lookup.notFoundHint":
    "Mosaic can't tell you whether an address has an account. If the code below doesn't match what the person sees, don't share.",

  "sharing.code.title": "Compare safety codes",
  "sharing.code.compare":
    "Compare this code with {email}, in person or on the phone. Don't use email or chat.",
  "sharing.code.shown": "Code Mosaic shows for {email}",
  "sharing.code.typed": "Code they read to you (optional)",
  "sharing.code.typedMatch": "This is the code above.",
  "sharing.code.matches": "It matches",
  "sharing.code.mismatch":
    "The codes differ. Don't share. Someone may sit between you and {email}.",
  "sharing.code.mismatchHelp":
    "Check the address and ask {email} to read the code again. If it still differs, cancel and tell them another way.",
  "sharing.code.verified": "{email} is verified.",
  "sharing.code.status.new": "New contact",
  "sharing.code.status.pinned": "Not verified yet",
  "sharing.code.status.verified": "Verified",
  "sharing.code.status.changed": "Key changed",
  "sharing.code.status.conflicting": "Devices disagree",
  "sharing.code.conflicting":
    "Your devices saw different keys for {email}. Compare the code with them again before you share.",

  "sharing.role.label": "Role",
  "sharing.role.owner": "Owner",
  "sharing.role.manage": "Manager",
  "sharing.role.write": "Editor",
  "sharing.role.read": "Viewer",
  "sharing.role.desc.manage": "Can invite and remove people and change roles.",
  "sharing.role.desc.write": "Can add and change items.",
  "sharing.role.desc.read": "Can read only.",
  "sharing.role.honesty":
    "Anyone who can read an item can copy it. A viewer can't change the vault, but nothing stops them from keeping what they see, and removing someone later doesn't take back what they already downloaded.",

  "sharing.invite.send": "Send invite",
  "sharing.invite.sent": "Invite sent to {email}. They need to open Mosaic and accept it.",
  "sharing.invite.pending": "Waiting for {email}",
  "sharing.invite.expires": "Runs out {when}",
  "sharing.invite.cancel": "Withdraw invite",
  "sharing.invite.cancelled": "Invite withdrawn.",

  "sharing.inbox.title": "Invites",
  "sharing.inbox.nav": "Invites",
  "sharing.inbox.navCount": "Invites, {count} waiting",
  "sharing.inbox.intro":
    "People who invited you to a shared vault. Mosaic checks every invite before it shows it to you.",
  "sharing.inbox.empty":
    "No invites right now. When someone invites you to a shared vault, it shows up here.",
  "sharing.inbox.from": "From {email}",
  "sharing.inbox.review": "Review",
  "sharing.inbox.refresh": "Refresh",
  "sharing.inbox.preview.title": "Join {vault}?",
  "sharing.inbox.preview.body": "{email} invites you as {role}.",
  "sharing.inbox.preview.unnamed": "a shared vault",
  "sharing.inbox.preview.owner": "The vault belongs to {email}.",
  "sharing.inbox.verifySender": "Before you join, compare {email}'s safety code.",
  "sharing.inbox.verifyOwner":
    "{sender} isn't the owner. Before you join, compare the owner's safety code, too: it is the one that can change the vault's key.",
  "sharing.inbox.ownerCode": "Code for the owner, {email}",
  "sharing.inbox.accept": "Join vault",
  "sharing.inbox.decline": "Decline",
  "sharing.inbox.block": "Decline and block sender",
  "sharing.inbox.unreadable":
    "This invite can't be opened. It may be damaged or not meant for you. Ask {email} to send it again.",
  "sharing.inbox.joined": "You joined the vault. It shows up in your vault list in a moment.",
  "sharing.inbox.declined": "Invite declined.",
  "sharing.inbox.outgoing": "Invites you sent",
  "sharing.inbox.outgoing.empty": "None of your invites is waiting for an answer.",
  "sharing.inbox.unknownVault": "a vault",

  "sharing.members.title": "People with access",
  "sharing.members.you": "You",
  "sharing.members.unverified": "Not verified",
  "sharing.members.unconfirmed": "Not on the member list",
  "sharing.members.unconfirmed.help":
    "The server lists this person, but the vault's own member list doesn't. The next key change locks them out. If you don't know them, remove them.",
  "sharing.members.dropped": "No longer on the server",
  "sharing.members.dropped.help":
    "This person is on the vault's member list, but the server doesn't list them any more. They may have left. Check that this is expected.",
  "sharing.members.invited": "Invited",
  "sharing.members.removing": "Being removed",
  "sharing.members.verified": "Verified",
  "sharing.members.verify": "Verify…",
  "sharing.members.remove": "Remove",
  "sharing.members.remove.title": "Remove {email}?",
  "sharing.members.remove.confirm":
    "{email} keeps whatever they already downloaded. Mosaic changes the vault key now, and they can't read anything new.",
  "sharing.members.remove.confirmManager":
    "{email} keeps whatever they already downloaded. The server stops sending them updates now. The vault owner's device changes the vault key the next time it syncs, and until then only the server keeps {email} out.",
  "sharing.members.removed": "Removed.",
  "sharing.members.leave": "Leave vault",
  "sharing.members.leave.title": "Leave this vault?",
  "sharing.members.leave.confirm":
    "The copy on this device stays readable, but it stops updating. To join again you need a new invite.",
  "sharing.members.left": "You left the vault.",
  "sharing.members.roleOf": "Role of {email}",
  "sharing.members.invite": "Invite someone…",
  "sharing.members.empty": "Only you can open this vault so far.",
  "sharing.members.unknownEmail": "Unknown address",
  "sharing.members.managerNote":
    "You can invite and remove people. Only the owner's device changes the vault key, so a removal is complete once the owner's device has synced.",
  "sharing.members.readerNote": "You can read this vault but not change who has access.",
  "sharing.rotation.running":
    "A removal is waiting for the owner's device to change the vault key. Until it syncs, only the server keeps the removed person out.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# file keeps} other {# files keep}} the old protection because this device doesn't have {count, plural, one {it} other {them}}. A person you removed can still open {count, plural, one {it} other {them}} if they kept the old key. Add {count, plural, one {it} other {them}} again from a device that has {count, plural, one {it} other {them}}.",

  "sharing.keyChanged.title": "{email}'s safety code changed",
  "sharing.keyChanged.body":
    "The server now shows a different key for {email}. That happens when they set up Mosaic again, and it is also what an attack on the server would look like. Nothing is shared with the new key until you compared codes with them, in person or on the phone.",
  "sharing.keyChanged.old": "Code you compared before",
  "sharing.keyChanged.new": "New code",
  "sharing.keyChanged.accept": "Compare and accept",

  "sharing.finding.region": "Sharing warnings",
  "sharing.finding.unknownEmail": "an unknown person",
  "sharing.finding.verify": "Verify…",
  "sharing.finding.remove": "Remove",
  "sharing.finding.identityMismatch":
    "The server shows a different sharing key for your account than this device made, so sharing is paused. Share nothing, and check your server and your other devices. Then choose “I've checked…”.",
  "sharing.finding.rosterInvalid":
    "The member list of {vault} is damaged, or someone who isn't allowed to changed it. Don't add people to it until you checked who has access. Then choose “I've checked…”.",
  "sharing.finding.shareUntrusted":
    "A new key for {vault} was refused: {sender} sent it, but they aren't allowed to change this vault's key. The vault stays on its old key. Ask the owner whether they changed it. Then choose “I've checked…”.",
  "sharing.finding.memberUnverified":
    "You haven't verified {email} in {vault}. Mosaic sends them no new keys, and the next key change locks them out. Compare their safety code, or remove them.",
  "sharing.finding.memberDropped":
    "{email} is on the member list of {vault}, but the server doesn't list them any more. They may have left, or the server removed them. Check that this is expected, and remove them if it isn't.",
  "sharing.finding.accessLost":
    "You no longer have access to {vault}. The copy on this device stays readable but doesn't update. Ask the owner to invite you again.",
  "sharing.finding.rotationBlocked":
    "The key of {vault} couldn't be changed after you removed someone, so they still hold the old key. Sync again. If this stays, open the member list and check who has access.",

  "emergency.title": "Emergency access",
  "emergency.body":
    "Name people who can ask for access to your vaults if something happens to you. This is not account recovery: they get read-only access to the vaults you choose, only after a waiting time, and you can refuse during that time. It doesn't replace your Emergency Kit.",
  "emergency.folderUnsupported":
    "Emergency access needs the Mosaic server. A sync folder can't enforce a waiting time.",
  "emergency.granted.title": "People who can request access",
  "emergency.granted.empty": "Nobody can ask for access to your vaults.",
  "emergency.granted.add": "Add a person…",
  "emergency.granted.scopeAll": "All your vaults",
  "emergency.granted.scopeSome": "{count, plural, one {# vault} other {# vaults}}",
  "emergency.granted.wait": "Waits {days, plural, one {# day} other {# days}}",
  "emergency.granted.waitingUntil": "Asked for access. It opens {when} unless you refuse.",
  "emergency.granted.approved": "You approved the request.",
  "emergency.granted.released": "Access was released. They can read the vaults.",
  "emergency.granted.idle": "No request",
  "emergency.granted.remove": "Remove access",
  "emergency.granted.remove.title": "Remove emergency access for {email}?",
  "emergency.granted.remove.body":
    "They can't ask any more. If their access was already released, Mosaic changes the keys of the vaults they opened; they keep what they already downloaded.",
  "emergency.granted.removed": "Emergency access removed.",
  "emergency.grant.title": "Allow emergency access",
  "emergency.grant.lookup": "Who should be able to ask?",
  "emergency.grant.scope": "What they can open",
  "emergency.grant.scope.all": "All my vaults, including new ones",
  "emergency.grant.scope.selected": "Only the vaults I choose",
  "emergency.grant.scope.pick": "Pick at least one vault.",
  "emergency.grant.wait": "Waiting time",
  "emergency.grant.wait.option": "{days, plural, one {# day} other {# days}}",
  "emergency.grant.waitHint":
    "Open Mosaic at least once within this time after a request, so you can see it and refuse. When the time is up, the person gets read-only access with no further question to you.",
  "emergency.grant.serverTrust":
    "The waiting time is kept by the server. Someone who controls the server could skip it.",
  "emergency.grant.vaultKeysOnly":
    "This covers the keys of your vaults only. It doesn't give access to your account, your master password or your Secret Key.",
  "emergency.grant.submit": "Allow emergency access",
  "emergency.grant.done": "{email} can now ask for emergency access.",
  "emergency.grant.needVerify": "Compare {email}'s safety code first.",
  "emergency.received.title": "People who named you",
  "emergency.received.empty": "Nobody has named you for emergency access.",
  "emergency.received.request": "Request access",
  "emergency.received.requested": "Request sent. {email} sees it the next time they open Mosaic.",
  "emergency.received.requestHint":
    "{email} sees your request in Mosaic. After {days, plural, one {# day} other {# days}} you can open their vaults, read-only, unless they refuse.",
  "emergency.received.countdown": "You can open their vaults {when}.",
  "emergency.received.claim": "Open their vaults",
  "emergency.received.claimed":
    "{count, plural, one {# vault was} other {# vaults were}} added, read-only.",
  "emergency.received.cooldown": "{email} refused. You can ask again {when}.",
  "emergency.received.verifyFirst":
    "Compare {email}'s safety code before you open their vaults. Mosaic only uses a key you verified.",
  "emergency.received.verify": "Compare code…",
  "emergency.received.approved": "{email} approved your request.",
  "emergency.received.waitDays": "Waits {days, plural, one {# day} other {# days}}",
  "emergency.banner.region": "Emergency access request",
  "emergency.banner.body":
    "{email} asked for emergency access to your vaults. It opens {when} unless you refuse.",
  "emergency.banner.refuse": "Refuse",
  "emergency.banner.approve": "Approve now…",
  "emergency.banner.approve.title": "Approve emergency access now?",
  "emergency.banner.approve.body":
    "{email} gets read-only access to your vaults right away. Enter your master password to confirm.",
  "emergency.banner.approve.confirm": "Approve",
  "emergency.banner.refused": "Request refused.",
  "emergency.banner.approved": "Access approved.",
} as const satisfies Record<string, string>;

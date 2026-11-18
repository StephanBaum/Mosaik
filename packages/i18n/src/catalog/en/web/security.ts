/** English strings new on the Security page (website v2 §5.3). Where/How reuse landing copy. */
export const security = {
  "web.security.eyebrow": "Security",
  "web.security.title": "Trust no server. Not even ours.",
  "web.security.sub":
    "{name} locks every entry on your device before it moves, lets you choose where it’s kept, and checks that whoever keeps it plays fair.",
  "web.security.stolen.eyebrow": "If we get hacked",
  "web.security.stolen.title": "A stolen server holds noise.",
  "web.security.stolen.add":
    "Every entry is sealed on its own. Without your master password and your Secret Key, a copy of our database is a pile of scrambled files.",
  "web.security.see.title": "What we see, and what we don’t.",
  "web.security.see.yes": "We see",
  "web.security.see.yes1": "Your email address, if you use hosted sync.",
  "web.security.see.yes2": "Your devices, so you can sign one out.",
  "web.security.see.yes3": "How much locked data you store.",
  "web.security.see.no": "We never see",
  "web.security.see.no1": "Your passwords, notes, files or codes.",
  "web.security.see.no2": "Which websites you have logins for.",
  "web.security.see.no3": "Your master password or your Secret Key.",
  "web.security.see.footer":
    "Your cloud provider, with folder sync, sees scrambled files: their names, sizes and times, nothing inside.",
  "web.security.cta.secondary": "Read the comparison",
} as const satisfies Record<string, string>;

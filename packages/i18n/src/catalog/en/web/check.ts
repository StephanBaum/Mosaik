/**
 * English strings of the self-check "How exposed are you?" (website v2 §2.1, §5.5): multiple
 * choice only, no text inputs, nothing stored or sent. Plain and honest: no statistics, no fear.
 */
export const selfCheck = {
  "web.check.eyebrow": "Free self-check",
  "web.check.title": "How exposed are you?",
  "web.check.sub":
    "Seven questions, about two minutes. You only tick answers; nobody asks for a password. Your answers stay on this page and are gone when you close it.",
  "web.check.formLabel": "Self-check questions",
  "web.check.q1": "Roughly how many online accounts do you have?",
  "web.check.q1.a": "Fewer than 20",
  "web.check.q1.b": "20 to 100",
  "web.check.q1.c": "More than 100",
  "web.check.q1.d": "No idea",
  "web.check.q2": "Do you use the same password on more than one site?",
  "web.check.q2.a": "Never",
  "web.check.q2.b": "Only for a few unimportant sites",
  "web.check.q2.c": "Yes, for some important ones",
  "web.check.q2.d": "Mostly the same few passwords everywhere",
  "web.check.q3": "Where do you keep your passwords today?",
  "web.check.q3.a": "In a password manager",
  "web.check.q3.b": "Saved in my browser",
  "web.check.q3.c": "In notes, a file or on paper",
  "web.check.q3.d": "Only in my head",
  "web.check.q4": "Do your email and bank accounts use two-step sign-in (2FA)?",
  "web.check.q4.a": "Yes, all of them",
  "web.check.q4.b": "Some of them",
  "web.check.q4.c": "No",
  "web.check.q4.d": "I’m not sure",
  "web.check.q5": "If a site you use reported a leak tomorrow, what would you do?",
  "web.check.q5.a": "Change that one password, done",
  "web.check.q5.b": "Change it in several places, because I reused it",
  "web.check.q5.c": "I wouldn’t know where to start",
  "web.check.q6": "How do you come up with a new password?",
  "web.check.q6.a": "A password manager makes it",
  "web.check.q6.b": "I invent something long and new each time",
  "web.check.q6.c": "I change a familiar one a little",
  "web.check.q6.d": "I use the same pattern every time",
  "web.check.q7": "When did you last close accounts you no longer use?",
  "web.check.q7.a": "In the past year",
  "web.check.q7.b": "Longer ago",
  "web.check.q7.c": "Never",
  "web.check.submit": "Show my result",
  "web.check.incomplete": "Answer all seven questions to see your result.",
  "web.check.noscript": "The result needs JavaScript. Nothing is sent either way.",
  "web.check.restart": "Start again",
  "web.check.result.eyebrow": "Your result",
  "web.check.low.title": "You’re well protected.",
  "web.check.low.text":
    "You already do what matters most. From here a password manager mainly saves you effort, and keeps it that way on all your devices.",
  "web.check.mid.title": "A few doors are open.",
  "web.check.mid.text":
    "Nothing dramatic, but some habits make one leak more costly than it needs to be. The steps below close them, most within an afternoon.",
  "web.check.high.title": "One leak could reach a lot.",
  "web.check.high.text":
    "Right now a single leaked password could open several of your accounts. That’s common, and it’s fixable. Start with the first step below today.",
  "web.check.next": "Your next steps",
  "web.check.tip.twofa":
    "Turn on two-step sign-in for your email first. Whoever controls your email can reset almost everything else.",
  "web.check.tip.reuse":
    "Start with the accounts that unlock others: your email, then banking and shopping. Give each its own password.",
  "web.check.tip.store":
    "A note or a file can be read by anyone who gets the device or the account it syncs to. Move the passwords into an encrypted vault, then delete the file.",
  "web.check.tip.leak":
    "Write down your five most important accounts today. If one leaks, you know where to look and what to change.",
  "web.check.tip.create":
    "Patterns like a season plus a year are the first thing attackers try. Long and random wins, and you don’t have to remember it.",
  "web.check.tip.browser":
    "Browser-saved passwords beat reuse. A password manager adds passkeys, 2FA codes and filling only on the exact site.",
  "web.check.tip.many":
    "With that many accounts, nobody remembers a different password for each. That’s a job for a password manager, not your memory.",
  "web.check.tip.review":
    "Old accounts still hold your data. Close the ones you no longer use, starting with any that share a password.",
  "web.check.tip.keep": "Keep your Emergency Kit somewhere safe, so you can always get back into your vault.",
  "web.check.private": "Nothing was stored or sent. Reload the page and your answers are gone.",
  "web.check.kit": "Get the switch kit",
} as const satisfies Record<string, string>;

# Landing page copy: English

Final page copy in page order. Each section lists its id (anchor and i18n key prefix), eyebrow, headline, body, items and CTAs. Notes for the designer are in [brackets] and are not shown on the page. The only placeholder is `{PRICE}`.

---

## nav

- **Logo:** Mosaic
- **Links:** How it works (`#how-it-works`) · Features (`#features`) · Pricing (`#pricing`) · Teams (`#teams`) · FAQ (`#faq`)
- **Button:** Download free (`#download`)
- **Language switch:** EN · DE
- **Mobile menu button label (aria):** Open menu / Close menu

---

## hero

[Full-bleed hero image: dark acrylic landscape, an open field with a lone barn on a windy evening, pixel-sorted. Text sits left or lower-left on the darkest part of the sky or field. Keep the lines short as written; break them where the line breaks are.]

- **Eyebrow:** Password manager for every device

**Headline options**

1. **Every login. One click.<br>Yours alone.** *(recommended)*
2. **Remember one password.<br>Forget the rest.**
3. **Everything you log in with.<br>In one place only you can open.**

[Why 1: it says the whole promise in six words: everything in one place, filled in for you, private. It reads well in three short beats over the image. Option 2 is the fallback if the hero needs to be warmer and simpler.]

- **Subhead:**
  Passwords, passkeys and 2FA codes in one place.<br>
  Filled in with a click, on every device.<br>
  Locked so that only you can read them.
- **Primary CTA:** Download free (`#download`)
- **Secondary CTA:** See how it works (`#how-it-works`)
- **Line under the buttons:** Free for Windows, macOS, Linux, iPhone and Android.
- **Hero image alt text:** A lone barn in an open field at dusk, grass bent by the wind, painted in dark acrylic and broken into streaks of sorted pixels.

---

## problem

- **Eyebrow:** The everyday mess
- **Headline:** Too many passwords. Not enough you.

**Body:**
You have more accounts than you could ever remember. So you do what everyone does. You reuse a few favorites, write some down and hit "Forgot password" on the rest. It works, until it doesn't.

**Items:**
- **One leak opens every door.** A site you used once gets breached. Attackers try that email and password everywhere else, and wherever you reused it, they're in.
- **Fake pages look real.** One letter off in the address, the same logo, the same login box. You type your password before you notice.
- **Codes, codes, codes.** Every important login wants a six-digit code from your phone too. Grab your phone, find the app, type fast before it runs out.
- **Passkeys get stuck.** You make a passkey on your phone, then sit down at your laptop, and it isn't there.

**Closing line:**
A good password manager fixes all of this. The real question is which one you trust with everything.

---

## where-it-lives

- **Eyebrow:** Where your vault lives
- **Headline:** Most password managers keep your vault on their servers.

**Body:**
That's how they sync your devices. Your vault is encrypted, which is good. But it means one company holds millions of vaults in one place. That makes it a target worth years of effort.

**Story block 1**
- **Label:** 2022
- **Headline:** LastPass gets breached.
- **Text:** Attackers broke into LastPass and walked out with copies of customers' vaults. The passwords inside were encrypted. The website addresses were not. And a stolen vault never expires: the attackers can keep guessing master passwords offline, for as long as they like, and nobody will ever see them try.

**Story block 2**
- **Label:** 2026
- **Headline:** Researchers turn the server against its users.
- **Text:** A team from ETH Zurich and USI showed that a hacked password-manager server can do more than sit on your data. It can actively attack the people using Bitwarden, LastPass and Dashlane, in the worst cases far enough to get at what's in their vaults. Some of the problems have since been fixed. The lesson stands.

**Closing line:**
None of this makes password managers a bad idea. They're still far safer than reused passwords. It means two things matter more than anyone told you: where your vault lives, and whether your app checks what it gets back.

- **Source line (small):** Sources: LastPass security incident notice, December 2022. Scarlata et al., "Zero Knowledge (About) Encryption", USENIX Security 2026.

---

## how-it-works

- **Eyebrow:** How Mosaic works
- **Headline:** Locked on your device. Kept where you choose. Checked every time.
- **Intro:** Mosaic is built so that your passwords stay yours, wherever they're stored.

**Step 1**
- **Number:** 01
- **Headline:** Two keys, both yours.
- **Text:** Your vault opens with two keys. Your master password lives in your head. Your Secret Key is made by your device when you sign up and stays on your devices and on the Emergency Kit you print or save. Everything is locked before it leaves your device. A stolen copy opens nothing, even for someone who has guessed your master password.

**Step 2**
- **Number:** 02
- **Headline:** You decide where it syncs.
- **Text:** Keep your vault in your own Dropbox, iCloud Drive, OneDrive or Nextcloud folder, or in your company's Dropbox Business team folder. Or let us run the sync for you. It's the same lock either way. Your cloud stores scrambled files and can't read a single entry.

**Step 3**
- **Number:** 03
- **Headline:** It notices when something's off.
- **Text:** If a server or a cloud folder hands Mosaic an old copy of your vault, holds back your latest changes or serves a file that was tampered with, Mosaic doesn't quietly go along. It warns you and keeps the warning up until you've looked.

- **For the technical (small):** Each item is sealed on its own with XChaCha20-Poly1305 under keys derived with Argon2id from your master password and a 128-bit Secret Key. Signed checkpoints catch rollback, frozen updates and diverging copies on both sync options. Only published, standard building blocks.

[Visual suggestion: three cards, one per step. Step 3 shows the app's warning banner in a calm amber, labeled "Sample".]

- **Step 3 visual alt text:** Mosaic showing a warning that the synced copy of the vault is older than one the devices have already seen.

---

## features

- **Eyebrow:** Every day
- **Headline:** Everything you log in with. Handled.
- **Intro:** One app for every password, passkey and code you have, on every screen you use.

**Items (headline + text):**

1. **Autofill that can't be tricked.**
   Click, and Mosaic fills your login. Only on the exact site it belongs to, never in a hidden frame, never on a look-alike. On a fake page, nothing happens.
2. **Passkeys on every device.**
   Create and use passkeys in your browser and on your phone. They sync to all your devices instead of getting stuck on one.
3. **2FA codes that fill themselves.**
   Scan the QR code once. From then on, the code fills in right after your password. No more reaching for your phone.
4. **A desktop app you'll actually like.**
   Windows, macOS and Linux. Find anything with one shortcut. Open your vault with your fingerprint, your face or a security key.
5. **Your phone, fully covered.**
   iPhone and Android, filling logins and passkeys right inside your apps.
6. **More than passwords.**
   Secure notes, files, cards, Wi-Fi passwords, recovery codes, software licenses. Add your own fields to anything.
7. **Strong passwords, no effort.**
   Make a strong password in one click. See which old ones are weak, reused or showed up in a known breach, and fix them first.
8. **Share with the people you trust.**
   Share a login or a whole vault with family or coworkers. Remove someone, and the vault gets a new lock.
9. **Emergency access.**
   Pick someone you trust. If something happens to you, they can ask for access and get it after a waiting period you set, unless you say no.
10. **SSH keys, kept safe.**
    For developers: keep your SSH keys in the vault. The built-in SSH agent asks you before every use.
11. **Right in your terminal.**
    The `mosaic` command finds logins, makes passwords and pulls 2FA codes into your scripts.
12. **No tracking. At all.**
    No analytics, no ads, no usage data. Mosaic doesn't watch what you do.

**Platform strip:** Windows · macOS · Linux · iPhone · Android · Chrome · Edge · Firefox · Command line

[Visual suggestion: bento grid. Large tiles for 1 (click, then fill), 2, 3 and 4 with live product shots; the rest as small tiles.]

- **Autofill visual alt text:** The Mosaic browser extension filling a login after a click on the sign-in page.
- **Desktop visual alt text:** The Mosaic desktop app with a search box and a list of logins.
- **Phone visual alt text:** An iPhone filling a saved login into an app's sign-in screen.

---

## only-mosaic

- **Eyebrow:** Only Mosaic
- **Headline:** What the others don't do.
- **Intro:** We compared Mosaic with the best-known password managers. Three things only Mosaic does.

**Items:**

1. **Your cloud or ours. Same lock.**
   Sync through your own cloud folder, your team's Dropbox Business folder or our hosted server, and switch whenever you like. The others make you choose: their cloud only, or your own storage with no hosted option.
2. **It notices when a server tries to fool it.**
   An old copy passed off as new, changes held back, a tampered file: Mosaic spots it and tells you. None of the others we compared describe anything like it.
3. **Our server can't test password guesses.**
   When you sign in to hosted sync, Mosaic proves it knows your password without handing the server anything a guess could be checked against. Not the password, not a hash of it. None of the others we compared sign you in this way.

**And unlike most password managers:**
- Nothing fills until you click. There's no setting to change that.
- Logins fill only on the exact address you saved them for. A login for login.yourbank.com never fills on offers.yourbank.com.
- No telemetry, ever. Not on by default, not off by default. It doesn't exist.

- **For the technical (small):** 1: one sync protocol over two transports (folder and hosted). 2: rollback and freeze detection on both, plus fork detection across devices on hosted sync. 3: OPAQUE sign-in; the others use SRP or send a client-side hash.
- **Link:** See the full comparison
[Designer: the link goes to the comparison page. No competitor logos in this section.]

---

## switch

- **Eyebrow:** Switching
- **Headline:** Moving in takes minutes.

**Body:**
Export your vault from your old app, open the file in Mosaic and check the preview. Duplicates are caught before anything is saved. Your passwords keep working exactly as before, so there's nothing to reset.

**Items:**
- **Bring everything:** 1Password, Bitwarden, LastPass, KeePass, Enpass, Chrome, or any spreadsheet export.
- **Passkeys come too,** from apps that support the new passkey exchange format.
- **Leave whenever you want.** Export your whole vault at any time. Your data is never locked in.

- **CTA:** Download free (`#download`)

---

## pricing

- **Eyebrow:** Pricing
- **Headline:** The app is free. All of it.
- **Intro:** Every feature, on every device, at no cost. You pay only if you want us to run the sync.

**Card 1: Free**
- **Name:** Free
- **Price:** 0
- **Tagline:** Everything Mosaic does, synced through a cloud you already have.
- **Items:**
  - Apps for Windows, macOS, Linux, iPhone and Android
  - Browser extension for Chrome, Edge and Firefox
  - Passwords, passkeys, 2FA codes, SSH keys, notes and files
  - Import from your old password manager
  - Sync through your own Dropbox, iCloud Drive, OneDrive or Nextcloud folder
  - Shared vaults in a shared or team folder
  - The `mosaic` command line
- **CTA:** Download free

**Card 2: Hosted**
- **Name:** Hosted
- **Price:** {PRICE}
- **Tagline:** We run the sync. You don't set up a thing.
- **Items:**
  - Everything in Free
  - Sync through the Mosaic server, nothing to configure
  - Emergency access with a waiting period you set
  - Two-step sign-in for your sync account
  - Same lock as Free: we can't read your vault
  - Move back to a cloud folder whenever you like
- **CTA:** Start with hosted sync

[Designer: `{PRICE}` includes the billing period, e.g. "€X / month". Both cards equal width; Hosted gets the accent border.]

- **Line under the cards:** No ads, no tracking and no data sales on either plan. Hosted sync is how we pay the bills.

---

## teams

- **Eyebrow:** Teams and family
- **Headline:** Bring the people you trust.

**Body:**
The Wi-Fi password, the streaming login, the bank account you share, the server keys your whole team needs. Put them in a shared vault and everyone has what they need, on their own devices, under their own lock.

**Items:**
- **Shared vaults.** One vault for the family, one per team or project. Everyone sees what's in theirs, nothing more.
- **Check who you're sharing with.** Compare a short safety code with the other person once. If their key ever changes, Mosaic tells you.
- **Clean goodbyes.** When someone leaves, remove them and the vault gets a new lock.
- **In your company's cloud.** Keep team vaults in a Dropbox Business team folder or on Nextcloud, or on the hosted server. No new vendor cloud to approve.
- **A way in when it counts.** Emergency access lets a trusted person reach the vault if you can't.

- **CTA:** Get Mosaic for your team (`#download`)

[Visual suggestion: two or three people's devices around one shared vault card.]

- **Visual alt text:** Three devices showing the same shared vault, each opened by its own owner.

---

## faq

- **Eyebrow:** Questions
- **Headline:** Before you install.

1. **Can you see my passwords?**
   No. Your vault is locked on your device before it goes anywhere, and only your two keys open it. If you use hosted sync, we see your email address, your devices and how much locked data you store, so we can run your account. Never what's inside.

2. **What if I forget my master password?**
   Nobody can reset it. Not us, not anyone. That's the price of a vault only you can open, and we'd rather say it now than after you sign up. Your Emergency Kit has a line to write your master password down by hand; keep it somewhere safe. If you set up emergency access, a person you trust can also reach your vault after a waiting period.

3. **Is it safe to keep everything in one place?**
   Safer than the alternative. Reused passwords mean one leak opens many accounts. With Mosaic every account gets its own strong password, the vault is locked with two keys only you hold, it locks itself when you step away, and logins fill only on the exact site they belong to.

4. **What happens if Mosaic gets hacked?**
   The attacker gets locked data they can't open. Without your master password and your Secret Key there's nothing to read, and our server holds nothing to test password guesses against. If someone tampers with your synced data, the app warns you.

5. **What if Mosaic shuts down?**
   Your vault lives on your devices, and if you sync through a cloud folder, in your own cloud. The app keeps working without us. You can export everything at any time, and the sync server can run on your own machine.

6. **Why is the app free?**
   Because running it costs us nothing: it lives on your devices and your storage. We charge for the one thing that does cost money, hosting the sync. No ads, no data sales, no tracking.

7. **Do I have to change all my passwords?**
   No. Import them and keep going. Mosaic shows you which ones are weak, reused or leaked, so you can fix the important ones first.

8. **Does it work on my phone and in my browser?**
   Yes. Apps for iPhone and Android fill logins and passkeys inside your apps. Extensions for Chrome, Edge and Firefox fill on the web, always after your click.

9. **What if I lose my laptop or phone?**
   Whoever finds it still needs your master password. Mosaic locks itself when you're idle and when the device goes to sleep. With hosted sync, you can also sign the lost device out from any other one.

10. **Does it work offline?**
    Yes. Your vault is on your device, so everything works without internet. Changes sync the next time you're online, and if two devices edited the same thing, you keep both versions.

[Designer: accordion, first item open.]

---

## download

- **Eyebrow:** Get Mosaic
- **Headline:** One password to remember. Everything else, handled.
- **Body:** Download Mosaic, bring over your old passwords and let the app do the remembering. Free on every device.
- **Primary CTA:** Download free
- **Secondary CTA:** Compare plans (`#pricing`)

**Platform buttons:**
- Windows
- macOS
- Linux
- iPhone
- Android
- Chrome
- Edge
- Firefox
- Command line

- **Line under the buttons:** Every download is signed. Check it before you install.
- **Link:** How to check a download

[Designer: detect the visitor's system and show its button first and larger. The hero image may return here, cropped to the sky.]

---

## footer

- **Logo:** Mosaic
- **About line:** Mosaic is a password manager that keeps your logins on your devices, readable only by you. No ads. No tracking.
- **Column "Product":** Features · How it works · Pricing · Teams and family · FAQ
- **Column "Download":** Windows · macOS · Linux · iPhone · Android · Browser extensions · Command line
- **Column "More":** Compare · How to check a download · Contact
- **Legal line:** Imprint · Privacy
- **Language switch:** English · Deutsch

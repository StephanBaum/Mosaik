# Mosaic CLI

`mosaic` is the command line of the Mosaic password manager (package `mosaic-cli`, crate `crates/cli`).
It opens a vault store directly and shares the core crate with the apps. Mosaic is pre-audit.

Normative sources: `docs/CRYPTO_SPEC.md` §6.8 (session cache) and §11.2 (encrypted export),
ADR-0071 to ADR-0075 (`docs/DECISIONS.md`), `docs/THREAT_MODEL.md` §4.7 and R105 to R111. This file
describes the commands, the output contract and the security behavior; the man page and the shell
completions are generated from the same table (`crates/cli/src/spec.rs`).

## Profiles and stores

| Profile | Store | Notes |
|---|---|---|
| `shared` (default) | the desktop's store, `<data_dir>/io.github.stephanbaum.mosaic/vault.db` | works while the desktop is not running; otherwise exit 5 `storeInUse` |
| `own` | `<data_dir>/io.github.stephanbaum.mosaic/cli/vault.db` | created by `mosaic setup own-store`; a separate device of the same account that syncs like any device and coexists with the desktop |

`--store PATH` or `MOSAIC_STORE` overrides both (a path is not a secret). The CLI holds the store's
exclusive lock for one command only. It never talks to the running desktop (ADR-0071). While a key
rotation job is open in the store, `import`, `sync` and `recover` refuse with exit 10.

Keystore (shared with the desktop): service `io.github.stephanbaum.mosaic`; the Secret Key entry of the
desktop; the CLI's hosted sync token `mosaic-cli-sync-token-<account>`; the session key
`mosaic-cli-session-<account_id>` (below).

## Commands

```
mosaic [--store PATH | --profile shared|own] [--json] [--lang en|de] [--no-input] [--quiet] <command>
  unlock      [--ttl 5m] [--password-stdin | --password-fd N] [--secret-key-prompt]
  lock
  status
  list        [--vault V] [--type T] [--favorites] [--trash]
  search      QUERY [--vault V]
  get         REF [--field F]... [--reveal] [--copy [--clear-after 30s]]
  totp        REF [--copy]
  generate    [--length N | --words N [--separator S]] [--no-upper|--no-lower|--no-digits|--no-symbols]
              [--site DOMAIN] [--copy]
  run         [--env NAME=REF]... [--env-file FILE] -- COMMAND [ARGS...]
  inject      [--in FILE] [--out FILE] [--reveal]
  ssh         list | public-key REF | agent-status
  import      FORMAT FILE [--vault V] [--dry-run] [--file-password-prompt]
  export      FILE [--vault V]... [--include-attachments]
  sync        [--status | --acknowledge]
  setup       own-store (--folder PATH | --server URL --email E)
  recover
  completions bash|zsh|fish|powershell
  help        [COMMAND] [--man]
  --version
```

Not in M8: account creation (the Emergency Kit flow stays in the desktop), editing or deleting items,
key rotation, an SSH agent, hardware-protected vaults, quick unlock, plaintext export, sharing and
emergency access.

### References

`mosaic://<vault>/<item>[/<field>]`. Each part is a name (percent-encoded, NFC, exact match under the
core's title rules) or a 32-hex or hyphenated UUID. `get` and `totp` also take a bare item UUID or a
query that matches exactly one item. More than one match is exit 7 `ambiguousRef` with the candidates'
vault, title and ID (never a secret).

Field names are the core's keys (`login.username`, `login.password`, `card.number`, `notes`, ...) with
the aliases `username`, `password`, `otp`, `url`, and `field:<label>` for custom fields. Whether a field
is secret comes from the core model, never from the CLI.

### Sessions

`mosaic unlock` creates an explicit session (CRYPTO_SPEC §6.8): AK sealed in
`<runtime_dir>/mosaic/session-<account_id hex>.mcls` (mode 0600, directory 0700) under a key that only the
OS keystore holds. `runtime_dir` is `$XDG_RUNTIME_DIR`, else `<local_data_dir>/io.github.stephanbaum.mosaic/cli/run`.
The default TTL is 5 minutes, `--ttl` accepts 1 to 30 minutes, and the TTL is absolute. On Linux
`runtime_dir` is `$XDG_RUNTIME_DIR` only: without it `mosaic unlock` refuses to create a session (no
persistent fallback). On macOS and Windows both parts persist, also across a reboot, until they are swept,
locked or deleted by a credential change; every `mosaic` invocation (and the desktop at start, lock and
unlock) first deletes the parts whose expiry has passed. A password change, Secret Key rotation, profile
upgrade or re-key deletes every session. `mosaic lock` deletes both parts.

Be clear about what a session is: until both parts are deleted, anything that can read the keystore entry
and the file (same-user malware, a Windows profile backup, which holds both) can get AK without the master
password, with a faked clock as well, and AK stays valid until a re-key. Within the TTL any same-user process
can also just run `mosaic get --reveal`. Use `mosaic lock` when you are done. If the
keystore refuses (or is a memory fallback) there is no session: the command still works and prints
`sessionNotCached`. Without a session each command prompts for the master password; the Secret Key comes
from the OS keystore, else a prompt.

### Encrypted export

`mosaic export FILE` writes the `.mosaicx` format of CRYPTO_SPEC §11.2 under a passphrase of its own,
typed twice (never taken from argv or the environment), with Argon2id at the `standard` profile or
stronger. Attachments only with `--include-attachments` (a warning lists the items that have files). The
file is written atomically with mode 0600 (and the Mark of the Web on Windows). `mosaic import mosaicx FILE`
(and every Mosaic client) reads it back through the normal import preview. A wrong passphrase and a
damaged file both report `exportUnreadable` (exit 3). There is no plaintext export.

## Security behavior

- **No secret in argv or the environment.** No flag takes a password, Secret Key, file password or export
  passphrase as a value; `--password=...` style input is a usage error naming the safe options. Input comes
  from a no-echo prompt on the controlling terminal (120 s timeout, `promptTimeout`), `--password-stdin`
  (one line) or `--password-fd N` (Unix). `MOSAIC_PASSWORD`, `MOSAIC_SECRET_KEY`, `MOSAIC_SESSION` are never
  read; the first two print the warning `envSecretIgnored`. `--no-input` turns any prompt into exit 3
  `inputRequired`.
- **Reveal.** `get` prints non-secret fields; secret fields show `••••••••` (JSON `{"concealed":true}`).
  `--reveal` prints them. `--field <secret>` without `--reveal` or `--copy` is exit 9 `revealRequired`.
  `totp` and `generate` print their result. `inject` writes resolved secrets only to `--out FILE` (0600,
  atomic) or a non-terminal stdout; a terminal stdout needs `--reveal`.
- **`run`** resolves every reference, drops the session and closes the store, then starts `COMMAND` with the
  parent environment minus every `MOSAIC_*` variable plus the requested ones (names match
  `[A-Za-z_][A-Za-z0-9_]*`; `--env-file` lines are `NAME=REF`). The exit code is the child's (125 if it
  could not start). The child can read, log or pass on its environment (R106).
- **Clipboard.** `--copy` starts a short-lived helper (`mosaic __clipboard-hold`, hidden) and writes the
  value to its stdin pipe. The helper sets the clipboard with the history-exclusion hints, waits 30 s (5 to
  300 s with `--clear-after`) and clears it only if it still holds the value (the helper keeps the plaintext in a zeroizing buffer and compares it in constant time; it does not hash it). On Unix the helper runs in its own process group.
- **No logging, no telemetry.** Release builds abort on panic.

## Output contract

`--json`: stdout carries exactly one document `{"v":1,"data":...}`; errors go to stderr as
`{"v":1,"error":{"code":"storeInUse","message":"..."}}`. New fields are additive; a breaking change raises
`v`. Golden files pin the schemas (`crates/cli/tests/it/golden/*.json`).

- `list` and `search` item: `{"id","vault":{"id","name"},"type","title","username","urls":[],"favorite","trashed"}`.
- `get` adds `"fields":{"<key>":{"value":...}|{"concealed":true}}`,
  `"otp":{"code","secondsRemaining"}|null`, `"attachments":[{"name","size","mime"}]`.
- `status`: `{"store","profile","account","locked","session":{"expiresAt"}|null,
  "sync":{"backend","halted","findings"}|null}`.

Text mode prints aligned columns for lists and `key: value` for `get`, localized (EN or DE: `--lang`, then
`MOSAIC_LANG`, then `LC_ALL`/`LC_MESSAGES`/`LANG`). `--quiet` prints only the requested value. Control and
bidirectional characters in titles are shown escaped. JSON output, error codes, help and the man page are
English only.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | ok |
| 1 | other error |
| 2 | usage error |
| 3 | authentication failed or input required |
| 4 | not found |
| 5 | store in use |
| 6 | sync halted or sync error |
| 7 | ambiguous reference |
| 8 | vault part locked (hardware-protected or unavailable) |
| 9 | reveal required |
| 10 | rotation in progress |
| 125 | `run` could not start the command |
| 130 | interrupted |

Error codes reuse the desktop's names where they mean the same (`wrongCredentials`, `storeInUse`,
`syncPasswordNeedsServer`, `vaultUnavailable`, ...) plus `revealRequired`, `ambiguousRef`, `inputRequired`,
`promptTimeout`, `envSecretIgnored`, `sessionExpired`, `sessionNotCached`, `secretInArgv`,
`rotationInProgress`, `exportUnreadable`, and (M9) `twoFactorMethodUnsupported` and `syncSessionRevoked` (both exit 3).

- **`twoFactorMethodUnsupported`** (ADR-0079): the hosted account's only second factor is a security key, which the
  CLI cannot use (no WebAuthn, no printed approval link). EN: "This account signs in with a security key, which the
  command line cannot use. Add an authenticator-app code in the desktop app, then try again." The DE text is in the
  catalog.
- **`syncSessionRevoked`** (CRYPTO_SPEC §9.5): the server ended the stored session for a reason and says which
  (`rekey`, `passwordChange`, `deviceRemoved`, `operator`); the message names it. For `rekey`: "This account was
  re-keyed on another device: sign in with the new Secret Key from your newest Emergency Kit" (`mosaic recover`).
  JSON errors carry the reason as `"detail":{"revoked":"rekey"}`. The reason is the server's claim and only picks the
  text.

## Commands that change state

- `totp` on an HOTP item advances and stores the counter first.
- `import` prints a preview (counts, duplicates, warnings) and, without `--dry-run`, asks
  "Import N items into V? [y/N]" on the terminal (`--no-input` refuses). File passwords (KDBX) by prompt only.
- `sync` runs one `sync_once` with the store's backend. A halt or a finding is exit 6 with the finding list.
  `--acknowledge` shows the findings and needs the word `acknowledge` typed on the terminal (no `--yes`).
- `setup own-store` refuses if the own store exists; it joins the account as a separate device (folder: the
  M6 folder join; hosted: login, second factor prompt, account fetch); the Secret Key goes to the keystore.
- `recover` handles a password, Secret Key or re-key change made elsewhere: it prompts for the password,
  then for the Secret Key of the newest kit, and saves the new Secret Key.

## Behavior worth knowing

- `setup own-store --server URL --email E` joins over the hosted backend (OPAQUE sign-in, a second-factor
  prompt if the account has one) and stores the session token as `mosaic-cli-sync-token-<account>`. The second
  factor is a TOTP code only: after `TwoFactorRequired`, `setup own-store` and `recover` ask the server which methods
  the account has (`GET /v1/login/methods`) and prompt for a code when TOTP is one of them; an account with only
  security keys exits 3 with `twoFactorMethodUnsupported` (a browser handoff for the CLI is owner decision OD-M9-12).
  `mosaic recover` adopts a master password change made elsewhere on a hosted store: it unlocks with the
  old password, asks for the new one (and the newest Secret Key if the login fails), and deletes every CLI
  session on success. Folder backends and re-key adoption are not handled by `recover` (`notSupported`).
- Hardware-protected vaults are skipped by `export` and refused by references (exit 8).
- A session lives in a slot named by account **and** install, so the `shared` and the `own` store of one
  account each keep their own session. The keystore entry and the file are named by that slot ID.
- `get --reveal` prints a warning on stderr when stdout is a terminal. Single-value output to a terminal
  shows C0 and C1 control characters and bidirectional controls as `\u{...}` escapes (line feed and tab
  stay); output to a pipe or file is raw.
- `run` on Unix replaces the `mosaic` process with the command (`exec`), after the store and the session
  are closed. On Windows `mosaic` waits and passes on the child's exit code when it is 0 to 255, else 1;
  125 is a spawn failure. During the session TTL the child can call `mosaic get --reveal` through the
  user's session (R106).
- The environment variables `MOSAIC_*` are removed from the child (case-insensitively on Windows).
- On Windows the clipboard helper inherits the handles its caller gave `mosaic`: a caller that waits for
  end-of-file on a pipe of `mosaic ... --copy` waits for the helper too. Use a terminal, or redirect to a
  file.
- A prompt that times out, an error and `Ctrl-C` restore the terminal mode (Unix; the Windows console
  resets its mode for the shell). `Ctrl-C` exits 130.
- Import reads every file through a 256 MiB cap; a `.mosaicx` item that exceeds its own byte cap is refused
  (`LimitExceeded`).
- `export` never overwrites: the file is linked into place, so a file that appeared meanwhile wins and the
  export fails. On Windows the file carries the Mark of the Web (`Zone.Identifier`, `ZoneId=3`) when the
  volume is NTFS.

## Packaging

`cargo build -p mosaic-cli --release` builds `mosaic` on Windows, macOS and Linux. `mosaic help --man`
prints the man page (committed as `crates/cli/man/mosaic.1`); `mosaic completions <shell>` prints a
completion script (committed in `crates/cli/completions/`).

Release builds (M9, ADR-0082) install `mosaic` only to protected, admin-owned locations, because the session cache and
the `run` and clipboard protections assume that same-user code cannot replace the binary: the per-machine MSI puts it in
`%ProgramFiles%\Mosaic\` (and on the system `PATH`), a signed macOS `.pkg` installs a root-owned
`/usr/local/bin/mosaic` (the app bundle carries one too), and deb and rpm install `/usr/bin/mosaic`. Builds are
Authenticode or Developer ID signed (hardened runtime) once the owner has certificates, and every release is listed in
a minisign-signed `SHA256SUMS` (CRYPTO_SPEC §17). The release job checks that no Cargo feature (such as `test-util`) is
enabled. The owner runbook is `docs/RELEASE.md` and the user's verification steps are in `docs/VERIFY.md` (both
written in M9 T9). Package-manager channels are not part of M9.

The installers come from the desktop build (`apps/desktop/src-tauri/tauri.release.conf.json`, built in CI by
`.github/workflows/release.yml`): the sidecars `mosaic` and `mosaic-native-host` are built first and placed in
`apps/desktop/src-tauri/binaries/` as `<name>-<target triple>[.exe]`; Tauri's `externalBin` installs them beside the app
(`%ProgramFiles%\Mosaic\`, `Contents/MacOS/`, `/usr/bin/`). Targets are `msi`, `dmg`, `app`, `deb` and `rpm`; there is
no NSIS and no AppImage because both install to user-writable places. The MSI is per-machine and appends its folder
to the system `PATH` (`wix/native-messaging.wxs`). A `mosaic` found earlier on the `PATH` in a user-writable folder
still wins over the installed one: check `where mosaic` or `command -v mosaic` after installing.

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
  300 s with `--clear-after`) and clears it only if it still holds the SHA-256 of its value.
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
`rotationInProgress`, `exportUnreadable`.

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

## What this version does not do yet

`mosaic recover` and `mosaic setup own-store --server` (hosted join with a second-factor prompt) are in
the table but answer `notSupported` (exit 1); `mosaic sync` against a hosted backend works once the app
has signed in and stored the token (`mosaic-cli-sync-token-<account>` is read, never written by the CLI).
Hardware-protected vaults are skipped by `export` and refused by references (exit 8).

Behavior worth knowing:

- The session entry is per account, not per store: a session made for the `shared` store is refused (and
  both parts deleted) when it is used on the `own` store, because the install ID differs. Run `mosaic
  unlock` again for the store you use.
- `get --reveal` prints a warning on stderr when stdout is a terminal (scrollback keeps the value).
- On Windows the clipboard helper inherits the handles its caller gave `mosaic`: a caller that waits for
  end-of-file on a pipe of `mosaic ... --copy` waits for the helper (the clear timeout) too. Use a
  terminal, or redirect output to a file.
- A prompt that times out (120 s) ends the command while the terminal driver may still have echo off on
  Unix; `stty echo` restores it (THREAT_MODEL R109).
- `Ctrl-C` ends the command with the system default (exit 130 in most shells); there is no handler.

## Packaging

`cargo build -p mosaic-cli --release` builds `mosaic` on Windows, macOS and Linux. `mosaic help --man`
prints the man page (committed as `crates/cli/man/mosaic.1`); `mosaic completions <shell>` prints a
completion script (committed in `crates/cli/completions/`). Signing, installers and package-manager
channels are M9.

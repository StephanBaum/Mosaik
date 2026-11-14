# Mosaic sync server: operator guide

`mosaic-server` is the self-hostable sync server of Mosaic (M6, ADR-0044). It
stores what your devices upload as ciphertext and lets them find each other's
changes. It never sees a master password, a Secret Key, a vault key or any
plaintext. Mosaic is pre-audit software; read `docs/THREAT_MODEL.md` (R57 to
R62) before you host other people's data.

The server is one process with one data directory: a SQLite database, the
attachment files and a secret file. TLS is the job of a reverse proxy.

## What the server stores

Only what CRYPTO_SPEC §12 lists:

- the account email (normalized), the account ID, the salt and Argon2 profile,
  the OPAQUE registration record, the wrapped account key and an account
  version;
- sessions as SHA-256 hashes of their tokens, with their times;
- devices (ID, the name you gave it, times) and a short audit log (event type,
  time, device: no content), kept 365 days;
- vault IDs, wrapped vault keys, the encrypted vault metadata, records as
  ciphertext with their kind (item or tombstone), size and sequence number, and
  attachments as ciphertext files;
- each device's latest checkpoint (device ID, counter, `issued_at_ms`,
  per-vault generation, sequence and an opaque 32-byte `view_digest`, and the
  MAC the clients check; CRYPTO_SPEC §12 is the source of this list);
- the TOTP secret of a second factor, sealed under a key derived from the
  secret file;
- for security keys (M9, see "Two-step verification with a security key"
  below): a random 16-byte user handle per account and, per registered key,
  its credential ID, public key, label, sign count and times. A running
  browser handoff is held in memory only;
- for revoked sessions (M9): the token hash, the account, the reason and the
  time, for 30 days;
- for sharing and emergency access (M6b, see "Sharing and emergency access"
  below): each account public identity key and its wrapped private key, the
  encrypted contact book, roles and members of shared vaults, the encrypted
  roster of each vault and its history, invites and key shares (sealed
  ciphertext), emergency grants with their state and seals, and block lists.
  The server sees the sharing graph (who shares with whom, with which role) but
  never a vault name, a key or an item;
- for key rotation (M7, see "Re-key" below): the account key epoch (a small
  counter that says which account key a wrapping belongs to), rotation
  requests (vault, requester, time) and invites cancelled by a re-key.

IP addresses are held in memory for rate limiting and are **never written to
disk**, not to the database and not to the log. The log has one line per
request: method, route template, status and duration. It never holds a body,
an email, an IP address, a token or an ID. There is no telemetry and no
third-party call.

## Install

Build from source (`cargo build --release -p mosaic-server`) or use the
container in `deploy/` (`deploy/README.md`; the image is built from source, no
image is published before M9).

```sh
mosaic-server init         --config /etc/mosaic/mosaic-server.toml
mosaic-server check-config --config /etc/mosaic/mosaic-server.toml
mosaic-server serve        --config /etc/mosaic/mosaic-server.toml
```

`init` creates the data directory, the database and the secret file, and
refuses to run if the secret file or the database already exists.

## Configuration

`mosaic-server.toml`. Unknown keys are an error. See
`deploy/mosaic-server.example.toml` for a full example.

| Key | Meaning |
|---|---|
| `listen` | Address and port to listen on, for example `127.0.0.1:8743` behind a proxy on the same host. |
| `canonical_origin` | The public address, `https://host[:port]`, lowercase, no path. It is the OPAQUE server identity: clients bind their login to it and `GET /v1/info` echoes it. Choose it once; changing it makes every existing login record unusable. `http://` is accepted only for `localhost`, `127.0.0.1` and `[::1]` (local testing). |
| `data_dir` | Holds `mosaic-server.sqlite` (and its `-wal` and `-shm` files) and `blobs/`. |
| `secret_file` | The secret file (created by `init`, mode 0600 on Unix). |
| `registration` | `invite` (default for new installs: a code from `invite create` is needed), `open` or `closed`. Open registration lets anyone learn whether an email is registered (R58). |
| `trusted_proxies` | IP addresses of reverse proxies whose `X-Forwarded-For` header is believed. Default: `127.0.0.1` and `::1`. |
| `[webauthn] enabled` | Security keys as a second factor (default `true`). The relying-party ID is the host of `canonical_origin`; an origin whose host is an IP address never offers security keys. `false` hides the feature: `GET /v1/info?features=1` says `webauthn: false` and the WebAuthn routes answer `webauthn_unavailable`; registered keys stay stored and still count as a second factor, so an account whose only second factor is a security key cannot finish a sign-in until the setting is back on or you run `account reset-second-factor` for it. |
| `[limits] max_request_bytes` | Largest JSON body, 2 MiB to 64 MiB (default 8 MiB). Attachments have their own limit (the 50 MiB plaintext maximum plus the envelope). |
| `[limits] account_attachment_bytes` | Attachment bytes per account, soft-deleted ones included (default 1 GiB). |
| `[limits] account_record_count` / `account_record_bytes` | Records and record ciphertext per account (default 50 000 and 256 MiB). |
| `[limits] session_idle_days` / `session_max_days` | A session ends after this many idle days (default 30) or this many days in all (default 180). |
| `[limits] max_concurrent_requests` | Requests handled at once, server-wide; more wait in the queue (default 512). |
| `[limits] max_connections` | Open connections at once, server-wide; at the cap new clients wait in the kernel backlog (default 1024). |
| `[limits] max_key_generations` | Key generations per vault (default 10 000). |
| `[limits] soft_delete_days` | A deleted attachment is kept this long, so a device can restore it (default 30). |

Fixed limits: 32 devices, 64 sessions and 1 000 vaults per account, 32 waiting polls per account, ten second-factor failures per account per hour (wrong TOTP codes, wrong handoff codes and failed security-key checks together), 10 security keys per account, 3 running handoffs per account and 10 000 per server. On Unix the server creates the data directory with mode 0700 and the database, WAL and secret files with 0600, and refuses to start if the secret file is readable by group or others.

## Command line

```text
mosaic-server init          --config <file>
mosaic-server serve         --config <file>
mosaic-server check-config  --config <file>
mosaic-server invite create --config <file>
mosaic-server backup        --config <file> --out <file> [--blobs]
mosaic-server account delete --config <file> --email <address>
mosaic-server account reset-second-factor --config <file> --email <address>
```

- `invite create` prints a 20-character code once and stores only its hash.
  A code works for one registration.
- `account delete` removes the account, its vaults, records and attachment
  files. Sessions end at once.
- `account reset-second-factor` removes the account's TOTP secret and every
  security key, ends every session (recorded as revoked by the operator) and
  writes the audit event `SecondFactorReset`. Use it only after you have
  confirmed who is asking, outside Mosaic: it is the only way back for a user
  who lost every second factor, and anyone who talks you into it gets the
  same (THREAT_MODEL R115). The password and the Secret Key are still needed
  to sign in.
  The command runs in its own process, so a running server may still hold a
  browser handoff of the account in memory. It cannot be finished: the finish
  checks, in the same transaction, that the session which started it still
  exists, and the reset deleted every session. No restart is needed.
- `check-config` validates the file and says whether the directory is
  initialised.

## Reverse proxy and the client address

Run the server behind a proxy that terminates TLS 1.3 (Caddy, nginx, Traefik).
The server speaks plain HTTP: do not expose its port. Clients refuse a server
that is not `https://` (except on localhost).

Rate limits are per client IP address. Behind a proxy every request comes from
the proxy, so:

1. add the proxy's address to `trusted_proxies`;
2. make the proxy **set** `X-Forwarded-For` to the client address (Caddy and
   nginx with `proxy_set_header X-Forwarded-For $remote_addr` do; do not append
   to a header the client sent).

The server reads the last hop of the header, and only from a peer listed in
`trusted_proxies`. If the proxy is missing from the list, every client shares
one bucket and legitimate users are rate limited; if an untrusted address is in
the list, clients can choose their own address and the limits mean nothing.
IPv6 clients are limited by their /64.

Caddy example (`deploy/Caddyfile`):

```text
sync.example.com {
	tls { protocols tls1.3 }
	request_body { max_size 64MB }
	reverse_proxy 127.0.0.1:8743
}
```

nginx needs `client_max_body_size 64m;`, `proxy_read_timeout 60s;` (the long
poll waits up to 25 s) and `proxy_buffering off;` for large downloads.

**The security-key page.** The server serves one static page, `GET /webauthn/`
with `page.js`, `page.css` and `strings.json`, under the canonical origin. The
proxy must pass these paths unchanged and must not replace or weaken the
response headers the server sets (a strict `Content-Security-Policy` with
`frame-ancestors 'none'`, `Referrer-Policy: no-referrer`, `Cache-Control:
no-store`, `X-Content-Type-Options: nosniff`, `Cross-Origin-Opener-Policy`,
`Permissions-Policy`), must not cache them, and must not inject scripts
(analytics, error pages). The page has to be reachable at exactly the canonical
origin: a browser asserts only for the host it is on.

## systemd

```ini
[Unit]
Description=Mosaic sync server
After=network.target

[Service]
User=mosaic
ExecStart=/usr/local/bin/mosaic-server serve --config /etc/mosaic/mosaic-server.toml
Restart=on-failure
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/mosaic

[Install]
WantedBy=multi-user.target
```

## Backup and restore

Back up the **data directory and the secret file together**.

```sh
mosaic-server backup --config /etc/mosaic/mosaic-server.toml \
    --out /backups/mosaic-2026-11-06.sqlite --blobs
```

`backup` writes a consistent copy of the database (`VACUUM INTO`, safe while
the server runs) and, with `--blobs`, the attachment files to
`<out>.blobs/`. Copy the secret file (`secret_file` in the config) separately;
it is not part of `backup`. To restore: stop the server, put the database file
at `data_dir/mosaic-server.sqlite` (delete any old `-wal` and `-shm` files),
the blobs under `data_dir/blobs/`, and the secret file at its path. Without the
secret file nobody can log in (the OPAQUE server key is in it) and the vault
data on the devices is unaffected. Treat backups like the live data: they hold
the same ciphertext and the same secret.

## The secret file cannot be rotated

The OPAQUE server setup lives in the secret file. A new setup makes every stored
registration record useless: every account would have to register again and
every device join again. Rotation is therefore not supported; protect and back
up the file instead. If it leaks, treat it as a server compromise (R61): start
a new server with a new setup and migrate the users by having them register
again from a device that holds the vault.

## Slow clients

The server limits request time, body size and concurrent requests, and closes
a connection that takes longer than 10 s to send a request head or sits idle
that long between requests (R64). It caps open connections server-wide
(`[limits] max_connections`, default 1024): at the cap it stops accepting until
one closes. The cap is not per IP, so one client can fill it: keep the reverse
proxy's per-IP connection limits on (Caddy and nginx do by default).

## Upgrades

The database carries its schema version and migrates itself when a newer
server opens it. A server refuses a database written by a newer version. Back
up before upgrading.

Schema version 4 (M9) adds the security-key tables, the per-session
`second_factor_until`, `revoked_sessions` and the audit kinds 17 to 20. The
migration runs on start and changes no existing row. After it, an M8 server
cannot open the database: restore the backup to go back.

Changing `canonical_origin` invalidates every registered security key as well
as every login record (the relying-party ID is its host). Affected users need
`account reset-second-factor` and a new registration.

## Operating notes

- Accounts and sessions: a user can remove a device and delete the account
  from the app; both need a fresh login (within five minutes) and, when
  two-step verification is on, a TOTP code or a security-key confirmation in
  the browser from the same session within the last five minutes.
- The second factor guards access to the server account. It adds nothing to
  vault confidentiality (the master password and the Secret Key do that).
- A stolen session token lets a thief download and upload ciphertext and
  soft-delete attachments until the device is removed or the master password
  is changed: a password change ends every other session of the account (R57).
  A session also remembers the account version it logged in at, and a
  password change from a session whose version is out of date is refused
  (`account_version_mismatch`).
- A password change also ends the pending (not yet finished) logins of the
  account, and a login that started before the change cannot finish after it.
- Development note: databases created before the in-place edit of schema
  version 1 (the `sessions.account_version` column, the checkpoint body
  limit) must be recreated; the server is unreleased.
- Housekeeping runs every hour: expired sessions, audit events older than 365
  days, invites that ended more than 60 days ago and attachments deleted more
  than `soft_delete_days` ago.
- Run one server process per data directory.

## Sharing and emergency access

Schema version 2 (M6b) adds sharing between accounts on this server and
emergency access. It needs no configuration. The server stores and serves
ciphertexts, seals and public keys, and checks only their shape and the rules
below. It never opens a seal. Folder stores do not use any of this.

Wire conventions are those of the other routes (JSON, camelCase, IDs as
hyphenated UUIDs, bytes as unpadded base64url). Roles are numbers: 1 owner,
2 manage, 3 write, 4 read. A membership has an origin: 0 owner, 1 invite,
2 emergency release. The new error codes are in `api/mod.rs::codes`.

### Routes

| Route | Rules |
|---|---|
| `PUT /v1/account/identity` | First write wins. The same pair again is 204. A different pair is `409 identity_exists` with the stored pair. `GET /v1/account?identity=1` adds `identity`. |
| `POST /v1/directory/lookup` | Any valid email gets `200 {accountId, ipk}`; see below. |
| `GET /v1/directory/{id}` | Only for accounts the caller shares a vault, an incoming invite or an emergency grant with. Else 404. An invite to the target is never a reason (it would tell a real recipient from a fake one). |
| `GET`/`PUT /v1/account/contacts` | The encrypted contact book with a compare-and-swap (`conflict {currentSeq}`). |
| `POST /v1/vaults/{v}/invites` | Owner or manager. Role at most the caller's own and never owner; generation must be current; one open invite per vault and recipient (`invite_exists`); `already_member`; `reinvite_blocked`; `members_limit` (50 members and open invites); `invite_limit` (200 open invites per sender). Valid for 14 days. |
| `GET /v1/invites` | Incoming invites (with sender email and seal) and the caller's sent invites. Listing marks incoming invites delivered. |
| `GET /v1/invites/{id}/vault` | The vault metadata and current roster, for the recipient of an open invite. |
| `POST /v1/invites/{id}/accept` | Body `{wrapped, device, generation}`. Adds the membership (origin invite) and the caller's wrapped key. `invite_stale {currentGeneration}` unless `generation` equals both the invite's and the vault's current generation, `invite_expired` (410). |
| `POST /v1/invites/{id}/decline` | Optionally blocks the sender. A declined or cancelled invite stays in the sender's list (state 2 or 3, `delivered`) until the sender deletes it. |
| `DELETE /v1/invites/{id}` | Sender or manager. Cancels an open invite; removes a finished one. |
| `GET /v1/vaults/{v}/members` | Any member: members with role, origin and time, and open invites. |
| `PUT /v1/vaults/{v}/members/{a}` | Owner or manager changes a role: never to or from owner, never above the caller's own. |
| `DELETE /v1/vaults/{v}/members/{a}` | Owner or manager removes a member: the target's role must be at most the caller's, never the owner, never the caller (`role_forbidden`). Deletes the membership, keys, shares and open invites; a re-invite waits for the next generation. The owner's next run rotates the key. |
| `PUT /v1/vaults/{v}/roster` | Owner or manager; current generation; compare-and-swap on `baseSeq`. |
| `POST /v1/vaults/{v}/rotate` | **Owner only.** One transaction: the new key, a share for every other member, refreshed seals for every open invite, the new roster, the removal of the revoked accounts, and the generation and head bump. `members_mismatch {members, invites}` unless `seals` and `revoke` are disjoint and together equal the members except the owner, and `inviteSeals` equals the open invites. `generation_taken`, `head_moved`, `generation_mismatch` as for the plain key upload. |
| `PUT /v1/vaults/{v}/keys/{g}/claim` | A member claims the key of the current generation from a share addressed to them, or (emergency membership) from a released emergency seal. `no_share` otherwise. |
| `POST /v1/vaults/{v}/leave` | A member leaves; the owner gets `owner_cannot_leave`. |
| `PUT /v1/vaults/{v}/keys/{g}` | Unchanged for an unshared vault; `members_mismatch` once the vault has a second member or an open invite. |
| `GET /v1/vaults/{v}/changes?sharing=1` | Adds `role`, `shares` (every unclaimed share addressed to the caller) and `rosters` (newer than `since`). Without the flag the page is unchanged. |
| `POST`/`DELETE /v1/blocks/{id}` | Blocks or unblocks an account ID (it need not exist). |
| `GET /v1/emergency` | The caller's grants given and received. |
| `PUT /v1/emergency/grants/{grantee}` | Fresh login. Wait of 1, 2, 7, 14 or 30 days; scope 1 (all) or 2 (selected); a seal per vault. Only vaults the caller owns (`emergency_owner_only`). `emergency_waiting` while a request is open, `emergency_released` once approved or released. At most 20 grants given and 20 received. |
| `PUT /v1/emergency/grants/{grantee}/seals` | The grantor keeps the seals current; a seal is replaced only by one of the same or a newer generation. |
| `DELETE /v1/emergency/grants/{grantee}` | The grantor deletes a grant in any state, with its seals, and the grantee's origin-emergency memberships (with keys and shares) in the grantor's vaults. |
| `POST /v1/emergency/grants/{grantor}/request` | The grantee asks. Answers `{releaseAt}`. `emergency_cooldown {until}` within 24 h of a refusal. |
| `POST /v1/emergency/grants/{grantee}/decline` | The grantor refuses: back to idle with a 24 h cooldown. |
| `POST /v1/emergency/grants/{grantee}/approve` | The grantor approves (fresh login). |
| `POST /v1/emergency/grants/{grantor}/release` | The grantee gets the seals once the wait is over (`emergency_waiting {releaseAt}` before) or after approval, and a `read` membership with origin emergency in every vault of the grant. |

Writes need a role: records, metadata and attachments need write or stronger
(`role_forbidden` for read). The roles are enforced by the server only; the
cryptographic line is who holds the vault key (CRYPTO_SPEC §8.2). Storage
quotas of a shared vault are charged to its owner. A request from an account
that is not a member of a vault still gets `403 forbidden`, as before.

### Re-invites and removed members

A removal (a revoking rotation) or a leave at generation g blocks a re-invite
of the same account until the vault has rotated past g (`reinvite_blocked`).
The client roster merge rule makes a re-invite at the removal generation
lose, and the server offers no way around it. After a rotation that revokes an
account, its memberships, wrapped keys, shares and open invites are gone.

### Abuse limits

- `POST /v1/directory/lookup`: 30 per hour per session and 60 per hour per
  account, then `429 rate_limited`.
- Invites: 50 per hour per account. At most 50 members and open invites per
  vault, 200 open invites per sender, 100 visible open invites per recipient.
- Emergency requests: 5 per hour per account.
- Blocks: 1000 per account.

### No user enumeration, no email

`directory/lookup` answers every valid address with 200. An unknown address
gets the same fake account ID the pre-login route gives (an HMAC of the
address) and a fake identity key (`HMAC-SHA-256` under a key derived from the
secret file, run through the identity-key derivation, so it is a valid key like a real one). A registered account without a published key
gets its real ID and the fake key. Invites to such accounts are stored like
any other and simply never accepted. An invite from a blocked sender, or to a
full inbox, is stored but never shown, so the sender sees `201` and, on a
repeat, `invite_exists`, exactly as for any recipient. Lookup targets are not
logged. The server sends no email: invites and emergency requests reach people
through the app (a long poll wakes both accounts) and the safety-code
comparison.

Residual leaks, accepted: response timing, and whether an invite is ever
delivered (the sender list shows `delivered`).

### Emergency access and its limits

The server keeps the waiting period (`requestedAt + waitDays`) and the
cooldown, using its own clock. Someone who controls the server can skip the
wait; the clients say so. Emergency access hands over vault keys. It does not
recover an account. Deleting a grant removes the seals but keeps the memberships
a release created; the owner next rotation lists them in `revoke`.

## Re-key

Schema version 3 (M7) adds the account key epoch (`accounts.ak_epoch`, from 1),
rotation requests and the audit kinds 15 (account re-keyed) and 16 (rotation
requested). The migration runs on start. `POST /v1/register/finish` carries `akEpoch`: 1
for a new account, the device's own epoch when a device that re-keyed on a folder switches
to this server (the server rejects 0; the value is a `u32`). The epoch then rises only by
re-keys, one at a time. A re-key is the full rotation of an
account's keys (CRYPTO_SPEC §6.6): a new account key, a new identity key and a
new vault key for every vault the account owns. The server stores what the
client uploads and never opens a wrapping or a seal.

| Route | Rules |
|---|---|
| `POST /v1/account/rekey/start` | Fresh login. Body `{registrationRequest, akEpoch}`, answer `{registrationResponse}`. The session must have logged in at the current account version (`account_version_mismatch`) and `akEpoch` must be the stored epoch plus one (`epoch_mismatch {currentEpoch}`). |
| `POST /v1/account/rekey/finish` | Fresh login, same two checks. Body (cap 4 MiB, at most 1000 vaults): `registrationUpload`, `wrappedAk`, `salt`, `profile`, `akEpoch`, `identity` (`{ipk, wrappedIsk}` or null), `vaults`, `memberRewraps`, `emergencySeals`, `device`. Answer `{accountVersion, akEpoch}`. One transaction, or nothing. |
| `GET /v1/account?epoch=1` | Adds `akEpoch`. A flag of its own, so that `identity=1` and the plain body stay as older clients parse them. `epoch=1&identity=1` gives both. The desktop reads it on start (`hosted::account_epoch`): when the server holds other keys than the device and states an epoch above the device's, the status is `syncAccountRekeyed` (a re-key made elsewhere); an equal epoch is a password change, a lower one is no re-key (the engine reports `AccountRollback`). The epoch is the server's unauthenticated claim (THREAT_MODEL R90): at worst it asks for the new Secret Key. |
| `GET /v1/vaults/{v}/changes?sharing=1&rotation=1` | For the owner, adds `rotationRequested`: a member that re-keyed asked for a rotation. Without `rotation=1` the page is the M6b page. |

What `finish` checks, before it writes anything:

- the profile is not below the stored one (`profile_downgrade`);
- the identity: an account that has one must send a different one (`400`
  otherwise), because the wrapped private key is under the old account key.
  This is the only route that may replace an identity (`PUT /v1/account/identity`
  stays first write wins);
- `vaults` is exactly the set of vaults the account owns (`rekey_mismatch
  {set: "vaults"}`), each at its current generation plus one (`generation_taken`,
  `generation_mismatch`) with `expectedHead` equal to the head (`head_moved
  {headSeq, vaultId}`). A shared vault (a member or an open invite) follows the
  rules of `POST /rotate`: a roster of the new generation, a seal for every
  member and every open invite, nobody revoked (`members_mismatch`). An unshared
  vault takes no seals;
- `memberRewraps` is exactly the set of vaults shared to the account for which
  it holds the key of the current generation, at that generation
  (`rekey_mismatch {set: "memberRewraps"}`);
- `emergencySeals` is exactly one fresh seal for each seal the account gave, at
  the new generation of the vault (`rekey_mismatch {set: "emergencySeals"}`).

What it applies: the OPAQUE record, wrapped account key, salt, profile, epoch
and identity; the new key row of each owned vault (and its roster, shares and
invite seals); the re-wrapped current row of each vault shared to the account,
with the head moved so the account's other devices fetch it; every key row of
an older generation of this account deleted (they are under the old account
key); the key shares addressed to the account deleted; the emergency seals
addressed to the account deleted (the grantor re-seals after verifying the new
code); incoming open invites set to state 5, `cancelled_by_rekey` (the sender's list shows it
with `delivered`; the sender's client reads state 5 as cancelled, queues the revocation of
a delivered one and deletes it with `DELETE /v1/invites/{id}`, which removes an invite of
any finished state, 5 included); open invites
the account sent for vaults it does not own cancelled (state 3: their seal is
from the old identity key); a rotation request to the owner of each vault
shared to the account; audit 15 for the account and 16 for each owner; the poll
token of every affected account. The server holds no hardware-key (PRF)
wrapping of the account key yet, so there is nothing of that kind to delete
(CRYPTO_SPEC §6.6 says it would go with the re-key).

Every other session of the account ends, pending second-factor sessions and
half-finished OPAQUE logins included; the changing session stays and moves to
the new account version. A lost answer is settled by a login with the new
password: the account then shows the new wrapped key and epoch.

The epoch never falls and rises by exactly one: a database trigger enforces it.
A rotation request lives until the owner rotates the vault (any rotation) or
the requester leaves it. Rate limit: the session limit, as for a password
change.

What you see as the operator: the audit kinds 15 and 16, the epoch, the
rotation requests (vault, requester, time), invite state 5. What you do not
see: any key, any password, any seal content. The sharing graph is as visible as
before.

## Two-step verification with a security key

Schema version 4 (M9) adds WebAuthn security keys (and passkeys) as a second
factor next to TOTP (CRYPTO_SPEC §9.3, ADR-0077, ADR-0078). The desktop app
cannot talk to a security key for your server's address itself, so the
ceremony runs in the user's normal browser, on a page this server serves:

1. The app asks the server for a **handoff** and shows a 6-digit code.
2. The app opens `https://<canonical origin>/webauthn/#<handoff id>` in the
   default browser.
3. The user types the code on that page; only then does the page get the
   WebAuthn request. The user touches the key.
4. The page posts the result to the server; the app, which polls, continues.

Handoffs live in memory only and expire after five minutes at the latest; a
restart ends every running one (the user starts again). The relying-party
library is `passkey-auth` (ES256 and EdDSA only, no attestation check). The
server accepts security keys and platform authenticators alike
(`authenticatorAttachment` is not limited), and accepts an assertion without a
user handle; a user handle that is present must belong to the account.

If the database holds security keys but `[webauthn]` is off (or the canonical
origin is an IP address), the server logs a warning at startup. Those accounts
cannot finish a login until you turn it back on or reset their second factor.

If the operating system's random number source fails, the WebAuthn library
panics. A debug build answers that request with a bare `500`; a release build
stops, and `Restart=on-failure` brings it back (running handoffs are lost).

| Route | Auth | Rules |
|---|---|---|
| `GET /v1/info?features=1` | none | Adds `features: {webauthn}`. |
| `GET /v1/login/methods` | a pending login | `{totp, webauthn}`: what the account can use. |
| `POST /v1/login/webauthn/handoff` | a pending login | Starts a login handoff: `{handoffId, matchCode, expiresAt}`. |
| `POST /v1/login/webauthn/handoff/state` | the same pending login | Body `{handoffId}`. Answers `{state}` (`waiting`, `done`, `failed`, `expired`). On `done` the login is complete, as after `POST /v1/login/totp`. |
| `POST /v1/account/webauthn/handoff` | a session with a fresh login | Body `{purpose: "register" \| "confirm", label?}`. `register` adds a key (label 1 to 64 characters, at most 10 keys); `confirm` proves a second factor for the next five minutes. |
| `POST /v1/account/webauthn/handoff/state` | the same session | Body `{handoffId}`. Answers `{state}`. |
| `GET /v1/account/webauthn` | a session | `[{credentialRef, label, createdAt, lastUsedAt}]`; `credentialRef` is the first 16 bytes of SHA-256 of the credential ID. |
| `POST /v1/account/webauthn/remove` | a session with a fresh login, plus a TOTP `code` or a `confirm` from the last five minutes | Removes one key; audit event `WebauthnRemoved`. Removing the last factor turns two-step verification off. |
| `GET /webauthn/`, `page.js`, `page.css`, `strings.json` | none | The static page, with the headers listed under "Reverse proxy". |
| `POST /v1/webauthn/handoff/options` | the handoff ID and the typed code | Body `{handoffId, code}`. Five wrong codes end the handoff. Answers `{purpose, deviceName, requestedAt, publicKey}`. |
| `POST /v1/webauthn/handoff/finish` | the handoff ID | Body `{handoffId, credential}` (the browser's credential JSON), at most 16 KiB. Answers `{ok: true}`, `twoFactorInvalid`, `handoffExpired` or `rateLimited`. |

No route has the handoff ID in its path: it travels only in the URL fragment
(which the browser keeps to itself) and in request bodies, so your access and
proxy logs never record it. Every request body refuses unknown keys. The two unauthenticated handoff routes
are limited per client IP address (10 a minute, burst 20); wrong codes and
failed checks also count against the account's hourly second-factor budget.
Removing TOTP, removing a key, removing a device and deleting the account accept
either a TOTP code or a recent `confirm`, so an account with only security keys
can still do them.

Audit events: 17 security key added, 18 removed, 19 second factor reset by the
operator, 20 a key's signature counter went backwards (the sign-in is refused;
it can mean a cloned key).

What you see as the operator: credential IDs, public keys, labels, counters and
times. What you do not see: anything that helps decrypt a vault. A security key
protects access to the server account; vault data stays protected by the
master password and the Secret Key.

## Revoked sessions

When the server ends sessions for a reason, it remembers the token hashes for
30 days in `revoked_sessions` with the reason: `rekey` (a re-key from another
device), `passwordChange`, `deviceRemoved` or `operator`
(`account reset-second-factor`). A device that later uses such a token gets the
usual `401 unauthorized`, with `detail: {revoked: "<reason>"}`, and shows a
matching message (for a re-key: sign in with the new Secret Key from the newest
Emergency Kit). Logout and expiry are not recorded. The hourly housekeeping
drops older rows (CRYPTO_SPEC §9.5, ADR-0080).

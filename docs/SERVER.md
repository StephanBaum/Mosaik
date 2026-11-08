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
  secret file.

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
| `[limits] max_request_bytes` | Largest JSON body, 2 MiB to 64 MiB (default 8 MiB). Attachments have their own limit (the 50 MiB plaintext maximum plus the envelope). |
| `[limits] account_attachment_bytes` | Attachment bytes per account, soft-deleted ones included (default 1 GiB). |
| `[limits] account_record_count` / `account_record_bytes` | Records and record ciphertext per account (default 50 000 and 256 MiB). |
| `[limits] session_idle_days` / `session_max_days` | A session ends after this many idle days (default 30) or this many days in all (default 180). |
| `[limits] max_concurrent_requests` | Requests handled at once, server-wide; more wait in the queue (default 512). |
| `[limits] max_key_generations` | Key generations per vault (default 10 000). |
| `[limits] soft_delete_days` | A deleted attachment is kept this long, so a device can restore it (default 30). |

Fixed limits: 32 devices, 64 sessions and 1 000 vaults per account, 32 waiting polls per account, and ten wrong TOTP codes per account per hour. On Unix the server creates the data directory with mode 0700 and the database, WAL and secret files with 0600, and refuses to start if the secret file is readable by group or others.

## Command line

```text
mosaic-server init          --config <file>
mosaic-server serve         --config <file>
mosaic-server check-config  --config <file>
mosaic-server invite create --config <file>
mosaic-server backup        --config <file> --out <file> [--blobs]
mosaic-server account delete --config <file> --email <address>
```

- `invite create` prints a 20-character code once and stores only its hash.
  A code works for one registration.
- `account delete` removes the account, its vaults, records and attachment
  files. Sessions end at once.
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

The server limits request time, body size and concurrent requests, but has no
header read timeout of its own. The reverse proxy is the defence against
slow-header clients (slowloris): keep its read timeouts and per-IP connection
limits on (Caddy and nginx do by default).

## Upgrades

The database carries its schema version and migrates itself when a newer
server opens it. A server refuses a database written by a newer version. Back
up before upgrading.

## Operating notes

- Accounts and sessions: a user can remove a device and delete the account
  from the app; both need a fresh login (within five minutes) and a TOTP code
  when two-step verification is on.
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
  days and attachments deleted more than `soft_delete_days` ago.
- Run one server process per data directory.

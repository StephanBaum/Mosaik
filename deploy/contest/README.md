# Security Challenge test server

The kit for the public test server of the
[Mosaic Security Challenge](../../docs/BUG_BOUNTY.md). It is **not deployed**.
The owner decides on a host and a budget first.

Mosaic is pre-audit software. This server is a target, not a service.

## What you should know

- **No real data.** Every account on it is a challenge account or a stranger's
  test account. Do not put anything real on it.
- **Wiped every week.** The data is deleted and the challenge accounts are
  created again, with new flags.
- **Logs are kept for 7 days.** The operator can read them. They hold request
  lines and addresses, so assume that what you send is seen.
- **The host** is `{CONTEST_HOST}`. It has no value until the owner picks one.
- It is a separate machine from anything real. Do not run it next to other
  services.

## Files

| File | What it is |
|---|---|
| `compose.yaml` | the server and Caddy, hardened like `deploy/compose.yaml` |
| `Caddyfile` | TLS 1.3 only, access log kept 7 days |
| `mosaic-server.contest.toml` | the server config: open registration, small limits |

## Set up

You need Docker and a host name that points at the machine.

```sh
cd deploy/contest
cp mosaic-server.contest.toml mosaic-server.toml   # set canonical_origin
export CONTEST_HOST=your.host.name
docker compose run --rm mosaic-server init --config /config/mosaic-server.toml
docker compose up -d
```

## Seed the challenge accounts

The seeder creates the accounts of [`CHALLENGE.md`](../../docs/CHALLENGE.md)
through the normal client paths. Run it on a machine with the Rust toolchain,
against the public address:

```sh
cargo run -p mosaic-server --example contest_seed -- \
  --server https://$CONTEST_HOST --out contest-out
```

It takes a few minutes. It waits between steps because the server limits
registrations and sign-ins.

It writes into `contest-out/`:

- `public.json`: what you publish. Addresses, the passwords the rules
  publish, hashes of the downloads. It holds no flag and no secret.
- `private.json`: the flags and every secret the rules do not publish. Keep
  it. Never commit it, never publish it. The folder is in `.gitignore`.
- `c2-store/`, `c3.mosaicx`, `c6-vault/`: the downloads of C2, C3 and C6.

### Running it twice

It is safe. A second run stops before it touches the server if
`contest-out/` already holds a `public.json` or `private.json`. A server that
already has the accounts refuses the registration, and the seeder says so and
writes nothing.

To reseed, wipe both sides first. That is the weekly reset below.

## Weekly reset

1. Save `private.json` of the old season if you need the flags.
2. Stop the server and delete its data (this also removes the logs):

   ```sh
   docker compose down -v
   rm -rf contest-out
   ```

3. Start again: the `init` and `up -d` commands above, then the seeder.
4. Publish the new `public.json` and the new downloads.

A cron job on the host can run these steps. The server never holds a
`private.json`.

## Not tested here

The seeder was tested on a loopback server. The compose files were written
without Docker and have not been run. The HTTPS path of the seeder has no
automated test. Try the whole kit once on a throwaway host before you
announce a season.

The server allows three registrations per address an hour. The seeder uses
exactly three (c1, c4 and hidden), so run it from an address that is not
also used for testing.

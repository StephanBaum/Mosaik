# Deploying the Mosaic sync server

Read `docs/SERVER.md` first (what the server stores, configuration, backups,
reverse proxy and the trusted proxy header). Mosaic is pre-audit software.

| File | Purpose |
|---|---|
| `Dockerfile` | Multi-stage build from the repository root: Rust builder, distroless non-root runtime, base images pinned by digest. |
| `compose.yaml` | The server on an internal network plus Caddy (TLS 1.3, automatic certificates). |
| `Caddyfile` | The proxy configuration used by the compose file. |
| `mosaic-server.example.toml` | The server configuration for the container layout (`/data`, `/config`). |

No image is published before M9; the compose file builds it from source. The
image and the compose file are built and smoke-tested in CI (the `server-image`
job), not on the development machines.

## Container

```sh
cd deploy
cp mosaic-server.example.toml mosaic-server.toml   # set canonical_origin
export MOSAIC_DOMAIN=sync.example.com              # the host of canonical_origin
docker compose run --rm mosaic-server init --config /config/mosaic-server.toml
docker compose up -d
docker compose run --rm mosaic-server invite create --config /config/mosaic-server.toml
```

Back up the `mosaic-data` volume (it holds the secret file) as described in
`docs/SERVER.md`.

## Without a container

Build with `cargo build --release -p mosaic-server`, copy
`target/release/mosaic-server` to `/usr/local/bin`, write a config with
`listen = "127.0.0.1:8743"` and your own paths, run `init`, and put Caddy or
nginx in front. `docs/SERVER.md` has a systemd unit and the proxy settings.

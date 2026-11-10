# syntax=docker/dockerfile:1
#
# The Mosaic sync server (M6, ADR-0044). Build from the repository root:
#
#   docker build -f deploy/Dockerfile -t mosaic-server .
#
# Multi-stage: a Rust builder, then a distroless runtime with no shell, no
# package manager and a non-root user. Base images are pinned by digest;
# update the digests deliberately (and the toolchain tag together with
# rust-toolchain.toml).
#
# Runtime layout:
#   /config/mosaic-server.toml   the config (mount it read-only)
#   /data                        the database, the attachments and the secret
#                                file (a volume; back it up, docs/SERVER.md)
#
# First start:  mosaic-server init  --config /config/mosaic-server.toml
# Then:         mosaic-server serve --config /config/mosaic-server.toml (the default)

# rust:1.99.1-bookworm (the toolchain of rust-toolchain.toml)
FROM rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS builder
# Use the image's toolchain as is: no component downloads for rust-toolchain.toml.
ENV RUSTUP_TOOLCHAIN=1.99.1
# Reproducible builds (M9, ADR-0086, docs/REPRODUCIBLE_BUILDS.md). The release workflow passes
# --build-arg SOURCE_DATE_EPOCH=<commit time>; the default 0 is for ad-hoc builds. The sources
# always sit in /src and the cargo home is /usr/local/cargo, so these remaps give the same
# paths in the binary however the build context was named on the host.
ARG SOURCE_DATE_EPOCH=0
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH} \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="--remap-path-prefix=/src=/mosaic --remap-path-prefix=/usr/local/cargo=/cargo -C link-arg=-Wl,--build-id=none"
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p mosaic-server --bin mosaic-server \
 && install -d -o 65532 -g 65532 /out/data \
 && install -m 0755 target/release/mosaic-server /out/mosaic-server

# gcr.io/distroless/cc-debian12:nonroot (glibc and libgcc only)
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=builder /out/mosaic-server /usr/local/bin/mosaic-server
# An empty data directory owned by the runtime user: a new volume mounted on
# it inherits this ownership.
COPY --from=builder --chown=65532:65532 /out/data /data
VOLUME ["/data"]
EXPOSE 8743
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/mosaic-server"]
CMD ["serve", "--config", "/config/mosaic-server.toml"]

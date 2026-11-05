# Mosaic patch of rqrr 0.11.0

Upstream: rqrr 0.11.0 (crates.io), unmodified except as listed below.

## Why

`PreparedImage::find_groupings` tries every viable neighbour pair of every
capstone (finder pattern) and clones the whole prepared image (up to 8 MB) per
attempt. An image full of fake finder squares (1:1:3:1:1 runs with no valid
grid) therefore runs for hours. `capstones_from_image` is crate-private, so the
limit cannot be applied from outside. Mosaic scans untrusted screenshots and
screen captures (ADR-0031, threat model R32).

## Diff summary

- `src/prepare.rs`: two constants `MAX_CAPSTONES = 64` and
  `MAX_GROUP_ATTEMPTS = 256`; `find_groupings` returns no groups if there are
  more than `MAX_CAPSTONES` capstones and stops after `MAX_GROUP_ATTEMPTS`
  calls to `SkewedGridLocation::from_group` in total. Marked `// Mosaic patch:`.
- `Cargo.toml`: dev-dependencies removed; `Cargo.lock`, `.cargo-ok`,
  `.cargo_vcs_info.json` and `src/test_data` (only used by upstream unit tests)
  dropped.

## Dropping it

Once upstream bounds this work (check the release notes), delete `vendor/rqrr`,
the `[patch.crates-io]` entry and the `exclude` entry in the root `Cargo.toml`
and in `crates/qr/fuzz/Cargo.toml`, and bump the `rqrr` version.

## Second patch: no panic on degenerate perspective transforms (2026-11-05)

The CI fuzz smoke (`scan_luma`) hit `assert!(x >= i32::MIN as f64)` in
`Perspective::map` (`src/geometry.rs`): a hostile image can produce a transform
that maps to NaN, an infinity or a value outside `i32`. Upstream asserts, which
aborts Mosaic's release builds. `map` now rounds and clamps each coordinate to
`±MAX_COORD` (16383, NaN to the lower bound) via `clamp_coord`. The bound also
keeps the later `i32` products of coordinate differences (`identify/grid.rs`,
`geometry.rs` line intersection) from overflowing. Real points lie inside the
image (Mosaic limits sides to 16384 px), so decoding of valid codes is
unchanged. The vendored crate's own unit tests need upstream's `image`
dev-dependency (removed), so the regression check is the `scan_luma` fuzz smoke
that found it. Marked `// Mosaic patch:`.

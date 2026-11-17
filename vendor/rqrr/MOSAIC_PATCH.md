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
keeps the later `i32` products of coordinate differences (`identify/grid.rs`)
from overflowing; the line intersection was not covered, see the fourth patch.
Real points lie inside the
image (Mosaic limits sides to 16384 px), so decoding of valid codes is
unchanged. The vendored crate's own unit tests need upstream's `image`
dev-dependency (removed), so the regression check is the `scan_luma` fuzz smoke
that found it. Marked `// Mosaic patch:`.

## Third patch: no panic on a degenerate capstone (2026-11-15)

The CI fuzz smoke (`scan_luma`) hit `.expect("rotated perspective can't fail")` in
`rotate_capstone` (`src/identify/grid.rs`): a hostile image can give capstone corners
from which `Perspective::create` finds no transform. Mosaic's release builds abort on
a panic, so scanning such a screenshot closed the app. `rotate_capstone` now returns
`Option<()>` and `SkewedGridLocation::from_group` (which already returns `Option`)
drops that grouping. Valid codes decode as before. Marked `// Mosaic patch:`.

## Fourth patch: panic audit (2026-11-16)

Every panic site in `src` outside `#[cfg(test)]` was checked against hostile
pixels, frame sizes within Mosaic's limits (grey-level frames of at least 21 by
21 and at most 2^23 pixels, sides up to 16384 px) and grid sizes read from the
image. Each reachable site now drops the QR candidate instead of panicking, and
so do the unreachable ones that were cheap to change. Valid codes decode as
before. All marked `// Mosaic patch:`.

Reachable, fixed:

- `geometry.rs` `line_intersect`: computed in `i32`; the product of an edge
  length squared and a coordinate overflows for large capstones (three 1316 px
  finder patterns in a 2896 by 2896 frame), a debug-build panic, and in release
  a wrapped numerator of `i32::MIN` over a determinant of -1 panics on the
  division. Now computed in `i128`; a result beyond `±MAX_COORD` returns `None`
  (`SkewedGridLocation::from_group` drops the grouping), which also keeps the
  alignment-search products in `identify/grid.rs` inside `i32`. Real codes
  within the 2^23 pixel limit stay below the old overflow, so their result is
  unchanged. Test: `oversized_finder_patterns_do_not_overflow`
  (`crates/qr/tests/redteam.rs`), which panics without this patch.
- `identify/grid.rs` `measure_timing_pattern`: `assert!(scan >= 1)` fires when
  neither timing scan line between the capstones sees a colour change (for
  example rings that touch only diagonally, or a stone region bridging the
  capstones). It now returns `None` and `from_group` drops the grouping. No
  input was constructed: about 70,000 generated images with three finder
  patterns (notched, warped and near-degenerate rings) did not hit it.
- `identify/grid.rs` `find_alignment_pattern`: `step_size * step_size <
  size_estimate * 100` uses `saturating_mul`. On a 32-bit target the product
  could overflow (a debug panic) or wrap into a much longer search; on 64-bit
  targets nothing changes.

Unreachable, made fail-safe:

- `prepare.rs` `get_region` returns `Option`: no `unwrap` on the region cache
  (a discarded region is always cached), no `expect` on `pop_lru` (the cache is
  full there), no `panic!` on a white pixel (callers pass non-white pixels).
- `prepare.rs` `repaint_and_apply` returns `Option` instead of panicking on a
  white or same-coloured source; `flood_fill` returns early instead of
  `assert_ne!(from, to)`. Callers (`detect.rs` `create_capstone`,
  `is_capstone`, `identify/grid.rs`) use `?` or treat `None` as no match.
- `identify/match_capstones.rs`: the score sort uses `unwrap_or(Equal)`
  instead of `expect`; scores are never NaN, so the order is unchanged.

Unreachable, left as upstream:

- `decode.rs`: `RawData::push` assert (a grid of at most 177 by 177 cells
  fills at most 3917 of 8896 bytes), the two `take_bits` asserts (the pointer
  never passes the length; at most 16 bits are read), `correct_block`'s assert
  and the codeword indexing (fixed version table, versions 1 to 40 only:
  `Version::from_size` rejects the rest before any data is read), the
  `Unknown mask value` panic (the mask is 3 bits).
- `prepare.rs`: `NonZeroUsize::new(251).unwrap()` (a constant),
  `without_preparation`'s assert (only `prepare_from_bitmap` calls it, with 0
  and 1), the `checked_mul` `expect`s (Mosaic frames have at most 2^23 pixels),
  pixel indexing (every caller checks or clamps coordinates).
- `detect.rs`: the line scanner's subtractions (the last five runs never sum
  past the position), the ratio division (a region has at least one pixel),
  the `i32` corner scores (sides up to 16384 px).
- `identify/grid.rs`: `version_from_grid_size` (grid sizes are at least 17),
  the version table index (checked before use), the remaining `i32` products
  (clamped coordinates, second patch).
- `lib.rs`: `write_grid_to`'s `unwrap` (the `img` feature is off) and
  `SimpleGrid` (unused by Mosaic). The `unwrap`s in `detect.rs` tests and the
  `decode.rs` test panic are test-only.

Overflow that only wraps in release and never reaches indexing (for example
corner scores on frames wider than 16384 px, which Mosaic does not produce from
images) was noted, not changed.

### Fourth patch, addendum: bounded alignment search (2026-11-16)

`find_alignment_pattern` (`src/identify/grid.rs`) spirals out until `step_size² ≥ 100 · size_estimate`.
A misshaped capstone can make `size_estimate` about 2·10⁹, so one candidate asked for about
2·10¹¹ iterations: not a panic, but a hung scan (R32). The spiral now stops at
`MAX_ALIGN_STEP = 1024` steps (search radius about ±512 px, at most about 10⁶ iterations). A
real alignment pattern lies within about ten module widths of the estimate, so codes with
modules up to about 100 px decode as before. Marked `// Mosaic patch:`.


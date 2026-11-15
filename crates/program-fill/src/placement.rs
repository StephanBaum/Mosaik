//! Where Mosaic's chip and list go (ADR-0164): the chip right of the field (left
//! when there is no room), the list under the chip's field (above when there
//! is no room), on the field's monitor, never over the field. Pure integer
//! geometry in physical pixels, safe on monitors at negative coordinates and
//! at any scale.

use crate::verify::{Rect, probe_points};

/// The chip's side, in logical pixels.
pub const CHIP_PX: i32 = 24;
/// The gap between field and chip or list, in logical pixels.
pub const GAP_PX: i32 = 4;
/// The list's width, in logical pixels.
pub const LIST_WIDTH_PX: i32 = 320;
/// The list's preferred height, in logical pixels.
pub const LIST_HEIGHT_PX: i32 = 360;
/// The least list height worth showing, in logical pixels.
pub const LIST_MIN_HEIGHT_PX: i32 = 160;

const MIN_SCALE: f64 = 0.5;
const MAX_SCALE: f64 = 5.0;

/// A rectangle in `i64`, so no sum overflows.
#[derive(Clone, Copy)]
struct Area {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl Area {
    fn of(rect: Rect) -> Self {
        Self {
            left: i64::from(rect.left),
            top: i64::from(rect.top),
            right: i64::from(rect.right),
            bottom: i64::from(rect.bottom),
        }
    }

    const fn width(self) -> i64 {
        self.right - self.left
    }

    const fn height(self) -> i64 {
        self.bottom - self.top
    }

    const fn intersects(self, other: Self) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    fn rect(self) -> Option<Rect> {
        Some(Rect {
            left: i32::try_from(self.left).ok()?,
            top: i32::try_from(self.top).ok()?,
            right: i32::try_from(self.right).ok()?,
            bottom: i32::try_from(self.bottom).ok()?,
        })
    }
}

/// `logical * scale`, rounded, at least 1; `None` for a scale outside
/// 0.5 to 5.0 (and NaN).
#[expect(
    clippy::cast_possible_truncation,
    reason = "the product of a small logical size and a scale of at most 5 fits i64 exactly"
)]
fn physical(logical: i32, scale: f64) -> Option<i64> {
    if !(MIN_SCALE..=MAX_SCALE).contains(&scale) {
        return None;
    }
    Some(((f64::from(logical) * scale).round() as i64).max(1))
}

/// The common checks: a usable scale, a testable field, a field on the monitor.
fn inputs(field: Rect, monitor: Rect, scale: f64) -> Option<(Area, Area)> {
    physical(1, scale)?;
    probe_points(field)?;
    let (field, monitor) = (Area::of(field), Area::of(monitor));
    (monitor.width() > 0 && monitor.height() > 0 && field.intersects(monitor))
        .then_some((field, monitor))
}

/// The chip's rectangle: a square of `CHIP_PX` right of the field, or left of
/// it when the monitor has no room, centred on the field and kept on the
/// monitor. `None` when neither side has room, or for an odd field or scale.
#[must_use]
pub fn chip_rect(field: Rect, monitor: Rect, scale: f64) -> Option<Rect> {
    let (field, monitor) = inputs(field, monitor, scale)?;
    let side = physical(CHIP_PX, scale)?;
    let gap = physical(GAP_PX, scale)?;
    if monitor.height() < side {
        return None;
    }
    let right_x = field.right + gap;
    let x = if right_x + side <= monitor.right {
        right_x
    } else {
        let left_x = field.left - gap - side;
        if left_x < monitor.left {
            return None;
        }
        left_x
    };
    let y = (field.top + (field.height() - side) / 2).clamp(monitor.top, monitor.bottom - side);
    Area {
        left: x,
        top: y,
        right: x + side,
        bottom: y + side,
    }
    .rect()
}

/// The list's rectangle: `LIST_WIDTH_PX` wide (at most the monitor), starting
/// at the field's left edge and kept on the monitor, below the field when the
/// preferred height fits, else above, else on the larger side with the height
/// cut to fit if that leaves at least `LIST_MIN_HEIGHT_PX`.
#[must_use]
pub fn list_rect(field: Rect, monitor: Rect, scale: f64) -> Option<Rect> {
    let (field, monitor) = inputs(field, monitor, scale)?;
    let width = physical(LIST_WIDTH_PX, scale)?.min(monitor.width());
    let want = physical(LIST_HEIGHT_PX, scale)?;
    let least = physical(LIST_MIN_HEIGHT_PX, scale)?;
    let gap = physical(GAP_PX, scale)?;
    let x = field.left.clamp(monitor.left, monitor.right - width);
    let below_top = field.bottom + gap;
    let above_bottom = field.top - gap;
    let below_room = (monitor.bottom - below_top).max(0);
    let above_room = (above_bottom - monitor.top).max(0);
    let (top, height) = if below_room >= want {
        (below_top, want)
    } else if above_room >= want {
        (above_bottom - want, want)
    } else if below_room >= above_room && below_room >= least {
        (below_top, below_room)
    } else if above_room >= least {
        (monitor.top, above_room)
    } else {
        return None;
    };
    Area {
        left: x,
        top,
        right: x + width,
        bottom: top + height,
    }
    .rect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const MONITOR: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    fn field(left: i32, top: i32) -> Rect {
        Rect {
            left,
            top,
            right: left + 200,
            bottom: top + 30,
        }
    }

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.left >= outer.left
            && inner.top >= outer.top
            && inner.right <= outer.right
            && inner.bottom <= outer.bottom
    }

    fn disjoint(a: Rect, b: Rect) -> bool {
        !Area::of(a).intersects(Area::of(b))
    }

    #[test]
    fn the_chip_sits_right_of_the_field_or_left_when_there_is_no_room() {
        let f = field(500, 400);
        let chip = chip_rect(f, MONITOR, 1.0).unwrap();
        assert_eq!(chip.left, f.right + GAP_PX);
        assert_eq!(chip.right - chip.left, CHIP_PX);
        assert!(inside(chip, MONITOR) && disjoint(chip, f));
        let edge = field(1900 - 200, 400);
        let chip = chip_rect(edge, MONITOR, 1.0).unwrap();
        assert_eq!(chip.right, edge.left - GAP_PX);
        assert!(inside(chip, MONITOR) && disjoint(chip, edge));
        // No room on either side.
        let wide = Rect {
            left: 5,
            top: 400,
            right: 1915,
            bottom: 430,
        };
        assert_eq!(chip_rect(wide, MONITOR, 1.0), None);
    }

    #[test]
    fn the_list_opens_below_or_flips_above_and_shrinks_to_fit() {
        let f = field(500, 400);
        let list = list_rect(f, MONITOR, 1.0).unwrap();
        assert_eq!(list.top, f.bottom + GAP_PX);
        assert_eq!(list.right - list.left, LIST_WIDTH_PX);
        assert_eq!(list.bottom - list.top, LIST_HEIGHT_PX);
        let low = field(500, 1000);
        let list = list_rect(low, MONITOR, 1.0).unwrap();
        assert_eq!(list.bottom, low.top - GAP_PX);
        assert!(inside(list, MONITOR) && disjoint(list, low));
        let short = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 300,
        };
        let mid = field(500, 100);
        let list = list_rect(mid, short, 1.0).unwrap();
        assert!(list.bottom - list.top < LIST_HEIGHT_PX);
        assert!(list.bottom - list.top >= LIST_MIN_HEIGHT_PX);
        assert!(inside(list, short) && disjoint(list, mid));
        let tiny = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 100,
        };
        assert_eq!(list_rect(field(500, 30), tiny, 1.0), None);
    }

    #[test]
    fn placement_on_a_left_monitor_with_negative_coordinates_and_mixed_dpi() {
        let monitor = Rect {
            left: -2560,
            top: -200,
            right: 0,
            bottom: 1240,
        };
        let f = Rect {
            left: -400,
            top: 300,
            right: -10,
            bottom: 340,
        };
        let chip = chip_rect(f, monitor, 1.5).unwrap();
        assert_eq!(chip.right - chip.left, 36);
        assert!(chip.right <= f.left && inside(chip, monitor));
        let list = list_rect(f, monitor, 1.5).unwrap();
        assert_eq!(list.right - list.left, 480);
        assert!(inside(list, monitor) && disjoint(list, f) && disjoint(chip, f));
    }

    #[test]
    fn odd_fields_and_scales_place_nothing() {
        let f = field(500, 400);
        let inverted = Rect {
            left: 700,
            top: 430,
            right: 500,
            bottom: 400,
        };
        let thin = Rect {
            left: 500,
            top: 400,
            right: 503,
            bottom: 430,
        };
        let off = field(5000, 400);
        for odd in [inverted, thin, off] {
            assert_eq!(chip_rect(odd, MONITOR, 1.0), None);
            assert_eq!(list_rect(odd, MONITOR, 1.0), None);
        }
        for scale in [0.0, f64::NAN, 9.0, 0.49, f64::INFINITY] {
            assert_eq!(chip_rect(f, MONITOR, scale), None);
            assert_eq!(list_rect(f, MONITOR, scale), None);
        }
    }

    proptest! {
        #[test]
        fn placements_stay_on_the_monitor_and_off_the_field(
            fl in -3000_i32..3000, ft in -3000_i32..3000, fw in 0_i32..1500, fh in 0_i32..300,
            ml in -3000_i32..3000, mt in -3000_i32..3000, mw in 0_i32..3000, mh in 0_i32..2000,
            scale in 0.5_f64..5.0,
        ) {
            let f = Rect { left: fl, top: ft, right: fl + fw, bottom: ft + fh };
            let m = Rect { left: ml, top: mt, right: ml + mw, bottom: mt + mh };
            if let Some(chip) = chip_rect(f, m, scale) {
                prop_assert!(inside(chip, m) && disjoint(chip, f));
            }
            if let Some(list) = list_rect(f, m, scale) {
                prop_assert!(inside(list, m) && disjoint(list, f));
            }
        }
    }
}

//! The one place that touches the UI Automation source of ADR-0157 D1: the
//! shim's [`Uia`]. Everything here maps shim facts to the pure model of
//! [`crate::verify`].

use std::time::Instant;

use mosaic_winshim::{ShimError, Uia, UiaElement};

use crate::verify::{FieldFacts, FieldKind, Point, ProbeError, Rect, RuntimeId};

/// An element read through UI Automation.
pub(crate) type UiaElementRef = UiaElement;

/// `UIA_EditControlTypeId`.
const EDIT_CONTROL_TYPE: i32 = 50004;

/// The shim's errors as the probe's.
pub(crate) fn probe_error(err: ShimError) -> ProbeError {
    match err {
        ShimError::Timeout => ProbeError::Timeout,
        ShimError::NotFound => ProbeError::Gone,
        _ => ProbeError::Os,
    }
}

/// The model's view of an element.
fn field_facts(element: &UiaElementRef) -> FieldFacts {
    let facts = element.facts();
    let [left, top, right, bottom] = facts.rect;
    FieldFacts {
        runtime_id: RuntimeId(facts.runtime_id.clone()),
        pid: facts.pid,
        kind: if facts.is_password {
            FieldKind::Password
        } else {
            FieldKind::Text
        },
        enabled: facts.enabled,
        offscreen: facts.offscreen,
        rect: Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// The focused element when it is an edit control, with its facts and the
/// native window handle of its nearest window. `None` when nothing or a
/// non-edit element has focus.
pub(crate) fn focused_field(
    uia: &Uia,
    deadline: Instant,
) -> Result<Option<(UiaElementRef, FieldFacts, isize)>, ProbeError> {
    let Some(element) = uia.focused(deadline).map_err(probe_error)? else {
        return Ok(None);
    };
    if element.facts().control_type != EDIT_CONTROL_TYPE {
        return Ok(None);
    }
    let window = uia
        .nearest_window(&element, deadline)
        .map_err(probe_error)?;
    let facts = field_facts(&element);
    Ok(Some((element, facts, window)))
}

/// Whether two elements are the same UI element.
pub(crate) fn same(
    uia: &Uia,
    a: &UiaElementRef,
    b: &UiaElementRef,
    deadline: Instant,
) -> Result<bool, ProbeError> {
    uia.same(a, b, deadline).map_err(probe_error)
}

/// Whether the element at `p` is the target or inside it (nothing covers the
/// point). An empty point counts as covered.
pub(crate) fn clear_at(
    uia: &Uia,
    target: &UiaElementRef,
    p: Point,
    deadline: Instant,
) -> Result<bool, ProbeError> {
    let Some(at_point) = uia
        .element_from_point(p.x, p.y, deadline)
        .map_err(probe_error)?
    else {
        return Ok(false);
    };
    uia.is_within(&at_point, target, deadline)
        .map_err(probe_error)
}

//! Keyboard input: typing UTF-16 units and a few keys with `SendInput`, and
//! the held-modifier check (ADR-0163).
//!
//! The typed units can be a password. The `INPUT` records that carry them are
//! built in a zeroizing vector of exact capacity (never grown, so no
//! reallocated copy is left behind) and wiped when it drops. Windows keeps its
//! own copies in the input queue (R173).
#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY, VK_A, VK_BACK, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
};
use zeroize::{DefaultIsZeroes, Zeroizing};

use crate::error::{Result, ShimError};

/// The most UTF-16 units [`send_unicode`] types in one call.
pub const MAX_UNITS_PER_CALL: usize = 16;

/// A key [`send_key`] presses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// Enter.
    Enter,
    /// Ctrl+A.
    SelectAll,
    /// Backspace.
    Backspace,
}

/// One `INPUT` record. A transparent wrapper, so a vector of them is a vector
/// of `INPUT` and can be wiped as one.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
struct InputRecord(INPUT);

impl DefaultIsZeroes for InputRecord {}

fn keyboard(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> InputRecord {
    InputRecord(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}

fn vk_press(vk: VIRTUAL_KEY, records: &mut Vec<InputRecord>) {
    records.push(keyboard(vk, 0, KEYBD_EVENT_FLAGS(0)));
}

fn vk_release(vk: VIRTUAL_KEY, records: &mut Vec<InputRecord>) {
    records.push(keyboard(vk, 0, KEYEVENTF_KEYUP));
}

/// Hands the records to the input queue; the number of events inserted
/// (fewer than sent when another process holds the input lock or UIPI blocks).
fn send(records: &[InputRecord]) -> Result<u32> {
    if records.is_empty() {
        return Ok(0);
    }
    let size = i32::try_from(size_of::<INPUT>()).map_err(|_| ShimError::Os(0))?;
    // SAFETY: `InputRecord` is `repr(transparent)` over `INPUT`, so the
    // pointer and length of `records` describe exactly a slice of `INPUT`; the
    // slice does not outlive `records`.
    let inputs =
        unsafe { core::slice::from_raw_parts(records.as_ptr().cast::<INPUT>(), records.len()) };
    // SAFETY: `inputs` is a valid slice of initialised `INPUT` records and
    // `size` is `size_of::<INPUT>()`, as `SendInput` requires.
    Ok(unsafe { SendInput(inputs, size) })
}

/// Types UTF-16 units into the focused window: for each unit a key down and a
/// key up with `KEYEVENTF_UNICODE`, so the layout does not matter. A surrogate
/// pair goes in as its two units. Returns the number of events inserted (two
/// per unit when all went in).
///
/// # Errors
/// [`ShimError::TooLong`] for more than [`MAX_UNITS_PER_CALL`] units.
pub fn send_unicode(units: &[u16]) -> Result<u32> {
    if units.len() > MAX_UNITS_PER_CALL {
        return Err(ShimError::TooLong);
    }
    let mut records: Zeroizing<Vec<InputRecord>> =
        Zeroizing::new(Vec::with_capacity(units.len().saturating_mul(2)));
    for &unit in units {
        records.push(keyboard(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE));
        records.push(keyboard(
            VIRTUAL_KEY(0),
            unit,
            KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
        ));
    }
    send(&records)
}

/// Presses a key: Enter (2 events), Ctrl+A (4: Ctrl down, A down, A up, Ctrl
/// up) or Backspace (2). Returns the number of events inserted.
///
/// # Errors
/// An OS error only for a malformed record size.
pub fn send_key(key: Key) -> Result<u32> {
    let mut records: Vec<InputRecord> = Vec::with_capacity(4);
    match key {
        Key::Enter => {
            vk_press(VK_RETURN, &mut records);
            vk_release(VK_RETURN, &mut records);
        }
        Key::Backspace => {
            vk_press(VK_BACK, &mut records);
            vk_release(VK_BACK, &mut records);
        }
        Key::SelectAll => {
            vk_press(VK_CONTROL, &mut records);
            vk_press(VK_A, &mut records);
            vk_release(VK_A, &mut records);
            vk_release(VK_CONTROL, &mut records);
        }
    }
    send(&records)
}

/// Whether Shift, Ctrl, Alt or a Windows key is physically held (`AltGr` counts:
/// Windows reports it as Ctrl+Alt).
#[must_use]
pub fn modifiers_down() -> bool {
    [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
        .into_iter()
        .any(|vk| {
            // SAFETY: takes a virtual-key code by value and only reads the
            // keyboard state.
            let state = unsafe { GetAsyncKeyState(i32::from(vk.0)) };
            state.cast_unsigned() & 0x8000 != 0
        })
}

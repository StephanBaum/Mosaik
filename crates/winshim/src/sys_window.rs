//! Window facts: the foreground window, its root, its owner process, and
//! whether it is cloaked or minimised.
#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]

use core::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, HWND_TOPMOST, IsIconic,
    IsWindow, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetWindowPos, ShowWindow,
};

use crate::error::{Result, ShimError};

/// Facts about the top-level window of a window handle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowFacts {
    /// The root (top-level) window handle value.
    pub root: isize,
    /// The process that owns the root window.
    pub owner_pid: u32,
    /// Whether the window is cloaked (hidden by the shell, such as a window on
    /// another virtual desktop).
    pub cloaked: bool,
    /// Whether the window is minimised.
    pub minimized: bool,
}

/// A window handle value as an `HWND`. Only ever passed to calls that validate
/// the handle themselves; it is never dereferenced.
fn hwnd_of(value: isize) -> HWND {
    HWND(core::ptr::without_provenance_mut::<c_void>(
        value.cast_unsigned(),
    ))
}

fn value_of(hwnd: HWND) -> isize {
    hwnd.0.addr().cast_signed()
}

/// The foreground window, `None` if there is none (the desktop is locked or
/// switching).
#[must_use]
pub fn foreground_window() -> Option<isize> {
    // SAFETY: takes no arguments and only reads shell state.
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.0.is_null()).then(|| value_of(hwnd))
}

/// The root window of `hwnd` and its owner, cloaked and minimised state.
///
/// # Errors
/// [`ShimError::NotFound`] for a null, destroyed or non-window handle.
pub fn window_facts(hwnd: isize) -> Result<WindowFacts> {
    let handle = hwnd_of(hwnd);
    // SAFETY: `IsWindow` accepts any handle value and only tests it. The Win32
    // calls below return zeros for a bad handle instead of an error, so it is
    // refused here.
    let is_window = unsafe { IsWindow(Some(handle)) };
    if handle.0.is_null() || !is_window.as_bool() {
        return Err(ShimError::NotFound);
    }
    // SAFETY: `handle` was just checked to be a window; the call only reads.
    let root = unsafe { GetAncestor(handle, GA_ROOT) };
    if root.0.is_null() {
        return Err(ShimError::NotFound);
    }
    let mut owner_pid = 0u32;
    // SAFETY: `root` is a window handle; `owner_pid` is a valid out pointer.
    let thread = unsafe { GetWindowThreadProcessId(root, Some(&raw mut owner_pid)) };
    if thread == 0 || owner_pid == 0 {
        return Err(ShimError::NotFound);
    }
    let mut cloaked = 0u32;
    // SAFETY: `root` is a window handle; `cloaked` is a valid, writable `u32`
    // and the size passed is that of a `u32`, as DWMWA_CLOAKED requires.
    unsafe {
        DwmGetWindowAttribute(
            root,
            DWMWA_CLOAKED,
            core::ptr::from_mut(&mut cloaked).cast::<c_void>(),
            4,
        )
    }?;
    // SAFETY: `root` is a window handle; the call only reads.
    let minimized = unsafe { IsIconic(root) }.as_bool();
    Ok(WindowFacts {
        root: value_of(root),
        owner_pid,
        cloaked: cloaked != 0,
        minimized,
    })
}

/// Brings a top-level window to the foreground. `false` when Windows refused
/// (the calling process has no right to take the foreground).
///
/// # Errors
/// [`ShimError::NotFound`] for a null, destroyed or non-top-level handle (a
/// child window is never brought forward).
pub fn set_foreground(hwnd: isize) -> Result<bool> {
    let facts = window_facts(hwnd)?;
    if facts.root != hwnd {
        return Err(ShimError::NotFound);
    }
    // SAFETY: `hwnd` was just checked to be a top-level window; the call asks
    // the shell to bring it forward and moves no memory.
    let done = unsafe { SetForegroundWindow(hwnd_of(hwnd)) };
    Ok(done.as_bool())
}

/// Shows one of Mosaic's own windows without taking focus, and keeps it on top.
///
/// # Errors
/// [`ShimError::NotFound`] for a null or destroyed handle,
/// [`ShimError::NotOwnWindow`] for a window of another process, or an OS error.
pub fn show_no_activate(hwnd: isize) -> Result<()> {
    let facts = window_facts(hwnd)?;
    if facts.owner_pid != std::process::id() {
        return Err(ShimError::NotOwnWindow);
    }
    let handle = hwnd_of(hwnd);
    // SAFETY: `handle` is a window of this process (checked above); showing it
    // without activation only changes its visibility state.
    let _was_visible = unsafe { ShowWindow(handle, SW_SHOWNOACTIVATE) };
    // SAFETY: as above; `HWND_TOPMOST` is a placement constant, the flags keep
    // position, size and activation unchanged.
    unsafe {
        SetWindowPos(
            handle,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }?;
    Ok(())
}

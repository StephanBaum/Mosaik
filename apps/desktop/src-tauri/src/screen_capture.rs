//! "Scan screen" (ADR-0031 items 4-5): one frame per display, taken with the
//! main window hidden, into zeroizing buffers. Windows: GDI via
//! `win-screenshot`. Linux: X11 `GetImage` via `x11rb`, not under Wayland.
//! macOS: unsupported (the paste-a-screenshot path covers it).

use std::time::Duration;

use mosaic_qr::OwnedFrame;
use tauri::{AppHandle, Manager as _, Runtime, WebviewWindow};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};

/// Time for the compositor to remove the hidden window before the capture.
pub const SETTLE: Duration = Duration::from_millis(250);

/// A screen frame above this many pixels (2^27) is refused. No rejection of
/// the user's own screen, but a bound.
#[cfg(any(windows, target_os = "linux"))]
const MAX_SCREEN_PIXELS: u64 = 134_217_728;

/// What can take screen frames.
pub trait ScreenCapture: Send + Sync {
    /// Whether this system can capture at all (drives `app_status.otpScreenScan`).
    fn supported(&self) -> bool;

    /// Captures every display once (at most `mosaic_qr::MAX_FRAMES`).
    ///
    /// # Errors
    /// `otpScanUnsupported` where not supported, `otpImageTooLarge` for a
    /// frame over the ceiling, `internal` when the OS call fails.
    fn capture(&self) -> IpcResult<Vec<OwnedFrame>>;
}

/// Tests and headless: unsupported.
pub struct NoScreenCapture;

impl ScreenCapture for NoScreenCapture {
    fn supported(&self) -> bool {
        false
    }

    fn capture(&self) -> IpcResult<Vec<OwnedFrame>> {
        Err(IpcError::new(ErrorCode::OtpScanUnsupported))
    }
}

/// The real capture, which hides the main window around the OS call.
pub struct NativeScreenCapture<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> NativeScreenCapture<R> {
    /// Capture for this app.
    #[must_use]
    pub const fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

/// Hides `main` on creation; on drop shows it again and focuses it. Every
/// path (success, error) restores the window because restoring lives in
/// `Drop`.
struct HiddenMain<R: Runtime> {
    window: Option<WebviewWindow<R>>,
}

impl<R: Runtime> HiddenMain<R> {
    fn new(app: &AppHandle<R>) -> Self {
        let window = app
            .get_webview_window("main")
            .filter(|w| w.is_visible().unwrap_or(false))
            .filter(|w| w.hide().is_ok());
        Self { window }
    }
}

impl<R: Runtime> Drop for HiddenMain<R> {
    fn drop(&mut self) {
        if let Some(window) = self.window.take() {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

impl<R: Runtime> ScreenCapture for NativeScreenCapture<R> {
    fn supported(&self) -> bool {
        platform::supported()
    }

    fn capture(&self) -> IpcResult<Vec<OwnedFrame>> {
        if !platform::supported() {
            return Err(IpcError::new(ErrorCode::OtpScanUnsupported));
        }
        let hidden = HiddenMain::new(&self.app);
        std::thread::sleep(SETTLE);
        let frames = platform::grab();
        drop(hidden);
        frames
    }
}

/// `otpImageTooLarge` for a frame over [`MAX_SCREEN_PIXELS`].
#[cfg(any(windows, target_os = "linux"))]
fn check_size(width: u32, height: u32) -> IpcResult<()> {
    if u64::from(width) * u64::from(height) > MAX_SCREEN_PIXELS {
        return Err(IpcError::new(ErrorCode::OtpImageTooLarge));
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use mosaic_qr::{OwnedFrame, PixelLayout};
    use zeroize::Zeroizing;

    use super::check_size;
    use crate::contract::errors::{IpcError, IpcResult};

    pub(super) fn supported() -> bool {
        true
    }

    /// `capture_display` returns the whole virtual screen as RGB with alpha 0
    /// (GDI), so `Rgbx8`; the buffer moves into the zeroizing frame.
    pub(super) fn grab() -> IpcResult<Vec<OwnedFrame>> {
        let buf = win_screenshot::capture::capture_display().map_err(|_| IpcError::INTERNAL)?;
        let (width, height) = (buf.width, buf.height);
        // Wiped on every path below, an oversized capture included.
        let pixels = Zeroizing::new(buf.pixels);
        check_size(width, height)?;
        let frame = OwnedFrame::new(width, height, PixelLayout::Rgbx8, pixels)
            .map_err(|_| IpcError::INTERNAL)?;
        Ok(vec![frame])
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use mosaic_qr::{OwnedFrame, PixelLayout};
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::VisualClass;
    use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat, ImageOrder, Screen, Setup};
    use zeroize::{Zeroize as _, Zeroizing};

    use super::check_size;
    use crate::contract::errors::{ErrorCode, IpcError, IpcResult};

    /// Largest strip read in one request.
    const STRIP_BYTES: usize = 4 << 20;

    /// `DISPLAY` set, `WAYLAND_DISPLAY` unset, `XDG_SESSION_TYPE` not "wayland".
    pub(super) fn supported() -> bool {
        let set = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty());
        let wayland_session =
            std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"));
        set("DISPLAY") && !set("WAYLAND_DISPLAY") && !wayland_session
    }

    fn unsupported() -> IpcError {
        IpcError::new(ErrorCode::OtpScanUnsupported)
    }

    /// Whether the root window is 32 bits per pixel, little endian, true
    /// colour with the masks `0xff0000`, `0xff00`, `0xff` (so `Bgrx8`).
    fn is_bgrx(setup: &Setup, screen: &Screen) -> bool {
        let format_ok = setup
            .pixmap_formats
            .iter()
            .any(|f| f.depth == screen.root_depth && f.bits_per_pixel == 32);
        let visual_ok = screen
            .allowed_depths
            .iter()
            .filter(|d| d.depth == screen.root_depth)
            .flat_map(|d| d.visuals.iter())
            .any(|v| {
                v.visual_id == screen.root_visual
                    && v.class == VisualClass::TRUE_COLOR
                    && v.red_mask == 0x00ff_0000
                    && v.green_mask == 0x0000_ff00
                    && v.blue_mask == 0x0000_00ff
            });
        setup.image_byte_order == ImageOrder::LSB_FIRST && format_ok && visual_ok
    }

    /// One frame: the root window spans every monitor. It is read in
    /// horizontal strips of at most 4 MiB, each copied into one zeroizing
    /// buffer that is allocated once, and the reply buffer is wiped.
    pub(super) fn grab() -> IpcResult<Vec<OwnedFrame>> {
        let (conn, num) = x11rb::connect(None).map_err(|_| unsupported())?;
        let setup = conn.setup();
        let screen = setup.roots.get(num).ok_or_else(unsupported)?;
        if !is_bgrx(setup, screen) {
            return Err(unsupported());
        }
        let (width, height) = (screen.width_in_pixels, screen.height_in_pixels);
        check_size(u32::from(width), u32::from(height))?;
        let row = usize::from(width) * 4;
        let total = row
            .checked_mul(usize::from(height))
            .ok_or(IpcError::INTERNAL)?;
        let mut pixels = Zeroizing::new(Vec::with_capacity(total));
        let strip = u16::try_from((STRIP_BYTES / row.max(1)).max(1)).unwrap_or(u16::MAX);
        let mut y = 0u16;
        while y < height {
            let rows = strip.min(height - y);
            let mut reply = conn
                .get_image(
                    ImageFormat::Z_PIXMAP,
                    screen.root,
                    0,
                    i16::try_from(y).map_err(|_| IpcError::INTERNAL)?,
                    width,
                    rows,
                    !0,
                )
                .map_err(|_| IpcError::INTERNAL)?
                .reply()
                .map_err(|_| IpcError::INTERNAL)?;
            pixels.extend_from_slice(&reply.data);
            reply.data.zeroize();
            y += rows;
        }
        let frame = OwnedFrame::new(
            u32::from(width),
            u32::from(height),
            PixelLayout::Bgrx8,
            pixels,
        )
        .map_err(|_| IpcError::INTERNAL)?;
        Ok(vec![frame])
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use mosaic_qr::OwnedFrame;

    use crate::contract::errors::{ErrorCode, IpcError, IpcResult};

    pub(super) fn supported() -> bool {
        false
    }

    pub(super) fn grab() -> IpcResult<Vec<OwnedFrame>> {
        Err(IpcError::new(ErrorCode::OtpScanUnsupported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_capture_is_unsupported() {
        let none = NoScreenCapture;
        assert!(!none.supported());
        assert_eq!(
            none.capture().unwrap_err().code,
            ErrorCode::OtpScanUnsupported
        );
    }

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn frames_over_the_ceiling_are_refused() {
        assert!(check_size(16_384, 8_192).is_ok());
        assert_eq!(
            check_size(16_385, 8_192).unwrap_err().code,
            ErrorCode::OtpImageTooLarge
        );
    }
}

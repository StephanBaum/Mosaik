//! The system clipboard through `arboard`, shared by the desktop app and the
//! CLI.
//!
//! - Every write is marked so the OS keeps it out of clipboard history and
//!   clipboard managers: on Windows the `ExcludeClipboardContentFromMonitorProcessing`,
//!   `CanIncludeInClipboardHistory = 0` and `CanUploadToCloudClipboard = 0`
//!   formats, on macOS `org.nspasteboard.ConcealedType`, on Linux
//!   `x-kde-passwordManagerHint = secret`.
//! - [`Clipboard::clear_if_matches`] empties the clipboard only if it still
//!   holds the value given (compared in constant time), so something the user
//!   copied since is never touched.
//!
//! Timeouts, worker threads and events are the caller's business (the desktop
//! has its clipboard service, the CLI a helper process). Residual exposure
//! (THREAT_MODEL R19): the OS clipboard and `arboard`'s conversion buffers
//! hold the value until it is cleared or replaced.

use mosaic_core::SecretString;
use subtle::ConstantTimeEq as _;
use zeroize::Zeroizing;

use crate::{PlatformError, Result};

/// What the clipboard holds, as an image.
#[derive(Debug)]
pub enum ImageRead {
    /// No image on the clipboard.
    Empty,
    /// An image the OS could not convert.
    Unreadable,
    /// The image, RGBA with straight alpha, in a zeroizing buffer.
    Rgba {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// `width * height * 4` bytes.
        pixels: Zeroizing<Vec<u8>>,
    },
}

/// A clipboard.
pub trait Clipboard {
    /// Writes `text`, excluded from history and monitoring where supported.
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when the clipboard cannot be written.
    fn set(&mut self, text: &str) -> Result<()>;

    /// The current text, or `None` if the clipboard holds no text.
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when the clipboard cannot be read right
    /// now (for example another process holds it): nothing may be assumed
    /// about its content.
    fn get_text(&mut self) -> Result<Option<SecretString>>;

    /// Empties the clipboard.
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when the clipboard cannot be written.
    fn clear(&mut self) -> Result<()>;

    /// The clipboard image (`arboard` decodes it).
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when the clipboard cannot be read right now.
    fn read_image(&mut self) -> Result<ImageRead>;

    /// Empties the clipboard if it still holds exactly `expected`. Returns
    /// whether it did; content that differs (or is not text) is left alone.
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when it cannot be read or cleared.
    fn clear_if_matches(&mut self, expected: &str) -> Result<bool> {
        let ours = self.get_text()?.is_some_and(|current| {
            bool::from(
                current
                    .expose_secret()
                    .as_bytes()
                    .ct_eq(expected.as_bytes()),
            )
        });
        if !ours {
            return Ok(false);
        }
        self.clear()?;
        Ok(true)
    }
}

/// The system clipboard through `arboard`.
pub struct SystemClipboard(arboard::Clipboard);

impl SystemClipboard {
    /// Opens the system clipboard. On Linux the handle must stay alive to keep
    /// serving what was set.
    ///
    /// # Errors
    /// [`PlatformError::Clipboard`] when there is none (for example no display
    /// server).
    pub fn new() -> Result<Self> {
        arboard::Clipboard::new()
            .map(Self)
            .map_err(|_| PlatformError::Clipboard)
    }
}

impl Clipboard for SystemClipboard {
    fn set(&mut self, text: &str) -> Result<()> {
        let set = self.0.set();
        #[cfg(windows)]
        let set = {
            use arboard::SetExtWindows as _;
            set.exclude_from_monitoring()
                .exclude_from_history()
                .exclude_from_cloud()
        };
        #[cfg(target_os = "macos")]
        let set = {
            use arboard::SetExtApple as _;
            set.exclude_from_history()
        };
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        let set = {
            use arboard::SetExtLinux as _;
            set.exclude_from_history()
        };
        set.text(text).map_err(|_| PlatformError::Clipboard)
    }

    fn get_text(&mut self) -> Result<Option<SecretString>> {
        match self.0.get_text() {
            Ok(text) => Ok(Some(SecretString::from(text))),
            Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => {
                Ok(None)
            }
            Err(_) => Err(PlatformError::Clipboard),
        }
    }

    fn clear(&mut self) -> Result<()> {
        self.0.clear().map_err(|_| PlatformError::Clipboard)
    }

    fn read_image(&mut self) -> Result<ImageRead> {
        match self.0.get_image() {
            Ok(image) => {
                let (Ok(width), Ok(height)) =
                    (u32::try_from(image.width), u32::try_from(image.height))
                else {
                    return Ok(ImageRead::Unreadable);
                };
                Ok(ImageRead::Rgba {
                    width,
                    height,
                    pixels: Zeroizing::new(image.bytes.into_owned()),
                })
            }
            Err(arboard::Error::ContentNotAvailable) => Ok(ImageRead::Empty),
            Err(arboard::Error::ConversionFailure) => Ok(ImageRead::Unreadable),
            Err(_) => Err(PlatformError::Clipboard),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text clipboard in memory, with a switch to fail reads.
    #[derive(Default)]
    struct Memory {
        text: Option<String>,
        fail_reads: bool,
    }

    impl Clipboard for Memory {
        fn set(&mut self, text: &str) -> Result<()> {
            self.text = Some(text.to_owned());
            Ok(())
        }
        fn get_text(&mut self) -> Result<Option<SecretString>> {
            if self.fail_reads {
                return Err(PlatformError::Clipboard);
            }
            Ok(self.text.clone().map(SecretString::from))
        }
        fn clear(&mut self) -> Result<()> {
            self.text = None;
            Ok(())
        }
        fn read_image(&mut self) -> Result<ImageRead> {
            Ok(ImageRead::Empty)
        }
    }

    #[test]
    fn clipboard_clear_if_matches_only_ours() {
        let mut clip = Memory::default();
        assert!(!clip.clear_if_matches("hunter2").unwrap(), "empty");
        clip.set("hunter2").unwrap();
        assert!(clip.clear_if_matches("hunter2").unwrap());
        assert!(clip.text.is_none(), "cleared");

        clip.set("my own text").unwrap();
        assert!(!clip.clear_if_matches("hunter2").unwrap(), "not ours");
        assert_eq!(clip.text.as_deref(), Some("my own text"), "left alone");
        assert!(
            !clip.clear_if_matches("my own").unwrap(),
            "prefix is not equal"
        );
        assert!(!clip.clear_if_matches("my own text!").unwrap());
        assert_eq!(clip.text.as_deref(), Some("my own text"));
    }

    #[test]
    fn an_unreadable_clipboard_is_an_error_and_untouched() {
        let mut clip = Memory::default();
        clip.set("hunter2").unwrap();
        clip.fail_reads = true;
        assert_eq!(
            clip.clear_if_matches("hunter2"),
            Err(PlatformError::Clipboard)
        );
        assert_eq!(clip.text.as_deref(), Some("hunter2"));
    }
}

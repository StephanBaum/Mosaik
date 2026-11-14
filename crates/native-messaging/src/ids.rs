//! Who may talk to the host: the pinned extension IDs and the caller check
//! (`docs/NATIVE_MESSAGING.md` §2).
//!
//! A browser starts the host with its caller's identity in the arguments:
//! Chromium passes `chrome-extension://<id>/` as the first argument (Windows
//! adds `--parent-window=<n>`); Firefox passes the manifest path and the
//! extension's ID. The browser only starts the host for extensions listed in
//! the host manifest, so this check is a second line, not the first.

use std::ffi::OsString;

use crate::bodies::BrowserKind;
use crate::error::Error;

/// The native messaging host name, and the base name of its manifests.
pub const HOST_NAME: &str = "io.github.stephanbaum.mosaic";

/// The Firefox add-on ID (`browser_specific_settings.gecko.id`). The owner
/// confirms it with AMO.
pub const FIREFOX_EXTENSION_ID: &str = "mosaic@stephanbaum.github.io";

/// The Chromium extension ID of unpacked dev builds: derived from the public
/// key in `apps/extension/keys/dev-public-key.b64`, which dev and e2e
/// manifests carry as `key`.
pub const DEV_CHROMIUM_EXTENSION_ID: &str = "lccdgfpfnmegbdgooanfejdbahhokikm";

/// The Chrome Web Store and Edge Add-ons IDs. Empty until the owner publishes
/// the listings.
pub const STORE_CHROMIUM_EXTENSION_IDS: &[&str] = &[];

const CHROMIUM_ORIGIN_PREFIX: &str = "chrome-extension://";

/// Who started the host.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Caller {
    /// A Chromium-family extension.
    Chromium {
        /// The 32-character extension ID.
        extension_id: String,
    },
    /// The Firefox add-on.
    Firefox,
}

impl Caller {
    /// The browser family, as the desktop is told.
    #[must_use]
    pub const fn browser(&self) -> BrowserKind {
        match self {
            Self::Chromium { .. } => BrowserKind::Chromium,
            Self::Firefox => BrowserKind::Firefox,
        }
    }
}

/// Whether the dev extension ID is allowed: in debug builds, and in a release build only with the
/// `preview` feature (ADR-0153). A pure function of the two flags, so a test can cover the
/// release case without a release build.
#[must_use]
const fn dev_id_allowed(debug: bool, preview: bool) -> bool {
    debug || preview
}

/// The allowed Chromium origins, `chrome-extension://<id>/`: the store IDs,
/// plus the dev ID in debug builds and in preview builds (ADR-0153) only.
#[must_use]
pub fn allowed_chromium_origins() -> Vec<String> {
    allowed_origins_for(dev_id_allowed(
        cfg!(debug_assertions),
        cfg!(feature = "preview"),
    ))
}

fn allowed_origins_for(dev_allowed: bool) -> Vec<String> {
    let dev = dev_allowed.then_some(DEV_CHROMIUM_EXTENSION_ID);
    STORE_CHROMIUM_EXTENSION_IDS
        .iter()
        .copied()
        .chain(dev)
        .map(|id| format!("{CHROMIUM_ORIGIN_PREFIX}{id}/"))
        .collect()
}

/// Identifies the caller from the host's full argument vector (`args[0]` is
/// the program).
///
/// - Chromium: `args[1]` must be one of [`allowed_chromium_origins`].
/// - Firefox: `args[2]` must be [`FIREFOX_EXTENSION_ID`] (`args[1]` is the
///   manifest path).
///
/// # Errors
/// [`Error::CallerNotAllowed`] for anything else.
pub fn caller_from_args(args: &[OsString]) -> Result<Caller, Error> {
    if let Some(origin) = args.get(1).and_then(|a| a.to_str())
        && origin.starts_with(CHROMIUM_ORIGIN_PREFIX)
    {
        let id = origin
            .strip_prefix(CHROMIUM_ORIGIN_PREFIX)
            .and_then(|rest| rest.strip_suffix('/'));
        let listed = allowed_chromium_origins().iter().any(|a| a == origin);
        return match id {
            Some(id) if listed => Ok(Caller::Chromium {
                extension_id: id.to_owned(),
            }),
            _ => Err(Error::CallerNotAllowed),
        };
    }
    if args.get(2).and_then(|a| a.to_str()) == Some(FIREFOX_EXTENSION_ID) {
        return Ok(Caller::Firefox);
    }
    Err(Error::CallerNotAllowed)
}

#[cfg(test)]
mod tests {
    use base64ct::{Base64, Encoding};
    use sha2::{Digest, Sha256};

    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    /// SHA-256 of the DER public key, first 16 bytes, every nibble as a
    /// letter `a` to `p`: how Chromium derives an extension ID from `key`.
    fn chromium_id_of(spki_base64: &str) -> String {
        let der = Base64::decode_vec(spki_base64.trim()).unwrap();
        Sha256::digest(&der)[..16]
            .iter()
            .flat_map(|b| [b >> 4, b & 15])
            .map(|n| char::from(b'a' + n))
            .collect()
    }

    #[test]
    fn the_dev_id_is_derived_from_the_committed_dev_key() {
        let key = include_str!("../../../apps/extension/keys/dev-public-key.b64");
        let id = chromium_id_of(key);
        assert_eq!(id.len(), 32);
        assert_eq!(id, DEV_CHROMIUM_EXTENSION_ID);
        assert!(
            DEV_CHROMIUM_EXTENSION_ID
                .bytes()
                .all(|b| (b'a'..=b'p').contains(&b))
        );
        // An RSA-2048 SPKI: 294 bytes of DER.
        assert_eq!(Base64::decode_vec(key.trim()).unwrap().len(), 294);
    }

    #[test]
    fn chromium_callers() {
        let origin = format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}/");
        let ok = caller_from_args(&args(&["host", &origin]));
        if cfg!(debug_assertions) || cfg!(feature = "preview") {
            assert_eq!(
                ok,
                Ok(Caller::Chromium {
                    extension_id: DEV_CHROMIUM_EXTENSION_ID.to_owned()
                })
            );
            // Windows adds the parent window handle.
            assert!(caller_from_args(&args(&["host", &origin, "--parent-window=123"])).is_ok());
        } else {
            assert_eq!(
                ok,
                Err(Error::CallerNotAllowed),
                "dev ID is debug or preview only"
            );
        }
        for bad in [
            "chrome-extension://abcdefghijklmnopabcdefghijklmnop/",
            "chrome-extension:///",
            "chrome-extension://",
            &format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}"),
            &format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}/x"),
            &format!("CHROME-EXTENSION://{DEV_CHROMIUM_EXTENSION_ID}/"),
            &format!("moz-extension://{DEV_CHROMIUM_EXTENSION_ID}/"),
        ] {
            assert_eq!(
                caller_from_args(&args(&["host", bad])),
                Err(Error::CallerNotAllowed),
                "{bad}"
            );
        }
    }

    #[test]
    fn firefox_callers() {
        assert_eq!(
            caller_from_args(&args(&[
                "host",
                "C:\\m\\manifest.json",
                FIREFOX_EXTENSION_ID
            ])),
            Ok(Caller::Firefox)
        );
        for bad in [
            args(&["host", "manifest.json", "other@example.org"]),
            args(&["host", "manifest.json"]),
            args(&["host", FIREFOX_EXTENSION_ID]),
            args(&["host", FIREFOX_EXTENSION_ID, "manifest.json"]),
            args(&["host", "manifest.json", "MOSAIC@stephanbaum.github.io"]),
        ] {
            assert_eq!(
                caller_from_args(&bad),
                Err(Error::CallerNotAllowed),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn garbage_is_refused() {
        assert_eq!(caller_from_args(&[]), Err(Error::CallerNotAllowed));
        assert_eq!(
            caller_from_args(&args(&["host"])),
            Err(Error::CallerNotAllowed)
        );
        assert_eq!(
            caller_from_args(&args(&["host", "", ""])),
            Err(Error::CallerNotAllowed)
        );
        assert_eq!(
            caller_from_args(&args(&["host", "--help"])),
            Err(Error::CallerNotAllowed)
        );
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let junk = vec![OsString::from("host"), OsString::from_vec(vec![0xff, 0xfe])];
            assert_eq!(caller_from_args(&junk), Err(Error::CallerNotAllowed));
        }
    }

    #[test]
    fn allowed_origins_end_with_a_slash_and_follow_the_build() {
        let origins = allowed_chromium_origins();
        assert!(
            origins
                .iter()
                .all(|o| o.starts_with("chrome-extension://") && o.ends_with('/'))
        );
        assert_eq!(
            origins.contains(&format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}/")),
            cfg!(debug_assertions) || cfg!(feature = "preview")
        );
        assert_eq!(
            origins.len(),
            STORE_CHROMIUM_EXTENSION_IDS.len()
                + usize::from(cfg!(debug_assertions) || cfg!(feature = "preview"))
        );
    }

    #[test]
    fn the_dev_id_needs_a_debug_or_preview_build() {
        let dev = format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}/");
        // The release case: no debug assertions, no preview feature. The dev ID is absent.
        assert!(!dev_id_allowed(false, false));
        assert!(!allowed_origins_for(dev_id_allowed(false, false)).contains(&dev));
        // Debug builds and preview builds (ADR-0153) accept it.
        for (debug, preview) in [(true, false), (false, true), (true, true)] {
            assert!(dev_id_allowed(debug, preview));
            assert!(allowed_origins_for(dev_id_allowed(debug, preview)).contains(&dev));
        }
        // Only the dev ID differs.
        assert_eq!(
            allowed_origins_for(true).len(),
            allowed_origins_for(false).len() + 1
        );
    }
}

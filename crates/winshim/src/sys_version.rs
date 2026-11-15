//! The product name of a program image, read from its version resource
//! (ADR-0164). It is display data and never an identity: program fill uses it
//! only to refuse a browser that was renamed to another file name.
#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]

use core::ffi::c_void;

use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::core::PCWSTR;

use crate::error::{Result, ShimError, utf16, wide_nul};
use crate::sys_process::ImageFile;

/// The largest version resource read; a larger one is treated as absent.
const MAX_VERSION_BYTES: u32 = 256 * 1024;
/// The longest product name accepted, in UTF-16 units.
const MAX_PRODUCT_UNITS: usize = 256;
/// The most language and code page pairs tried.
const MAX_TRANSLATIONS: usize = 4;

impl ImageFile {
    /// The `ProductName` of the image's version resource, `None` when it has
    /// none. The image is held open without write or delete sharing, so the
    /// file read by its final path is this file.
    ///
    /// # Errors
    /// An OS error for a path with a NUL or a name that is not valid UTF-16.
    pub fn product_name(&self) -> Result<Option<String>> {
        let path = wide_nul(self.final_path())?;
        // SAFETY: `path` is a NUL-terminated UTF-16 string that outlives the
        // call; no handle out parameter is requested.
        let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(path.as_ptr()), None) };
        if size == 0 || size > MAX_VERSION_BYTES {
            return Ok(None);
        }
        let mut block = vec![0u8; usize::try_from(size).map_err(|_| ShimError::Os(0))?];
        // SAFETY: `path` as above; `block` is a writable buffer of exactly
        // `size` bytes, the length passed.
        let read = unsafe {
            GetFileVersionInfoW(
                PCWSTR(path.as_ptr()),
                None,
                size,
                block.as_mut_ptr().cast::<c_void>(),
            )
        };
        if read.is_err() {
            return Ok(None);
        }
        let Some(translations) = query(&block, "\\VarFileInfo\\Translation", 1) else {
            return Ok(None);
        };
        let (pairs, _) = translations.as_chunks::<4>();
        for &[lang0, lang1, page0, page1] in pairs.iter().take(MAX_TRANSLATIONS) {
            let sub = format!(
                "\\StringFileInfo\\{:04x}{:04x}\\ProductName",
                u16::from_le_bytes([lang0, lang1]),
                u16::from_le_bytes([page0, page1])
            );
            if let Some(bytes) = query(&block, &sub, 2) {
                let mut units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&pair| u16::from_le_bytes(pair))
                    .collect();
                while units.last() == Some(&0) {
                    units.pop();
                }
                if units.is_empty() || units.len() > MAX_PRODUCT_UNITS {
                    continue;
                }
                return utf16(&units).map(Some);
            }
        }
        Ok(None)
    }
}

/// The bytes of the value `sub` inside the version `block`. `unit` is the size
/// of the length `VerQueryValueW` reports (bytes for binary values, UTF-16
/// units for strings). The answer is a view into `block`, checked to lie
/// inside it.
fn query<'a>(block: &'a [u8], sub: &str, unit: usize) -> Option<&'a [u8]> {
    let sub = wide_nul(sub).ok()?;
    let mut found: *mut c_void = core::ptr::null_mut();
    let mut length = 0u32;
    // SAFETY: `block` is a version resource filled by `GetFileVersionInfoW`;
    // `sub` is a NUL-terminated UTF-16 string; both out pointers are valid and
    // writable. The returned pointer is only compared with `block`'s range
    // below and never dereferenced here.
    let ok = unsafe {
        VerQueryValueW(
            block.as_ptr().cast::<c_void>(),
            PCWSTR(sub.as_ptr()),
            &raw mut found,
            &raw mut length,
        )
    };
    if !ok.as_bool() || found.is_null() {
        return None;
    }
    let offset = found.addr().checked_sub(block.as_ptr().addr())?;
    let len = usize::try_from(length).ok()?.checked_mul(unit)?;
    block.get(offset..offset.checked_add(len)?)
}

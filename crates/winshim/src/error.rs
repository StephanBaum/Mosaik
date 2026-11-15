//! The shim's error type.

/// Why a Windows call failed. Carries no data from the target program.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShimError {
    /// The OS refused access (the process or file is protected).
    #[error("access denied")]
    AccessDenied,
    /// The process, window, file or element does not exist (any more).
    #[error("not found")]
    NotFound,
    /// A UI Automation provider did not answer in time.
    #[error("timeout")]
    Timeout,
    /// Any other OS error: the `HRESULT` or Win32 code, `0` for a malformed
    /// answer (such as invalid UTF-16).
    #[error("os error {0:#x}")]
    Os(i32),
}

/// The shim's result type.
pub type Result<T> = core::result::Result<T, ShimError>;

const UIA_E_TIMEOUT: u32 = 0x8013_1505;
const UIA_E_ELEMENTNOTAVAILABLE: u32 = 0x8004_0201;

impl ShimError {
    /// Maps a Win32 error code (`ERROR_*`).
    pub(crate) fn from_win32(code: u32) -> Self {
        match code {
            5 => Self::AccessDenied,
            // ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_INVALID_PARAMETER, ERROR_NOT_FOUND
            2 | 3 | 87 | 1168 => Self::NotFound,
            other => Self::Os(i32::try_from(other).unwrap_or(i32::MAX)),
        }
    }

    /// Maps an `HRESULT`, unwrapping `HRESULT_FROM_WIN32`.
    pub(crate) fn from_hresult(hr: i32) -> Self {
        let bits = hr.cast_unsigned();
        if bits == UIA_E_TIMEOUT {
            return Self::Timeout;
        }
        if bits == UIA_E_ELEMENTNOTAVAILABLE {
            return Self::NotFound;
        }
        if bits >> 16 == 0x8007 {
            return Self::from_win32(bits & 0xFFFF);
        }
        Self::Os(hr)
    }
}

impl From<windows::core::Error> for ShimError {
    fn from(err: windows::core::Error) -> Self {
        Self::from_hresult(err.code().0)
    }
}

/// Decodes UTF-16 strictly.
pub(crate) fn utf16(units: &[u16]) -> Result<String> {
    String::from_utf16(units).map_err(|_| ShimError::Os(0))
}

/// A NUL-terminated UTF-16 copy of `s`; an embedded NUL is refused.
pub(crate) fn wide_nul(s: &str) -> Result<Vec<u16>> {
    if s.contains('\0') {
        return Err(ShimError::NotFound);
    }
    Ok(s.encode_utf16().chain(core::iter::once(0)).collect())
}

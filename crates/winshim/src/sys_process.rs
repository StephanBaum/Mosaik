//! Process identity: a held process handle, its image path, package, integrity
//! level, and the final form of a file path.
#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]

use core::ffi::c_void;
use std::fs::File;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, FILETIME, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GETFINALPATHNAMEBYHANDLE_FLAGS,
    GetDriveTypeW, GetFinalPathNameByHandleW, OPEN_EXISTING, VOLUME_NAME_DOS,
};
use windows::Win32::Storage::Packaging::Appx::GetPackageFamilyName;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
    WaitForSingleObject,
};
use windows::core::{PCWSTR, PWSTR};

use crate::error::{Result, ShimError, utf16, wide_nul};

/// First buffer for paths and names; one retry with [`LONG_BUFFER`].
const SHORT_BUFFER: usize = 1024;
/// The longest path Windows allows, in UTF-16 units.
const LONG_BUFFER: usize = 32_768;
/// `GetDriveTypeW` result for a network drive (`DRIVE_REMOTE`).
const DRIVE_REMOTE: u32 = 4;
/// `APPMODEL_ERROR_NO_PACKAGE`.
const NO_PACKAGE: u32 = 15_700;
/// `ERROR_INSUFFICIENT_BUFFER` as a [`ShimError::Os`] code.
const INSUFFICIENT_BUFFER: i32 = 122;

/// An owned kernel handle, closed on drop.
pub(crate) struct OwnedHandle(pub(crate) HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid handle this wrapper owns exclusively (it
        // came from a successful open call and is never copied out), and drop
        // runs once, so it is closed exactly once.
        let closed = unsafe { CloseHandle(self.0) };
        // Nothing useful can be done with a failed close.
        drop(closed);
    }
}

/// An open process handle (`PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`),
/// closed on drop. Holding it keeps the process object alive, so its PID
/// cannot be taken over by another process while the handle is open.
pub struct ProcessHandle {
    handle: OwnedHandle,
    pid: u32,
}

impl core::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl ProcessHandle {
    /// Opens the process `pid`.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] for no such process, [`ShimError::AccessDenied`]
    /// for a protected one.
    pub fn open(pid: u32) -> Result<Self> {
        // SAFETY: plain value arguments; the returned handle is owned by the
        // wrapper below and closed on drop.
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }?;
        Ok(Self {
            handle: OwnedHandle(handle),
            pid,
        })
    }

    /// The process ID this handle was opened for.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Whether the process has not exited yet.
    ///
    /// # Errors
    /// An OS error if the wait fails.
    pub fn is_running(&self) -> Result<bool> {
        // SAFETY: the handle is valid and has SYNCHRONIZE access; a zero
        // timeout makes the call non-blocking.
        let state = unsafe { WaitForSingleObject(self.handle.0, 0) };
        if state == WAIT_TIMEOUT {
            Ok(true)
        } else if state == WAIT_OBJECT_0 {
            Ok(false)
        } else {
            Err(last_error())
        }
    }

    /// When the process was created, as 100 ns ticks since 1601 (a `FILETIME`).
    /// A parent that started after its child is a reused process ID, not the
    /// parent.
    ///
    /// # Errors
    /// An OS error.
    pub fn start_time(&self) -> Result<u64> {
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        // SAFETY: the handle is valid and has query access; the four out
        // pointers are valid, writable `FILETIME`s.
        unsafe {
            GetProcessTimes(
                self.handle.0,
                &raw mut created,
                &raw mut exited,
                &raw mut kernel,
                &raw mut user,
            )
        }?;
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }

    /// The image path in Win32 form (`C:\...`); not yet resolved to its final
    /// form (see [`final_path`]).
    ///
    /// # Errors
    /// An OS error, or [`ShimError::Os`] with `0` for invalid UTF-16.
    pub fn image_path(&self) -> Result<String> {
        match self.image_path_with(SHORT_BUFFER) {
            Err(ShimError::Os(code)) if code == INSUFFICIENT_BUFFER => {
                self.image_path_with(LONG_BUFFER)
            }
            other => other,
        }
    }

    fn image_path_with(&self, capacity: usize) -> Result<String> {
        let mut buffer = vec![0u16; capacity];
        let mut size = u32::try_from(capacity).map_err(|_| ShimError::Os(0))?;
        // SAFETY: the handle is valid; `buffer` holds `size` UTF-16 units and
        // outlives the call; `size` is a valid out pointer to its length.
        unsafe {
            QueryFullProcessImageNameW(
                self.handle.0,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &raw mut size,
            )
        }?;
        let len = usize::try_from(size).map_err(|_| ShimError::Os(0))?;
        utf16(buffer.get(..len).ok_or(ShimError::Os(0))?)
    }

    /// The package family name of a packaged app, `None` for an unpackaged
    /// program.
    ///
    /// # Errors
    /// An OS error, or [`ShimError::Os`] with `0` for invalid UTF-16.
    pub fn package_family_name(&self) -> Result<Option<String>> {
        let mut length = 0u32;
        // SAFETY: the handle is valid; `length` is a valid out pointer; no
        // buffer is passed, so this only asks for the required length.
        let first = unsafe { GetPackageFamilyName(self.handle.0, &raw mut length, None) };
        if first.0 == NO_PACKAGE {
            return Ok(None);
        }
        if first != ERROR_INSUFFICIENT_BUFFER {
            return Err(ShimError::from_win32(first.0));
        }
        let mut buffer = vec![0u16; usize::try_from(length).map_err(|_| ShimError::Os(0))?];
        // SAFETY: the handle is valid; `buffer` holds `length` UTF-16 units
        // and outlives the call; `length` is a valid in/out pointer.
        let second = unsafe {
            GetPackageFamilyName(
                self.handle.0,
                &raw mut length,
                Some(PWSTR(buffer.as_mut_ptr())),
            )
        };
        if second.0 != 0 {
            return Err(ShimError::from_win32(second.0));
        }
        // `length` includes the terminating NUL.
        let end = buffer.iter().position(|u| *u == 0).unwrap_or(buffer.len());
        Ok(Some(utf16(buffer.get(..end).ok_or(ShimError::Os(0))?)?))
    }

    /// The RID of the token's mandatory integrity level (`0x1000` low,
    /// `0x2000` medium, `0x3000` high, `0x4000` system).
    ///
    /// # Errors
    /// An OS error.
    pub fn integrity_rid(&self) -> Result<u32> {
        integrity_of(self.handle.0)
    }
}

/// The integrity level RID of Mosaic's own process.
///
/// # Errors
/// An OS error.
pub fn current_integrity_rid() -> Result<u32> {
    // SAFETY: returns the always-valid pseudo handle of this process; it must
    // not be closed, and it is not wrapped.
    let me = unsafe { GetCurrentProcess() };
    integrity_of(me)
}

fn integrity_of(process: HANDLE) -> Result<u32> {
    let mut token = HANDLE::default();
    // SAFETY: `process` is a valid process handle with query access; `token`
    // is a valid out pointer.
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) }?;
    let token = OwnedHandle(token);
    let mut needed = 0u32;
    // SAFETY: the token handle is valid; a null buffer of length 0 only asks
    // for the required length through `needed`, a valid out pointer.
    let probe =
        unsafe { GetTokenInformation(token.0, TokenIntegrityLevel, None, 0, &raw mut needed) };
    match probe {
        Err(err) if err.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => {}
        Err(err) => return Err(err.into()),
        Ok(()) => return Err(ShimError::Os(0)),
    }
    // u64 elements give the buffer the alignment the label structure needs.
    let words = usize::try_from(needed)
        .map_err(|_| ShimError::Os(0))?
        .div_ceil(8);
    let mut buffer = vec![0u64; words];
    let bytes = u32::try_from(words.saturating_mul(8)).map_err(|_| ShimError::Os(0))?;
    // SAFETY: the token handle is valid; `buffer` is writable for `bytes`
    // bytes and outlives the call; `needed` is a valid out pointer.
    unsafe {
        GetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            bytes,
            &raw mut needed,
        )
    }?;
    if usize::try_from(needed).map_err(|_| ShimError::Os(0))? < size_of::<TOKEN_MANDATORY_LABEL>() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: the buffer is 8-aligned and holds at least a
    // `TOKEN_MANDATORY_LABEL` written by the call above; the read copies the
    // plain-data struct out.
    let label = unsafe { buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>().read() };
    let sid = label.Label.Sid;
    // SAFETY: `sid` points into `buffer`, which is alive, and is a valid SID
    // the call above wrote.
    let count = unsafe { GetSidSubAuthorityCount(sid) };
    // SAFETY: `count` is the non-null pointer the call above returned for the
    // valid SID, and points into `buffer`.
    let count = unsafe { count.read() };
    if count == 0 {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `count - 1` is below the SID's sub-authority count.
    let rid = unsafe { GetSidSubAuthority(sid, u32::from(count - 1)) };
    // SAFETY: `rid` is the non-null pointer the call above returned into the
    // valid SID in `buffer`.
    Ok(unsafe { rid.read() })
}

/// The final form of `path`: symbolic links, `..`, 8.3 names and substituted
/// drives resolved, with the `\\?\` prefix removed (`\\?\UNC\` becomes `\\`).
///
/// # Errors
/// [`ShimError::NotFound`] if the file cannot be opened, or an OS error.
pub fn final_path(path: &str) -> Result<String> {
    let wide = wide_nul(path)?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call;
    // no security attributes or template are passed. The handle is owned by
    // the wrapper below.
    let file = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }?;
    let file = OwnedHandle(file);
    final_path_of_handle(file.0)
}

/// The final path of the file a handle refers to.
fn final_path_of_handle(file: HANDLE) -> Result<String> {
    let mut buffer = vec![0u16; SHORT_BUFFER];
    let mut len = final_path_call(file, &mut buffer)?;
    if len > buffer.len() {
        buffer = vec![0u16; len];
        len = final_path_call(file, &mut buffer)?;
    }
    let text = utf16(buffer.get(..len).ok_or(ShimError::Os(0))?)?;
    Ok(strip_device_prefix(&text))
}

/// A program image held open, once, for every check of it (signature, hash):
/// opened for reading with sharing for reads only, so while it is held the file
/// cannot be renamed, deleted or written, and what is verified is what is
/// hashed. Closed on drop.
#[derive(Debug)]
pub struct ImageFile {
    file: File,
    final_path: String,
}

impl ImageFile {
    /// Opens the file at `path` and reads its final path through the handle.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] if it cannot be opened, or an OS error (a file
    /// that is open for writing elsewhere cannot be opened without sharing).
    pub fn open(path: &str) -> Result<Self> {
        if path.contains('\0') {
            return Err(ShimError::NotFound);
        }
        let file = File::options()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(path)
            .map_err(|err| match err.raw_os_error() {
                Some(code) => ShimError::from_win32(code.cast_unsigned()),
                None => ShimError::NotFound,
            })?;
        let final_path = final_path_of_handle(HANDLE(file.as_raw_handle()))?;
        Ok(Self { file, final_path })
    }

    /// The final path of the open file (no `\\?\` prefix).
    #[must_use]
    pub fn final_path(&self) -> &str {
        &self.final_path
    }

    /// The open file, for reading (seek to the start before hashing: the
    /// position is shared with the signature check).
    #[must_use]
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Whether the file is on a network path: a UNC path, or a drive the OS
    /// reports as remote.
    ///
    /// # Errors
    /// [`ShimError::Os`] with `0` for a path without a drive.
    pub fn is_network(&self) -> Result<bool> {
        if is_unc_path(&self.final_path) {
            return Ok(true);
        }
        let root = drive_root(&self.final_path).ok_or(ShimError::Os(0))?;
        let wide = wide_nul(&root)?;
        // SAFETY: `wide` is a NUL-terminated UTF-16 root path that outlives the
        // call.
        let kind = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
        Ok(kind == DRIVE_REMOTE)
    }

    /// Whether the image is the file the process runs: its path is read again
    /// from the process now and must resolve to this handle's final path.
    fn is_image_of(&self, process: &ProcessHandle) -> Result<bool> {
        let again = final_path(&process.image_path()?)?;
        Ok(again.eq_ignore_ascii_case(&self.final_path))
    }
}

/// `\\server\share\...`, the form [`final_path`] gives for a network path.
pub(crate) fn is_unc_path(path: &str) -> bool {
    path.starts_with(r"\\")
}

/// `C:\` for `C:\dir\x.exe`.
pub(crate) fn drive_root(path: &str) -> Option<String> {
    let mut chars = path.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next() == Some(':') && chars.next() == Some('\\')).then(|| format!(r"{letter}:\"))
}

impl ProcessHandle {
    /// Opens the image the process runs, once, for the signature and hash
    /// checks. The process's image path is read again after the open and must
    /// resolve to the open file, so a path swapped in between is not accepted.
    ///
    /// # Errors
    /// [`ShimError::NetworkImage`] for an image on a network path,
    /// [`ShimError::ImageChanged`] if the path no longer resolves to the open
    /// file, or an OS error.
    pub fn open_image(&self) -> Result<ImageFile> {
        let image = ImageFile::open(&self.image_path()?)?;
        if image.is_network()? {
            return Err(ShimError::NetworkImage);
        }
        if !image.is_image_of(self)? {
            return Err(ShimError::ImageChanged);
        }
        Ok(image)
    }
}

fn final_path_call(file: HANDLE, buffer: &mut [u16]) -> Result<usize> {
    // SAFETY: the file handle is valid (the caller holds the file open for the
    // call); `buffer` is a writable slice that outlives the call, and the API
    // receives its length.
    let len = unsafe {
        GetFinalPathNameByHandleW(
            file,
            buffer,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    };
    if len == 0 {
        return Err(last_error());
    }
    usize::try_from(len).map_err(|_| ShimError::Os(0))
}

/// `\\?\C:\x` becomes `C:\x`, `\\?\UNC\srv\share` becomes `\\srv\share`.
fn strip_device_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_owned()
    } else {
        path.to_owned()
    }
}

/// The parent process ID recorded for `pid`, `None` if it has none or the
/// process is not in the snapshot. The ID is a hint, not an identity: the
/// parent may have exited and its ID been reused, so a caller must verify the
/// parent (image, signer) before trusting it.
///
/// # Errors
/// An OS error if the process snapshot cannot be taken.
pub fn parent_pid(pid: u32) -> Result<Option<u32>> {
    // SAFETY: plain value arguments; the snapshot handle is owned by the
    // wrapper below and closed on drop.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }?;
    let snapshot = OwnedHandle(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).map_err(|_| ShimError::Os(0))?,
        ..PROCESSENTRY32W::default()
    };
    // SAFETY: the snapshot handle is valid; `entry` is a valid out structure
    // with `dwSize` set.
    let mut more = unsafe { Process32FirstW(snapshot.0, &raw mut entry) }.is_ok();
    while more {
        if entry.th32ProcessID == pid {
            return Ok(match entry.th32ParentProcessID {
                0 => None,
                parent => Some(parent),
            });
        }
        // SAFETY: as for `Process32FirstW` above.
        more = unsafe { Process32NextW(snapshot.0, &raw mut entry) }.is_ok();
    }
    Ok(None)
}

/// `pid`, then its recorded parent, then that one's parent, and so on, up to
/// `max_depth` entries; stops at a process without a parent, a repeated ID
/// (a cycle from ID reuse) or a failed lookup. Every entry after the first is a
/// hint, see [`parent_pid`].
///
/// # Errors
/// An OS error if the very first lookup fails.
pub fn process_chain(pid: u32, max_depth: usize) -> Result<Vec<u32>> {
    let mut chain = vec![pid];
    let mut current = pid;
    while chain.len() < max_depth {
        match parent_pid(current) {
            Ok(Some(parent)) if !chain.contains(&parent) => {
                chain.push(parent);
                current = parent;
            }
            Err(err) if chain.len() == 1 => return Err(err),
            _ => break,
        }
    }
    Ok(chain)
}

/// The Win32 error of the calling thread, as a [`ShimError`].
fn last_error() -> ShimError {
    ShimError::from(windows::core::Error::from_thread())
}

//! Authenticode: the verdict of `WinVerifyTrust` for an embedded or catalog
//! signature, and the leaf signer's subject (ADR-0157 D4).
#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]

use core::ffi::c_void;

use windows::Win32::Foundation::{
    CERT_E_REVOKED, HANDLE, HWND, TRUST_E_NOSIGNATURE, TRUST_E_PROVIDER_UNKNOWN,
    TRUST_E_SUBJECT_FORM_UNKNOWN,
};
use windows::Win32::Security::Cryptography::Catalog::{
    CATALOG_INFO, CryptCATAdminAcquireContext2, CryptCATAdminCalcHashFromFileHandle2,
    CryptCATAdminEnumCatalogFromHash, CryptCATAdminReleaseCatalogContext,
    CryptCATAdminReleaseContext, CryptCATCatalogInfoFromContext,
};
use windows::Win32::Security::Cryptography::{
    CERT_CONTEXT, CERT_NAME_ATTR_TYPE, CertGetNameStringW,
};
use windows::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_CATALOG_INFO, WINTRUST_DATA, WINTRUST_FILE_INFO,
    WTD_CHOICE_CATALOG, WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
    WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::core::{PCSTR, PCWSTR};

use crate::error::{Result, ShimError, wide_nul};
use crate::sys_process::OwnedHandle;

/// The most bytes read from any signer name or subject; a longer one is an
/// error, not a truncation.
const MAX_NAME_UNITS: usize = 1024;
const MAX_SUBJECT_BYTES: usize = 8 * 1024;

/// The Authenticode state of a file (ADR-0157 D4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Signature {
    /// A valid embedded or catalog signature that chains to a trusted root.
    Trusted {
        /// The DER subject of the leaf signing certificate.
        subject_der: Vec<u8>,
        /// The signer's common name, for display.
        common_name: Option<String>,
        /// The signer's organisation, for display.
        organization: Option<String>,
    },
    /// No signature, embedded or in a catalog.
    Unsigned,
    /// A signature that does not verify (bad digest, untrusted root, expired,
    /// a revocation check that failed).
    Untrusted,
    /// A revoked signing certificate.
    Revoked,
}

/// How one `WinVerifyTrust` attempt ended.
enum Verdict {
    Trusted(Signer),
    NoSignature,
    Revoked,
    Untrusted,
}

struct Signer {
    subject_der: Vec<u8>,
    common_name: Option<String>,
    organization: Option<String>,
}

/// The Authenticode state of the file at `path`: the embedded signature first,
/// then a SHA-256 and a SHA-1 catalog.
///
/// # Errors
/// [`ShimError::NotFound`] if the file cannot be opened, or an OS error.
pub fn verify_authenticode(path: &str) -> Result<Signature> {
    let wide = wide_nul(path)?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call;
    // no security attributes or template are passed. The handle is owned by
    // the wrapper.
    let file = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;
    let file = OwnedHandle(file);
    match verify_file(&wide, &file)? {
        Verdict::NoSignature => {}
        other => return Ok(signature_of(other)),
    }
    for algorithm in ["SHA256", "SHA1"] {
        if let Some(verdict) = verify_catalog(&wide, &file, algorithm)? {
            return Ok(signature_of(verdict));
        }
    }
    Ok(Signature::Unsigned)
}

fn signature_of(verdict: Verdict) -> Signature {
    match verdict {
        Verdict::Trusted(signer) => Signature::Trusted {
            subject_der: signer.subject_der,
            common_name: signer.common_name,
            organization: signer.organization,
        },
        Verdict::NoSignature => Signature::Unsigned,
        Verdict::Revoked => Signature::Revoked,
        Verdict::Untrusted => Signature::Untrusted,
    }
}

/// `WINTRUST_DATA` for the checks of ADR-0157 D4: no UI, whole-chain
/// revocation excluding the root.
fn base_data() -> WINTRUST_DATA {
    WINTRUST_DATA {
        cbStruct: size_u32::<WINTRUST_DATA>(),
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        ..WINTRUST_DATA::default()
    }
}

fn size_u32<T>() -> u32 {
    u32::try_from(size_of::<T>()).unwrap_or(u32::MAX)
}

/// Verifies the embedded signature of the file.
fn verify_file(wide: &[u16], file: &OwnedHandle) -> Result<Verdict> {
    let mut info = WINTRUST_FILE_INFO {
        cbStruct: size_u32::<WINTRUST_FILE_INFO>(),
        pcwszFilePath: PCWSTR(wide.as_ptr()),
        hFile: file.0,
        ..WINTRUST_FILE_INFO::default()
    };
    let mut data = base_data();
    data.dwUnionChoice = WTD_CHOICE_FILE;
    data.Anonymous.pFile = &raw mut info;
    run_verify(&mut data)
}

/// Verifies the file against a catalog of `algorithm`; `None` if no catalog
/// lists its hash.
fn verify_catalog(wide: &[u16], file: &OwnedHandle, algorithm: &str) -> Result<Option<Verdict>> {
    let Some(admin) = CatAdmin::acquire(algorithm)? else {
        return Ok(None);
    };
    let hash = admin.hash_of(file)?;
    let Some(catalog) = admin.catalog_of(&hash) else {
        return Ok(None);
    };
    let tag: Vec<u16> = hash
        .iter()
        .flat_map(|b| format!("{b:02X}").encode_utf16().collect::<Vec<u16>>())
        .chain(core::iter::once(0))
        .collect();
    let mut hash_copy = hash.clone();
    let mut info = WINTRUST_CATALOG_INFO {
        cbStruct: size_u32::<WINTRUST_CATALOG_INFO>(),
        pcwszCatalogFilePath: PCWSTR(catalog.file.as_ptr()),
        pcwszMemberTag: PCWSTR(tag.as_ptr()),
        pcwszMemberFilePath: PCWSTR(wide.as_ptr()),
        hMemberFile: file.0,
        pbCalculatedFileHash: hash_copy.as_mut_ptr(),
        cbCalculatedFileHash: u32::try_from(hash_copy.len()).map_err(|_| ShimError::Os(0))?,
        hCatAdmin: admin.handle,
        ..WINTRUST_CATALOG_INFO::default()
    };
    let mut data = base_data();
    data.dwUnionChoice = WTD_CHOICE_CATALOG;
    data.Anonymous.pCatalog = &raw mut info;
    run_verify(&mut data).map(Some)
}

/// Runs `WinVerifyTrust` (verify, read the signer, then always close).
fn run_verify(data: &mut WINTRUST_DATA) -> Result<Verdict> {
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    // SAFETY: `HWND::default()` is the "no parent window" value for the
    // no-UI choice; `action` and `data` are valid, exclusively borrowed and
    // outlive the call, and `data` points at a live file or catalog info.
    let result = unsafe {
        WinVerifyTrust(
            HWND::default(),
            &raw mut action,
            core::ptr::from_mut(data).cast::<c_void>(),
        )
    };
    let verdict = if result == 0 {
        read_signer(data.hWVTStateData).map(Verdict::Trusted)
    } else {
        Ok(verdict_of(result))
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: as above; this call releases the state data the verify call
    // opened, and it is made on every path after that call.
    let closed = unsafe {
        WinVerifyTrust(
            HWND::default(),
            &raw mut action,
            core::ptr::from_mut(data).cast::<c_void>(),
        )
    };
    // The close result carries no verdict.
    let _ = closed;
    verdict
}

fn verdict_of(result: i32) -> Verdict {
    let hr = windows::core::HRESULT(result);
    if hr == TRUST_E_NOSIGNATURE
        || hr == TRUST_E_SUBJECT_FORM_UNKNOWN
        || hr == TRUST_E_PROVIDER_UNKNOWN
    {
        Verdict::NoSignature
    } else if hr == CERT_E_REVOKED {
        Verdict::Revoked
    } else {
        Verdict::Untrusted
    }
}

/// Reads the leaf signer from the `WinVerifyTrust` state data.
fn read_signer(state: HANDLE) -> Result<Signer> {
    // SAFETY: `state` is the state handle of a verify call that returned
    // success and has not been closed yet.
    let provider = unsafe { WTHelperProvDataFromStateData(state) };
    if provider.is_null() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `provider` is the non-null provider data of the open state;
    // signer 0, no counter signer.
    let signer = unsafe { WTHelperGetProvSignerFromChain(provider, 0, false, 0) };
    if signer.is_null() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `signer` is non-null and points at the signer structure owned
    // by the open state; the read copies a field.
    let chain_len = unsafe { (*signer).csCertChain };
    // SAFETY: as above.
    let chain = unsafe { (*signer).pasCertChain };
    if chain_len == 0 || chain.is_null() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `chain` is non-null with `chain_len >= 1` entries; entry 0 is
    // the leaf; the read copies a pointer.
    let cert: *const CERT_CONTEXT = unsafe { (*chain).pCert };
    if cert.is_null() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `cert` is a non-null certificate context owned by the state.
    let info = unsafe { (*cert).pCertInfo };
    if info.is_null() {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `info` is non-null and belongs to `cert`; the read copies the
    // plain-data blob descriptor.
    let blob = unsafe { (*info).Subject };
    let len = usize::try_from(blob.cbData).map_err(|_| ShimError::Os(0))?;
    if blob.pbData.is_null() || len == 0 || len > MAX_SUBJECT_BYTES {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `pbData` is non-null and valid for `cbData` bytes while the
    // state is open; the bytes are copied before the state closes.
    let subject_der = unsafe { core::slice::from_raw_parts(blob.pbData, len) }.to_vec();
    Ok(Signer {
        subject_der,
        common_name: name_attribute(cert, c"2.5.4.3")?,
        organization: name_attribute(cert, c"2.5.4.10")?,
    })
}

/// A subject attribute of the certificate by OID, `None` if it has none.
fn name_attribute(cert: *const CERT_CONTEXT, oid: &core::ffi::CStr) -> Result<Option<String>> {
    let oid = PCSTR(oid.as_ptr().cast::<u8>());
    // SAFETY: `cert` is a valid certificate context of the open state; `oid`
    // is a NUL-terminated OID string with static lifetime; no buffer is
    // passed, so this only asks for the length.
    let needed = unsafe {
        CertGetNameStringW(
            cert,
            CERT_NAME_ATTR_TYPE,
            0,
            Some(oid.0.cast::<c_void>()),
            None,
        )
    };
    let len = usize::try_from(needed).map_err(|_| ShimError::Os(0))?;
    if len <= 1 {
        return Ok(None);
    }
    if len > MAX_NAME_UNITS {
        return Err(ShimError::Os(0));
    }
    let mut buffer = vec![0u16; len];
    // SAFETY: as above; `buffer` is writable for `len` units and the API
    // receives its length.
    let written = unsafe {
        CertGetNameStringW(
            cert,
            CERT_NAME_ATTR_TYPE,
            0,
            Some(oid.0.cast::<c_void>()),
            Some(&mut buffer),
        )
    };
    let written = usize::try_from(written).map_err(|_| ShimError::Os(0))?;
    // `written` counts the terminating NUL.
    let text = buffer
        .get(..written.saturating_sub(1))
        .ok_or(ShimError::Os(0))?;
    crate::error::utf16(text).map(Some)
}

/// A catalog administrator context, released on drop.
struct CatAdmin {
    handle: isize,
}

/// A catalog that lists a file hash, released on drop.
struct Catalog<'a> {
    admin: &'a CatAdmin,
    handle: isize,
    file: Vec<u16>,
}

impl CatAdmin {
    /// A context for `algorithm`; `None` if the OS has no such algorithm.
    fn acquire(algorithm: &str) -> Result<Option<Self>> {
        let name = wide_nul(algorithm)?;
        let mut handle = 0isize;
        // SAFETY: `handle` is a valid out pointer; `name` is a NUL-terminated
        // UTF-16 string that outlives the call; the optional arguments are
        // absent.
        let acquired = unsafe {
            CryptCATAdminAcquireContext2(&raw mut handle, None, PCWSTR(name.as_ptr()), None, None)
        };
        if acquired.is_err() || handle == 0 {
            return Ok(None);
        }
        Ok(Some(Self { handle }))
    }

    /// The hash of the file, by this context's algorithm.
    fn hash_of(&self, file: &OwnedHandle) -> Result<Vec<u8>> {
        let mut size = 0u32;
        // SAFETY: the context and file handle are valid; no buffer is passed,
        // so this only asks for the hash size through `size`.
        let probe = unsafe {
            CryptCATAdminCalcHashFromFileHandle2(self.handle, file.0, &raw mut size, None, None)
        };
        // The size query ends with ERROR_INSUFFICIENT_BUFFER (or success).
        drop(probe);
        let len = usize::try_from(size).map_err(|_| ShimError::Os(0))?;
        if len == 0 || len > 128 {
            return Err(ShimError::Os(0));
        }
        let mut hash = vec![0u8; len];
        // SAFETY: as above; `hash` is writable for `size` bytes and outlives
        // the call.
        unsafe {
            CryptCATAdminCalcHashFromFileHandle2(
                self.handle,
                file.0,
                &raw mut size,
                Some(hash.as_mut_ptr()),
                None,
            )
        }?;
        hash.truncate(usize::try_from(size).map_err(|_| ShimError::Os(0))?);
        Ok(hash)
    }

    /// The first catalog that lists `hash`, with its file path.
    fn catalog_of(&self, hash: &[u8]) -> Option<Catalog<'_>> {
        // SAFETY: the context is valid; `hash` is a slice that outlives the
        // call; no previous catalog is passed.
        let handle = unsafe { CryptCATAdminEnumCatalogFromHash(self.handle, hash, None, None) };
        if handle == 0 {
            return None;
        }
        let mut catalog = Catalog {
            admin: self,
            handle,
            file: Vec::new(),
        };
        let mut info = CATALOG_INFO {
            cbStruct: size_u32::<CATALOG_INFO>(),
            ..CATALOG_INFO::default()
        };
        // SAFETY: `handle` is a valid catalog context of this admin; `info`
        // is a valid out structure with `cbStruct` set.
        unsafe { CryptCATCatalogInfoFromContext(handle, &raw mut info, 0) }.ok()?;
        let end = info.wszCatalogFile.iter().position(|u| *u == 0)?;
        let mut file = info.wszCatalogFile.get(..end)?.to_vec();
        file.push(0);
        catalog.file = file;
        Some(catalog)
    }
}

impl Drop for CatAdmin {
    fn drop(&mut self) {
        // SAFETY: `handle` is a context this wrapper acquired and owns; drop
        // runs once.
        let released = unsafe { CryptCATAdminReleaseContext(self.handle, 0) };
        let _ = released;
    }
}

impl Drop for Catalog<'_> {
    fn drop(&mut self) {
        // SAFETY: `handle` is a catalog context this wrapper obtained from
        // `admin`, which is still alive (borrowed); drop runs once.
        let released =
            unsafe { CryptCATAdminReleaseCatalogContext(self.admin.handle, self.handle, 0) };
        let _ = released;
    }
}

//! The identity of the program behind a window (ADR-0158): captured from the OS
//! (process image, package, Authenticode signature, integrity level), never from
//! window text. Identity data is not secret, but it is not logged either.

use std::io::Read;
use std::path::Path;

use mosaic_origin::program::{ProgramClass, ProgramIdentity, classify_program, fold_name};
use sha2::{Digest as _, Sha256};

/// The largest unsigned image that is hashed to bind it, in bytes. A bigger
/// image cannot be linked ([`CaptureRefusal::TooLargeToBind`]).
pub const MAX_UNSIGNED_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

/// How much of an image is read at a time.
const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// The exe a UWP window belongs to as a top-level window.
const APPLICATION_FRAME_HOST: &str = "applicationframehost.exe";

/// What the signer's certificate says about itself. For display only: it is
/// never part of a URL and never read by the matcher.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignerDisplay {
    /// The common name (`CN`).
    pub common_name: Option<String>,
    /// The organization (`O`).
    pub organization: Option<String>,
}

/// The state of an image's Authenticode signature (ADR-0157 D4). Only
/// `Trusted` and `Unsigned` images can be bound.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SignatureState {
    /// A valid signature that chains to a trusted root.
    Trusted {
        /// The DER subject of the leaf signing certificate.
        subject_der: Vec<u8>,
        /// The signer's name for display.
        display: SignerDisplay,
    },
    /// No signature.
    Unsigned,
    /// A signature that does not verify (bad digest, untrusted root, expired).
    Untrusted,
    /// A revoked signing certificate.
    Revoked,
}

/// A process as the OS reports it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawProcess {
    /// The process ID.
    pub pid: u32,
    /// The final path of the image (no `\\?\` prefix, no 8.3 names).
    pub final_path: String,
    /// The package family name, for a packaged app.
    pub package_family: Option<String>,
    /// The mandatory integrity level (the RID of the token's integrity SID).
    pub integrity_rid: u32,
}

/// Mosaic's own process.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OwnProcess {
    /// The process ID.
    pub pid: u32,
    /// The integrity level RID.
    pub integrity_rid: u32,
}

/// Whose identity a focused element has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentitySource {
    /// The process that owns the window (the element is in it).
    WindowOwner,
    /// The process of the element (a UWP window's content process).
    ElementProcess,
}

/// A program, identified for linking and matching.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CapturedIdentity {
    /// The process ID at capture time.
    pub pid: u32,
    /// The final path of the image.
    pub final_path: String,
    /// The exe file name, folded.
    pub exe_name: String,
    /// The package family, folded.
    pub package_family: Option<String>,
    /// SHA-256 of the signer's DER subject; `Some` only for a trusted signature.
    pub signer_subject_sha256: Option<[u8; 32]>,
    /// The signer's name, for display.
    pub signer_display: Option<SignerDisplay>,
    /// SHA-256 of the image file; only for unsigned images.
    pub file_sha256: Option<[u8; 32]>,
    /// The exclusion class.
    pub class: ProgramClass,
}

impl CapturedIdentity {
    /// The matcher's view of this identity (display data left out).
    #[must_use]
    pub fn as_identity(&self) -> ProgramIdentity<'_> {
        ProgramIdentity {
            package_family: self.package_family.as_deref(),
            exe_name: &self.exe_name,
            signer_subject_sha256: self.signer_subject_sha256,
            final_path: &self.final_path,
            file_sha256: self.file_sha256,
            class: self.class,
        }
    }
}

/// Why a program cannot be captured or linked.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CaptureRefusal {
    /// The target is one of Mosaic's own processes.
    #[error("own process")]
    OwnProcess,
    /// The target runs at a higher integrity level than Mosaic.
    #[error("elevated target")]
    Elevated,
    /// The focused element lives in another process than the window's owner
    /// (embedded web content, a host process).
    #[error("foreign process content")]
    ForeignContent,
    /// A browser, script host, Mosaic, broker or remote-desktop client.
    #[error("excluded program")]
    Excluded(ProgramClass),
    /// The image has a signature that does not verify.
    #[error("invalid signature")]
    InvalidSignature,
    /// The image's signing certificate is revoked.
    #[error("revoked signature")]
    RevokedSignature,
    /// An unsigned image too large to hash.
    #[error("too large to bind")]
    TooLargeToBind,
    /// The image path has no usable file name.
    #[error("bad path")]
    BadPath,
}

/// Why an image could not be hashed.
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    /// Over the size cap.
    #[error("too large")]
    TooLarge,
    /// The file could not be read.
    #[error("io")]
    Io,
}

/// The folded file name of a final path (its last component), or `None` when
/// there is none.
#[must_use]
pub fn exe_name_of(final_path: &str) -> Option<String> {
    let name = final_path.rsplit(['\\', '/']).next()?;
    if name.is_empty() || name.contains(':') {
        return None;
    }
    Some(fold_name(name))
}

/// Which process the identity comes from: the window owner when it is the
/// element's process, the element's process for a UWP window (owner
/// `ApplicationFrameHost.exe`, element with a package family). Everything else
/// is foreign content (`WebView2` included, ADR-0158 Q2).
///
/// # Errors
/// [`CaptureRefusal::ForeignContent`].
pub fn choose_identity_process(
    element: &RawProcess,
    window_owner: &RawProcess,
) -> Result<IdentitySource, CaptureRefusal> {
    if element.pid == window_owner.pid {
        return Ok(IdentitySource::WindowOwner);
    }
    let owner_is_frame_host =
        exe_name_of(&window_owner.final_path).is_some_and(|n| n == APPLICATION_FRAME_HOST);
    if owner_is_frame_host && element.package_family.is_some() {
        return Ok(IdentitySource::ElementProcess);
    }
    Err(CaptureRefusal::ForeignContent)
}

/// Builds the identity of `process`, or says why it cannot be used. In order:
/// Mosaic itself, a higher integrity level, a path without a file name, an
/// excluded class, then the signature state. A trusted image keeps the hash of
/// its signer's subject and drops the file hash; an unsigned one needs the file
/// hash (`None` means it was too large to hash).
///
/// # Errors
/// [`CaptureRefusal`].
pub fn assemble_identity(
    own: OwnProcess,
    process: &RawProcess,
    signature: SignatureState,
    file_sha256: Option<[u8; 32]>,
) -> Result<CapturedIdentity, CaptureRefusal> {
    if process.pid == own.pid {
        return Err(CaptureRefusal::OwnProcess);
    }
    if process.integrity_rid > own.integrity_rid {
        return Err(CaptureRefusal::Elevated);
    }
    let exe_name = exe_name_of(&process.final_path).ok_or(CaptureRefusal::BadPath)?;
    let package_family = process
        .package_family
        .as_deref()
        .map(str::to_ascii_lowercase);
    let class = classify_program(&exe_name, package_family.as_deref());
    if class != ProgramClass::Ordinary {
        return Err(CaptureRefusal::Excluded(class));
    }
    let (signer_subject_sha256, signer_display, file_sha256) = match signature {
        SignatureState::Revoked => return Err(CaptureRefusal::RevokedSignature),
        SignatureState::Untrusted => return Err(CaptureRefusal::InvalidSignature),
        SignatureState::Trusted {
            subject_der,
            display,
        } => (Some(sha256(&subject_der)), Some(display), None),
        SignatureState::Unsigned => (
            None,
            None,
            Some(file_sha256.ok_or(CaptureRefusal::TooLargeToBind)?),
        ),
    };
    Ok(CapturedIdentity {
        pid: process.pid,
        final_path: process.final_path.clone(),
        exe_name,
        package_family,
        signer_subject_sha256,
        signer_display,
        file_sha256,
        class,
    })
}

/// SHA-256 of an image file, streamed in 64 KiB pieces.
///
/// # Errors
/// [`HashError::TooLarge`] if the file is longer than `cap` bytes (by its
/// metadata, or by what was read), [`HashError::Io`] if it cannot be read.
pub fn hash_image_capped(path: &Path, cap: u64) -> Result<[u8; 32], HashError> {
    let file = std::fs::File::open(path).map_err(|_| HashError::Io)?;
    if file.metadata().map_err(|_| HashError::Io)?.len() > cap {
        return Err(HashError::TooLarge);
    }
    // One byte past the cap tells a file that grew while it was read.
    let mut reader = file.take(cap.saturating_add(1));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut total: u64 = 0;
    loop {
        let read = reader.read(&mut buffer).map_err(|_| HashError::Io)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > cap {
            return Err(HashError::TooLarge);
        }
        hasher.update(buffer.get(..read).ok_or(HashError::Io)?);
    }
    Ok(finish(hasher))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    finish(hasher)
}

fn finish(hasher: Sha256) -> [u8; 32] {
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    for (slot, byte) in out.iter_mut().zip(digest.iter()) {
        *slot = *byte;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(pid: u32, path: &str, pfn: Option<&str>, rid: u32) -> RawProcess {
        RawProcess {
            pid,
            final_path: path.to_owned(),
            package_family: pfn.map(str::to_owned),
            integrity_rid: rid,
        }
    }

    const OWN: OwnProcess = OwnProcess {
        pid: 10,
        integrity_rid: 0x2000,
    };

    fn trusted() -> SignatureState {
        SignatureState::Trusted {
            subject_der: b"subject".to_vec(),
            display: SignerDisplay {
                common_name: Some("Example".into()),
                organization: Some("Example Inc".into()),
            },
        }
    }

    #[test]
    fn mosaic_never_fills_its_own_windows() {
        assert_eq!(
            assemble_identity(
                OWN,
                &raw(10, r"C:\a\x.exe", None, 0x2000),
                SignatureState::Unsigned,
                Some([0; 32])
            ),
            Err(CaptureRefusal::OwnProcess)
        );
        assert_eq!(
            assemble_identity(
                OWN,
                &raw(
                    11,
                    r"C:\Program Files\Mosaic\mosaic-desktop.exe",
                    None,
                    0x2000
                ),
                trusted(),
                None
            ),
            Err(CaptureRefusal::Excluded(ProgramClass::Mosaic))
        );
    }

    #[test]
    fn elevated_target_gets_no_chip_and_no_fill() {
        assert_eq!(
            assemble_identity(OWN, &raw(11, r"C:\a\x.exe", None, 0x3000), trusted(), None),
            Err(CaptureRefusal::Elevated)
        );
        // Equal and lower levels are fine (the AppContainer level is lower).
        for rid in [0x2000, 0x1000] {
            assert!(
                assemble_identity(OWN, &raw(11, r"C:\a\x.exe", None, rid), trusted(), None).is_ok()
            );
        }
    }

    #[test]
    fn foreign_process_content_is_refused() {
        let element = raw(20, r"C:\Program Files\Tool\helper.exe", None, 0x2000);
        let owner = raw(21, r"C:\Windows\notepad.exe", None, 0x2000);
        assert_eq!(
            choose_identity_process(&element, &owner),
            Err(CaptureRefusal::ForeignContent)
        );
        // WebView2: the element lives in a child of the window's owner.
        let webview = raw(
            22,
            r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application\msedgewebview2.exe",
            None,
            0x2000,
        );
        assert_eq!(
            choose_identity_process(&webview, &owner),
            Err(CaptureRefusal::ForeignContent)
        );
        assert_eq!(
            choose_identity_process(&owner, &owner),
            Ok(IdentitySource::WindowOwner)
        );
    }

    #[test]
    fn uwp_content_takes_the_package_of_the_element_process() {
        let owner = raw(
            30,
            r"C:\Windows\System32\ApplicationFrameHost.exe",
            None,
            0x2000,
        );
        let packaged = raw(
            31,
            r"C:\Program Files\WindowsApps\calc\CalculatorApp.exe",
            Some("Microsoft.WindowsCalculator_8wekyb3d8bbwe"),
            0x1000,
        );
        assert_eq!(
            choose_identity_process(&packaged, &owner),
            Ok(IdentitySource::ElementProcess)
        );
        let plain = raw(32, r"C:\x\app.exe", None, 0x2000);
        assert_eq!(
            choose_identity_process(&plain, &owner),
            Err(CaptureRefusal::ForeignContent)
        );
    }

    #[test]
    fn only_trusted_and_unsigned_images_are_bindable() {
        let p = raw(11, r"C:\a\x.exe", None, 0x2000);
        assert_eq!(
            assemble_identity(OWN, &p, SignatureState::Revoked, None),
            Err(CaptureRefusal::RevokedSignature)
        );
        assert_eq!(
            assemble_identity(OWN, &p, SignatureState::Untrusted, Some([1; 32])),
            Err(CaptureRefusal::InvalidSignature)
        );
        assert!(assemble_identity(OWN, &p, trusted(), None).is_ok());
        assert!(assemble_identity(OWN, &p, SignatureState::Unsigned, Some([1; 32])).is_ok());
    }

    #[test]
    fn signed_identity_hashes_the_subject_and_drops_the_file_hash() {
        let p = raw(
            11,
            r"C:\Apps\Discord.EXE",
            Some("Pkg.Name_8WEKYB3D8BBWE"),
            0x2000,
        );
        let id = assemble_identity(OWN, &p, trusted(), Some([1; 32])).unwrap();
        let expected: [u8; 32] = Sha256::digest(b"subject").into();
        assert_eq!(id.signer_subject_sha256, Some(expected));
        assert_eq!(id.file_sha256, None);
        assert_eq!(id.exe_name, "discord.exe");
        assert_eq!(id.package_family.as_deref(), Some("pkg.name_8wekyb3d8bbwe"));
        assert_eq!(id.class, ProgramClass::Ordinary);
        assert_eq!(
            id.signer_display
                .as_ref()
                .and_then(|d| d.common_name.as_deref()),
            Some("Example")
        );
        // The matcher's view carries no display data and the same facts.
        let view = id.as_identity();
        assert_eq!(view.signer_subject_sha256, Some(expected));
        assert_eq!(view.exe_name, "discord.exe");
    }

    #[test]
    fn unsigned_image_over_the_cap_cannot_be_bound() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.exe");
        std::fs::write(&path, [7u8; 10]).unwrap();
        assert!(matches!(
            hash_image_capped(&path, 9),
            Err(HashError::TooLarge)
        ));
        let expected: [u8; 32] = Sha256::digest([7u8; 10]).into();
        assert_eq!(hash_image_capped(&path, 10).unwrap(), expected);
        assert!(matches!(
            hash_image_capped(&dir.path().join("missing.exe"), 10),
            Err(HashError::Io)
        ));
        let p = raw(11, r"C:\a\x.exe", None, 0x2000);
        assert_eq!(
            assemble_identity(OWN, &p, SignatureState::Unsigned, None),
            Err(CaptureRefusal::TooLargeToBind)
        );
    }

    #[test]
    fn exe_name_of_takes_the_last_component() {
        assert_eq!(
            exe_name_of(r"C:\A\Discord.EXE").as_deref(),
            Some("discord.exe")
        );
        assert_eq!(exe_name_of(r"\\srv\share\T.exe").as_deref(), Some("t.exe"));
        assert_eq!(exe_name_of(r"C:\"), None);
        assert_eq!(exe_name_of(""), None);
        assert_eq!(exe_name_of("C:"), None);
        let bad = raw(11, r"C:\", None, 0x2000);
        assert_eq!(
            assemble_identity(OWN, &bad, trusted(), None),
            Err(CaptureRefusal::BadPath)
        );
    }
}

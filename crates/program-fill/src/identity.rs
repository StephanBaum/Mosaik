//! The identity of the program behind a window (ADR-0158): captured from the OS
//! (process image, package, Authenticode signature, integrity level), never from
//! window text. Identity data is not secret, but it is not logged either.

use std::io::Read;
use std::time::Instant;

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
        /// The DER of the root certificate the signature chains to (it is in the
        /// machine's `Root` or `AuthRoot` store).
        root_der: Vec<u8>,
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
    /// `WebView2` content of the window owner: the identity is the owner's
    /// (ADR-0164).
    WebView2Host,
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
    /// The signer id ([`signer_id`]); `Some` only for a trusted signature.
    pub signer_id: Option<[u8; 32]>,
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
            signer_id: self.signer_id,
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
    /// The image is on a network path (UNC or a mapped network drive): a remote
    /// file can change under the handle, so it is never bound.
    #[error("image on a network path")]
    NetworkImage,
    /// The image path has no usable file name.
    #[error("bad path")]
    BadPath,
    /// `WebView2` content of a host program that is neither signed nor packaged
    /// (ADR-0164: no two-confirmation path for these).
    #[error("unsigned webview2 host")]
    UnsignedWebViewHost,
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
    /// The deadline passed before the whole file was read.
    #[error("timeout")]
    Timeout,
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

/// Product names (folded) of browsers. A browser copied to another file name keeps
/// the vendor's signature and its version resource, so the name list of
/// ADR-0158 does not exclude it; its product name does (ADR-0164).
const BROWSER_PRODUCTS: &[&str] = &[
    "google chrome",
    "chromium",
    "microsoft edge",
    "mozilla firefox",
    "firefox",
    "brave",
    "opera",
    "vivaldi",
    "internet explorer",
    "librewolf",
    "waterfox",
    "floorp",
    "thorium",
    "yandex browser",
];

/// The class a product name from a version resource gives: [`ProgramClass::Browser`]
/// when it names a browser (the whole name, or the name followed by a space and
/// more, such as `Google Chrome Beta`), whatever the file is called and whoever
/// signed it. A missing or odd name gives nothing.
#[must_use]
pub fn product_class(product: &str) -> Option<ProgramClass> {
    let folded = fold_name(product.trim());
    BROWSER_PRODUCTS
        .iter()
        .any(|name| {
            folded == *name
                || folded
                    .strip_prefix(name)
                    .is_some_and(|rest| rest.starts_with(' '))
        })
        .then_some(ProgramClass::Browser)
}

/// The `WebView2` process image name (folded).
pub const WEBVIEW2_EXE: &str = "msedgewebview2.exe";

/// The longest parent chain walked from a `WebView2` process to its host.
pub const MAX_WEBVIEW2_CHAIN: usize = 8;

/// One process of the parent chain of a `WebView2` process, read through an
/// open handle (never from a process ID alone).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChainProcess {
    /// The process ID.
    pub pid: u32,
    /// The folded image file name.
    pub exe_name: String,
    /// When the process was created (`FILETIME` ticks).
    pub start_time: u64,
}

/// Whether the focused element's process is a `WebView2` process that descends
/// from `host_pid`: `chain` is the element's process first, then its parents.
/// Every process before the host must be a `WebView2` process, and every
/// parent must have started no later than its child (a parent that started
/// after its child is a reused process ID).
#[must_use]
pub fn webview2_descends(chain: &[ChainProcess], host_pid: u32) -> bool {
    if chain
        .first()
        .is_none_or(|first| first.exe_name != WEBVIEW2_EXE)
    {
        return false;
    }
    for (child, parent) in chain.iter().zip(chain.iter().skip(1)) {
        if parent.start_time > child.start_time {
            return false;
        }
        if parent.pid == host_pid {
            return true;
        }
        if parent.exe_name != WEBVIEW2_EXE {
            return false;
        }
    }
    false
}

/// [`choose_identity_process`] with the `WebView2` rule of ADR-0164 (an
/// amendment of ADR-0158 Q2): content of a `WebView2` process that descends from
/// the window's owner counts as the owner's own content, so the identity is the
/// owner's. `chain` is the element process's parent chain (see
/// [`webview2_descends`]).
///
/// # Errors
/// [`CaptureRefusal::ForeignContent`].
pub fn choose_identity_process_webview2(
    element: &RawProcess,
    window_owner: &RawProcess,
    chain: &[ChainProcess],
) -> Result<IdentitySource, CaptureRefusal> {
    match choose_identity_process(element, window_owner) {
        Err(CaptureRefusal::ForeignContent)
            if exe_name_of(&element.final_path).is_some_and(|n| n == WEBVIEW2_EXE)
                && webview2_descends(chain, window_owner.pid) =>
        {
            Ok(IdentitySource::WebView2Host)
        }
        other => other,
    }
}

/// [`assemble_identity`] for the host of `WebView2` content: the host must be
/// signed (trusted) or packaged; an unsigned host is refused with no
/// two-confirmation path. A browser host is refused by the class check.
///
/// # Errors
/// [`CaptureRefusal`].
pub fn assemble_webview2_host_identity(
    own: OwnProcess,
    host: &RawProcess,
    signature: SignatureState,
    file_sha256: Option<[u8; 32]>,
) -> Result<CapturedIdentity, CaptureRefusal> {
    let identity = assemble_identity(own, host, signature, file_sha256)?;
    if identity.signer_id.is_none() && identity.package_family.is_none() {
        return Err(CaptureRefusal::UnsignedWebViewHost);
    }
    Ok(identity)
}

/// The cheap refusals that need no signature or hash: Mosaic itself, a higher
/// integrity level, a path without a file name, an excluded class. Run before
/// the image is opened, so a browser in the foreground costs no signature check.
///
/// # Errors
/// [`CaptureRefusal`].
pub fn screen_process(own: OwnProcess, process: &RawProcess) -> Result<(), CaptureRefusal> {
    if process.pid == own.pid {
        return Err(CaptureRefusal::OwnProcess);
    }
    if process.integrity_rid > own.integrity_rid {
        return Err(CaptureRefusal::Elevated);
    }
    let exe_name = exe_name_of(&process.final_path).ok_or(CaptureRefusal::BadPath)?;
    let family = process
        .package_family
        .as_deref()
        .map(str::to_ascii_lowercase);
    let class = classify_program(&exe_name, family.as_deref());
    if class != ProgramClass::Ordinary {
        return Err(CaptureRefusal::Excluded(class));
    }
    Ok(())
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
    screen_process(own, process)?;
    let exe_name = exe_name_of(&process.final_path).ok_or(CaptureRefusal::BadPath)?;
    let package_family = process
        .package_family
        .as_deref()
        .map(str::to_ascii_lowercase);
    let class = classify_program(&exe_name, package_family.as_deref());
    let (signer_id, signer_display, file_sha256) = match signature {
        SignatureState::Revoked => return Err(CaptureRefusal::RevokedSignature),
        SignatureState::Untrusted => return Err(CaptureRefusal::InvalidSignature),
        SignatureState::Trusted {
            subject_der,
            root_der,
            display,
        } => (
            Some(signer_id(&subject_der, &root_der)),
            Some(display),
            None,
        ),
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
        signer_id,
        signer_display,
        file_sha256,
        class,
    })
}

/// The id of a trusted signer: SHA-256 of the DER subject of the leaf signing
/// certificate followed by the SHA-256 of the DER of the root certificate the
/// chain ends in. The subject alone is copyable by anyone who can mint a
/// certificate with it; the root ties it to the authority that vouches for it.
/// A leaf renewed under the same root keeps the id; another root is another
/// signer ([`mosaic_origin::program::BindingChange::SignerChanged`]).
#[must_use]
pub fn signer_id(subject_der: &[u8], root_der: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(subject_der);
    hasher.update(sha256(root_der));
    finish(hasher)
}

/// SHA-256 of an image file, streamed in 64 KiB pieces.
///
/// # Errors
/// [`HashError::TooLarge`] if the file is longer than `cap` bytes (by its
/// metadata, or by what was read), [`HashError::Io`] if it cannot be read,
/// [`HashError::Timeout`] past `deadline`.
#[cfg(test)]
pub fn hash_image_capped(
    path: &std::path::Path,
    cap: u64,
    deadline: Instant,
) -> Result<[u8; 32], HashError> {
    let file = std::fs::File::open(path).map_err(|_| HashError::Io)?;
    hash_file_capped(&file, cap, deadline)
}

/// SHA-256 of an already open file, from its start (the caller holds the one
/// handle the signature was checked through). The deadline is checked between
/// 64 KiB reads.
///
/// # Errors
/// [`HashError::TooLarge`] if the file is longer than `cap` bytes (by its
/// metadata, or by what was read), [`HashError::Io`] if it cannot be read,
/// [`HashError::Timeout`] past `deadline`.
pub fn hash_file_capped(
    mut file: &std::fs::File,
    cap: u64,
    deadline: Instant,
) -> Result<[u8; 32], HashError> {
    use std::io::{Seek as _, SeekFrom};
    if file.metadata().map_err(|_| HashError::Io)?.len() > cap {
        return Err(HashError::TooLarge);
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| HashError::Io)?;
    // One byte past the cap tells a file that grew while it was read.
    let mut reader = Read::take(file, cap.saturating_add(1));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut total: u64 = 0;
    loop {
        if Instant::now() >= deadline {
            return Err(HashError::Timeout);
        }
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

    fn soon() -> Instant {
        Instant::now() + std::time::Duration::from_secs(30)
    }

    const OWN: OwnProcess = OwnProcess {
        pid: 10,
        integrity_rid: 0x2000,
    };

    fn trusted() -> SignatureState {
        SignatureState::Trusted {
            subject_der: b"subject".to_vec(),
            root_der: b"root".to_vec(),
            display: SignerDisplay {
                common_name: Some("Example".into()),
                organization: Some("Example Inc".into()),
            },
        }
    }

    #[test]
    fn screening_refuses_before_any_signature_work() {
        let ping = raw(11, r"C:\Windows\System32\PING.EXE", None, 0x2000);
        assert_eq!(screen_process(OWN, &ping), Ok(()));
        let me = raw(10, r"C:\Apps\mosaic.exe", None, 0x2000);
        assert_eq!(screen_process(OWN, &me), Err(CaptureRefusal::OwnProcess));
        let high = raw(12, r"C:\Apps\a.exe", None, 0x3000);
        assert_eq!(screen_process(OWN, &high), Err(CaptureRefusal::Elevated));
        let chrome = raw(
            13,
            r"C:\Program Files\Google\Chrome\chrome.exe",
            None,
            0x2000,
        );
        assert_eq!(
            screen_process(OWN, &chrome),
            Err(CaptureRefusal::Excluded(ProgramClass::Browser))
        );
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
        let expected = signer_id(b"subject", b"root");
        assert_eq!(id.signer_id, Some(expected));
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
        assert_eq!(view.signer_id, Some(expected));
        assert_eq!(view.exe_name, "discord.exe");
    }

    #[test]
    fn the_signer_id_binds_the_subject_to_its_root() {
        let id = signer_id(b"subject", b"root");
        // Spelled out: SHA-256(subject || SHA-256(root)).
        let mut inner = Sha256::new();
        inner.update(b"subject");
        inner.update(Sha256::digest(b"root"));
        let expected: [u8; 32] = inner.finalize().into();
        assert_eq!(id, expected);
        // A renewed leaf (same subject) under the same root is the same signer.
        assert_eq!(id, signer_id(b"subject", b"root"));
        // The same subject under another root (a root the user added) is not.
        assert_ne!(id, signer_id(b"subject", b"other root"));
        assert_ne!(id, signer_id(b"other subject", b"root"));
        // Moving bytes between the two parts changes the id.
        assert_ne!(signer_id(b"subjectr", b"oot"), id);
    }

    #[test]
    fn hashing_stops_at_the_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.exe");
        std::fs::write(&path, [1u8; 100]).unwrap();
        let past = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(matches!(
            hash_image_capped(&path, 1000, past),
            Err(HashError::Timeout)
        ));
        let later = Instant::now() + std::time::Duration::from_secs(30);
        assert!(hash_image_capped(&path, 1000, later).is_ok());
    }

    #[test]
    fn unsigned_image_over_the_cap_cannot_be_bound() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.exe");
        std::fs::write(&path, [7u8; 10]).unwrap();
        assert!(matches!(
            hash_image_capped(&path, 9, soon()),
            Err(HashError::TooLarge)
        ));
        let expected: [u8; 32] = Sha256::digest([7u8; 10]).into();
        assert_eq!(hash_image_capped(&path, 10, soon()).unwrap(), expected);
        assert!(matches!(
            hash_image_capped(&dir.path().join("missing.exe"), 10, soon()),
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

    #[test]
    fn a_renamed_browser_is_known_by_its_product_name() {
        for product in [
            "Google Chrome",
            "Google Chrome Beta",
            "Chromium",
            "Microsoft Edge",
            "Mozilla Firefox",
            "Firefox Developer Edition",
            "Brave",
            "Opera",
            "Vivaldi",
        ] {
            assert_eq!(
                product_class(product),
                Some(ProgramClass::Browser),
                "{product}"
            );
        }
        for product in [
            "",
            "Slack",
            "Bravely Default",
            "Operations Console",
            "Microsoft Office",
            "Microsoft Teams",
            "Chrome Remote Desktop Host",
        ] {
            assert_eq!(product_class(product), None, "{product}");
        }
    }

    // ---- WebView2 (ADR-0164 amendment of ADR-0158 Q2) ----

    fn link(pid: u32, exe: &str, start: u64) -> ChainProcess {
        ChainProcess {
            pid,
            exe_name: exe.to_owned(),
            start_time: start,
        }
    }

    const WEBVIEW_PATH: &str =
        r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application\msedgewebview2.exe";

    /// renderer 40, browser 41, host 42 (the window owner).
    fn host_chain() -> Vec<ChainProcess> {
        vec![
            link(40, WEBVIEW2_EXE, 300),
            link(41, WEBVIEW2_EXE, 200),
            link(42, "ms-teams.exe", 100),
        ]
    }

    #[test]
    fn webview2_host_signed_and_webview2_child_gets_an_offer() {
        let element = raw(40, WEBVIEW_PATH, None, 0x2000);
        let owner = raw(42, r"C:\Program Files\Teams\ms-teams.exe", None, 0x2000);
        assert_eq!(
            choose_identity_process_webview2(&element, &owner, &host_chain()),
            Ok(IdentitySource::WebView2Host)
        );
        let identity = assemble_webview2_host_identity(OWN, &owner, trusted(), None).unwrap();
        // The binding is the host's: its exe and signer, never the runtime's.
        assert_eq!(identity.exe_name, "ms-teams.exe");
        assert!(identity.signer_id.is_some());
        // A packaged but unsigned host is allowed too.
        let packaged = raw(
            42,
            r"C:\Program Files\WindowsApps\Pub.App_1.0\app.exe",
            Some("Pub.App_8wekyb3d8bbwe"),
            0x2000,
        );
        assert!(
            assemble_webview2_host_identity(
                OWN,
                &packaged,
                SignatureState::Unsigned,
                Some([1; 32])
            )
            .is_ok()
        );
    }

    #[test]
    fn webview2_unsigned_host_is_refused() {
        let owner = raw(42, r"C:\Users\a\Downloads\tool.exe", None, 0x2000);
        assert_eq!(
            assemble_webview2_host_identity(OWN, &owner, SignatureState::Unsigned, Some([1; 32])),
            Err(CaptureRefusal::UnsignedWebViewHost)
        );
    }

    #[test]
    fn webview2_browser_host_is_refused() {
        let owner = raw(
            42,
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            None,
            0x2000,
        );
        assert_eq!(
            assemble_webview2_host_identity(OWN, &owner, trusted(), None),
            Err(CaptureRefusal::Excluded(ProgramClass::Browser))
        );
    }

    #[test]
    fn webview2_process_not_descended_from_the_focused_host_is_refused() {
        let element = raw(40, WEBVIEW_PATH, None, 0x2000);
        let owner = raw(42, r"C:\Program Files\Teams\ms-teams.exe", None, 0x2000);
        let refused = Err(CaptureRefusal::ForeignContent);
        // The chain ends at another program.
        let other_host = vec![
            link(40, WEBVIEW2_EXE, 300),
            link(41, WEBVIEW2_EXE, 200),
            link(77, "other.exe", 100),
        ];
        assert_eq!(
            choose_identity_process_webview2(&element, &owner, &other_host),
            refused
        );
        // A different program sits between the renderer and the host.
        let middle = vec![
            link(40, WEBVIEW2_EXE, 300),
            link(55, "evil.exe", 200),
            link(42, "ms-teams.exe", 100),
        ];
        assert_eq!(
            choose_identity_process_webview2(&element, &owner, &middle),
            refused
        );
        // The parent ID was reused: the "parent" started after its child.
        let recycled = vec![link(40, WEBVIEW2_EXE, 300), link(42, "ms-teams.exe", 900)];
        assert_eq!(
            choose_identity_process_webview2(&element, &owner, &recycled),
            refused
        );
        // No chain at all, an empty one, or a first process that is no WebView2.
        assert_eq!(
            choose_identity_process_webview2(&element, &owner, &[]),
            refused
        );
        let not_webview = vec![link(40, "helper.exe", 300), link(42, "ms-teams.exe", 100)];
        let helper = raw(40, r"C:\Program Files\Teams\helper.exe", None, 0x2000);
        assert_eq!(
            choose_identity_process_webview2(&helper, &owner, &not_webview),
            refused
        );
    }

    #[test]
    fn webview2_host_replaced_between_offer_and_write_is_refused() {
        use crate::fake::{FakeProbe, FakeProcess, good_snapshot};
        use crate::verify::{FieldKind, PickSession, RuntimeId, VerifyFailure, WriteKind, check};
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;

        let owner = raw(42, r"C:\Program Files\Teams\ms-teams.exe", None, 0x2000);
        let identity = assemble_webview2_host_identity(OWN, &owner, trusted(), None).unwrap();
        let alive = Rc::new(Cell::new(true));
        let session = PickSession::new(
            FakeProcess {
                alive: Rc::clone(&alive),
            },
            &identity,
            40,
            42,
            7,
            RuntimeId(vec![1]),
            FieldKind::Text,
            0,
        );
        let probe = FakeProbe {
            snapshot: RefCell::new(Ok(good_snapshot(&session))),
        };
        assert_eq!(check(&probe, &session, WriteKind::Username, 1), Ok(()));
        // The host ended (another program took its place): nothing is written.
        alive.set(false);
        assert_eq!(
            check(&probe, &session, WriteKind::Username, 1),
            Err(VerifyFailure::ProcessGone)
        );
    }
}

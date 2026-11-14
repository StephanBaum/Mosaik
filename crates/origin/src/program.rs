//! Windows program URLs on login items (ADR-0158): which program a login belongs
//! to, and when an observed program is that program.
//!
//! Three canonical forms (only the canonical text is accepted):
//!
//! - `winapp://package/<family>`: a packaged app, by package family name.
//! - `winapp://signed/<exe>?signer=<b64url(sha256)>`: a program whose image has a
//!   trusted Authenticode signature, by the SHA-256 of the signer's DER subject
//!   and the folded exe name.
//! - `winapp://path/<b64url(path)>?sha256=<b64url(sha256)>`: an unsigned program,
//!   by its final path and the SHA-256 of its image file.
//!
//! The identity is never a window title, caption or version resource: display
//! data is not read here. Browsers, script and interpreter hosts, Mosaic's own
//! processes, credential brokers and remote-desktop clients never match and are
//! never bindable, whatever their signer ([`ProgramClass`]).

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::LoginUrl;

use crate::{MatchKind, Refusal, osa_distance_is_one};

/// The longest exe file name accepted, in decoded UTF-8 bytes.
pub const MAX_EXE_NAME_LEN: usize = 255;

/// The longest program path accepted, in decoded UTF-8 bytes.
pub const MAX_PROGRAM_PATH_LEN: usize = 4096;

const SCHEME_PREFIX: &str = "winapp://";
const SIGNER_KEY: &str = "signer=";
const SHA256_KEY: &str = "sha256=";
/// The longest base64url text of a path of [`MAX_PROGRAM_PATH_LEN`] bytes.
const MAX_PATH_TEXT_LEN: usize = MAX_PROGRAM_PATH_LEN.div_ceil(3) * 4;
/// Characters a Windows file name may not hold.
const FORBIDDEN_NAME_CHARS: &str = "\\/:*?\"<>|";
/// The lowercase Crockford base32 alphabet of a package publisher ID.
const PUBLISHER_ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Why a string is not a Windows program URL.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProgramUrlError {
    /// Not a `winapp://` URL of a known form.
    #[error("not a program url")]
    NotAProgramUrl,
    /// A bad package family name.
    #[error("bad package family")]
    BadPackage,
    /// A bad exe file name.
    #[error("bad exe name")]
    BadExe,
    /// The signer is not `signer=<43 base64url characters>` of 32 bytes.
    #[error("bad signer")]
    BadSigner,
    /// A bad program path.
    #[error("bad path")]
    BadPath,
    /// The hash is not `sha256=<43 base64url characters>` of 32 bytes.
    #[error("bad hash")]
    BadHash,
}

/// A parsed Windows program URL.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProgramUrl {
    /// A packaged app.
    Package {
        /// The package family name, ASCII lowercase.
        family: String,
    },
    /// A program with a trusted signature.
    Signed {
        /// The folded exe file name.
        exe: String,
        /// SHA-256 of the DER subject of the leaf signing certificate.
        signer: [u8; 32],
    },
    /// An unsigned program.
    Path {
        /// The final path, as captured.
        path: String,
        /// SHA-256 of the image file.
        sha256: [u8; 32],
    },
}

impl ProgramUrl {
    /// A package URL. The family is lowercased.
    ///
    /// # Errors
    /// [`ProgramUrlError::BadPackage`].
    pub fn package(family: &str) -> Result<Self, ProgramUrlError> {
        let family = family.to_ascii_lowercase();
        if !is_valid_family(&family) {
            return Err(ProgramUrlError::BadPackage);
        }
        Ok(Self::Package { family })
    }

    /// A signed-program URL. The exe name is folded ([`fold_name`]).
    ///
    /// # Errors
    /// [`ProgramUrlError::BadExe`].
    pub fn signed(
        exe_name: &str,
        signer_subject_sha256: [u8; 32],
    ) -> Result<Self, ProgramUrlError> {
        let exe = fold_name(exe_name);
        if !is_valid_exe(&exe) {
            return Err(ProgramUrlError::BadExe);
        }
        Ok(Self::Signed {
            exe,
            signer: signer_subject_sha256,
        })
    }

    /// An unsigned-program URL for a final path (a drive or UNC path, never a
    /// `\\?\` or `\\.\` form).
    ///
    /// # Errors
    /// [`ProgramUrlError::BadPath`].
    pub fn path(final_path: &str, file_sha256: [u8; 32]) -> Result<Self, ProgramUrlError> {
        if !is_valid_path(final_path) {
            return Err(ProgramUrlError::BadPath);
        }
        Ok(Self::Path {
            path: final_path.to_owned(),
            sha256: file_sha256,
        })
    }

    /// Parses the canonical form (surrounding whitespace is ignored). Another
    /// spelling of the same program (case, percent-encoding, padding, extra
    /// query members, a fragment) is refused.
    ///
    /// # Errors
    /// [`ProgramUrlError`].
    pub fn parse(s: &str) -> Result<Self, ProgramUrlError> {
        let rest = s
            .trim()
            .strip_prefix(SCHEME_PREFIX)
            .ok_or(ProgramUrlError::NotAProgramUrl)?;
        if rest.contains('#') {
            return Err(ProgramUrlError::NotAProgramUrl);
        }
        let (head, query) = match rest.split_once('?') {
            Some((head, query)) => (head, Some(query)),
            None => (rest, None),
        };
        let (kind, tail) = head
            .split_once('/')
            .ok_or(ProgramUrlError::NotAProgramUrl)?;
        match kind {
            "package" => {
                if query.is_some() || tail.bytes().any(|b| b.is_ascii_uppercase()) {
                    return Err(ProgramUrlError::BadPackage);
                }
                Self::package(tail)
            }
            "signed" => {
                let signer = query
                    .and_then(|q| q.strip_prefix(SIGNER_KEY))
                    .and_then(decode_hash)
                    .ok_or(ProgramUrlError::BadSigner)?;
                let exe = decode_exe(tail).ok_or(ProgramUrlError::BadExe)?;
                // The folded name is the only accepted one, and it must
                // print back to exactly this text.
                if fold_name(&exe) != exe || !is_valid_exe(&exe) || encode_exe(&exe) != tail {
                    return Err(ProgramUrlError::BadExe);
                }
                Ok(Self::Signed { exe, signer })
            }
            "path" => {
                let sha256 = query
                    .and_then(|q| q.strip_prefix(SHA256_KEY))
                    .and_then(decode_hash)
                    .ok_or(ProgramUrlError::BadHash)?;
                if tail.len() > MAX_PATH_TEXT_LEN {
                    return Err(ProgramUrlError::BadPath);
                }
                let bytes =
                    Base64UrlUnpadded::decode_vec(tail).map_err(|_| ProgramUrlError::BadPath)?;
                let path = String::from_utf8(bytes).map_err(|_| ProgramUrlError::BadPath)?;
                if Base64UrlUnpadded::encode_string(path.as_bytes()) != tail {
                    return Err(ProgramUrlError::BadPath);
                }
                Self::path(&path, sha256)
            }
            _ => Err(ProgramUrlError::NotAProgramUrl),
        }
    }

    /// The canonical string.
    #[must_use]
    pub fn canonical(&self) -> String {
        match self {
            Self::Package { family } => format!("{SCHEME_PREFIX}package/{family}"),
            Self::Signed { exe, signer } => format!(
                "{SCHEME_PREFIX}signed/{}?{SIGNER_KEY}{}",
                encode_exe(exe),
                Base64UrlUnpadded::encode_string(signer)
            ),
            Self::Path { path, sha256 } => format!(
                "{SCHEME_PREFIX}path/{}?{SHA256_KEY}{}",
                Base64UrlUnpadded::encode_string(path.as_bytes()),
                Base64UrlUnpadded::encode_string(sha256)
            ),
        }
    }
}

/// What kind of program an observed process is. Everything but
/// [`ProgramClass::Ordinary`] never matches and is never bindable.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ProgramClass {
    /// Any other program.
    Ordinary,
    /// A web browser (web fills go through the extension).
    Browser,
    /// A script or interpreter host, terminal or generic loader.
    Host,
    /// One of Mosaic's own processes.
    Mosaic,
    /// A credential, consent, logon or lock-screen broker.
    Broker,
    /// A remote-desktop, VM or remote-control client.
    Remote,
}

/// What was observed about a program, from the OS (never from window text).
#[derive(Clone, Copy, Debug)]
pub struct ProgramIdentity<'a> {
    /// The package family, folded.
    pub package_family: Option<&'a str>,
    /// The exe file name, folded.
    pub exe_name: &'a str,
    /// SHA-256 of the signer's DER subject; `Some` only for a trusted signature.
    pub signer_subject_sha256: Option<[u8; 32]>,
    /// The final path of the image.
    pub final_path: &'a str,
    /// SHA-256 of the image file; only for unsigned images.
    pub file_sha256: Option<[u8; 32]>,
    /// The exclusion class.
    pub class: ProgramClass,
}

/// What changed about a program that a saved URL was bound to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BindingChange {
    /// The same exe name now carries another signer.
    SignerChanged,
    /// The same path now holds another file.
    FileChanged,
}

const BROWSERS: &[&str] = &[
    "chrome.exe",
    "chromium.exe",
    "msedge.exe",
    "msedgewebview2.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "opera_gx.exe",
    "vivaldi.exe",
    "iexplore.exe",
    "librewolf.exe",
    "waterfox.exe",
    "floorp.exe",
    "zen.exe",
    "thorium.exe",
    "arc.exe",
    "whale.exe",
    "yandex.exe",
    "browser.exe",
];

const HOSTS: &[&str] = &[
    "java.exe",
    "javaw.exe",
    "python.exe",
    "pythonw.exe",
    "py.exe",
    "pyw.exe",
    "node.exe",
    "deno.exe",
    "bun.exe",
    "electron.exe",
    "powershell.exe",
    "powershell_ise.exe",
    "pwsh.exe",
    "cmd.exe",
    "conhost.exe",
    "openconsole.exe",
    "windowsterminal.exe",
    "wt.exe",
    "wscript.exe",
    "cscript.exe",
    "mshta.exe",
    "rundll32.exe",
    "regsvr32.exe",
    "dllhost.exe",
    "wsl.exe",
    "wslhost.exe",
    "bash.exe",
    "ruby.exe",
    "perl.exe",
    "php.exe",
];

const HOST_PACKAGES: &[&str] = &[
    "microsoft.windowsterminal_8wekyb3d8bbwe",
    "microsoft.windowsterminalpreview_8wekyb3d8bbwe",
];

const MOSAIC: &[&str] = &["mosaic.exe", "mosaic-desktop.exe", "mosaic-native-host.exe"];

const BROKERS: &[&str] = &[
    "credentialuibroker.exe",
    "consent.exe",
    "logonui.exe",
    "lockapp.exe",
    "applicationframehost.exe",
];

const REMOTE: &[&str] = &[
    "mstsc.exe",
    "msrdc.exe",
    "msrdcw.exe",
    "vmconnect.exe",
    "vmware.exe",
    "vmware-vmx.exe",
    "vmplayer.exe",
    "virtualbox.exe",
    "virtualboxvm.exe",
    "vncviewer.exe",
    "tvnviewer.exe",
    "teamviewer.exe",
    "anydesk.exe",
    "rustdesk.exe",
    "parsecd.exe",
    "remote-viewer.exe",
];

const REMOTE_PACKAGES: &[&str] = &["microsoft.remotedesktop_8wekyb3d8bbwe"];

/// Folds a name for comparison: each `char` becomes its simple lowercase
/// mapping when that is exactly one `char`, else stays. Case is folded, script
/// is not (a Cyrillic `о` stays), and the result is the same on every OS.
#[must_use]
pub fn fold_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(one), None) => one,
                _ => c,
            }
        })
        .collect()
}

/// Whether two paths are the same after [`fold_name`].
#[must_use]
pub fn paths_equal(a: &str, b: &str) -> bool {
    fold_name(a) == fold_name(b)
}

/// The exclusion class of a program, by file name (any signer) or package
/// family. Names are folded here again, so the call is safe with any case.
#[must_use]
pub fn classify_program(exe_folded: &str, package_family: Option<&str>) -> ProgramClass {
    let exe = fold_name(exe_folded);
    let family = package_family.map(str::to_ascii_lowercase);
    let in_list = |list: &[&str]| list.contains(&exe.as_str());
    let in_packages = |list: &[&str]| family.as_deref().is_some_and(|f| list.contains(&f));
    if in_list(MOSAIC) {
        ProgramClass::Mosaic
    } else if in_list(BROWSERS) {
        ProgramClass::Browser
    } else if in_list(HOSTS) || in_packages(HOST_PACKAGES) {
        ProgramClass::Host
    } else if in_list(BROKERS) {
        ProgramClass::Broker
    } else if in_list(REMOTE) || in_packages(REMOTE_PACKAGES) {
        ProgramClass::Remote
    } else {
        ProgramClass::Ordinary
    }
}

/// Whether the saved URL allows filling the observed program.
///
/// A program of any class but [`ProgramClass::Ordinary`] never matches. A
/// `package/` URL needs an equal family. A `signed/` URL needs the equal signer
/// **and** the equal exe name. A `path/` URL needs an image with no trusted
/// signature, an equal path and an equal file hash. A match is reported as
/// [`MatchKind::ExactOrigin`]; a web or app URL never matches a program.
///
/// # Errors
/// [`Refusal::NoMatch`].
pub fn program_fill_allowed(
    saved: &LoginUrl,
    observed: &ProgramIdentity<'_>,
) -> Result<MatchKind, Refusal> {
    // The class is the caller's reading; the names are checked again here.
    if observed.class != ProgramClass::Ordinary
        || classify_program(observed.exe_name, observed.package_family) != ProgramClass::Ordinary
    {
        return Err(Refusal::NoMatch);
    }
    let url = ProgramUrl::parse(saved.url.as_str()).map_err(|_| Refusal::NoMatch)?;
    let matches = match &url {
        ProgramUrl::Package { family } => observed.package_family == Some(family.as_str()),
        ProgramUrl::Signed { exe, signer } => {
            observed.signer_subject_sha256 == Some(*signer) && fold_name(observed.exe_name) == *exe
        }
        ProgramUrl::Path { path, sha256 } => {
            observed.signer_subject_sha256.is_none()
                && observed.file_sha256 == Some(*sha256)
                && paths_equal(observed.final_path, path)
        }
    };
    if matches {
        Ok(MatchKind::ExactOrigin)
    } else {
        Err(Refusal::NoMatch)
    }
}

/// Whether the saved URL was bound to this very program but the thing that
/// binds it changed: the signer of a `signed/` URL's exe, or the file at a
/// `path/` URL's path. The user is asked to link again; nothing is filled.
#[must_use]
pub fn binding_change(saved: &LoginUrl, observed: &ProgramIdentity<'_>) -> Option<BindingChange> {
    if observed.class != ProgramClass::Ordinary {
        return None;
    }
    match ProgramUrl::parse(saved.url.as_str()).ok()? {
        ProgramUrl::Signed { exe, signer } => (fold_name(observed.exe_name) == exe
            && observed.signer_subject_sha256.is_some_and(|s| s != signer))
        .then_some(BindingChange::SignerChanged),
        ProgramUrl::Path { path, sha256 } => (paths_equal(observed.final_path, &path)
            && observed.file_sha256.is_some_and(|h| h != sha256))
        .then_some(BindingChange::FileChanged),
        ProgramUrl::Package { .. } => None,
    }
}

/// An already bound exe name that `exe_folded` is one edit away from (one
/// substitution, swap, insertion or deletion in the names without `.exe`, both
/// at least 4 characters, so a homoglyph counts). The first such name in
/// lexicographic order. A warning when linking, never a match.
#[must_use]
pub fn program_lookalike_of<'a>(
    exe_folded: &str,
    bound_exes: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let stem = |name: &str| name.strip_suffix(".exe").unwrap_or(name).to_owned();
    let mine = stem(exe_folded);
    if mine.chars().count() < 4 {
        return None;
    }
    bound_exes
        .into_iter()
        .filter(|bound| {
            let theirs = stem(bound);
            theirs.chars().count() >= 4 && theirs != mine && osa_distance_is_one(&mine, &theirs)
        })
        .min()
}

fn is_valid_family(family: &str) -> bool {
    let Some((name, publisher)) = family.split_once('_') else {
        return false;
    };
    (3..=50).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && publisher.len() == 13
        && publisher.bytes().all(|b| PUBLISHER_ALPHABET.contains(&b))
}

fn is_valid_exe(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_EXE_NAME_LEN
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || FORBIDDEN_NAME_CHARS.contains(c))
}

/// The part of a final path after its drive or UNC prefix.
fn path_after_prefix(path: &str) -> Option<&str> {
    let drive = path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if drive && path.get(1..3) == Some(":\\") {
        return path.get(3..);
    }
    // `\\server\share\...`: both parts non-empty, then a separator.
    let unc = path.strip_prefix("\\\\")?;
    let mut parts = unc.splitn(3, '\\');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(server), Some(share), Some(rest)) if !server.is_empty() && !share.is_empty() => {
            Some(rest)
        }
        _ => None,
    }
}

fn is_valid_path(path: &str) -> bool {
    if path.is_empty() || path.len() > MAX_PROGRAM_PATH_LEN {
        return false;
    }
    if path.starts_with("\\\\?\\") || path.starts_with("\\\\.\\") {
        return false;
    }
    if path.chars().any(|c| c.is_control() || c == '/') {
        return false;
    }
    let Some(rest) = path_after_prefix(path) else {
        return false;
    };
    // After the prefix no colon, wildcard, quote or pipe is allowed. Windows
    // strips a trailing dot or space and resolves `.` and `..`: a final path
    // has none of them.
    !rest.chars().any(|c| ":*?\"<>|".contains(c))
        && !rest
            .split('\\')
            .any(|part| part == "." || part == ".." || part.ends_with('.') || part.ends_with(' '))
}

fn decode_hash(text: &str) -> Option<[u8; 32]> {
    if text.len() != 43 {
        return None;
    }
    let mut bytes = [0u8; 32];
    let decoded = Base64UrlUnpadded::decode(text, &mut bytes).ok()?;
    (decoded.len() == 32 && Base64UrlUnpadded::encode_string(&bytes) == text).then_some(bytes)
}

const fn is_unreserved(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

fn encode_exe(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        if is_unreserved(b) {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.extend(char::from_digit(u32::from(b >> 4), 16).map(|c| c.to_ascii_uppercase()));
            out.extend(char::from_digit(u32::from(b & 0x0F), 16).map(|c| c.to_ascii_uppercase()));
        }
    }
    out
}

/// Decodes `%XX` (uppercase hex) and unreserved bytes; anything else is refused.
fn decode_exe(text: &str) -> Option<String> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = upper_hex(bytes.next()?)?;
            let lo = upper_hex(bytes.next()?)?;
            out.push(hi << 4 | lo);
        } else if is_unreserved(b) {
            out.push(b);
        } else {
            return None;
        }
    }
    String::from_utf8(out).ok()
}

const fn upper_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

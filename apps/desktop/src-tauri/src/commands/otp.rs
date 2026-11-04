//! One-time-code setup commands (ADR-0031). Scans and parses run without the
//! session lock; the result is kept only if the vault is still unlocked.
//!
//! Rust captures, reads and decodes; the parsed secret stays in a pending
//! slot (`crate::otp_pending`) behind an opaque ID. The webview gets a
//! preview without the secret, the live code, and saves with
//! `OtpChange::SetPending`.

use std::fs::File;
use std::io::Read as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime};

use mosaic_core::SecretString;
use mosaic_core::model::OtpConfig;
use mosaic_qr::{Frame, ImageLimits, Payload, ScanError};
use mosaic_totp::{
    Algorithm, Digits, Otp, OtpAuth, OtpKind, OtpSecret, Period, SecretStrength, SecretText,
    validate_label,
};
use mosaic_vault::otp::config_from_otp;
use tauri::State;
use zeroize::Zeroizing;

use super::clipboard::now_unix_seconds;
use crate::clipboard::ClipboardImage;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::{OtpChange, OtpScheme};
use crate::contract::otp::{
    OtpAlgorithm, OtpParseRequest, OtpPending, OtpPendingRequest, OtpScanRequest, OtpScanResult,
    OtpScanSource, OtpSecretStrength,
};
use crate::contract::secret::SecretOut;
use crate::contract::totp::TotpCode;
use crate::dialogs;
use crate::otp_pending::{Clocks, PENDING_TTL, PendingOtp};
use crate::state::{AppState, Backend, guard};

/// `otp_scan`.
#[tauri::command]
pub async fn otp_scan(
    state: State<'_, AppState>,
    request: OtpScanRequest,
) -> IpcResult<OtpScanResult> {
    state.run(move |b| b.otp_scan(&request)).await
}

/// `otp_parse`.
#[tauri::command]
pub async fn otp_parse(
    state: State<'_, AppState>,
    request: OtpParseRequest,
) -> IpcResult<OtpPending> {
    state.run(move |b| b.otp_parse(request)).await
}

/// `otp_pending_code`.
#[tauri::command]
pub async fn otp_pending_code(
    state: State<'_, AppState>,
    request: OtpPendingRequest,
) -> IpcResult<TotpCode> {
    state.run(move |b| b.otp_pending_code(&request)).await
}

/// `otp_pending_discard`.
#[tauri::command]
pub async fn otp_pending_discard(
    state: State<'_, AppState>,
    request: OtpPendingRequest,
) -> IpcResult<()> {
    state.run(move |b| b.otp_pending_discard(&request)).await
}

/// Clears `otp_scanning` on drop.
struct ScanGuard<'a>(&'a AtomicBool);

impl<'a> ScanGuard<'a> {
    /// Takes the flag.
    ///
    /// # Errors
    /// `busy` if a scan is running.
    fn acquire(flag: &'a AtomicBool) -> IpcResult<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| IpcError::new(ErrorCode::Busy))?;
        Ok(Self(flag))
    }
}

impl Drop for ScanGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// `mosaic_qr::ScanError` to its error code; a future variant is `internal`.
fn scan_error(error: ScanError) -> IpcError {
    let code = match error {
        ScanError::NotFound => ErrorCode::OtpQrNotFound,
        ScanError::NotOtp => ErrorCode::OtpQrNotOtp,
        ScanError::Migration => ErrorCode::OtpQrMigration,
        ScanError::Multiple => ErrorCode::OtpQrMultiple,
        ScanError::InvalidOtp => ErrorCode::OtpQrInvalid,
        ScanError::Unreadable => ErrorCode::OtpImageUnreadable,
        ScanError::TooLarge => ErrorCode::OtpImageTooLarge,
        _ => ErrorCode::Internal,
    };
    IpcError::new(code)
}

fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidInput)
}

fn clocks() -> Clocks {
    (Instant::now(), SystemTime::now())
}

/// The clipboard image limit (the pixel count of the default image limits),
/// checked before the frame is searched.
///
/// # Errors
/// `otpImageTooLarge`.
pub(crate) fn check_clipboard_pixels(width: u32, height: u32) -> IpcResult<()> {
    if u64::from(width) * u64::from(height) > ImageLimits::DEFAULT.max_pixels {
        return Err(IpcError::new(ErrorCode::OtpImageTooLarge));
    }
    Ok(())
}

/// Empty text is no label.
fn non_empty(label: Option<&str>) -> Option<&str> {
    label.filter(|l| !l.is_empty())
}

/// The preview of a parsed setup (without the pending ID, which the caller
/// sets).
fn preview(auth: &OtpAuth) -> OtpPending {
    let (scheme, digits, period) = match auth.otp.kind {
        OtpKind::Totp { digits, period } => (OtpScheme::Totp, digits.get(), Some(period.get())),
        OtpKind::Hotp { digits, .. } => (OtpScheme::Hotp, digits.get(), None),
        OtpKind::Steam { period } => (OtpScheme::Steam, 5, Some(period.get())),
    };
    OtpPending {
        pending_id: String::new(),
        scheme,
        issuer: auth.issuer.as_ref().map(|i| i.expose_secret().to_owned()),
        account_name: non_empty(Some(auth.account_name.expose_secret())).map(str::to_owned),
        algorithm: match auth.otp.algorithm {
            Algorithm::Sha1 => OtpAlgorithm::Sha1,
            Algorithm::Sha256 => OtpAlgorithm::Sha256,
            Algorithm::Sha512 => OtpAlgorithm::Sha512,
        },
        digits,
        period,
        strength: match auth.otp.secret.strength() {
            SecretStrength::Weak => OtpSecretStrength::Weak,
            SecretStrength::BelowRfc4226 => OtpSecretStrength::BelowRfc4226,
            SecretStrength::Rfc4226 => OtpSecretStrength::Rfc4226,
        },
        expires_in_seconds: u32::try_from(PENDING_TTL.as_secs()).unwrap_or(u32::MAX),
    }
}

/// A pending entry taken for a save, with the labels to store.
pub(crate) struct TakenOtp {
    id: String,
    entry: PendingOtp,
    issuer: Option<String>,
    account: Option<String>,
}

impl TakenOtp {
    /// The stored configuration and HOTP counter.
    ///
    /// # Errors
    /// `invalidInput`.
    pub(crate) fn config(&self) -> IpcResult<(OtpConfig, u64)> {
        config_from_otp(
            &self.entry.auth.otp,
            self.issuer.as_deref(),
            self.account.as_deref(),
        )
        .map_err(|_| invalid())
    }
}

/// Reads a whole image file (at most the limit) into a zeroizing buffer.
///
/// # Errors
/// `otpImageUnreadable`, `otpImageTooLarge`.
fn read_image_file(path: &std::path::Path) -> IpcResult<Zeroizing<Vec<u8>>> {
    let limit = ImageLimits::DEFAULT.max_file_bytes;
    let unreadable = || IpcError::new(ErrorCode::OtpImageUnreadable);
    let too_large = || IpcError::new(ErrorCode::OtpImageTooLarge);
    let len = std::fs::metadata(path).map_err(|_| unreadable())?.len();
    let len = usize::try_from(len).map_err(|_| too_large())?;
    if len > limit {
        return Err(too_large());
    }
    let file = File::open(path).map_err(|_| unreadable())?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(len));
    let wanted = u64::try_from(limit)
        .map_err(|_| too_large())?
        .saturating_add(1);
    file.take(wanted)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable())?;
    if bytes.len() > limit {
        return Err(too_large());
    }
    Ok(bytes)
}

impl Backend {
    /// Fails with `locked` unless the vault is unlocked right now.
    fn require_unlocked(&self) -> IpcResult<()> {
        self.slot()?.unlocked().map(|_| ())
    }

    /// Captures every display and looks for a setup code.
    fn scan_screen(&self) -> IpcResult<OtpAuth> {
        if !self.screen.supported() {
            return Err(IpcError::new(ErrorCode::OtpScanUnsupported));
        }
        let frames = self.screen.capture()?;
        let views: Vec<Frame<'_>> = frames.iter().map(mosaic_qr::OwnedFrame::frame).collect();
        let result = mosaic_qr::scan_frames(&views);
        // Wiped before anything else happens.
        drop(views);
        drop(frames);
        result.map_err(scan_error)
    }

    /// Looks for a setup code in the clipboard image.
    fn scan_clipboard(&self) -> IpcResult<OtpAuth> {
        match self.clipboard.read_image()? {
            ClipboardImage::Empty => Err(IpcError::new(ErrorCode::OtpClipboardNoImage)),
            ClipboardImage::Unreadable => Err(IpcError::new(ErrorCode::OtpImageUnreadable)),
            ClipboardImage::Image(frame) => {
                check_clipboard_pixels(frame.width(), frame.height())?;
                mosaic_qr::scan_frames(&[frame.frame()]).map_err(scan_error)
            }
        }
    }

    /// Asks for an image file and looks for a setup code in it (`None`: the
    /// dialog was dismissed). The path is never logged or returned.
    fn scan_file(&self) -> IpcResult<Option<OtpAuth>> {
        let Some(path) = self.dialogs.open_file(dialogs::IMAGES)? else {
            return Ok(None);
        };
        let bytes = read_image_file(&path)?;
        let auth = mosaic_qr::scan_image(&bytes, &ImageLimits::DEFAULT).map_err(scan_error);
        drop(bytes);
        auth.map(Some)
    }

    /// Looks for a one-time-code setup code on the screen, in a clipboard
    /// screenshot or in an image file, and holds the result for the
    /// confirmation.
    ///
    /// # Errors
    /// `locked`, `busy`, `otpScanUnsupported`, `clipboardUnavailable`,
    /// `otpClipboardNoImage`, `otpImageUnreadable`, `otpImageTooLarge`,
    /// `otpQrNotFound`, `otpQrNotOtp`, `otpQrMigration`, `otpQrMultiple`,
    /// `otpQrInvalid`.
    pub fn otp_scan(&self, request: &OtpScanRequest) -> IpcResult<OtpScanResult> {
        self.touch();
        self.require_unlocked()?;
        let _scan = ScanGuard::acquire(&self.otp_scanning)?;
        let mut clipboard_cleared = false;
        let auth = match request.source {
            OtpScanSource::Screen => self.scan_screen()?,
            OtpScanSource::Clipboard => {
                let auth = self.scan_clipboard()?;
                clipboard_cleared = self.clipboard.clear_any().is_ok();
                auth
            }
            OtpScanSource::File => match self.scan_file()? {
                Some(auth) => auth,
                None => return Ok(OtpScanResult::Cancelled),
            },
        };
        // The vault may have locked during the scan: the setup is dropped.
        self.require_unlocked()?;
        Ok(OtpScanResult::Found {
            pending: self.hold_pending(auth)?,
            clipboard_cleared,
        })
    }

    /// Parses typed or pasted setup text and holds the result.
    ///
    /// # Errors
    /// `locked`, `otpUriInvalid`, `otpQrMigration`, `otpKeyInvalid`,
    /// `invalidInput`.
    pub fn otp_parse(&self, request: OtpParseRequest) -> IpcResult<OtpPending> {
        self.touch();
        self.require_unlocked()?;
        let auth = match request {
            OtpParseRequest::Uri { uri } => {
                let text = uri.expose_secret().trim_ascii();
                match mosaic_qr::classify(text) {
                    Payload::Migration => {
                        return Err(IpcError::new(ErrorCode::OtpQrMigration));
                    }
                    Payload::OtpAuth => {
                        OtpAuth::parse(text).map_err(|_| IpcError::new(ErrorCode::OtpUriInvalid))?
                    }
                    Payload::Other => return Err(IpcError::new(ErrorCode::OtpUriInvalid)),
                }
            }
            OtpParseRequest::Key {
                secret,
                issuer,
                account_name,
                algorithm,
                digits,
                period,
            } => parse_key(
                secret.expose_secret(),
                issuer.as_deref(),
                account_name.as_deref(),
                algorithm,
                digits,
                period,
            )?,
        };
        self.require_unlocked()?;
        self.hold_pending(auth)
    }

    /// The code of a pending setup, with its seconds remaining and period
    /// (`None` for HOTP is refused earlier).
    ///
    /// # Errors
    /// `locked`, `otpPendingExpired`, `invalidInput` (HOTP: the counter must
    /// not move before saving), `internal`.
    pub(crate) fn pending_code_parts(
        &self,
        pending_id: &str,
    ) -> IpcResult<(SecretString, Option<u32>, Option<u32>)> {
        self.require_unlocked()?;
        let mut pending = guard(&self.otp_pending)?;
        let entry = pending
            .get(pending_id, clocks())
            .ok_or(IpcError::new(ErrorCode::OtpPendingExpired))?;
        let otp = &entry.auth.otp;
        let period = match otp.kind {
            OtpKind::Hotp { .. } => return Err(invalid()),
            OtpKind::Totp { period, .. } | OtpKind::Steam { period } => period.get(),
        };
        let now = now_unix_seconds()?;
        let code = otp.code_at(now).map_err(|_| IpcError::INTERNAL)?;
        Ok((
            SecretString::from_str_copy(code.expose_secret())?,
            otp.seconds_remaining(now),
            Some(period),
        ))
    }

    /// The live code of a pending setup, for the confirmation screen. A
    /// background refresh, so it does not count as user activity.
    ///
    /// # Errors
    /// `locked`, `otpPendingExpired`, `invalidInput` (HOTP), `internal`.
    pub fn otp_pending_code(&self, request: &OtpPendingRequest) -> IpcResult<TotpCode> {
        let (code, seconds_remaining, period) = self.pending_code_parts(&request.pending_id)?;
        Ok(TotpCode {
            code: SecretOut::new(code),
            seconds_remaining,
            period,
        })
    }

    /// Forgets a pending setup. Unknown IDs, and a locked vault, are fine.
    ///
    /// # Errors
    /// `internal` on a poisoned mutex.
    pub fn otp_pending_discard(&self, request: &OtpPendingRequest) -> IpcResult<()> {
        guard(&self.otp_pending)?.remove(&request.pending_id);
        Ok(())
    }

    /// For `add_item` and `update_item`: if `change` is `SetPending`,
    /// validates the labels (empty means none; a failure takes nothing) and
    /// takes the entry. Other changes give `None`.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (labels), `otpPendingExpired`.
    pub(crate) fn take_pending_otp(
        &self,
        change: Option<&OtpChange>,
    ) -> IpcResult<Option<TakenOtp>> {
        let Some(OtpChange::SetPending {
            pending_id,
            issuer,
            account_name,
        }) = change
        else {
            return Ok(None);
        };
        self.require_unlocked()?;
        let issuer = non_empty(issuer.as_deref());
        let account = non_empty(account_name.as_deref());
        validate_label(issuer, account.unwrap_or("")).map_err(|_| invalid())?;
        let entry = guard(&self.otp_pending)?
            .take(pending_id, clocks())
            .ok_or(IpcError::new(ErrorCode::OtpPendingExpired))?;
        Ok(Some(TakenOtp {
            id: pending_id.clone(),
            entry,
            issuer: issuer.map(str::to_owned),
            account: account.map(str::to_owned),
        }))
    }

    /// After a save: a failed one puts the taken entry back, a successful one
    /// has consumed it.
    pub(crate) fn finish_pending_otp(&self, taken: Option<TakenOtp>, saved: bool) {
        if !saved && let Some(taken) = taken {
            self.restore_pending_otp(taken);
        }
    }

    /// Puts a taken entry back after a failed save (not into a locked vault).
    pub(crate) fn restore_pending_otp(&self, taken: TakenOtp) {
        if self.is_unlocked()
            && let Ok(mut pending) = self.otp_pending.lock()
        {
            pending.restore(taken.id, taken.entry);
        }
    }

    /// Stores `auth` and builds its preview.
    fn hold_pending(&self, auth: OtpAuth) -> IpcResult<OtpPending> {
        let mut preview = preview(&auth);
        preview.pending_id = guard(&self.otp_pending)?.insert(auth, clocks());
        Ok(preview)
    }
}

/// Builds a TOTP from a typed base32 key (spaces and hyphens are ignored)
/// and the advanced settings.
fn parse_key(
    secret: &str,
    issuer: Option<&str>,
    account: Option<&str>,
    algorithm: OtpAlgorithm,
    digits: u8,
    period: u32,
) -> IpcResult<OtpAuth> {
    let mut clean = Zeroizing::new(String::with_capacity(secret.len()));
    clean.extend(
        secret
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-'),
    );
    let secret =
        OtpSecret::from_base32(&clean).map_err(|_| IpcError::new(ErrorCode::OtpKeyInvalid))?;
    let digits = Digits::new(digits).map_err(|_| invalid())?;
    let period = Period::new(period).map_err(|_| invalid())?;
    let issuer = non_empty(issuer.map(str::trim));
    let account = non_empty(account.map(str::trim));
    validate_label(issuer, account.unwrap_or("")).map_err(|_| invalid())?;
    Ok(OtpAuth {
        otp: Otp {
            secret,
            algorithm: match algorithm {
                OtpAlgorithm::Sha1 => Algorithm::Sha1,
                OtpAlgorithm::Sha256 => Algorithm::Sha256,
                OtpAlgorithm::Sha512 => Algorithm::Sha512,
            },
            kind: OtpKind::Totp { digits, period },
        },
        issuer: issuer.map(SecretText::new),
        account_name: SecretText::new(account.unwrap_or("")),
    })
}

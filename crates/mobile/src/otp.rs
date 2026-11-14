//! One-time passwords: the current code of an item, and the setup of a new
//! generator (typed text or a camera frame). A parsed setup waits in a pending
//! slot under an opaque ID; its secret never leaves Rust (ADR-0031 item 2).

use std::sync::Arc;

use mosaic_core::model::OtpKind as StoredKind;
use mosaic_qr::{Frame, Payload, PixelLayout, ScanError, classify};
use mosaic_totp::{Algorithm, Digits, Otp, OtpAuth, OtpKind, OtpSecret, Period, SecretText};
use zeroize::Zeroizing;

use crate::Core;
use crate::error::{MobileError, Result};
use crate::items::parse_item_key;
use crate::secret::{SecretValue, text_of};
use crate::state::Phase;

/// How long a pending setup waits for its confirmation (ADR-0031 item 2).
pub(crate) const PENDING_TTL_MS: u64 = 600_000;
/// Most pending setups held at once; the oldest is dropped for a new one.
pub(crate) const MAX_PENDING: usize = 4;

/// A camera frame of grey levels (the Y plane). The pixels are wiped after the
/// scan.
pub struct LumaFrame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bytes per row.
    pub stride: u32,
    /// `stride * height` bytes (the last row may be short).
    pub pixels: Vec<u8>,
}

/// The scheme of a generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtpScheme {
    /// Time-based, RFC 6238.
    Totp,
    /// Counter-based, RFC 4226.
    Hotp,
    /// Steam Guard.
    Steam,
}

/// A parsed setup, without its secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtpPreview {
    /// The opaque ID of the pending slot.
    pub pending_id: String,
    /// The scheme.
    pub scheme: OtpScheme,
    /// The issuer.
    pub issuer: Option<String>,
    /// The account name.
    pub account_name: Option<String>,
    /// Code length.
    pub digits: u8,
    /// Time step in seconds (`None` for HOTP).
    pub period_s: Option<u32>,
    /// Seconds until the pending slot is dropped.
    pub expires_in_s: u32,
}

/// The generator of an item, without its secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtpView {
    /// The scheme.
    pub scheme: OtpScheme,
    /// Code length.
    pub digits: u8,
    /// Time step in seconds (`None` for HOTP).
    pub period_s: Option<u32>,
    /// The issuer.
    pub issuer: Option<String>,
    /// The account label.
    pub label: Option<String>,
}

/// The current code of an item.
pub struct OtpCode {
    /// The code, as ASCII text.
    pub code: Arc<SecretValue>,
    /// Time step in seconds (0 for HOTP).
    pub period_s: u32,
    /// Seconds until the code changes (0 for HOTP).
    pub remaining_s: u32,
    /// Code length.
    pub digits: u8,
}

struct Pending {
    id: String,
    auth: OtpAuth,
    created_ms: u64,
}

/// The pending setups.
#[derive(Default)]
pub(crate) struct PendingOtps {
    entries: Vec<Pending>,
}

impl PendingOtps {
    fn drop_expired(&mut self, now_ms: u64) {
        self.entries
            .retain(|e| now_ms < e.created_ms.saturating_add(PENDING_TTL_MS));
    }

    /// Stores `auth` and returns its new opaque ID (16 random bytes in hex).
    pub(crate) fn insert(&mut self, auth: OtpAuth, now_ms: u64) -> Result<String> {
        let bytes = mosaic_core::random_public_bytes::<16>()?;
        let id = crate::storage::hex(&bytes);
        self.drop_expired(now_ms);
        while self.entries.len() >= MAX_PENDING {
            self.entries.remove(0);
        }
        self.entries.push(Pending {
            id: id.clone(),
            auth,
            created_ms: now_ms,
        });
        Ok(id)
    }

    /// Removes and returns the entry unless it expired.
    pub(crate) fn take(&mut self, id: &str, now_ms: u64) -> Option<OtpAuth> {
        self.drop_expired(now_ms);
        let at = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(at).auth)
    }

    /// Whether an unexpired entry with this ID exists.
    pub(crate) fn contains(&mut self, id: &str, now_ms: u64) -> bool {
        self.drop_expired(now_ms);
        self.entries.iter().any(|e| e.id == id)
    }

    /// Forgets one entry.
    pub(crate) fn remove(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    /// Forgets every entry (lock).
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

fn scheme_of(kind: &OtpKind) -> (OtpScheme, u8, Option<u32>) {
    match kind {
        OtpKind::Totp { digits, period } => (OtpScheme::Totp, digits.get(), Some(period.get())),
        OtpKind::Hotp { digits, .. } => (OtpScheme::Hotp, digits.get(), None),
        OtpKind::Steam { period } => (OtpScheme::Steam, 5, Some(period.get())),
    }
}

/// The stored generator as the detail shows it.
pub(crate) fn view_of(config: &mosaic_core::model::OtpConfig) -> OtpView {
    let (scheme, period_s) = match config.kind {
        StoredKind::Totp => (OtpScheme::Totp, Some(config.period)),
        StoredKind::Hotp => (OtpScheme::Hotp, None),
        StoredKind::Steam => (OtpScheme::Steam, Some(config.period)),
    };
    OtpView {
        scheme,
        digits: config.digits,
        period_s,
        issuer: config.issuer.as_ref().and_then(|t| non_empty(t.as_str())),
        label: config.label.as_ref().and_then(|t| non_empty(t.as_str())),
    }
}

fn scan_error(e: ScanError) -> MobileError {
    match e {
        ScanError::NotOtp | ScanError::NotSetupCode => MobileError::OtpQrNotOtp,
        ScanError::Migration => MobileError::OtpQrMigration,
        ScanError::Multiple => MobileError::OtpQrMultiple,
        ScanError::InvalidOtp => MobileError::OtpQrInvalid,
        ScanError::NotFound => MobileError::QrNotFound,
        _ => MobileError::InvalidInput,
    }
}

/// A typed base32 key (spaces and hyphens ignored) as a default TOTP.
fn parse_key(text: &str) -> Result<OtpAuth> {
    let mut clean = Zeroizing::new(String::with_capacity(text.len()));
    clean.extend(
        text.chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-'),
    );
    let secret = OtpSecret::from_base32(&clean).map_err(|_| MobileError::OtpKeyInvalid)?;
    Ok(OtpAuth {
        otp: Otp {
            secret,
            algorithm: Algorithm::Sha1,
            kind: OtpKind::Totp {
                digits: Digits::DEFAULT,
                period: Period::DEFAULT,
            },
        },
        issuer: None,
        account_name: SecretText::new(""),
    })
}

impl Core {
    fn hold_pending(&self, auth: OtpAuth) -> Result<OtpPreview> {
        let (scheme, digits, period_s) = scheme_of(&auth.otp.kind);
        let issuer = auth
            .issuer
            .as_ref()
            .and_then(|i| non_empty(i.expose_secret()));
        let account_name = non_empty(auth.account_name.expose_secret());
        let now = self.now_ms();
        let mut guard = self.guard()?;
        if !matches!(guard.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        let pending_id = guard.slots.otp.insert(auth, now)?;
        Ok(OtpPreview {
            pending_id,
            scheme,
            issuer,
            account_name,
            digits,
            period_s,
            expires_in_s: u32::try_from(PENDING_TTL_MS / 1000).unwrap_or(u32::MAX),
        })
    }

    /// Parses typed or pasted setup text: an `otpauth://` URI or a base32 key.
    ///
    /// # Errors
    /// `locked`, `otpQrMigration` (a Google Authenticator export),
    /// `otpUriInvalid`, `otpKeyInvalid`, `invalidInput`.
    pub fn otp_from_text(&self, text: Vec<u8>) -> Result<OtpPreview> {
        let text = text_of(text)?;
        let trimmed = text.trim_ascii();
        let auth = match classify(trimmed) {
            Payload::Migration => return Err(MobileError::OtpQrMigration),
            Payload::OtpAuth => OtpAuth::parse(trimmed).map_err(|_| MobileError::OtpUriInvalid)?,
            Payload::Other => parse_key(trimmed)?,
        };
        self.hold_pending(auth)
    }

    /// Looks for one `otpauth://` code in a camera frame. `None`: no code in
    /// this frame. The pixels are wiped.
    ///
    /// # Errors
    /// `locked`, `otpQrNotOtp`, `otpQrMigration`, `otpQrMultiple`,
    /// `otpQrInvalid`, `invalidInput` for a malformed frame.
    pub fn scan_otp_qr(&self, frame: LumaFrame) -> Result<Option<OtpPreview>> {
        let LumaFrame {
            width,
            height,
            stride,
            pixels,
        } = frame;
        let pixels = Zeroizing::new(pixels);
        if !matches!(self.guard()?.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        let stride = usize::try_from(stride).map_err(|_| MobileError::InvalidInput)?;
        let frame = Frame::new(width, height, stride, PixelLayout::Luma8, &pixels)
            .map_err(|_| MobileError::InvalidInput)?;
        match mosaic_qr::scan_frames(&[frame]) {
            Ok(auth) => self.hold_pending(auth).map(Some),
            Err(ScanError::NotFound) => Ok(None),
            Err(e) => Err(scan_error(e)),
        }
    }

    /// Drops a pending setup.
    pub fn otp_pending_discard(&self, pending_id: String) {
        if let Ok(mut guard) = self.guard() {
            guard.slots.otp.remove(&pending_id);
        }
    }

    /// The stored generator for a pending setup, taken out of its slot.
    pub(crate) fn take_pending_otp(
        &self,
        pending_id: &str,
    ) -> Result<(mosaic_core::model::OtpConfig, u64)> {
        let now = self.now_ms();
        let auth = self
            .guard()?
            .slots
            .otp
            .take(pending_id, now)
            .ok_or(MobileError::NotFound)?;
        mosaic_vault::otp::config_from_otp_auth(&auth).map_err(MobileError::from)
    }

    /// Whether a pending setup exists (checked before an item is saved).
    pub(crate) fn pending_otp_exists(&self, pending_id: &str) -> Result<bool> {
        let now = self.now_ms();
        Ok(self.guard()?.slots.otp.contains(pending_id, now))
    }

    /// The current one-time code of an item. A counter-based code advances its
    /// counter before the code is returned.
    ///
    /// # Errors
    /// `locked`, `notFound` (no item, or no generator), `invalidInput`.
    pub fn otp_code(&self, vault_id: &str, item_id: &str) -> Result<OtpCode> {
        let generation = self.generation();
        let (vault, item) = parse_item_key(vault_id, item_id)?;
        let now = self.now_s();
        let (text, remaining, period, digits) = self.with_session(|session| {
            let code = session.otp_code(vault, item, now)?;
            let config = session.item(vault, item)?.otp();
            let period = config.as_ref().map_or(0, |c| match c.kind {
                StoredKind::Hotp => 0,
                _ => c.period,
            });
            let text = Zeroizing::new(code.code.expose_secret().to_owned());
            let digits = u8::try_from(text.len()).unwrap_or(u8::MAX);
            Ok((text, code.seconds_remaining.unwrap_or(0), period, digits))
        })?;
        Ok(OtpCode {
            code: self.issue_gated(SecretValue::from_text(&text), generation)?,
            period_s: period,
            remaining_s: remaining,
            digits,
        })
    }
}

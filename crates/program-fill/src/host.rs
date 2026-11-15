//! The host of program fill (ADR-0164): what the desktop service asks of the
//! platform. On Windows `WindowsHost` implements it with one worker thread; on
//! every OS the fake in `crate::fake` stands in for it in tests.

use core::fmt;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use mosaic_core::SecretString;

use crate::detect::FieldRole;
use crate::fill::{Clock, Continuation, FillOutcome};
use crate::identity::{CaptureRefusal, CapturedIdentity};
use crate::verify::{ProbeError, Rect, RuntimeId};

/// Names one captured field; a new capture gets a new token, and a request
/// with an old token is refused.
pub type Token = u64;

/// A login field the host found, with the program behind it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldOffer {
    /// The capture's token.
    pub token: Token,
    /// The program.
    pub identity: CapturedIdentity,
    /// What the field is.
    pub role: FieldRole,
    /// The field's screen rectangle.
    pub rect: Rect,
    /// The focused element.
    pub target: RuntimeId,
}

/// What the host reports.
#[derive(Clone, PartialEq, Eq, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "short-lived values handed to a sink one at a time; boxing the offer would only add an allocation"
)]
pub enum HostEvent {
    /// A login field has focus.
    Field(FieldOffer),
    /// The previous field lost focus (or none qualifies).
    Gone,
    /// The program is refused; `near` is the focused element's rectangle.
    Refused {
        /// Why.
        refusal: CaptureRefusal,
        /// Where the focused element is, when it was read.
        near: Option<Rect>,
    },
    /// Focus is not in a login field.
    NoField {
        /// Where the focused element is, when it was read.
        near: Option<Rect>,
    },
    /// An armed continuation matched: the password field of the second step.
    Continue {
        /// The new capture's token.
        token: Token,
    },
}

/// What to write for one pick. The password never leaves this struct except
/// into the shim's buffers.
pub struct FillRequest {
    /// The capture to fill.
    pub token: Token,
    /// The user name.
    pub username: Option<String>,
    /// The password.
    pub password: Option<SecretString>,
    /// Whether Enter is pressed after the password.
    pub submit: bool,
    /// The monotonic time of the pick, in milliseconds.
    pub picked_at_ms: u64,
    /// Whether this is the second step of a two-step login.
    pub continuation: bool,
}

impl fmt::Debug for FillRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FillRequest")
            .field("token", &self.token)
            .field("submit", &self.submit)
            .field("continuation", &self.continuation)
            .finish_non_exhaustive()
    }
}

/// Receives the host's events (from the host's own thread).
pub type HostSink = Arc<dyn Fn(HostEvent) + Send + Sync>;

/// The platform side of program fill.
pub trait Host: Send + Sync {
    /// Starts watching focus; events go to `sink`.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn start(&self, sink: HostSink) -> Result<(), ProbeError>;
    /// Stops watching and releases everything held.
    fn stop(&self);
    /// Captures the focused field at once (the hotkey): `Field`, `Refused` or
    /// `NoField`.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn capture_now(&self) -> Result<HostEvent, ProbeError>;
    /// Whether the capture's window holds exactly one usable edit, the focused
    /// one.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn lone_text(&self, token: Token) -> Result<bool, ProbeError>;
    /// Fills the capture; re-verified before every write.
    ///
    /// # Errors
    /// [`ProbeError`] (an unknown or old token is `Gone`).
    fn fill(&self, request: FillRequest) -> Result<FillOutcome, ProbeError>;
    /// Puts focus back into the capture's field.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn refocus(&self, token: Token) -> Result<(), ProbeError>;
    /// The capture's field rectangle now; `None` when it has none.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn field_rect(&self, token: Token) -> Result<Option<Rect>, ProbeError>;
    /// Arms the second step of a two-step login after `token` was filled.
    fn arm_continuation(&self, token: Token, continuation: Continuation);
    /// Whether the five probe points of `rect` resolve to the window `own_hwnd`.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn window_clear(&self, own_hwnd: isize, rect: Rect) -> Result<bool, ProbeError>;
}

/// Milliseconds on a process-wide monotonic clock (the same base for the
/// desktop service's pick time and the worker's checks).
#[must_use]
pub fn monotonic_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The real clock of a fill: [`monotonic_ms`] and `thread::sleep`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        monotonic_ms()
    }

    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fill_request_never_prints_its_values() {
        let request = FillRequest {
            token: 9,
            username: Some("alice@example.com".to_owned()),
            password: Some(SecretString::from_str_copy("hunter2-secret").unwrap()),
            submit: true,
            picked_at_ms: 5,
            continuation: false,
        };
        let text = format!("{request:?}");
        assert!(text.contains("token"));
        assert!(!text.contains("alice"));
        assert!(!text.contains("hunter2"));
    }
}

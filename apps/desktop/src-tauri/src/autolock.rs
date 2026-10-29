//! Auto-lock (brief §6, UX_GUIDELINES §8): on idle, on system sleep and on
//! screen lock, each configurable in `Settings`.
//!
//! What triggers a lock, per OS:
//!
//! | OS | Idle | Lock on resume (clock gap) | Before sleep | Screen lock / user switch |
//! |---|---|---|---|---|
//! | Linux | yes | yes | logind `PrepareForSleep` | logind `Session.Lock`, `org.freedesktop.ScreenSaver` and `org.gnome.ScreenSaver` `ActiveChanged` |
//! | Windows | yes | yes | not detected (needs unsafe FFI) | not detected (needs unsafe FFI) |
//! | macOS | yes | yes | not detected (needs unsafe FFI) | not detected (needs unsafe FFI) |
//!
//! On resume, up to one [`TICK`] (2 s) passes before the lock. App quit
//! locks through `RunEvent::Exit`.
//!
//! - **Idle** is measured inside Mosaic: the time since the last user action
//!   the Rust side sees (a command the user caused, such as a search, a
//!   reveal, a copy or an edit, and the window gaining focus). Background
//!   polls (`session_state`, `totp_code`, `get_settings`, ...) do not count.
//!   This locks no later than system-wide idle would (any use of Mosaic is
//!   also use of the system), so it is the conservative reading. Reading
//!   the OS idle time needs `GetLastInputInfo` / `IOKit` / `XScreenSaver`
//!   bindings that are `unsafe` (or unmaintained) and is not used.
//! - **Resume** is detected by a monitor thread that ticks every
//!   [`TICK`]: when the wall clock or the monotonic clock jumped more than
//!   [`SLEEP_GAP`] past the tick, the machine was suspended (or the process
//!   was frozen that long), and the vault locks with reason `sleep` before
//!   anything else runs. A false positive only locks early.
//! - **Windows and macOS sleep and screen-lock notifications**
//!   (`WM_POWERBROADCAST`, `WTSRegisterSessionNotification`, `NSWorkspace`
//!   notifications) need a window procedure hook or Objective-C callbacks
//!   with `unsafe` FFI, which this crate forbids, and Tauri does not expose
//!   them. They are a recorded gap (`docs/PROGRESS.md`): until a vetted
//!   wrapper exists, idle and resume detection cover these systems.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use crate::contract::events::LockReason;
use crate::state::Backend;

/// How often the monitor checks.
pub const TICK: Duration = Duration::from_secs(2);
/// A clock jump this far past the tick counts as a suspend.
pub const SLEEP_GAP: Duration = Duration::from_secs(30);

/// When the user last did something in Mosaic, on both clocks: the
/// monotonic clock stops during suspend on Linux and macOS, the wall clock
/// does not, so idle time is the larger of the two readings.
pub struct Activity {
    last: Mutex<(Instant, SystemTime)>,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            last: Mutex::new((Instant::now(), SystemTime::now())),
        }
    }
}

impl Activity {
    /// Records a user action now.
    pub fn touch(&self) {
        if let Ok(mut last) = self.last.lock() {
            *last = (Instant::now(), SystemTime::now());
        }
    }

    /// How long since the last user action, at `now` / `wall`.
    #[must_use]
    pub fn idle_at(&self, now: Instant, wall: SystemTime) -> Duration {
        self.last.lock().map_or(Duration::MAX, |last| {
            now.saturating_duration_since(last.0)
                .max(wall.duration_since(last.1).unwrap_or(Duration::ZERO))
        })
    }

    /// Moves the last action into the past on both clocks.
    #[cfg(test)]
    pub(crate) fn backdate(&self, by: Duration) {
        if let Ok(mut last) = self.last.lock() {
            if let Some(earlier) = last.0.checked_sub(by) {
                last.0 = earlier;
            }
            if let Some(earlier) = last.1.checked_sub(by) {
                last.1 = earlier;
            }
        }
    }

    /// Moves the last action into the past on the wall clock only (tests).
    #[cfg(test)]
    pub(crate) fn backdate_wall(&self, by: Duration) {
        if let Ok(mut last) = self.last.lock()
            && let Some(earlier) = last.1.checked_sub(by)
        {
            last.1 = earlier;
        }
    }
}

/// Detects a suspend from two clocks between ticks.
#[derive(Clone, Copy, Debug)]
pub struct SleepDetector {
    wall: SystemTime,
    mono: Instant,
}

impl SleepDetector {
    /// Starts observing at the given readings.
    #[must_use]
    pub const fn new(wall: SystemTime, mono: Instant) -> Self {
        Self { wall, mono }
    }

    /// Takes new readings; true if more than `expected + SLEEP_GAP` passed
    /// on either clock since the last ones. The monotonic clock stops during
    /// suspend on Linux and macOS, and the wall clock does not; on Windows
    /// both run on, and the tick itself arrives late.
    pub fn observe(&mut self, wall: SystemTime, mono: Instant, expected: Duration) -> bool {
        let wall_elapsed = wall.duration_since(self.wall).unwrap_or(Duration::ZERO);
        let mono_elapsed = mono.saturating_duration_since(self.mono);
        self.wall = wall;
        self.mono = mono;
        let limit = expected.saturating_add(SLEEP_GAP);
        wall_elapsed > limit || mono_elapsed > limit
    }
}

/// What one monitor tick decides.
#[must_use]
pub fn tick_decision(
    backend: &Backend,
    slept: bool,
    now: Instant,
    wall: SystemTime,
) -> Option<LockReason> {
    let settings = backend.settings_snapshot();
    // A sleep locks even while a command holds the session: `lock` waits for
    // the mutex and does nothing when the vault is already locked.
    if slept && settings.lock_on_sleep {
        return Some(LockReason::Sleep);
    }
    if !backend.is_unlocked() {
        return None;
    }
    let minutes = settings.auto_lock_minutes?;
    let limit = Duration::from_secs(u64::from(minutes).saturating_mul(60));
    (backend.activity.idle_at(now, wall) >= limit).then_some(LockReason::Idle)
}

/// The monitor thread; it ends when the backend is dropped or
/// [`Monitor::stop`] is called.
pub struct Monitor {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Monitor {
    /// Starts ticking every `tick`.
    #[must_use]
    pub fn spawn(backend: Weak<Backend>, tick: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("mosaic-autolock".into())
            .spawn(move || {
                let mut detector = SleepDetector::new(SystemTime::now(), Instant::now());
                while !flag.load(Ordering::Relaxed) {
                    thread::park_timeout(tick);
                    let Some(backend) = backend.upgrade() else {
                        return;
                    };
                    let now = Instant::now();
                    let slept = detector.observe(SystemTime::now(), now, tick);
                    if let Some(reason) = tick_decision(&backend, slept, now, SystemTime::now()) {
                        let _ = backend.lock(reason);
                    }
                }
            })
            .ok();
        Self { stop, thread }
    }

    /// Stops the thread and waits for it.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// An OS notification that may lock the vault.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OsEvent {
    /// The system is about to sleep.
    Sleep,
    /// The screen locked (or the screen saver started).
    ScreenLock,
}

impl Backend {
    /// Locks for an OS event if the settings ask for it.
    pub fn on_os_event(&self, event: OsEvent) {
        let settings = self.settings_snapshot();
        let reason = match event {
            OsEvent::Sleep if settings.lock_on_sleep => LockReason::Sleep,
            OsEvent::ScreenLock if settings.lock_on_screen_lock => LockReason::ScreenLock,
            OsEvent::Sleep | OsEvent::ScreenLock => return,
        };
        let _ = self.lock(reason);
    }
}

/// Starts the OS event sources of this platform; each delivers to `sink`
/// from its own thread. Returns the names of the sources that started.
pub fn spawn_os_sources(sink: &Arc<dyn Fn(OsEvent) + Send + Sync>) -> Vec<&'static str> {
    #[cfg(target_os = "linux")]
    {
        linux::spawn(sink)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = sink;
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    //! logind and screen-saver signals over D-Bus (zbus, blocking API).

    use std::sync::Arc;
    use std::thread;

    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::OwnedObjectPath;

    use super::OsEvent;

    type Sink = Arc<dyn Fn(OsEvent) + Send + Sync>;

    /// Listens to `signal` of `interface` at `path` on `destination` and
    /// maps each signal's body to an event.
    fn listen<F>(
        connection: Connection,
        destination: &'static str,
        path: String,
        interface: &'static str,
        signal: &'static str,
        sink: Sink,
        map: F,
    ) -> bool
    where
        F: Fn(&zbus::Message) -> Option<OsEvent> + Send + 'static,
    {
        thread::Builder::new()
            .name(format!("mosaic-{signal}"))
            .spawn(move || {
                let Ok(proxy) = Proxy::new(&connection, destination, path, interface) else {
                    return;
                };
                let Ok(signals) = proxy.receive_signal(signal) else {
                    return;
                };
                for message in signals {
                    if let Some(event) = map(&message) {
                        sink(event);
                    }
                }
            })
            .is_ok()
    }

    fn active_true(message: &zbus::Message) -> bool {
        message.body().deserialize::<bool>().unwrap_or(false)
    }

    pub(super) fn spawn(sink: &Sink) -> Vec<&'static str> {
        let mut started = Vec::new();
        if let Ok(system) = Connection::system() {
            if listen(
                system.clone(),
                "org.freedesktop.login1",
                "/org/freedesktop/login1".to_owned(),
                "org.freedesktop.login1.Manager",
                "PrepareForSleep",
                Arc::clone(sink),
                |m| active_true(m).then_some(OsEvent::Sleep),
            ) {
                started.push("logind PrepareForSleep");
            }
            let session: Option<OwnedObjectPath> = Proxy::new(
                &system,
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
            )
            .ok()
            .and_then(|manager| manager.call("GetSessionByPID", &(std::process::id(),)).ok());
            if let Some(session) = session
                && listen(
                    system,
                    "org.freedesktop.login1",
                    session.as_str().to_owned(),
                    "org.freedesktop.login1.Session",
                    "Lock",
                    Arc::clone(sink),
                    |_| Some(OsEvent::ScreenLock),
                )
            {
                started.push("logind Session.Lock");
            }
        }
        if let Ok(session_bus) = Connection::session() {
            for (destination, path, interface) in [
                (
                    "org.freedesktop.ScreenSaver",
                    "/org/freedesktop/ScreenSaver",
                    "org.freedesktop.ScreenSaver",
                ),
                (
                    "org.gnome.ScreenSaver",
                    "/org/gnome/ScreenSaver",
                    "org.gnome.ScreenSaver",
                ),
            ] {
                if listen(
                    session_bus.clone(),
                    destination,
                    path.to_owned(),
                    interface,
                    "ActiveChanged",
                    Arc::clone(sink),
                    |m| active_true(m).then_some(OsEvent::ScreenLock),
                ) {
                    started.push(interface);
                }
            }
        }
        started
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_tick_is_not_a_sleep() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let mut detector = SleepDetector::new(wall, mono);
        assert!(!detector.observe(wall + TICK, mono + TICK, TICK));
        assert!(!detector.observe(
            wall + TICK * 2 + Duration::from_secs(5),
            mono + TICK * 2 + Duration::from_secs(5),
            TICK
        ));
    }

    #[test]
    fn a_jump_on_either_clock_is_a_sleep() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        // Linux / macOS: the monotonic clock stood still.
        let mut detector = SleepDetector::new(wall, mono);
        assert!(detector.observe(wall + Duration::from_secs(600), mono + TICK, TICK));
        // Windows: both clocks ran, the tick came late.
        let mut detector = SleepDetector::new(wall, mono);
        assert!(detector.observe(
            wall + Duration::from_secs(600),
            mono + Duration::from_secs(600),
            TICK
        ));
    }

    #[test]
    fn a_wall_clock_set_backwards_is_not_a_sleep() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let mut detector = SleepDetector::new(wall, mono);
        let earlier = wall.checked_sub(Duration::from_secs(3600)).unwrap();
        assert!(!detector.observe(earlier, mono + TICK, TICK));
    }

    #[test]
    fn activity_measures_idle_time() {
        let activity = Activity::default();
        let now = Instant::now();
        let wall = SystemTime::now();
        assert!(activity.idle_at(now, wall) < Duration::from_secs(5));
        activity.backdate(Duration::from_secs(120));
        assert!(activity.idle_at(Instant::now(), SystemTime::now()) >= Duration::from_secs(120));
        activity.touch();
        assert!(activity.idle_at(Instant::now(), SystemTime::now()) < Duration::from_secs(5));
    }

    #[test]
    fn idle_counts_wall_clock_time() {
        let activity = Activity::default();
        activity.backdate_wall(Duration::from_secs(120));
        assert!(activity.idle_at(Instant::now(), SystemTime::now()) >= Duration::from_secs(120));
    }
}

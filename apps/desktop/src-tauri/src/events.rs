//! The event sink: the webview's latest `subscribe_events` channel
//! (`docs/DESKTOP_IPC.md` §4). Shared by the backend and its background
//! workers (clipboard auto-clear, auto-lock, OS event sources), so it is a
//! cheap clonable handle.

use std::sync::{Arc, Mutex};

use tauri::ipc::Channel;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::events::AppEvent;

/// Where events go: the channel of the latest subscriber, if any.
#[derive(Clone, Default)]
pub struct Events(Arc<Mutex<Option<Channel<AppEvent>>>>);

impl Events {
    /// Sets the channel (the latest subscriber wins).
    ///
    /// # Errors
    /// `internal` on a poisoned mutex.
    pub fn subscribe(&self, channel: Channel<AppEvent>) -> IpcResult<()> {
        *self.0.lock().map_err(|_| IpcError::INTERNAL)? = Some(channel);
        Ok(())
    }

    /// Sends an event to the webview, if it subscribed. Delivery failures (a
    /// reloading webview) are ignored: events are notifications, and the
    /// webview re-reads the state after subscribing.
    pub fn emit(&self, event: &AppEvent) {
        if let Ok(guard) = self.0.lock()
            && let Some(channel) = guard.as_ref()
        {
            let _ = channel.send(event.clone());
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! A channel that records what it receives, as JSON text.

    use std::sync::{Arc, Mutex};

    use tauri::ipc::{Channel, InvokeResponseBody};

    use crate::contract::events::AppEvent;

    /// The JSON of every event sent to the returned channel.
    pub(crate) type Log = Arc<Mutex<Vec<String>>>;

    pub(crate) fn recording_channel() -> (Channel<AppEvent>, Log) {
        let log: Log = Arc::default();
        let sink = Arc::clone(&log);
        let channel = Channel::new(move |body| {
            let text = match body {
                InvokeResponseBody::Json(json) => json,
                InvokeResponseBody::Raw(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            };
            sink.lock().unwrap().push(text);
            Ok(())
        });
        (channel, log)
    }
}

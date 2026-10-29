//! Native file dialogs, opened from Rust (ADR-0019: the webview gets no
//! dialog or fs permission). The Emergency Kit save and the import file
//! choice go through [`Dialogs`], so the backend stays testable without a
//! window.
//!
//! [`NativeDialogs`] uses `rfd`, the library behind `tauri-plugin-dialog`,
//! the way the plugin does: the dialog is created on the main thread and
//! awaited on another, parented to the main window. The plugin itself is
//! not used because it forces Common Controls v6, which makes Windows test
//! binaries fail to start (ADR-0019).

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc;

use tauri::{AppHandle, Manager as _, Runtime};

use crate::contract::errors::{IpcError, IpcResult};

/// A file type filter.
#[derive(Clone, Copy, Debug)]
pub struct FileFilter {
    /// What the dialog shows for it (a format name, not translated).
    pub name: &'static str,
    /// Extensions without the dot.
    pub extensions: &'static [&'static str],
}

/// PDF files.
pub const PDF: FileFilter = FileFilter {
    name: "PDF",
    extensions: &["pdf"],
};

/// The dialogs the backend can open.
pub trait Dialogs: Send + Sync {
    /// Asks where to save a file; `None` if the user cancels.
    ///
    /// # Errors
    /// `internal` if no dialog can be shown.
    fn save_file(&self, suggested_name: &str, filter: FileFilter) -> IpcResult<Option<PathBuf>>;

    /// Asks which file to open; `None` if the user cancels.
    ///
    /// # Errors
    /// `internal` if no dialog can be shown.
    fn open_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>>;
}

/// No dialogs (tests, headless): every request is cancelled.
pub struct NoDialogs;

impl Dialogs for NoDialogs {
    fn save_file(&self, _: &str, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }

    fn open_file(&self, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }
}

/// A dialog the user has not answered yet.
type Pending = Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>>;

/// Native dialogs parented to the main window.
pub struct NativeDialogs<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> NativeDialogs<R> {
    /// Dialogs for this app.
    #[must_use]
    pub const fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    fn run<F>(&self, build: F) -> IpcResult<Option<PathBuf>>
    where
        F: FnOnce(rfd::AsyncFileDialog) -> Pending + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let app = self.app.clone();
        self.app
            .run_on_main_thread(move || {
                let mut dialog = rfd::AsyncFileDialog::new();
                if let Some(window) = app.get_webview_window("main") {
                    dialog = dialog.set_parent(&window);
                }
                let future = build(dialog);
                std::thread::spawn(move || {
                    let chosen = tauri::async_runtime::block_on(future);
                    let _ = tx.send(chosen.map(|handle| handle.path().to_path_buf()));
                });
            })
            .map_err(|_| IpcError::INTERNAL)?;
        rx.recv().map_err(|_| IpcError::INTERNAL)
    }
}

impl<R: Runtime> Dialogs for NativeDialogs<R> {
    fn save_file(&self, suggested_name: &str, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        let name = suggested_name.to_owned();
        self.run(move |dialog| {
            Box::pin(
                dialog
                    .add_filter(filter.name, filter.extensions)
                    .set_file_name(name)
                    .save_file(),
            )
        })
    }

    fn open_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.run(move |dialog| {
            Box::pin(
                dialog
                    .add_filter(filter.name, filter.extensions)
                    .pick_file(),
            )
        })
    }
}

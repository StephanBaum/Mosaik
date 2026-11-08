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

/// PNG and JPEG images (QR screenshots).
pub const IMAGES: FileFilter = FileFilter {
    name: "PNG, JPEG",
    extensions: &["png", "jpg", "jpeg"],
};

/// JSON files (the passkey export).
pub const JSON: FileFilter = FileFilter {
    name: "JSON",
    extensions: &["json"],
};

/// Any file (attachments). An empty extension list adds no filter.
pub const ANY_FILE: FileFilter = FileFilter {
    name: "All files",
    extensions: &[],
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

    /// Asks which folder to use (the sync folder); `None` if the user
    /// cancels or this build has no dialogs.
    ///
    /// # Errors
    /// `internal` if no dialog can be shown.
    fn pick_folder(&self) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }

    /// Asks a yes/no question in a native dialog the webview cannot answer
    /// for the user (the sync acknowledgement, ADR-0047): an OK/Cancel
    /// dialog of the system's own kind (no custom labels, so no two labels can
    /// make either click a yes). `true` only if the user chose OK. A build
    /// without dialogs answers `false` (fails closed).
    ///
    /// # Errors
    /// `internal` if no dialog can be shown.
    fn confirm(&self, _title: &str, _message: &str) -> IpcResult<bool> {
        Ok(false)
    }

    /// Like [`Dialogs::confirm`], with the action's own verb on the
    /// confirming button ("Remove", "Leave vault") and `cancel` on the other,
    /// so the button says what it does. Native dialogs show exactly these two
    /// labels; the default asks the plain OK/Cancel question.
    ///
    /// # Errors
    /// `internal` if no dialog can be shown.
    fn confirm_action(
        &self,
        title: &str,
        message: &str,
        _action: &str,
        _cancel: &str,
    ) -> IpcResult<bool> {
        self.confirm(title, message)
    }
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

    /// A warning dialog with `buttons`; `true` only for the confirming button.
    fn ask(
        &self,
        title: &str,
        message: &str,
        buttons: rfd::MessageButtons,
        action: Option<&str>,
    ) -> IpcResult<bool> {
        let (tx, rx) = mpsc::channel();
        let app = self.app.clone();
        let (title, message) = (title.to_owned(), message.to_owned());
        self.app
            .run_on_main_thread(move || {
                let mut dialog = rfd::AsyncMessageDialog::new()
                    .set_title(title)
                    .set_description(message)
                    .set_level(rfd::MessageLevel::Warning)
                    .set_buttons(buttons);
                if let Some(window) = app.get_webview_window("main") {
                    dialog = dialog.set_parent(&window);
                }
                let future = dialog.show();
                std::thread::spawn(move || {
                    let _ = tx.send(tauri::async_runtime::block_on(future));
                });
            })
            .map_err(|_| IpcError::INTERNAL)?;
        let answer = rx.recv().map_err(|_| IpcError::INTERNAL)?;
        // A custom-labelled dialog answers with the label that was clicked:
        // only the confirming label is a yes.
        Ok(match answer {
            rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => true,
            rfd::MessageDialogResult::Custom(label) => Some(label.as_str()) == action,
            _ => false,
        })
    }
}

/// The dialog with `filter`, or unfiltered for [`ANY_FILE`].
fn with_filter(dialog: rfd::AsyncFileDialog, filter: FileFilter) -> rfd::AsyncFileDialog {
    if filter.extensions.is_empty() {
        dialog
    } else {
        dialog.add_filter(filter.name, filter.extensions)
    }
}

impl<R: Runtime> Dialogs for NativeDialogs<R> {
    fn save_file(&self, suggested_name: &str, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        let name = suggested_name.to_owned();
        self.run(move |dialog| {
            Box::pin(with_filter(dialog, filter).set_file_name(name).save_file())
        })
    }

    fn open_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.run(move |dialog| Box::pin(with_filter(dialog, filter).pick_file()))
    }

    fn pick_folder(&self) -> IpcResult<Option<PathBuf>> {
        self.run(|dialog| Box::pin(dialog.pick_folder()))
    }

    fn confirm(&self, title: &str, message: &str) -> IpcResult<bool> {
        self.ask(title, message, rfd::MessageButtons::OkCancel, None)
    }

    fn confirm_action(
        &self,
        title: &str,
        message: &str,
        action: &str,
        cancel: &str,
    ) -> IpcResult<bool> {
        self.ask(
            title,
            message,
            rfd::MessageButtons::OkCancelCustom(action.to_owned(), cancel.to_owned()),
            Some(action),
        )
    }
}

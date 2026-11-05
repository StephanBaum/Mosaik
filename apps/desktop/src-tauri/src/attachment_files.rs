//! The file side of attachments (ADR-0034): names, MIME hints, bounded
//! reads, atomic writes and the image preview. Pure functions, no session:
//! the commands in `commands::attachments` decide when to call them.
//!
//! Everything that touches decrypted bytes holds them in zeroizing buffers.

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use mosaic_core::{AttachmentId, MAX_ATTACHMENT_LEN, is_spoofing_char, strip_spoofing};
use mosaic_qr::{OwnedFrame, PixelLayout, ScanError};
use zeroize::Zeroizing;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::ids::UnixMillis;

/// Largest side of the preview, in pixels.
pub const PREVIEW_MAX_SIDE: u32 = 2048;
/// Most pixels in the preview (4 bytes each: at most 16 MiB).
pub const PREVIEW_MAX_PIXELS: u64 = 4_194_304;
/// The preview header: width as `u32` LE, then height as `u32` LE.
pub const PREVIEW_HEADER_LEN: usize = 8;
/// What the preview decoder accepts: the ADR-0031 limits (16384 px per side,
/// 40 MP, 512 MiB of decoder memory) with the attachment size as file limit.
pub const PREVIEW_LIMITS: mosaic_qr::ImageLimits = mosaic_qr::ImageLimits {
    max_file_bytes: MAX_ATTACHMENT_LEN,
    ..mosaic_qr::ImageLimits::DEFAULT
};
/// Longest file name stored in an item, in bytes.
pub const MAX_STORED_NAME_LEN: usize = 255;
/// Longest file name suggested to the save dialog, in bytes.
pub const MAX_SAVE_NAME_LEN: usize = 200;
/// An extension (with its dot) up to this long survives the cut.
const MAX_EXTENSION_LEN: usize = 16;

/// Windows device names, which no file may be called (with any extension).
const RESERVED_STEMS: &[&str] = &[
    "CON",
    "PRN",
    "AUX",
    "NUL",
    "COM1",
    "COM2",
    "COM3",
    "COM4",
    "COM5",
    "COM6",
    "COM7",
    "COM8",
    "COM9",
    "LPT1",
    "LPT2",
    "LPT3",
    "LPT4",
    "LPT5",
    "LPT6",
    "LPT7",
    "LPT8",
    "LPT9",
    "COM\u{b9}",
    "COM\u{b2}",
    "COM\u{b3}",
    "LPT\u{b9}",
    "LPT\u{b2}",
    "LPT\u{b3}",
];

fn code(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}

/// `text` cut to at most `max` bytes on a character boundary.
fn cut_bytes(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or_default()
}

/// The name stored with an attachment: the file name of the chosen path,
/// lossy UTF-8, without spoofing or control characters, trimmed and cut to
/// [`MAX_STORED_NAME_LEN`]. Never empty.
#[must_use]
pub fn stored_name(raw: Option<&OsStr>) -> String {
    let lossy = raw.map(OsStr::to_string_lossy).unwrap_or_default();
    let (stripped, _) = strip_spoofing(&lossy);
    let cleaned: String = stripped.chars().filter(|c| !c.is_control()).collect();
    let name = cut_bytes(cleaned.trim(), MAX_STORED_NAME_LEN).trim_end();
    if name.is_empty() {
        "file".to_owned()
    } else {
        name.to_owned()
    }
}

fn is_reserved_stem(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let upper = stem.to_uppercase();
    RESERVED_STEMS.contains(&upper.as_str())
}

/// Cuts `name` to `max` bytes, keeping its last extension when that is at
/// most [`MAX_EXTENSION_LEN`] bytes (dot included).
fn cut_keeping_extension(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_owned();
    }
    let extension = name
        .rfind('.')
        .filter(|&dot| dot > 0)
        .and_then(|dot| name.get(dot..))
        .filter(|ext| ext.len() <= MAX_EXTENSION_LEN)
        .unwrap_or_default();
    let stem = name.get(..name.len() - extension.len()).unwrap_or_default();
    let stem = cut_bytes(stem, max - extension.len()).trim_end_matches(['.', ' ']);
    format!("{stem}{extension}")
}

/// A file name that is safe to suggest to a save dialog on every OS: no
/// spoofing or control characters (a right-to-left override can disguise
/// `exe` as `jpg`), no separators or characters Windows refuses, no
/// trailing dots or spaces, no leading dot (a hidden file), no device name,
/// at most [`MAX_SAVE_NAME_LEN`] bytes. Never empty.
#[must_use]
pub fn safe_file_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .filter(|&c| !is_spoofing_char(c) && !c.is_control())
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = replaced.trim().trim_end_matches(['.', ' ']);
    let mut safe = match trimmed.strip_prefix('.') {
        Some(rest) => format!("_{rest}"),
        None => trimmed.to_owned(),
    };
    if is_reserved_stem(&safe) {
        safe.insert(0, '_');
    }
    let safe = cut_keeping_extension(&safe, MAX_SAVE_NAME_LEN);
    if safe.is_empty() {
        "attachment".to_owned()
    } else {
        safe
    }
}

/// A MIME type for display and hints, from the leading bytes first, then
/// the lowercased extension. It never decides anything about security.
#[must_use]
pub fn sniff_mime(name: &str, head: &[u8]) -> &'static str {
    if head.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return "image/png";
    }
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if head.starts_with(b"%PDF-") {
        return "application/pdf";
    }
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// Whether the preview can decode this type: PNG and JPEG only.
#[must_use]
pub fn previewable_mime(mime: &str) -> bool {
    matches!(mime, "image/png" | "image/jpeg")
}

/// Reads a regular file of at most `max` bytes into a zeroizing buffer. The
/// size is checked before anything is read, and the read itself stops one
/// byte past it, so a file that grows meanwhile is refused too.
///
/// # Errors
/// `attachmentFileUnreadable` (cannot open or read, not a regular file),
/// `attachmentTooLarge`, `internal` (no memory).
pub fn read_bounded(path: &Path, max: usize) -> IpcResult<Zeroizing<Vec<u8>>> {
    let unreadable = || code(ErrorCode::AttachmentFileUnreadable);
    let too_large = || code(ErrorCode::AttachmentTooLarge);
    let file = File::open(path).map_err(|_| unreadable())?;
    let meta = file.metadata().map_err(|_| unreadable())?;
    if !meta.is_file() {
        return Err(unreadable());
    }
    let len = usize::try_from(meta.len()).map_err(|_| too_large())?;
    if len > max {
        return Err(too_large());
    }
    // One byte of room for the probe: the buffer never reallocates, so no
    // unwiped copy of the file is left behind.
    let mut buf = Zeroizing::new(Vec::new());
    buf.try_reserve_exact(len.saturating_add(1))
        .map_err(|_| IpcError::INTERNAL)?;
    let limit = u64::try_from(len)
        .map_err(|_| too_large())?
        .saturating_add(1);
    file.take(limit)
        .read_to_end(&mut buf)
        .map_err(|_| unreadable())?;
    if buf.len() > len {
        return Err(too_large());
    }
    Ok(buf)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        // No execute bit, nothing for group or others.
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Marks a saved file as coming from the internet (`ZoneId=3`), so opening
/// it triggers `SmartScreen` and Office Protected View. Best effort: file
/// systems without alternate data streams ignore it.
#[cfg(windows)]
fn mark_of_the_web(path: &Path) {
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let _ = std::fs::write(stream, b"[ZoneTransfer]\r\nZoneId=3\r\n");
}

/// Writes `bytes` to `path` atomically: a temporary file in the same folder
/// (`.<16 hex>.mosaic-partial`, created new, mode 0600 on Unix), synced,
/// then renamed over the target. On any error the temporary file is removed.
/// On Windows the Mark of the Web is added afterwards.
///
/// # Errors
/// `attachmentSaveFailed`, `internal` (no randomness).
pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> IpcResult<()> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|_| IpcError::INTERNAL)?;
    let temp = dir.join(format!(
        ".{:016x}.mosaic-partial",
        u64::from_le_bytes(random)
    ));
    let result = write_new_file(&temp, bytes).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
        return Err(code(ErrorCode::AttachmentSaveFailed));
    }
    #[cfg(windows)]
    mark_of_the_web(path);
    Ok(())
}

/// When an attachment was added, from the timestamp of its UUIDv7 ID; `None`
/// for an ID of another version. Display only.
#[must_use]
pub fn added_at(id: &AttachmentId) -> Option<UnixMillis> {
    let bytes = id.as_bytes();
    if bytes.get(6)? >> 4 != 7 {
        return None;
    }
    let millis = bytes
        .get(..6)?
        .iter()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
    UnixMillis::try_from(millis).ok()
}

/// The smallest `k >= 1` whose `ceil(w / k)` by `ceil(h / k)` fits the
/// preview limits.
fn downscale_factor(width: u32, height: u32) -> u32 {
    let fits = |k: u32| {
        let (w, h) = (width.div_ceil(k), height.div_ceil(k));
        w <= PREVIEW_MAX_SIDE
            && h <= PREVIEW_MAX_SIDE
            && u64::from(w) * u64::from(h) <= PREVIEW_MAX_PIXELS
    };
    (1..=width.max(height)).find(|&k| fits(k)).unwrap_or(1)
}

/// One pixel as straight RGBA. A luma image becomes grey, a missing alpha
/// is opaque.
fn rgba_at(frame: &OwnedFrame, col: u32, row: u32) -> [u8; 4] {
    let bpp = frame.layout().bytes_per_pixel();
    let index = u64::from(row) * u64::from(frame.width()) + u64::from(col);
    let start = usize::try_from(index)
        .ok()
        .and_then(|i| i.checked_mul(bpp))
        .unwrap_or(usize::MAX);
    let Some(px) = start
        .checked_add(bpp)
        .and_then(|end| frame.pixels().get(start..end))
    else {
        return [0; 4];
    };
    let at = |i: usize| px.get(i).copied().unwrap_or(0);
    match frame.layout() {
        PixelLayout::Luma8 => [at(0), at(0), at(0), 255],
        PixelLayout::LumaA8 => [at(0), at(0), at(0), at(1)],
        PixelLayout::Rgb8 | PixelLayout::Rgbx8 => [at(0), at(1), at(2), 255],
        PixelLayout::Rgba8 => [at(0), at(1), at(2), at(3)],
        PixelLayout::Bgrx8 => [at(2), at(1), at(0), 255],
    }
}

/// The per-channel average of the `k` by `k` block at output pixel
/// (`ox`, `oy`); blocks at the right and bottom edge average what is there.
fn block_average(frame: &OwnedFrame, k: u32, ox: u32, oy: u32) -> [u8; 4] {
    let x_end = ox.saturating_add(1).saturating_mul(k).min(frame.width());
    let y_end = oy.saturating_add(1).saturating_mul(k).min(frame.height());
    let mut sum = [0u64; 4];
    let mut count = 0u64;
    for row in oy * k..y_end {
        for col in ox * k..x_end {
            for (total, value) in sum.iter_mut().zip(rgba_at(frame, col, row)) {
                *total += u64::from(value);
            }
            count += 1;
        }
    }
    let average =
        |total: u64| u8::try_from(total.checked_div(count).unwrap_or(0)).unwrap_or(u8::MAX);
    sum.map(average)
}

/// Decodes an attachment as an image and returns what the webview paints:
/// `[width u32 LE][height u32 LE][width * height * 4 RGBA bytes]`, box
/// downscaled so that neither side exceeds [`PREVIEW_MAX_SIDE`] and the
/// area stays within [`PREVIEW_MAX_PIXELS`]. Every buffer on the way
/// (decoded pixels, the result) is zeroizing; the caller owns the result.
///
/// # Errors
/// `attachmentPreviewTooLarge` (over a decoder limit),
/// `attachmentPreviewUnsupported` (not a PNG or JPEG, or damaged),
/// `internal` (no memory).
pub fn render_preview(plaintext: &[u8]) -> IpcResult<Zeroizing<Vec<u8>>> {
    let frame = mosaic_qr::decode_image(plaintext, &PREVIEW_LIMITS).map_err(|e| match e {
        ScanError::TooLarge => code(ErrorCode::AttachmentPreviewTooLarge),
        _ => code(ErrorCode::AttachmentPreviewUnsupported),
    })?;
    let k = downscale_factor(frame.width(), frame.height());
    let (width, height) = (frame.width().div_ceil(k), frame.height().div_ceil(k));
    let pixels =
        usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| IpcError::INTERNAL)?;
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(PREVIEW_HEADER_LEN.saturating_add(pixels.saturating_mul(4)))
        .map_err(|_| IpcError::INTERNAL)?;
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    for oy in 0..height {
        for ox in 0..width {
            out.extend_from_slice(&block_average(&frame, k, ox, oy));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use mosaic_qr::testing::png;

    use super::*;

    fn frame(width: u32, height: u32, layout: PixelLayout, pixels: Vec<u8>) -> OwnedFrame {
        OwnedFrame::new(width, height, layout, Zeroizing::new(pixels)).unwrap()
    }

    fn header(preview: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_le_bytes(preview[at..at + 4].try_into().unwrap());
        (word(0), word(4))
    }

    #[test]
    fn safe_file_name_handles_the_hostile_cases() {
        for (input, expected) in [
            ("CON.txt", "_CON.txt"),
            ("com1.tar.gz", "_com1.tar.gz"),
            ("a/b\\c:d", "a_b_c_d"),
            ("\u{202E}gpj.exe", "gpj.exe"),
            ("...", "attachment"),
            ("name. ", "name"),
            (".bashrc", "_bashrc"),
            ("", "attachment"),
            ("LPT\u{b9}", "_LPT\u{b9}"),
            ("report.pdf", "report.pdf"),
            ("a\u{0}b\nc", "abc"),
            ("Konto ä.txt", "Konto ä.txt"),
        ] {
            assert_eq!(safe_file_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn safe_file_name_cuts_long_names_but_keeps_the_extension() {
        let long = format!("{}.pdf", "x".repeat(300));
        let safe = safe_file_name(&long);
        assert_eq!(safe.len(), MAX_SAVE_NAME_LEN);
        assert!(
            Path::new(&safe)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        );
        // A long "extension" is part of the stem and is cut with it.
        let odd = format!("a.{}", "y".repeat(300));
        assert_eq!(safe_file_name(&odd).len(), MAX_SAVE_NAME_LEN);
        // Multi-byte characters are never split.
        let wide = "ä".repeat(150);
        let cut = safe_file_name(&wide);
        assert!(cut.len() <= MAX_SAVE_NAME_LEN);
        assert!(cut.chars().all(|c| c == 'ä'));
    }

    #[test]
    fn stored_name_strips_control_and_spoofing_characters() {
        let name = stored_name(Some(OsStr::new("  re\u{202E}port\u{0}\n.pdf ")));
        assert_eq!(name, "report.pdf");
        assert_eq!(stored_name(None), "file");
        assert_eq!(stored_name(Some(OsStr::new("\u{200B}\t"))), "file");
        let long = stored_name(Some(OsStr::new(&"ö".repeat(300))));
        assert!(long.len() <= MAX_STORED_NAME_LEN);
        assert!(long.chars().all(|c| c == 'ö'));
    }

    #[test]
    fn sniff_mime_prefers_the_magic_bytes() {
        let png_head = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0];
        assert_eq!(sniff_mime("notes.txt", &png_head), "image/png");
        assert_eq!(sniff_mime("x.txt", &[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(sniff_mime("x.bin", b"%PDF-1.7"), "application/pdf");
        assert_eq!(sniff_mime("DATA.CSV", b""), "text/csv");
        assert_eq!(sniff_mime("a.zip", b"PK"), "application/zip");
        assert_eq!(sniff_mime("a.exe", b"MZ"), "application/octet-stream");
        assert_eq!(sniff_mime("noextension", b""), "application/octet-stream");
        assert!(previewable_mime("image/png") && previewable_mime("image/jpeg"));
        assert!(!previewable_mime("application/pdf") && !previewable_mime("image/gif"));
    }

    #[test]
    fn read_bounded_reads_a_file_and_refuses_over_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, b"hello").unwrap();
        assert_eq!(read_bounded(&path, 5).unwrap().as_slice(), b"hello");
        let error = read_bounded(&path, 4).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentTooLarge);
    }

    #[test]
    fn read_bounded_refuses_over_limit_without_reading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sparse.bin");
        let max = 1024;
        let file = File::create(&path).unwrap();
        file.set_len(u64::try_from(max).unwrap() + 1).unwrap();
        let error = read_bounded(&path, max).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentTooLarge);
    }

    #[test]
    fn read_bounded_refuses_directories_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let error = read_bounded(dir.path(), 10).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentFileUnreadable);
        let error = read_bounded(&dir.path().join("missing"), 10).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentFileUnreadable);
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_file_atomic(&path, b"first").unwrap();
        write_file_atomic(&path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
        assert_eq!(entries(dir.path()), vec!["out.txt".to_owned()]);
    }

    #[test]
    fn write_atomic_failure_removes_temp() {
        let dir = tempfile::tempdir().unwrap();
        // The target is a directory: the rename fails after the temp file
        // was written.
        let target = dir.path().join("taken");
        std::fs::create_dir(&target).unwrap();
        let error = write_file_atomic(&target, b"data").unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentSaveFailed);
        assert_eq!(entries(dir.path()), vec!["taken".to_owned()]);
        // A missing folder fails at once.
        let error = write_file_atomic(&dir.path().join("no/such/dir/f"), b"x").unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentSaveFailed);
    }

    #[cfg(unix)]
    #[test]
    fn write_mode_is_0600() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.bin");
        write_file_atomic(&path, b"x").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(windows)]
    #[test]
    fn saved_files_carry_the_mark_of_the_web() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        write_file_atomic(&path, b"x").unwrap();
        let mut stream = path.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        // NTFS only; a temp folder on another file system has no streams.
        if let Ok(zone) = std::fs::read_to_string(stream) {
            assert!(zone.contains("ZoneId=3"));
        }
    }

    #[test]
    fn added_at_from_v7() {
        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(&[0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e]);
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        let id = AttachmentId::from_bytes(bytes).unwrap();
        assert_eq!(added_at(&id), Some(0x019a_2b3c_4d5e));
    }

    #[test]
    fn added_at_none_for_v4() {
        let mut bytes = [7u8; 16];
        bytes[6] = 0x47;
        bytes[8] = 0x80;
        let id = AttachmentId::from_bytes(bytes).unwrap();
        assert_eq!(added_at(&id), None);
    }

    #[test]
    fn render_preview_png_header_and_len() {
        let pixels: Vec<u8> = (0..4 * 3 * 4)
            .map(|i| u8::try_from(i * 5).unwrap())
            .collect();
        let source = frame(4, 3, PixelLayout::Rgba8, pixels.clone());
        let preview = render_preview(&png(&source)).unwrap();
        assert_eq!(header(&preview), (4, 3));
        assert_eq!(preview.len(), PREVIEW_HEADER_LEN + 4 * 3 * 4);
        assert_eq!(
            &preview[PREVIEW_HEADER_LEN..],
            pixels.as_slice(),
            "k = 1 is lossless"
        );
    }

    #[test]
    fn render_preview_downscales_4097_by_k3() {
        let source = frame(4097, 3, PixelLayout::Luma8, vec![200; 4097 * 3]);
        let preview = render_preview(&png(&source)).unwrap();
        // k = 2 would leave 2049 columns, so k = 3: ceil(4097 / 3) by ceil(3 / 3).
        assert_eq!(header(&preview), (1366, 1));
        assert_eq!(preview.len(), PREVIEW_HEADER_LEN + 1366 * 4);
        assert!(
            preview[PREVIEW_HEADER_LEN..]
                .chunks(4)
                .all(|px| px == [200, 200, 200, 255]),
            "a flat image stays flat, edge block included"
        );
    }

    #[test]
    fn render_preview_luma_and_alpha_to_rgba() {
        let luma = frame(2, 1, PixelLayout::Luma8, vec![10, 250]);
        let preview = render_preview(&png(&luma)).unwrap();
        assert_eq!(
            &preview[PREVIEW_HEADER_LEN..],
            &[10, 10, 10, 255, 250, 250, 250, 255]
        );
        let luma_alpha = frame(1, 1, PixelLayout::LumaA8, vec![90, 33]);
        let preview = render_preview(&png(&luma_alpha)).unwrap();
        assert_eq!(
            &preview[PREVIEW_HEADER_LEN..],
            &[90, 90, 90, 33],
            "straight alpha"
        );
        let rgb = frame(1, 1, PixelLayout::Rgb8, vec![1, 2, 3]);
        let preview = render_preview(&png(&rgb)).unwrap();
        assert_eq!(&preview[PREVIEW_HEADER_LEN..], &[1, 2, 3, 255]);
    }

    #[test]
    fn render_preview_pdf_is_unsupported() {
        let error = render_preview(b"%PDF-1.7 not an image").unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentPreviewUnsupported);
        let error = render_preview(b"").unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentPreviewUnsupported);
        let mut truncated = png(&frame(8, 8, PixelLayout::Luma8, vec![0; 64]));
        truncated.truncate(30);
        let error = render_preview(&truncated).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentPreviewUnsupported);
    }

    #[test]
    fn render_preview_huge_header_is_too_large() {
        // A PNG header that claims 20000 x 20000 pixels; no pixel data.
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&20_000u32.to_be_bytes());
        ihdr.extend_from_slice(&20_000u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        let mut chunk = b"IHDR".to_vec();
        chunk.extend_from_slice(&ihdr);
        let crc = crc32(&chunk);
        bytes.extend_from_slice(&chunk);
        bytes.extend_from_slice(&crc.to_be_bytes());
        let error = render_preview(&bytes).unwrap_err();
        assert_eq!(error.code, ErrorCode::AttachmentPreviewTooLarge);
    }

    /// CRC-32 (IEEE), for the hand-built PNG chunk.
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
}

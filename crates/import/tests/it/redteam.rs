//! Red team of the importers (RT-parsers, 2026-11-13): hostile export files
//! a user got from somewhere else. Tests marked `ignore` demonstrate an open
//! finding; the others are regression tests of attacks that held.

#![expect(
    clippy::unwrap_used,
    clippy::format_push_string,
    clippy::indexing_slicing,
    reason = "tests"
)]

use std::io::{Cursor, Read as _, Write as _};
use std::time::{Duration, Instant};

use mosaic_core::model::{ItemType, LoginUrl, TextField};
use mosaic_core::{DeviceId, ItemId, Timestamp, VaultId};
use mosaic_import::{Error, Format, parse};
use mosaic_vault::session::ItemRef;

// ---------------------------------------------------------------------------
// RT-parsers-1: `Builder::notes` joins every further note by copying the whole
// text so far (`draft.rs`, `notes()`), so a CXF item with many `note`
// credentials (or a KDBX entry with many `Notes` strings) costs
// O(n^2 * size) before the entry is even sized against the item limit. A
// 32 MiB file keeps the import busy for minutes; a 200 MiB one for hours.
// ---------------------------------------------------------------------------

fn cxf_with_note_credentials(count: usize, content_len: usize) -> Vec<u8> {
    let content = "x".repeat(content_len);
    let mut json = String::from(
        r#"{"version":{"major":1,"minor":0},"accounts":[{"items":[{"title":"notes","credentials":["#,
    );
    for i in 0..count {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"type":"note","content":{{"fieldType":"string","value":"{content}"}}}}"#
        ));
    }
    json.push_str("]}]}]}");
    json.into_bytes()
}

#[test]
#[ignore = "RT-parsers-1: Builder::notes re-copies the whole notes text per note credential; a CXF file with many notes is quadratic"]
fn rt_parsers_1_many_note_credentials_stay_linear() {
    // 16,000 notes of 2,000 bytes: a 32 MiB file. Linear: well under a
    // second. Today: 16,000^2 * 2,000 / 2 bytes of copying (and as much
    // zeroizing), minutes.
    let input = cxf_with_note_credentials(16_000, 2_000);
    let started = Instant::now();
    let _ = parse(Format::Cxf, &input, None);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
}

// ---------------------------------------------------------------------------
// RT-parsers-3: the KDBX key-derivation limits admit a file that asks for
// 1 GiB of Argon2 memory and 64 GiB of fills (`kdbx/mod.rs`), all before the
// password is checked. On a 7.4 GB desktop under load, and on the M10 phone,
// a 1 GiB allocation is an OOM kill, and the time is minutes of a pinned
// core per attempt. KeePassXC's default is 64 MiB; nothing a user owns needs
// more than 256 MiB.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "RT-parsers-3: a hostile KDBX file may ask for 1 GiB of Argon2 memory and 64 GiB of fills before the password is checked"]
fn rt_parsers_3_kdbx_argon2_limits_fit_a_phone() {
    use mosaic_import::kdbx::{MAX_ARGON2_MEMORY_KIB, MAX_ARGON2_WORK};
    assert!(
        MAX_ARGON2_MEMORY_KIB <= 256 * 1024,
        "memory limit is {MAX_ARGON2_MEMORY_KIB} KiB"
    );
    // About 15 s single-threaded at 1 GiB/s.
    assert!(
        MAX_ARGON2_WORK <= 16 * 1024 * 1024,
        "work limit is {MAX_ARGON2_WORK} KiB of fills"
    );
}

// ---------------------------------------------------------------------------
// Attacks that held.
// ---------------------------------------------------------------------------

fn login_item(url: &str, user: &str) -> mosaic_core::model::Item {
    let device = DeviceId::from_bytes([7; 16]).unwrap();
    let at = Timestamp::from_unix_millis(0);
    let mut item = mosaic_core::model::Item::new(&ItemType::Login, device, at).unwrap();
    {
        let mut e = item.edit(device, at);
        e.set_title("Existing").unwrap();
        e.set_text(TextField::Username, user).unwrap();
        e.add_url(&LoginUrl::new(url)).unwrap();
    }
    item
}

/// A login URL with userinfo (`https://bank.test@evil.test/`) names
/// `evil.test`, not the bank: the site key, the duplicate check and the title
/// derived from the URL all use the real host, so the preview cannot be made
/// to show one site while the item belongs to another.
#[test]
fn userinfo_in_an_imported_url_does_not_hide_the_real_host() {
    let csv = "url,username,password\nhttps://bank.test@evil.test/login,alice,pw\n";
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let d = &import.entries[0].draft;
    assert_eq!(
        d.title(),
        Some("evil.test"),
        "the title names the real host"
    );

    let vault = VaultId::new_v7().unwrap();
    let (evil_id, bank_id) = (ItemId::new_v7().unwrap(), ItemId::new_v7().unwrap());
    let evil = login_item("https://evil.test", "alice");
    let bank = login_item("https://bank.test", "alice");
    let refs = [
        ItemRef {
            vault_id: vault,
            item_id: bank_id,
            item: &bank,
        },
        ItemRef {
            vault_id: vault,
            item_id: evil_id,
            item: &evil,
        },
    ];
    let preview = import.preview(&refs);
    let dup = preview.entries[0].duplicate_of.unwrap();
    assert_eq!(
        dup.item_id, evil_id,
        "a duplicate of the evil.test login, not the bank"
    );
}

/// Look-alike and invisible characters in a title are removed and reported,
/// so an imported entry cannot pass for another item in the preview.
#[test]
fn spoofing_characters_in_titles_are_stripped() {
    let csv = "name,username,password\nPay\u{200b}Pal\u{202e},alice,pw\n";
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let e = &import.entries[0];
    assert_eq!(e.draft.title(), Some("PayPal"));
    assert!(
        e.warnings
            .contains(&mosaic_import::EntryWarning::TitleCleaned)
    );
}

fn zip_with_member(name: &str, data_len: u64) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, options).unwrap();
    std::io::copy(&mut std::io::repeat(0).take(data_len), &mut zip).unwrap();
    zip.finish().unwrap().into_inner()
}

/// A 1PUX whose `export.data` inflates past 64 MiB (a few KiB of deflated
/// zeros) is refused at the cap, not inflated into memory.
#[test]
fn onepux_zip_bomb_export_data_is_refused_at_the_cap() {
    let bomb = zip_with_member(
        "export.data",
        u64::try_from(mosaic_import::onepux::MAX_EXPORT_DATA_LEN).unwrap() + 1,
    );
    assert!(bomb.len() < 1024 * 1024, "the archive itself is small");
    assert_eq!(
        parse(Format::OnePassword, &bomb, None).unwrap_err(),
        Error::TooLarge
    );
}

/// Deflated attachment members past 50 MiB are left out, and the entry still
/// imports (the attachment is reported, not the file refused).
#[test]
fn onepux_oversized_attachment_is_left_out_not_inflated() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("export.data", options).unwrap();
    zip.write_all(
        br#"{"accounts":[{"vaults":[{"attrs":{"name":"V"},"items":[{"categoryUuid":"006",
            "overview":{"title":"Doc"},"details":{"documentAttributes":{"fileName":"big.bin",
            "documentId":"doc1"}}}]}]}]}"#,
    )
    .unwrap();
    zip.start_file("files/doc1__big.bin", options).unwrap();
    std::io::copy(
        &mut std::io::repeat(0).take(u64::try_from(mosaic_core::MAX_ATTACHMENT_LEN).unwrap() + 1),
        &mut zip,
    )
    .unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let import = parse(Format::OnePassword, &bytes, None).unwrap();
    let e = &import.entries[0];
    assert!(e.draft.attachments.is_empty());
    assert!(
        e.warnings
            .contains(&mosaic_import::EntryWarning::AttachmentTooLarge)
    );
}

/// A `files/` member with a path-like name is stored under its cleaned file
/// name; the import never uses the member name as a path.
#[test]
fn onepux_attachment_names_are_not_paths() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("export.data", options).unwrap();
    // The file name carries a right-to-left override (U+202E) and path
    // separators, written as the raw character in the JSON text.
    let export_data = format!(
        r#"{{"accounts":[{{"vaults":[{{"attrs":{{"name":"V"}},"items":[{{"categoryUuid":"006",
            "overview":{{"title":"Doc"}},"details":{{"documentAttributes":{{
            "fileName":"..\\..\\evil{}fdp.exe","documentId":"d"}}}}}}]}}]}}]}}"#,
        '\u{202e}'
    );
    zip.write_all(export_data.as_bytes()).unwrap();
    zip.start_file("files/d__x", options).unwrap();
    zip.write_all(b"payload").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let import = parse(Format::OnePassword, &bytes, None).unwrap();
    let a = &import.entries[0].draft.attachments[0];
    assert!(!a.file_name.as_str().contains('\u{202e}'));
    assert_eq!(a.file_name.as_str(), "..\\..\\evilfdp.exe");
    assert_eq!(a.data.expose_secret(), b"payload");
}

/// RT-parsers-6: an OTP secret from a file that is far too short for RFC 4226
/// (128 bits at least; here 8 bits) is stored as a working generator without
/// a word. A user who imports a file someone else prepared (a "migration
/// helper", a shared vault export) gets a second factor whose codes anyone
/// can compute. The value should stay a hidden custom field with
/// `OtpUnreadable`, or at least carry a warning.
#[test]
#[ignore = "RT-parsers-6: imported otpauth secrets below RFC 4226's 128-bit minimum are stored as working generators without a warning"]
fn weak_otp_secrets_from_a_file_are_not_silently_accepted() {
    // "GE" is one byte of secret.
    let csv = "name,username,password,otp\nx,u,p,otpauth://totp/x?secret=GE\n";
    let import = parse(Format::Csv, csv.as_bytes(), None).unwrap();
    let d = &import.entries[0].draft;
    if d.otp.is_some() {
        let (config, _) = d.otp.as_ref().unwrap();
        assert!(
            config.secret.expose_secret().len() >= 10,
            "a stored generator has at least a 10-byte secret"
        );
    } else {
        assert!(
            import.entries[0]
                .warnings
                .contains(&mosaic_import::EntryWarning::OtpUnreadable)
        );
    }
}

/// A CSV with a quoted field that never closes, 100 MiB of commas, or a row
/// wider than the column cap is refused without a hang.
#[test]
fn csv_edge_cases_are_bounded() {
    let wide = format!("a,b\n{}", ",".repeat(mosaic_import::csv::MAX_COLUMNS + 1));
    assert_eq!(
        parse(Format::Csv, wide.as_bytes(), None).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        parse(Format::Csv, b"a,b\n\"open", None).unwrap_err(),
        Error::Malformed
    );
    let started = Instant::now();
    let many = format!("name,url\n{}", "x,https://e.test\n".repeat(200_000));
    let import = parse(Format::Csv, many.as_bytes(), None);
    assert!(
        matches!(import, Err(Error::LimitExceeded)),
        "over MAX_ENTRIES"
    );
    assert!(started.elapsed() < Duration::from_secs(20));
}

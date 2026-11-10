//! The folder file-name parser: any name parses or is ignored without a
//! panic, and a name that parses is exactly the name Mosaic would write for
//! what it parsed to.
#![no_main]

use std::ffi::OsStr;

use libfuzzer_sys::fuzz_target;
use mosaic_sync::wire::folder_names::{
    FolderName, account_name, blob_delete_name, blob_name, dir_name, is_checkpoint_candidate,
    parse, rekey_marker_name, segment_name,
};

fuzz_target!(|data: &[u8]| {
    let Ok(name) = std::str::from_utf8(data) else {
        return;
    };
    let _ = is_checkpoint_candidate(OsStr::new(name));
    let Some(parsed) = parse(OsStr::new(name)) else {
        return;
    };
    let canonical = match parsed {
        FolderName::Segment { seq } => segment_name(seq),
        FolderName::Account { version, device } => account_name(version, &device),
        FolderName::RekeyMarker { from_epoch, device } => rekey_marker_name(from_epoch, &device),
        FolderName::Blob(id) => blob_name(&id),
        FolderName::BlobDelete { attachment, device } => blob_delete_name(&attachment, &device),
        FolderName::Head => "head.mck".to_owned(),
        FolderName::DeviceInfo => "info.mdev".to_owned(),
        FolderName::Dir(id) => dir_name(&id),
    };
    assert_eq!(canonical, name, "only canonical names parse");
});

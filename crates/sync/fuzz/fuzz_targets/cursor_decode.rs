//! The folder cursor: arbitrary bytes never panic the decoder, and a cursor
//! that decodes re-encodes to exactly the same bytes (it is at most 64 KiB, so
//! nothing is dropped).
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_sync::wire::cursor::FolderCursor;

fuzz_target!(|data: &[u8]| {
    if let Ok(cursor) = FolderCursor::decode(data) {
        let again = cursor.encode().expect("a decoded cursor encodes");
        assert_eq!(again, data, "one cursor, one encoding");
    }
});

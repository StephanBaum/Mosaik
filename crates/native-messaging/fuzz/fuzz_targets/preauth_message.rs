//! Pre-auth messages in both directions: nothing panics, and a message that
//! parses serializes to a canonical form that parses to the same message.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_native_messaging::preauth::{ClientMessage, ServerMessage};

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = ClientMessage::parse(data) {
        let canonical = serde_json::to_vec(&message).expect("serialize");
        let back = ClientMessage::parse(&canonical).expect("re-parse");
        assert_eq!(serde_json::to_vec(&back).expect("serialize"), canonical);
    }
    if let Ok(message) = ServerMessage::parse(data) {
        let canonical = serde_json::to_vec(&message).expect("serialize");
        let back = ServerMessage::parse(&canonical).expect("re-parse");
        assert_eq!(serde_json::to_vec(&back).expect("serialize"), canonical);
    }
});

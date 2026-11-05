//! The extension envelope: a request that parses re-serializes to a request
//! that parses to the same ID, op and verbatim body; a desktop message that
//! parses is a validated response or a known event and parses again after
//! serialization. Nothing panics on any input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_native_messaging::protocol::{DesktopMessage, Op, RequestEnvelope};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = serde_json::from_slice::<RequestEnvelope<'_>>(data) {
        let again = serde_json::to_vec(&request).expect("serialize");
        let back: RequestEnvelope<'_> = serde_json::from_slice(&again).expect("re-parse");
        assert_eq!(
            (back.v, back.id, back.op.as_str(), back.body.get()),
            (request.v, request.id, request.op.as_str(), request.body.get())
        );
        if let Some(op) = Op::from_wire(&request.op) {
            assert_eq!(op.as_str(), request.op);
        }
    }
    match DesktopMessage::parse(data) {
        Ok(DesktopMessage::Response(response)) => {
            response.validate().expect("validated");
            let again = serde_json::to_vec(&response).expect("serialize");
            assert!(matches!(
                DesktopMessage::parse(&again),
                Ok(DesktopMessage::Response(_))
            ));
        }
        Ok(DesktopMessage::Event(event)) => {
            let again = serde_json::to_vec(&event).expect("serialize");
            match DesktopMessage::parse(&again) {
                Ok(DesktopMessage::Event(back)) => assert_eq!(back, event),
                _ => panic!("event did not round-trip"),
            }
        }
        Err(_) => {}
    }
});

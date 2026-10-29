//! The agent wire parser: never panics on any body; whatever parses
//! re-encodes to a body that parses to the same value (byte-for-byte for
//! request identities and sign requests, whose fields fill the body
//! exactly); responses round-trip exactly; and the length prefix accepts
//! exactly 1 to `MAX_MESSAGE_LEN`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_ssh_agent::wire::{self, MAX_MESSAGE_LEN, Request, Response, WireError};

fuzz_target!(|data: &[u8]| {
    match Request::parse(data) {
        Ok(req) => {
            let again = req.encode().expect("re-encode");
            assert_eq!(Request::parse(&again).expect("re-parse"), req);
            if matches!(req, Request::RequestIdentities | Request::Sign(_)) {
                assert_eq!(again, data, "strict request not canonical");
            }
        }
        Err(e) => assert!(matches!(
            e,
            WireError::Empty | WireError::Truncated | WireError::TrailingData | WireError::TooLong
        )),
    }
    if let Ok(resp) = Response::parse(data) {
        assert_eq!(resp.encode().expect("re-encode"), data, "response not canonical");
    }
    if let Some((prefix, _)) = data.split_first_chunk::<4>() {
        let n = u32::from_be_bytes(*prefix) as usize;
        match wire::body_len(*prefix) {
            Ok(len) => assert!(len == n && (1..=MAX_MESSAGE_LEN).contains(&n)),
            Err(_) => assert!(n == 0 || n > MAX_MESSAGE_LEN),
        }
    }
    if !data.is_empty() && data.len() <= MAX_MESSAGE_LEN {
        let framed = wire::frame(data).expect("frame");
        assert_eq!(&framed[4..], data);
    }
});

//! Attachment opening.
//!
//! - Mode 0: seal the rest of the input under the fixed `FK` and check the
//!   round trip.
//! - Mode `0xFE`: take an authentic golden attachment (`data[1]` picks the
//!   empty or the small one), apply the fuzzer's edits, and require the
//!   original plaintext if and only if the bytes are unchanged.
//! - Mode `0xFD` (structured, multi-chunk): seal `n * 64 KiB + extra` bytes
//!   (`n = data[1] % 4`, `extra = data[2]`), then swap, drop or duplicate
//!   whole chunks (`data[3..]`). Only the identity edit may open; everything
//!   else is `Decryption` or `Malformed`. This exercises the canonical-chunk
//!   logic on authentic data, which random inputs never reach.
//! - Otherwise: arbitrary input never opens, except the exact golden
//!   attachments (the keys are fixed). CI runs this target with
//!   `-max_len=270000` so raw inputs can span several chunks.
//!
//! Keys are fixed (the golden keys), so a crash reproduces from its input.
#![no_main]

mod common;

use common::{STRUCTURED, TAMPER};
use libfuzzer_sys::fuzz_target;
use mosaic_core::attachment::CHUNK_LEN;
use mosaic_core::{Error, open_attachment, seal_attachment};

/// `version || alg || chunk_log2 || nonce_prefix[19]`.
const HEADER_LEN: usize = 22;
const CHUNK_CT_LEN: usize = CHUNK_LEN + 16;

fuzz_target!(|data: &[u8]| {
    let fk = common::fk();
    let vault = common::vault();
    let item = common::item();
    let att = common::attachment();

    match data.split_first() {
        Some((&0, pt)) => {
            let ct = seal_attachment(&fk, &vault, &item, &att, pt).expect("seal");
            let back = open_attachment(&fk, &vault, &item, &att, &ct).expect("open");
            assert_eq!(back.expose_secret(), pt);
        }
        Some((&TAMPER, rest)) => {
            let Some((&which, ops)) = rest.split_first() else {
                return;
            };
            let (golden, pt): (_, &[u8]) = if which % 2 == 0 {
                (common::ATTACHMENT_EMPTY, b"")
            } else {
                (common::ATTACHMENT_SMALL, b"attachment bytes")
            };
            let base = common::hex_file(golden);
            let bytes = common::mutate(&base, ops);
            let r = open_attachment(&fk, &vault, &item, &att, &bytes);
            if bytes == base {
                assert_eq!(r.expect("unchanged attachment opens").expose_secret(), pt);
            } else {
                let e = r.expect_err("a modified attachment must not open");
                assert!(common::tamper_error_ok(e), "unexpected error {e:?}");
            }
        }
        Some((&STRUCTURED, rest)) => structured(rest),
        _ => match open_attachment(&fk, &vault, &item, &att, data) {
            // With fixed keys, only the exact golden attachments open.
            Ok(pt) => {
                let golden = [
                    (common::ATTACHMENT_EMPTY, &b""[..]),
                    (common::ATTACHMENT_SMALL, &b"attachment bytes"[..]),
                ];
                assert!(
                    golden
                        .iter()
                        .any(|(g, p)| data == common::hex_file(g) && pt.expose_secret() == *p),
                    "only a golden attachment may open"
                );
            }
            Err(e) => assert!(
                matches!(
                    e,
                    Error::Decryption
                        | Error::Malformed
                        | Error::SizeLimit
                        | Error::UnsupportedVersion
                        | Error::UnsupportedAlgorithm
                ),
                "unexpected error {e:?}"
            ),
        },
    }
});

fn structured(rest: &[u8]) {
    let &[n, extra, op, a, b, ..] = rest else {
        return;
    };
    let len = usize::from(n % 4) * CHUNK_LEN + usize::from(extra);
    let pt: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    let (fk, vault, item, att) = (
        common::fk(),
        common::vault(),
        common::item(),
        common::attachment(),
    );
    let ct = seal_attachment(&fk, &vault, &item, &att, &pt).expect("seal");
    let (header, body) = ct.split_at(HEADER_LEN);
    let mut chunks: Vec<&[u8]> = body.chunks(CHUNK_CT_LEN).collect();
    let count = chunks.len();
    let (i, j) = (usize::from(a) % count, usize::from(b) % count);
    let identity = match op % 3 {
        0 => {
            chunks.swap(i, j);
            i == j
        }
        1 => {
            chunks.remove(i);
            false
        }
        _ => {
            let dup = chunks[i];
            chunks.insert(i, dup);
            false
        }
    };
    let mut edited = header.to_vec();
    for c in &chunks {
        edited.extend_from_slice(c);
    }
    let r = open_attachment(&fk, &vault, &item, &att, &edited);
    if identity {
        assert_eq!(edited, ct);
        assert_eq!(
            r.expect("identity edit opens").expose_secret(),
            pt.as_slice()
        );
    } else {
        let e = r.expect_err("reordered, dropped or duplicated chunks must not open");
        assert!(
            matches!(e, Error::Decryption | Error::Malformed),
            "unexpected error {e:?}"
        );
    }
}

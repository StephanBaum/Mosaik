//! The encrypted export reader and writer (CRYPTO_SPEC §11.2).
//!
//! The container is exercised under a fixed `K_x` (the `*_for_fuzzing`
//! constructors skip Argon2id, which would cost 256 MiB per run), so a crash
//! reproduces from its input.
//!
//! - Mode 0: write the rest of the input (in pieces of `data[1] + 1` bytes) and
//!   read it back: the plaintext must be identical.
//! - Mode `0xFE`: write `data[1] * 1000` bytes of pattern, apply the
//!   fuzzer's edits to the file (`data[2..]`), and require the original
//!   plaintext if and only if the bytes are unchanged; any change must be
//!   an error, from the first call that fails to the end of the stream.
//! - Otherwise: arbitrary input is a header and chunks; it must never panic,
//!   and with the fixed key it cannot be authentic, so a full read must
//!   fail.
#![no_main]

mod common;

use std::io::Read as _;

use common::{TAMPER, mutate};
use libfuzzer_sys::fuzz_target;
use mosaic_core::{Error, ExportReader, ExportWriter, SecretBytes};

fn kx() -> SecretBytes<32> {
    SecretBytes::from_array(&mut [0x4B; 32])
}

fn seal(pt: &[u8], piece: usize) -> Vec<u8> {
    let mut w = ExportWriter::new_with_key_for_fuzzing(Vec::new(), kx()).expect("writer");
    for p in pt.chunks(piece.max(1)) {
        w.write_chunk(p).expect("write");
    }
    w.finish().expect("finish")
}

/// Reads the whole stream through `next_chunk`, checking that the `Read`
/// adapter agrees on a second reader.
fn read_all(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut reader = ExportReader::open_with_key_for_fuzzing(bytes, kx())?;
    let mut out = Vec::new();
    while let Some(chunk) = reader.next_chunk()? {
        out.extend_from_slice(chunk.expose_secret());
    }
    // A finished stream stays finished.
    assert!(matches!(reader.next_chunk(), Ok(None)));
    Ok(out)
}

fn read_adapter(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut reader = ExportReader::open_with_key_for_fuzzing(bytes, kx()).map_err(|_| ())?;
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

fn error_ok(e: Error) -> bool {
    matches!(
        e,
        Error::Decryption
            | Error::Malformed
            | Error::SizeLimit
            | Error::UnsupportedVersion
            | Error::UnsupportedAlgorithm
            | Error::UnknownProfile
    )
}

fuzz_target!(|data: &[u8]| {
    match data.split_first() {
        Some((&0, rest)) => {
            let Some((&piece, pt)) = rest.split_first() else {
                return;
            };
            let file = seal(pt, usize::from(piece) + 1);
            assert_eq!(read_all(&file).expect("own export reads"), pt);
        }
        Some((&TAMPER, rest)) => {
            let Some((&len, ops)) = rest.split_first() else {
                return;
            };
            let pt: Vec<u8> = (0..usize::from(len) * 1000)
                .map(|i| (i % 251) as u8)
                .collect();
            let base = seal(&pt, 4096);
            let bytes = mutate(&base, ops);
            let r = read_all(&bytes);
            if bytes == base {
                assert_eq!(r.expect("unchanged export reads"), pt);
                assert_eq!(read_adapter(&bytes).expect("adapter reads"), pt);
            } else {
                let e = r.expect_err("a modified export must not read to the end");
                assert!(error_ok(e), "unexpected error {e:?}");
                assert!(read_adapter(&bytes).is_err(), "adapter must fail too");
            }
        }
        _ => {
            if let Err(e) = read_all(data) {
                assert!(error_ok(e), "unexpected error {e:?}");
            } else {
                panic!("arbitrary input read as an authentic export under a fixed key");
            }
        }
    }
});

//! `PUT /v1/checkpoints/{deviceId}`: the server cannot check the MAC, so it
//! checks the shape with `Checkpoint::decode_unverified`. Arbitrary bytes
//! never panic it, a body is accepted only for the device it names, and what
//! is accepted has sorted, unique entries within the bound.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::InstallId;
use mosaic_sync::wire::checkpoint::{Checkpoint, MAX_ENTRIES};

fuzz_target!(|data: &[u8]| {
    // The device at its fixed offset, so that most inputs get past the path
    // check, and one that is not in the body.
    let named = data.get(5..21).and_then(|d| InstallId::from_slice(d).ok());
    let stranger = InstallId::from_bytes([0xee; 16]).expect("not nil");
    if let Ok(cp) = Checkpoint::decode_unverified(data, stranger) {
        assert_eq!(cp.device_id, stranger);
    }
    if let Some(device) = named
        && let Ok(cp) = Checkpoint::decode_unverified(data, device)
    {
        assert_eq!(cp.device_id, device);
        assert!(cp.entries.len() <= MAX_ENTRIES);
        assert!(cp.entries.windows(2).all(|w| w[0].vault_id < w[1].vault_id));
    }
});

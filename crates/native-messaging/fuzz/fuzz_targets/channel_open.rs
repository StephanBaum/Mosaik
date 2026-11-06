//! AEAD frames: arbitrary bytes never open (and never panic) and poison the
//! session (CRYPTO_SPEC §15: a failed open refuses every later frame); any
//! plaintext sealed by one end opens at the other; one flipped bit in a sealed
//! frame never opens; the frame length check accepts exactly 1 to the cap.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::browser_channel::{ChannelSession, PairingKey, Role};
use mosaic_native_messaging::framing::check_len;
use mosaic_native_messaging::protocol::MAX_FRAME_BYTES;

fn sessions() -> (ChannelSession, ChannelSession) {
    let key = PairingKey::from_bytes(&mut [1; 32]);
    let (id, nh, nd) = ([2u8; 16], [3u8; 32], [4u8; 32]);
    (
        key.session(Role::Host, &id, &nh, &nd).expect("host session"),
        key.session(Role::Desktop, &id, &nh, &nd).expect("desktop session"),
    )
}

fuzz_target!(|data: &[u8]| {
    let (mut host, mut desktop) = sessions();
    // Arbitrary bytes on their own session: they never open, and the failure
    // poisons that session, so even a genuine frame is refused afterwards.
    let (mut sender, mut probe) = sessions();
    assert!(probe.open(data).is_err(), "arbitrary bytes opened");
    let genuine = sender.seal(data).expect("seal");
    assert!(probe.open(&genuine).is_err(), "a poisoned session opened a frame");

    let frame = host.seal(data).expect("seal");
    assert_eq!(desktop.open(&frame).expect("open").expose_secret(), data);

    let mut bad = frame.clone();
    let at = data.len().wrapping_mul(7) % bad.len();
    bad[at] ^= 1 << (data.len() % 8);
    let (_, mut fresh) = sessions();
    assert!(fresh.open(&bad).is_err(), "a flipped bit opened");

    if let Some((prefix, _)) = data.split_first_chunk::<4>() {
        let n = u32::from_be_bytes(*prefix);
        match check_len(n) {
            Ok(len) => assert!(len == n as usize && (1..=MAX_FRAME_BYTES).contains(&len)),
            Err(_) => assert!(n == 0 || n as usize > MAX_FRAME_BYTES),
        }
    }
});

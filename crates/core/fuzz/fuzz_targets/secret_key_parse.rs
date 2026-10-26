//! Secret Key parsing: never panics; anything accepted re-displays to a
//! string that parses to the same key. `matches_last_group` on a fixed key
//! agrees with a straightforward reference normalization for every input,
//! including over-long and invalid ones.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::{Error, SecretBytes, SecretKey};

/// The golden Secret Key `00 01 .. 0f` (`tests/golden/secret_key_display.txt`).
fn fixed_key() -> SecretKey {
    let mut bytes: [u8; 16] = core::array::from_fn(|i| i as u8);
    SecretKey::from_secret(SecretBytes::from_array(&mut bytes))
}

/// §5.2 leniency written out plainly: drop `-` and whitespace, upper-case
/// ASCII, map `O` to `0` and `I`/`L` to `1`.
fn reference_normalize(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    match SecretKey::parse(s) {
        Ok(sk) => {
            let shown = sk.to_display().expect("display");
            let again = SecretKey::parse(shown.expose_secret()).expect("re-parse");
            assert!(again == sk, "display/parse round trip");
            assert!(sk.matches_last_group(sk.last_group().expect("group").expose_secret()));
        }
        Err(e) => assert!(
            e == Error::SecretKeyMalformed || e == Error::SecretKeyMistyped,
            "unexpected error {e:?}"
        ),
    }

    let sk = fixed_key();
    let group = sk.last_group().expect("group");
    let expected = reference_normalize(s) == group.expose_secret();
    assert_eq!(
        sk.matches_last_group(s),
        expected,
        "matches_last_group disagrees with the reference"
    );
});

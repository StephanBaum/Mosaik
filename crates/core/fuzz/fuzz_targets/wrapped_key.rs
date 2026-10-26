//! Wrapped-key decoding and opening.
//!
//! - Mode 0: wrap the fixed AK under the fixed `W` and check the round trip.
//! - Mode `0xFE`: take an authentic golden wrapping (`data[1]` picks one of
//!   the three AK wrappings or the VK wrapping), apply the fuzzer's edits,
//!   and require the original key if and only if the bytes are unchanged.
//! - Otherwise: arbitrary input decodes strictly, re-encodes to itself, and
//!   never unwraps, except the exact golden wrappings (the keys are fixed).
//!
//! Keys are fixed (the golden keys), so a crash reproduces from its input.
#![no_main]

mod common;

use common::TAMPER;
use libfuzzer_sys::fuzz_target;
use mosaic_core::{DeviceId, Error, KeyGeneration, UnlockMethod, WrappedKey};

fuzz_target!(|data: &[u8]| {
    let account = common::account();
    let vault = common::vault();
    let device = DeviceId::from_bytes(common::DEVICE).expect("id");
    let w = common::w();
    let ak = common::ak();

    match data.split_first() {
        Some((&0, rest)) => {
            let method = match rest.first().copied().unwrap_or(0) % 3 {
                0 => UnlockMethod::Password,
                1 => UnlockMethod::QuickUnlock(device),
                _ => UnlockMethod::hardware_key(rest),
            };
            let wrapped = ak.wrap(&w, &account, &method).expect("wrap");
            let decoded = WrappedKey::from_bytes(wrapped.as_bytes()).expect("decode");
            let back = decoded
                .unwrap_account_key(&w, &account, &method)
                .expect("unwrap");
            assert!(back == ak);
        }
        Some((&TAMPER, rest)) => {
            let Some((&which, ops)) = rest.split_first() else {
                return;
            };
            let methods = [
                (common::WRAPPED_AK_PASSWORD, UnlockMethod::Password),
                (common::WRAPPED_AK_QUICK, UnlockMethod::QuickUnlock(device)),
                (
                    common::WRAPPED_AK_HARDWARE,
                    UnlockMethod::hardware_key(b"credential-id"),
                ),
            ];
            let golden = match methods.get(usize::from(which % 4)) {
                Some((g, _)) => g,
                None => &common::WRAPPED_VK,
            };
            let base = common::hex_file(golden);
            let bytes = common::mutate(&base, ops);
            let unchanged = bytes == base;
            let decoded = WrappedKey::from_bytes(&bytes);
            if let Some((_, method)) = methods.get(usize::from(which % 4)) {
                let r = decoded.and_then(|k| k.unwrap_account_key(&w, &account, method));
                check(unchanged, r.map(|k| k == ak));
            } else {
                let r = decoded
                    .and_then(|k| k.unwrap_vault_key(&ak, &account, &vault, KeyGeneration::FIRST));
                check(unchanged, r.map(|k| k == common::vk(1)));
            }
        }
        _ => {
            let Ok(k) = WrappedKey::from_bytes(data) else {
                return;
            };
            assert_eq!(k.as_bytes().as_slice(), data);
            // With fixed keys, only the exact golden wrapping for a method
            // unwraps, and only to the golden key.
            let only_golden = |opened: Result<bool, Error>, golden: &str| match opened {
                Ok(same_key) => assert!(
                    same_key && data == common::hex_file(golden).as_slice(),
                    "only the golden wrapping may unwrap"
                ),
                Err(e) => assert_eq!(e, Error::Decryption),
            };
            for (golden, method) in [
                (common::WRAPPED_AK_PASSWORD, UnlockMethod::Password),
                (common::WRAPPED_AK_QUICK, UnlockMethod::QuickUnlock(device)),
                (
                    common::WRAPPED_AK_HARDWARE,
                    UnlockMethod::hardware_key(b"credential-id"),
                ),
            ] {
                let r = k.unwrap_account_key(&w, &account, &method);
                only_golden(r.map(|key| key == ak), golden);
            }
            let r = k.unwrap_vault_key(&ak, &account, &vault, KeyGeneration::FIRST);
            only_golden(r.map(|key| key == common::vk(1)), common::WRAPPED_VK);
        }
    }
});

/// An unchanged golden wrapping unwraps to the golden key; any edit fails
/// with a documented error.
fn check(unchanged: bool, r: Result<bool, Error>) {
    if unchanged {
        assert_eq!(
            r,
            Ok(true),
            "unchanged wrapping must unwrap to the golden key"
        );
    } else {
        let e = r.expect_err("a modified wrapping must not unwrap");
        assert!(common::tamper_error_ok(e), "unexpected error {e:?}");
    }
}

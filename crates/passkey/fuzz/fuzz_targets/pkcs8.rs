//! `Es256Key::from_pkcs8_der` on arbitrary bytes: it never panics, and a key
//! that loads re-encodes to a PKCS#8 document that loads to the same public
//! key (COSE key and SPKI).
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_passkey::Es256Key;

fuzz_target!(|data: &[u8]| {
    let Ok(key) = Es256Key::from_pkcs8_der(data) else {
        return;
    };
    let der = key.to_pkcs8_der().expect("a loaded key encodes");
    let again = Es256Key::from_pkcs8_der(der.expose_secret()).expect("own encoding loads");
    assert_eq!(again.cose_public_key(), key.cose_public_key());
    assert_eq!(again.public_key_spki_der(), key.public_key_spki_der());
    // The public key is the SEC1 point behind the COSE key and the SPKI.
    let cose = key.cose_public_key();
    let spki = key.public_key_spki_der();
    assert_eq!(&spki[27..59], &cose[10..42]);
    assert_eq!(&spki[59..91], &cose[45..77]);
});

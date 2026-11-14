//! The UniFFI bindings generator (`scripts/mobile/bindgen.sh`): Kotlin and Swift
//! from the compiled library. Built only with the `bindgen` feature.
#![forbid(unsafe_code)]

fn main() {
    uniffi::uniffi_bindgen_main();
}

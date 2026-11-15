//! Windows program URLs (ADR-0158): arbitrary text never panics the parser; an
//! accepted URL prints back to its trimmed input; a program of any class but
//! `Ordinary` never matches a parsed URL.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::model::LoginUrl;
use mosaic_origin::program::{
    ProgramClass, ProgramIdentity, ProgramUrl, binding_change, program_fill_allowed,
};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(url) = ProgramUrl::parse(text) else {
        return;
    };
    assert_eq!(url.canonical(), text.trim(), "program url round trip");
    let saved = LoginUrl::new(&url.canonical());
    let (exe, signer, hash, path, family) = match &url {
        ProgramUrl::Package { family } => ("x.exe", None, None, "C:\\x.exe", Some(family.as_str())),
        ProgramUrl::Signed { exe, signer } => (exe.as_str(), Some(*signer), None, "C:\\x.exe", None),
        ProgramUrl::Path { path, sha256 } => ("x.exe", None, Some(*sha256), path.as_str(), None),
    };
    for class in [
        ProgramClass::Browser,
        ProgramClass::Host,
        ProgramClass::Mosaic,
        ProgramClass::Broker,
        ProgramClass::Remote,
    ] {
        let observed = ProgramIdentity {
            package_family: family,
            exe_name: exe,
            signer_id: signer,
            final_path: path,
            file_sha256: hash,
            class,
        };
        assert!(program_fill_allowed(&saved, &observed).is_err());
        assert!(binding_change(&saved, &observed).is_none());
    }
});

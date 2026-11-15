//! Tests of the shim API. Anything that needs a human at the desktop is
//! `#[ignore]` and part of the manual checklist in `docs/PROGRAM_FILL.md`.

use crate::{
    ProcessHandle, ShimError, Signature, Uia, current_integrity_rid, final_path, foreground_window,
    parent_pid, process_chain, verify_authenticode, window_facts,
};

const KERNEL32: &str = r"C:\Windows\System32\kernel32.dll";

#[test]
fn own_process_identity() {
    let h = ProcessHandle::open(std::process::id()).unwrap();
    assert_eq!(h.pid(), std::process::id());
    assert!(h.is_running().unwrap());
    let exe = std::env::current_exe().unwrap();
    assert!(
        final_path(&h.image_path().unwrap())
            .unwrap()
            .eq_ignore_ascii_case(&final_path(exe.to_str().unwrap()).unwrap())
    );
    assert_eq!(h.package_family_name().unwrap(), None);
    assert_eq!(h.integrity_rid().unwrap(), current_integrity_rid().unwrap());
    assert!(h.integrity_rid().unwrap() >= 0x1000);
}

#[test]
fn bad_pids_are_errors() {
    assert!(ProcessHandle::open(0).is_err());
    assert!(ProcessHandle::open(u32::MAX).is_err());
}

#[test]
fn a_finished_process_is_not_running() {
    let mut child = std::process::Command::new("cmd")
        .args(["/C", "exit", "0"])
        .spawn()
        .unwrap();
    let handle = ProcessHandle::open(child.id()).unwrap();
    child.wait().unwrap();
    // The handle keeps the process object, so the answer is still available.
    assert!(!handle.is_running().unwrap());
}

#[test]
fn final_path_strips_the_device_prefix() {
    let path = final_path(r"C:\Windows\System32\..\System32\kernel32.dll").unwrap();
    assert!(path.starts_with(r"C:\"), "{path}");
    assert!(!path.contains(r"\\?\"), "{path}");
    assert!(path.eq_ignore_ascii_case(KERNEL32), "{path}");
}

#[test]
fn final_path_of_nothing_is_an_error() {
    assert_eq!(
        final_path(r"C:\this\does\not\exist.exe"),
        Err(ShimError::NotFound)
    );
    assert!(final_path("bad\0path").is_err());
}

#[test]
fn parent_ids_chain_up_and_stop() {
    let me = std::process::id();
    assert!(parent_pid(me).unwrap().is_some());
    let chain = process_chain(me, 16).unwrap();
    assert_eq!(chain.first(), Some(&me));
    assert!(chain.len() >= 2 && chain.len() <= 16);
    let mut unique = chain.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), chain.len(), "no repeated ID");
    assert_eq!(process_chain(me, 1).unwrap(), vec![me]);
    assert_eq!(parent_pid(u32::MAX).unwrap(), None);
}

#[test]
fn a_system_binary_is_trusted_and_names_microsoft() {
    match verify_authenticode(KERNEL32).unwrap() {
        Signature::Trusted {
            subject_der,
            organization,
            common_name,
        } => {
            assert_ne!(subject_der.len(), 0);
            assert_eq!(subject_der.first(), Some(&0x30), "a DER SEQUENCE");
            assert!(
                organization
                    .as_deref()
                    .is_some_and(|o| o.contains("Microsoft")),
                "{organization:?} {common_name:?}"
            );
        }
        other => panic!("kernel32.dll is {other:?}"),
    }
}

#[test]
fn a_catalog_signed_binary_is_trusted() {
    // notepad.exe is signed through a catalog on Windows 10.
    let notepad = r"C:\Windows\System32\notepad.exe";
    if std::path::Path::new(notepad).exists() {
        assert!(matches!(
            verify_authenticode(notepad).unwrap(),
            Signature::Trusted { .. }
        ));
    }
}

#[test]
fn a_test_binary_is_unsigned() {
    let exe = std::env::current_exe().unwrap();
    assert_eq!(
        verify_authenticode(exe.to_str().unwrap()).unwrap(),
        Signature::Unsigned
    );
}

#[test]
fn a_tampered_signed_file_is_untrusted() {
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("kernel32-copy.dll");
    let mut bytes = std::fs::read(KERNEL32).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xFF;
    std::fs::write(&copy, &bytes).unwrap();
    assert_eq!(
        verify_authenticode(copy.to_str().unwrap()).unwrap(),
        Signature::Untrusted
    );
}

#[test]
fn verifying_a_missing_file_is_an_error() {
    assert_eq!(
        verify_authenticode(r"C:\this\does\not\exist.dll"),
        Err(ShimError::NotFound)
    );
}

#[test]
fn window_facts_of_nothing_is_an_error() {
    assert_eq!(window_facts(0), Err(ShimError::NotFound));
    assert_eq!(window_facts(0x7FFF_FFF0), Err(ShimError::NotFound));
}

#[test]
fn the_foreground_window_reads_without_panicking() {
    // There may be no interactive desktop (CI); both answers are fine.
    if let Some(hwnd) = foreground_window()
        && let Ok(facts) = window_facts(hwnd)
    {
        assert_ne!(facts.owner_pid, 0);
    }
}

#[test]
fn a_uia_client_starts_and_stops_on_a_worker_thread() {
    // Creating and dropping the client (COM init and uninit) needs no desktop.
    std::thread::spawn(|| {
        let uia = Uia::new(2000, 2000).unwrap();
        drop(uia);
        // A second client on the same thread works after the first dropped.
        drop(Uia::new(2000, 2000).unwrap());
    })
    .join()
    .unwrap();
}

#[test]
#[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]
fn uia_reads_the_focused_element() {
    let uia = Uia::new(2000, 2000).unwrap();
    let element = uia.focused().unwrap().expect("something has focus");
    let facts = element.facts();
    assert_ne!(facts.pid, 0);
    assert_ne!(facts.runtime_id.len(), 0);
    assert!(uia.same(&element, &element).unwrap());
    assert!(uia.is_within(&element, &element).unwrap());
    let [left, top, right, bottom] = facts.rect;
    if right > left && bottom > top {
        let center = uia
            .element_from_point(i32::midpoint(left, right), i32::midpoint(top, bottom))
            .unwrap();
        assert!(center.is_some());
    }
}

#[test]
fn public_api_has_no_unsafe_fn_and_no_windows_types() {
    let sources = [
        include_str!("lib.rs"),
        include_str!("error.rs"),
        include_str!("sys_process.rs"),
        include_str!("sys_trust.rs"),
        include_str!("sys_uia.rs"),
        include_str!("sys_window.rs"),
    ];
    let public_unsafe = ["pub", " unsafe", " fn"].concat();
    let windows_path = ["windows", "::"].concat();
    let raw_types = ["HANDLE", "HWND", "*mut", "*const"];
    for source in sources {
        assert!(!source.contains(&public_unsafe));
        for line in source.lines() {
            let line = line.trim_start();
            if line.starts_with("pub ") && !line.starts_with("pub(crate)") {
                assert!(!line.contains(&windows_path), "{line}");
                for raw in raw_types {
                    assert!(!line.contains(raw), "{line}");
                }
            }
        }
    }
}

#[test]
fn unsafe_stays_in_the_sys_modules() {
    // `unsafe` blocks and expectations only in `sys_*` modules.
    let keyword = ["unsafe", " {"].concat();
    let expectation = ["expect(", "unsafe_code"].concat();
    for (name, source) in [
        ("lib.rs", include_str!("lib.rs")),
        ("error.rs", include_str!("error.rs")),
    ] {
        assert!(!source.contains(&keyword), "{name}");
        assert!(!source.contains(&expectation), "{name}");
    }
}

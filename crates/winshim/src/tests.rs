//! Tests of the shim API. Anything that needs a human at the desktop is
//! `#[ignore]` and part of the manual checklist in `docs/PROGRAM_FILL.md`.

use std::time::{Duration, Instant};

use crate::{
    ImageFile, ProcessHandle, ShimError, Signature, Uia, current_integrity_rid, final_path,
    foreground_window, parent_pid, process_chain, window_facts,
};

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

fn verify(path: &str) -> Result<Signature, ShimError> {
    ImageFile::open(path)?.verify_authenticode(soon())
}

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
    match verify(KERNEL32).unwrap() {
        Signature::Trusted {
            subject_der,
            root_der,
            organization,
            common_name,
        } => {
            assert_ne!(subject_der.len(), 0);
            assert_eq!(root_der.first(), Some(&0x30), "a DER certificate");
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
            verify(notepad).unwrap(),
            Signature::Trusted { .. }
        ));
    }
}

#[test]
fn a_test_binary_is_unsigned() {
    let exe = std::env::current_exe().unwrap();
    assert_eq!(verify(exe.to_str().unwrap()).unwrap(), Signature::Unsigned);
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
        verify(copy.to_str().unwrap()).unwrap(),
        Signature::Untrusted
    );
}

#[test]
fn verifying_a_missing_file_is_an_error() {
    assert_eq!(
        verify(r"C:\this\does\not\exist.dll"),
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
    let element = uia.focused(soon()).unwrap().expect("something has focus");
    let facts = element.facts();
    assert_ne!(facts.pid, 0);
    assert_ne!(facts.runtime_id.len(), 0);
    assert!(uia.same(&element, &element, soon()).unwrap());
    assert!(uia.is_within(&element, &element, soon()).unwrap());
    let [left, top, right, bottom] = facts.rect;
    if right > left && bottom > top {
        let center = uia
            .element_from_point(
                i32::midpoint(left, right),
                i32::midpoint(top, bottom),
                soon(),
            )
            .unwrap();
        assert!(center.is_some());
    }
}

#[test]
fn an_expired_deadline_stops_every_step() {
    let past = Instant::now();
    std::thread::sleep(Duration::from_millis(5));
    let image = ImageFile::open(KERNEL32).unwrap();
    assert_eq!(image.verify_authenticode(past), Err(ShimError::Timeout));
    std::thread::spawn(move || {
        let uia = Uia::new(2000, 2000).unwrap();
        assert_eq!(uia.focused(past).unwrap_err(), ShimError::Timeout);
        assert_eq!(
            uia.element_from_point(0, 0, past).unwrap_err(),
            ShimError::Timeout
        );
    })
    .join()
    .unwrap();
}

#[test]
fn an_open_image_cannot_be_renamed_or_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("held.exe");
    std::fs::write(&path, b"MZ").unwrap();
    let image = ImageFile::open(path.to_str().unwrap()).unwrap();
    assert!(
        std::fs::rename(&path, dir.path().join("moved.exe")).is_err(),
        "rename must fail while the image is held"
    );
    assert!(std::fs::remove_file(&path).is_err());
    assert!(!image.is_network().unwrap());
    drop(image);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn a_process_image_opens_once_and_matches_its_path() {
    let me = ProcessHandle::open(std::process::id()).unwrap();
    let image = me.open_image().unwrap();
    let exe = final_path(std::env::current_exe().unwrap().to_str().unwrap()).unwrap();
    assert!(image.final_path().eq_ignore_ascii_case(&exe));
}

#[test]
fn network_paths_are_recognised() {
    use crate::sys_process::{drive_root, is_unc_path};
    assert!(is_unc_path(r"\\srv\share\a.exe"));
    assert!(!is_unc_path(r"C:\a.exe"));
    assert_eq!(drive_root(r"C:\dir\a.exe").as_deref(), Some(r"C:\"));
    assert_eq!(drive_root(r"\\srv\share"), None);
}

#[test]
fn the_element_cache_includes_the_read_only_property() {
    use windows::Win32::UI::Accessibility::UIA_ValueIsReadOnlyPropertyId;
    assert!(crate::sys_uia::CACHED_PROPERTIES.contains(&UIA_ValueIsReadOnlyPropertyId));
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

#[test]
fn clients_started_at_the_same_time_all_start() {
    // UI Automation's first use fails on concurrent threads without the
    // start-up lock (CreateCacheRequest returns E_FAIL).
    let threads: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(|| Uia::new(2000, 2000).map(drop)))
        .collect();
    for thread in threads {
        thread.join().unwrap().unwrap();
    }
}

#[test]
#[ignore = "needs a file signed under a root only in the per-user store: docs/PROGRAM_FILL.md checklist"]
fn a_signature_rooted_in_a_user_only_store_is_untrusted() {
    // `MOSAIC_USER_ROOT_SIGNED` is a file whose signature `WinVerifyTrust`
    // accepts only because its root is in `HKCU\...\SystemCertificates\Root`.
    let path = std::env::var("MOSAIC_USER_ROOT_SIGNED").expect("set MOSAIC_USER_ROOT_SIGNED");
    assert_eq!(verify(&path).unwrap(), Signature::Untrusted);
}

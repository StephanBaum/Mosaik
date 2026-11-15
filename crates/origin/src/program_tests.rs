//! Tests of the Windows program URLs and their matching (ADR-0158).

use mosaic_core::model::LoginUrl;

use crate::program::{
    BindingChange, ProgramClass, ProgramIdentity, ProgramUrl, binding_change, classify_program,
    fold_name, paths_equal, program_fill_allowed, program_lookalike_of,
};
use crate::{MatchKind, Refusal};

const SIGNER: [u8; 32] = [0x11; 32];
const OTHER: [u8; 32] = [0x22; 32];
const HASH: [u8; 32] = [0x33; 32];
const PATH: &str = r"C:\Users\a\AppData\Local\Tool\tool.exe";

fn id<'a>(
    exe: &'a str,
    path: &'a str,
    signer: Option<[u8; 32]>,
    hash: Option<[u8; 32]>,
) -> ProgramIdentity<'a> {
    ProgramIdentity {
        package_family: None,
        exe_name: exe,
        signer_id: signer,
        final_path: path,
        file_sha256: hash,
        class: ProgramClass::Ordinary,
    }
}

fn saved(u: &ProgramUrl) -> LoginUrl {
    LoginUrl::new(&u.canonical())
}

#[test]
fn program_url_round_trips() {
    for u in [
        ProgramUrl::package("Microsoft.WindowsNotepad_8wekyb3d8bbwe").unwrap(),
        ProgramUrl::signed("Discord.exe", SIGNER).unwrap(),
        ProgramUrl::path(PATH, HASH).unwrap(),
    ] {
        assert_eq!(ProgramUrl::parse(&u.canonical()).unwrap(), u);
    }
    assert_eq!(
        ProgramUrl::package("Microsoft.WindowsNotepad_8wekyb3d8bbwe")
            .unwrap()
            .canonical(),
        "winapp://package/microsoft.windowsnotepad_8wekyb3d8bbwe"
    );
    assert!(
        ProgramUrl::signed("Discord.exe", SIGNER)
            .unwrap()
            .canonical()
            .starts_with("winapp://signed/discord.exe?signer=")
    );
}

#[test]
fn program_url_parse_refuses_non_canonical_forms() {
    let good = ProgramUrl::signed("tool.exe", SIGNER).unwrap().canonical();
    for bad in [
        good.replace("winapp://", "WINAPP://"),
        format!("{good}&x=1"),
        format!("{good}#f"),
        good.replace("tool.exe", "Tool.exe"),
        good.replace("tool.exe", "tool%2eexe"),
        good.replace("tool.exe", "tool%2fexe"),
        good.replace("tool.exe", "%54ool.exe"),
        good.replace("tool.exe", "tool%2Dexe"),
        good.replace("?signer=", "?sha256="),
        format!("{good}="),
        "winapp://signed/?signer=".into(),
        "winapp://package/ab_8wekyb3d8bbwe".into(),
        "winapp://package/name_8WEKYB3D8BBWE".into(),
        "winapp://package/name_8wekyb3d8bbwe?x=1".into(),
        "winapp://other/x".into(),
        "androidapp://com.example".into(),
    ] {
        assert!(ProgramUrl::parse(&bad).is_err(), "{bad}");
    }
    // Outer whitespace is the only tolerated difference.
    assert!(ProgramUrl::parse(&format!("  {good}\n")).is_ok());
    // Another spelling of the path form: padding, extra query, wrong key.
    let path = ProgramUrl::path(PATH, HASH).unwrap().canonical();
    for bad in [
        format!("{path}&x=1"),
        path.replace("?sha256=", "?signer="),
        path.replace("path/", "path/=="),
        "winapp://path/QzpcdC5leGU".into(),
    ] {
        assert!(ProgramUrl::parse(&bad).is_err(), "{bad}");
    }
}

#[test]
fn exe_names_fold_case_but_not_script() {
    assert_eq!(fold_name("DISCORD.EXE"), "discord.exe");
    assert_eq!(
        ProgramUrl::signed("Discord.Exe", SIGNER),
        ProgramUrl::signed("discord.exe", SIGNER)
    );
    assert_ne!(fold_name("disc\u{043e}rd.exe"), "discord.exe"); // Cyrillic o stays
    assert_eq!(fold_name("\u{0131}.exe"), "\u{0131}.exe"); // dotless i stays
    let cyrillic = ProgramUrl::signed("disc\u{043e}rd.exe", SIGNER).unwrap();
    assert_ne!(cyrillic, ProgramUrl::signed("discord.exe", SIGNER).unwrap());
    assert_eq!(ProgramUrl::parse(&cyrillic.canonical()).unwrap(), cyrillic);
    assert!(ProgramUrl::signed("a\\b.exe", SIGNER).is_err());
    assert!(ProgramUrl::signed("", SIGNER).is_err());
    assert!(ProgramUrl::signed("..", SIGNER).is_err());
}

#[test]
fn path_urls_accept_only_final_drive_or_unc_paths() {
    assert!(ProgramUrl::path(r"\\server\share\t.exe", HASH).is_ok());
    for p in [
        r"\\?\C:\t.exe",
        r"\\.\C:\t.exe",
        "C:/t.exe",
        "t.exe",
        r"C:t.exe",
        "",
        r"C:\a\..\t.exe",
        r"C:\a.\t.exe",
        r"C:\a \t.exe",
        r"C:\a\t.exe.",
        r"\\server\t.exe",
        r"\\\share\t.exe",
        "C:\\a\u{0}b.exe",
        r"C:\a:stream\t.exe",
    ] {
        assert!(ProgramUrl::path(p, HASH).is_err(), "{p}");
    }
    assert!(paths_equal(r"C:\Tools\T.exe", r"c:\tools\t.EXE"));
    // The length cap holds on the decoded path.
    let long = format!(r"C:\{}", "a".repeat(4094));
    assert!(ProgramUrl::path(&long, HASH).is_err());
}

#[test]
fn signed_binding_needs_equal_signer_and_exe_name() {
    let url = saved(&ProgramUrl::signed("Discord.exe", SIGNER).unwrap());
    let o = id("discord.exe", r"C:\x\discord.exe", Some(SIGNER), None);
    assert_eq!(program_fill_allowed(&url, &o), Ok(MatchKind::ExactOrigin));
    // The path may differ: the binding is the signer and the name.
    let o = id("discord.exe", r"D:\y\discord.exe", Some(SIGNER), None);
    assert_eq!(program_fill_allowed(&url, &o), Ok(MatchKind::ExactOrigin));
}

#[test]
fn same_exe_other_signer_does_not_match() {
    let url = saved(&ProgramUrl::signed("discord.exe", SIGNER).unwrap());
    let o = id("discord.exe", r"C:\x.exe", Some(OTHER), None);
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
    assert_eq!(binding_change(&url, &o), Some(BindingChange::SignerChanged));
    // An unsigned image with that name is not a signer change.
    let o = id("discord.exe", r"C:\x.exe", None, Some(HASH));
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
    assert_eq!(binding_change(&url, &o), None);
}

#[test]
fn same_signer_other_exe_does_not_match() {
    let url = saved(&ProgramUrl::signed("discord.exe", SIGNER).unwrap());
    let o = id("discord2.exe", r"C:\x.exe", Some(SIGNER), None);
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
    assert_eq!(binding_change(&url, &o), None);
}

#[test]
fn unsigned_binding_needs_equal_path_and_hash() {
    let url = saved(&ProgramUrl::path(PATH, HASH).unwrap());
    let upper = PATH.to_ascii_uppercase();
    let same = id("tool.exe", &upper, None, Some(HASH));
    assert_eq!(
        program_fill_allowed(&url, &same),
        Ok(MatchKind::ExactOrigin)
    );
    let other_hash = id("tool.exe", PATH, None, Some(OTHER));
    assert_eq!(
        program_fill_allowed(&url, &other_hash),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        binding_change(&url, &other_hash),
        Some(BindingChange::FileChanged)
    );
    let other_path = id("tool.exe", r"C:\Other\tool.exe", None, Some(HASH));
    assert_eq!(
        program_fill_allowed(&url, &other_path),
        Err(Refusal::NoMatch)
    );
    assert_eq!(binding_change(&url, &other_path), None);
    // No hash means nothing to compare: never a match.
    let no_hash = id("tool.exe", PATH, None, None);
    assert_eq!(program_fill_allowed(&url, &no_hash), Err(Refusal::NoMatch));
}

#[test]
fn signed_image_never_matches_a_path_binding() {
    let url = saved(&ProgramUrl::path(PATH, HASH).unwrap());
    let o = id("tool.exe", PATH, Some(SIGNER), Some(HASH));
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
}

#[test]
fn package_binding_needs_equal_family() {
    let family = "microsoft.windowsnotepad_8wekyb3d8bbwe";
    let url = saved(&ProgramUrl::package(family).unwrap());
    let mut o = id("notepad.exe", r"C:\x.exe", None, None);
    o.package_family = Some(family);
    assert_eq!(program_fill_allowed(&url, &o), Ok(MatchKind::ExactOrigin));
    o.package_family = Some("microsoft.windowscalculator_8wekyb3d8bbwe");
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
    o.package_family = None;
    assert_eq!(program_fill_allowed(&url, &o), Err(Refusal::NoMatch));
}

#[test]
fn browsers_hosts_brokers_and_remote_clients_never_match() {
    for (exe, class) in [
        ("chrome.exe", ProgramClass::Browser),
        ("python.exe", ProgramClass::Host),
        ("consent.exe", ProgramClass::Broker),
        ("mstsc.exe", ProgramClass::Remote),
        ("mosaic-desktop.exe", ProgramClass::Mosaic),
    ] {
        assert_eq!(classify_program(exe, None), class);
        assert_eq!(classify_program(&exe.to_ascii_uppercase(), None), class);
        let url = ProgramUrl::signed(exe, SIGNER).unwrap();
        let mut o = id(exe, r"C:\x.exe", Some(SIGNER), None);
        o.class = class;
        assert_eq!(
            program_fill_allowed(&saved(&url), &o),
            Err(Refusal::NoMatch),
            "{exe}"
        );
        // A caller that forgot to set the class is still refused by name.
        o.class = ProgramClass::Ordinary;
        assert_eq!(
            program_fill_allowed(&saved(&url), &o),
            Err(Refusal::NoMatch),
            "{exe}"
        );
    }
    assert_eq!(
        classify_program("x.exe", Some("microsoft.windowsterminal_8wekyb3d8bbwe")),
        ProgramClass::Host
    );
    assert_eq!(
        classify_program("x.exe", Some("Microsoft.RemoteDesktop_8wekyb3d8bbwe")),
        ProgramClass::Remote
    );
    assert_eq!(
        classify_program("notepad.exe", None),
        ProgramClass::Ordinary
    );
}

#[test]
fn web_and_app_urls_never_match_a_program() {
    let o = id("discord.exe", r"C:\x.exe", Some(SIGNER), None);
    for url in [
        "https://discord.com",
        "androidapp://com.discord",
        "winapp://nonsense",
        "discord.exe",
    ] {
        assert_eq!(
            program_fill_allowed(&LoginUrl::new(url), &o),
            Err(Refusal::NoMatch),
            "{url}"
        );
    }
}

#[test]
fn lookalike_names_are_found_without_matching() {
    // One substituted homoglyph, one inserted letter, one swap.
    assert_eq!(
        program_lookalike_of("disc\u{043e}rd.exe", ["discord.exe"]),
        Some("discord.exe")
    );
    assert_eq!(
        program_lookalike_of("dlscord.exe", ["discord.exe"]),
        Some("discord.exe")
    );
    assert_eq!(
        program_lookalike_of("discrod.exe", ["discord.exe"]),
        Some("discord.exe")
    );
    // The lexicographically first of several.
    assert_eq!(
        program_lookalike_of("toolx.exe", ["tooly.exe", "toolz.exe"]),
        Some("tooly.exe")
    );
    // Itself, far names and short names are none.
    assert_eq!(program_lookalike_of("discord.exe", ["discord.exe"]), None);
    assert_eq!(program_lookalike_of("slack.exe", ["discord.exe"]), None);
    assert_eq!(program_lookalike_of("abc.exe", ["abd.exe"]), None);
}

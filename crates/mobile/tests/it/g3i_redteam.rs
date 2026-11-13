//! M10-G3 red team (iOS): the service identifiers of a hostile page or app,
//! the record hint of a `QuickType` identity, the passkey calls in hash mode,
//! and the store lifecycle under the extension's races. A hole found here is
//! `#[ignore = "M10-G3i-<n>: ..."]` until its fix; the other tests pin
//! defenses that held.

use std::time::{Duration, Instant};

use mosaic_mobile::{
    CoreContext, FillTarget, FillTargetKind, IosAssertRequest, IosRegisterRequest, ItemKey,
    LoginDraft, MobileError, OriginWarning, OtpEdit, Platform, ServiceIdentifier,
    ServiceIdentifierKind, SessionState, SettingsUpdate, UrlDraft,
};

use crate::common::{BOOT, PASSWORD, Phone};

const USER_HANDLE: [u8; 8] = [4; 8];

fn url(u: &str, etld1: bool, allow_http: bool) -> UrlDraft {
    UrlDraft {
        url: u.to_owned(),
        match_etld1: etld1,
        allow_http,
    }
}

fn login(phone: &Phone, title: &str, username: &str, urls: Vec<UrlDraft>) -> ItemKey {
    phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: title.to_owned(),
            username: username.to_owned(),
            password: Some(b"very-secret-Passw0rd".to_vec()),
            urls,
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap()
}

fn domain(value: &str) -> ServiceIdentifier {
    ServiceIdentifier {
        kind: ServiceIdentifierKind::Domain,
        value: value.to_owned(),
    }
}

fn page_url(value: &str) -> ServiceIdentifier {
    ServiceIdentifier {
        kind: ServiceIdentifierKind::Url,
        value: value.to_owned(),
    }
}

fn origin_of(target: &FillTarget) -> &str {
    match &target.kind {
        FillTargetKind::Web { origin } => origin,
        FillTargetKind::AndroidApp { .. } => panic!("an iOS target is a web origin"),
    }
}

/// What the extension does for one identifier: the target, then the fill
/// session and the candidates (titles), or the error of either step.
fn list(
    phone: &Phone,
    identifier: ServiceIdentifier,
) -> Result<(FillTarget, String, Vec<String>), MobileError> {
    let target = phone.core.fill_target_ios(identifier)?;
    let found = phone.core.fill_candidates(target.clone())?;
    let titles = found.candidates.iter().map(|c| c.title.clone()).collect();
    Ok((target, found.session_id, titles))
}

fn wait_for_busy(core: &mosaic_mobile::Core) {
    let start = Instant::now();
    while core.state() != SessionState::Busy {
        assert!(start.elapsed() < Duration::from_secs(20), "never busy");
        std::thread::sleep(Duration::from_millis(2));
    }
}

// ---- Service identifiers ----------------------------------------------------------------------

/// Attack: a `.domain` identifier that is not a plain host (a port, a path,
/// userinfo, a scheme, a trailing dot, whitespace, a confusable, a zero-width
/// or a full-width character) in the hope that the core reads it as
/// `github.com` while the page is elsewhere, or as elsewhere while the login
/// of `github.com` is still offered. Defense: the domain goes through the
/// WHATWG host parser or is refused, the target is its canonical ASCII form,
/// and a login is offered only when its saved origin **equals** that target.
/// So whenever the GitHub login appears, the target is `https://github.com`.
#[test]
fn g3i_domain_identifier_disguises_never_reach_another_sites_login() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "GitHub",
        "octo",
        vec![url("https://github.com", false, false)],
    );
    login(
        &phone,
        "Bank",
        "alice",
        vec![url("https://bank.example", false, false)],
    );
    let disguises = [
        "github.com",
        "GITHUB.COM",
        " github.com ",
        "github.com:443",
        "github.com:8443",
        "github.com/login",
        "octo@github.com",
        "https://github.com",
        "github.com.",
        "github.com evil.example",
        "github.com\tevil.example",
        "github.com\u{200b}",
        "github.com%2eevil.example",
        "g\u{456}thub.com",
        "\u{ff47}ithub.com",
        "github.com\u{3002}",
        "xn--gthub-n2e.com",
        "github.com.evil.example",
        "evil.example.github.com",
        "bank.example",
        "127.0.0.1",
        "[::1]",
        "localhost",
        "",
    ];
    for disguise in disguises {
        match list(&phone, domain(disguise)) {
            Ok((target, _, titles)) => {
                let origin = origin_of(&target);
                assert!(
                    origin.starts_with("https://"),
                    "{disguise:?}: a domain identifier is always an https target, got {origin}"
                );
                assert_eq!(
                    origin.trim_start_matches("https://"),
                    target.display,
                    "{disguise:?}: the display is the host the match uses"
                );
                if titles.contains(&"GitHub".to_owned()) {
                    assert_eq!(
                        origin, "https://github.com",
                        "{disguise:?}: the GitHub login reached {origin}"
                    );
                    assert_eq!(titles, ["GitHub"]);
                }
                if titles.contains(&"Bank".to_owned()) {
                    assert_eq!(
                        origin, "https://bank.example",
                        "{disguise:?}: the Bank login reached {origin}"
                    );
                }
            }
            Err(e) => assert!(
                matches!(e, MobileError::InvalidInput | MobileError::InsecureOrigin),
                "{disguise:?}: {e:?}"
            ),
        }
    }
    // The cases with a fixed expectation.
    let (target, _, titles) = list(&phone, domain("GITHUB.COM")).unwrap();
    assert_eq!(target.display, "github.com");
    assert_eq!(titles, ["GitHub"]);
    let (target, _, titles) = list(&phone, domain("g\u{456}thub.com")).unwrap();
    assert_eq!(target.display, "xn--gthub-n2e.com");
    assert!(titles.is_empty(), "a confusable host gets no login");
    assert!(
        matches!(target.warning, Some(OriginWarning::Punycode { .. })),
        "{:?}",
        target.warning
    );
    let page = phone.core.fill_target_ios(domain("github.com.")).unwrap();
    assert!(
        phone
            .core
            .fill_candidates(page)
            .unwrap()
            .candidates
            .is_empty(),
        "a trailing dot is another origin: nothing fills"
    );
    for not_a_domain in [
        "127.0.0.1",
        "[::1]",
        "github.com:443",
        "octo@github.com",
        "",
    ] {
        assert_eq!(
            phone.core.fill_target_ios(domain(not_a_domain)).err(),
            Some(MobileError::InvalidInput),
            "{not_a_domain:?}"
        );
    }
}

/// Attack: a `.URL` identifier that hides the real origin behind userinfo,
/// a backslash, a port, a scheme the core does not fill, a suffix label or a
/// loopback address. Defense: the origin is the WHATWG origin of the URL
/// (scheme, host, port), the one the address bar shows; userinfo, path, query
/// and fragment are dropped; `http` needs `allow_http` (loopback included on a
/// phone, M10-G3-4), and any other scheme is refused.
#[test]
fn g3i_url_identifier_disguises_keep_the_origin_of_the_page() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "GitHub",
        "octo",
        vec![url("https://github.com", false, false)],
    );
    login(
        &phone,
        "Local",
        "dev",
        vec![url("http://localhost", false, false)],
    );
    let github = |page: &str| {
        let (target, _, titles) = list(&phone, page_url(page)).unwrap();
        assert_eq!(origin_of(&target), "https://github.com", "{page}");
        assert_eq!(titles, ["GitHub"], "{page}");
    };
    // The address bar shows github.com for these.
    github("https://github.com/login?next=/settings#x");
    github("HTTPS://GITHUB.COM:443/");
    github("https://evil.example@github.com/");
    github("https://evil.example:secret@github.com/");
    github("https://github.com\\@evil.example");
    github("https://github.com\\.evil.example");
    github("https://github.com/@evil.example");
    github("https://github.com?@evil.example");
    github("https://github.com#@evil.example");
    github("https://github%2Ecom/");

    // The address bar shows something else for these: no GitHub login.
    let elsewhere = |page: &str, origin: &str| {
        let (target, _, titles) = list(&phone, page_url(page)).unwrap();
        assert_eq!(origin_of(&target), origin, "{page}");
        assert!(titles.is_empty(), "{page}: {titles:?}");
    };
    elsewhere("https://github.com@evil.example/", "https://evil.example");
    elsewhere(
        "https://github.com:443@evil.example/",
        "https://evil.example",
    );
    elsewhere("https://github.com:8443/", "https://github.com:8443");
    elsewhere(
        "https://github.com.evil.example/",
        "https://github.com.evil.example",
    );
    elsewhere("https://evil.example/github.com", "https://evil.example");
    elsewhere("https://github.com./", "https://github.com.");
    elsewhere("https://www.github.com/", "https://www.github.com");
    elsewhere("https://xn--80ak6aa92e.com/", "https://xn--80ak6aa92e.com");
    elsewhere("https://127.0.0.1:8443/", "https://127.0.0.1:8443");
    elsewhere("https://[::1]/", "https://[::1]");

    // `http`: the target exists, and the fill is refused as insecure without `allow_http`, loopback too.
    for page in [
        "http://github.com/",
        "http://localhost/",
        "http://localhost:3000/",
        "http://127.0.0.1/",
        "http://[::1]/",
        "http://0x7f000001/",
    ] {
        let target = phone.core.fill_target_ios(page_url(page)).unwrap();
        assert!(origin_of(&target).starts_with("http://"), "{page}");
        let result = phone.core.fill_candidates(target);
        if page == "http://localhost/" {
            assert_eq!(
                result.err(),
                Some(MobileError::InsecureOrigin),
                "{page}: the saved http login exists, and plain http is refused without allow_http"
            );
        } else {
            // An https login never matches an http page of its host (no downgrade), and the other
            // origins have no login at all.
            assert!(
                result.unwrap().candidates.is_empty(),
                "{page}: no login for this origin"
            );
        }
    }

    // Schemes the core never fills, and strings that are no URL at all.
    for page in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "blob:https://github.com/5ce0e8a4",
        "data:text/html,<form>",
        "about:blank",
        "ftp://github.com/",
        "ws://github.com/",
        "chrome-extension://abc/",
    ] {
        assert_eq!(
            phone.core.fill_target_ios(page_url(page)).err(),
            Some(MobileError::InsecureOrigin),
            "{page}"
        );
    }
    for page in [
        "",
        "github.com",
        "//github.com",
        "https://",
        "https://user@",
        "not a url",
    ] {
        assert_eq!(
            phone.core.fill_target_ios(page_url(page)).err(),
            Some(MobileError::InvalidInput),
            "{page:?}"
        );
    }
}

/// Attack: a page under a lookalike or a parent of a saved registrable domain
/// with the eTLD+1 opt-in. Defense: the opt-in matches equal registrable
/// domains with equal schemes and any port; a suffix label, another public
/// suffix, an `http` downgrade and a confusable never match, and the lookalike
/// is named in the warning.
#[test]
fn g3i_etld1_opt_in_does_not_cross_registrable_domains_or_schemes() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "GitHub",
        "octo",
        vec![url("https://github.com", true, false)],
    );
    let titles = |page: &str| {
        let (_, _, titles) = list(&phone, page_url(page)).unwrap();
        titles
    };
    assert_eq!(titles("https://gist.github.com/"), ["GitHub"]);
    assert_eq!(
        titles("https://github.com:8443/"),
        ["GitHub"],
        "any port, by design"
    );
    assert!(titles("https://github.com.evil.example/").is_empty());
    assert!(titles("https://github.co/").is_empty());
    assert!(titles("https://githuh.com/").is_empty());
    assert!(titles("https://xn--gthub-1ye.com/").is_empty());
    assert!(titles("https://com/").is_empty());
    assert!(
        titles("http://gist.github.com/").is_empty(),
        "the opt-in never crosses schemes (no downgrade)"
    );
    // The lookalikes are named.
    for page in ["https://github.co/", "https://githuh.com/"] {
        let target = phone.core.fill_target_ios(page_url(page)).unwrap();
        assert_eq!(
            phone.core.fill_warning(target).unwrap(),
            Some(OriginWarning::Lookalike {
                saved_host: "github.com".to_owned()
            }),
            "{page}"
        );
    }
}

// ---- The record hint and the fill session ---------------------------------------------------

/// Attack: the `recordIdentifier` of a `QuickType` identity, or the key of a
/// tapped row, names another login (another site, a login with a generator,
/// another vault, a malformed key), or a fill session of another request.
/// Defense: `fill_values` and `fill_otp` check the chosen login against the
/// target the fill session holds, so the hint cannot fill another site's
/// password or one-time code, and the session is the request's own.
#[test]
fn g3i_the_record_hint_cannot_fill_or_read_a_code_for_another_site() {
    let (phone, _) = Phone::with_account();
    let github = login(
        &phone,
        "GitHub",
        "octo",
        vec![url("https://github.com", false, false)],
    );
    let preview = phone
        .core
        .otp_from_text(b"JBSWY3DPEHPK3PXP".to_vec())
        .unwrap();
    let bank = phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: "Bank".to_owned(),
            username: "alice".to_owned(),
            password: Some(b"bank-pw".to_vec()),
            urls: vec![url("https://bank.example", false, false)],
            notes: None,
            otp: OtpEdit::SetPending {
                id: preview.pending_id,
            },
            favorite: false,
        })
        .unwrap();
    let (_, evil_session, titles) = list(&phone, domain("evil.example")).unwrap();
    assert!(titles.is_empty());
    // Neither login matches evil.example, hint or not.
    for key in [&github, &bank] {
        assert_eq!(
            phone
                .core
                .fill_values(evil_session.clone(), key.clone())
                .err(),
            Some(MobileError::NoMatch)
        );
        assert_eq!(
            phone.core.fill_otp(evil_session.clone(), key.clone()).err(),
            Some(MobileError::NoMatch)
        );
    }
    // The GitHub session gives no Bank password and no Bank code either.
    let (_, github_session, titles) = list(&phone, domain("github.com")).unwrap();
    assert_eq!(titles, ["GitHub"]);
    assert_eq!(
        phone
            .core
            .fill_values(github_session.clone(), bank.clone())
            .err(),
        Some(MobileError::NoMatch)
    );
    assert_eq!(
        phone
            .core
            .fill_otp(github_session.clone(), bank.clone())
            .err(),
        Some(MobileError::NoMatch)
    );
    assert_eq!(
        phone
            .core
            .fill_otp(github_session.clone(), github.clone())
            .err(),
        Some(MobileError::NotFound),
        "the GitHub login has no generator"
    );
    // Another vault, a malformed key: refused before anything is read.
    let foreign_vault = ItemKey {
        vault_id: "00000000-0000-7000-8000-00000000beef".to_owned(),
        item_id: github.item_id.clone(),
    };
    assert!(matches!(
        phone
            .core
            .fill_values(github_session.clone(), foreign_vault)
            .err(),
        Some(MobileError::NotFound | MobileError::VaultUnavailable | MobileError::InvalidInput)
    ));
    for bad in ["", "v", "../x", "00000000-0000-7000-8000-00000000beef/x"] {
        assert_eq!(
            phone
                .core
                .fill_values(
                    github_session.clone(),
                    ItemKey {
                        vault_id: github.vault_id.clone(),
                        item_id: bad.to_owned(),
                    }
                )
                .err(),
            Some(MobileError::InvalidInput),
            "{bad:?}"
        );
    }
    // A session nobody opened, and a released one.
    assert_eq!(
        phone
            .core
            .fill_values(
                "0123456789abcdef0123456789abcdef".to_owned(),
                github.clone()
            )
            .err(),
        Some(MobileError::NotFound)
    );
    phone.core.fill_release(github_session.clone());
    assert_eq!(
        phone.core.fill_values(github_session, github).err(),
        Some(MobileError::NotFound)
    );
    // The Bank code comes only through the Bank session.
    let (_, bank_session, titles) = list(&phone, domain("bank.example")).unwrap();
    assert_eq!(titles, ["Bank"]);
    let code = phone.core.fill_otp(bank_session, bank).unwrap();
    assert_eq!(code.expose_bytes().len(), 6);
    code.wipe();
}

// ---- Passkeys ----------------------------------------------------------------------------------

/// Attack: an rp ID that is not a canonical DNS name (a port, a scheme, a
/// path, an underscore, a hyphen at the edge, a zero-width character, a
/// U-label, a public suffix, loopback, an IP), for a list, an assertion or a
/// registration. Defense: `canonical_rp_id` refuses them all (`rpIdInvalid`);
/// a different valid rp ID finds no passkey of `example.com`; and the exclude
/// list of one relying party cannot probe for the passkeys of another.
#[test]
fn g3i_passkey_rp_id_tricks_on_ios_are_refused_and_never_cross_sites() {
    let (phone, _) = Phone::with_account();
    let hash = vec![0x42; 32];
    let register = |rp: &str, exclude: Vec<Vec<u8>>| {
        phone.core.passkey_ios_register(IosRegisterRequest {
            rp_id: rp.to_owned(),
            user_name: "alice".to_owned(),
            user_handle: USER_HANDLE.to_vec(),
            client_data_hash: hash.clone(),
            algorithms: vec![-7],
            exclude_credential_ids: exclude,
            vault_id: phone.vault_id(),
            user_verified: true,
        })
    };
    let made = register("example.com", Vec::new()).unwrap();
    let assert_with = |rp: &str, credential_id: Vec<u8>| {
        phone.core.passkey_ios_assert(IosAssertRequest {
            rp_id: rp.to_owned(),
            client_data_hash: hash.clone(),
            credential_id,
            user_verified: true,
        })
    };
    let invalid = [
        "example.com:443",
        "https://example.com",
        "example.com/",
        "exa_mple.com",
        "-example.com",
        "example-.com",
        "example.com\u{200b}",
        "b\u{fc}cher.example",
        "com",
        "co.uk",
        "localhost",
        "127.0.0.1",
        "[::1]",
        "",
        " example.com",
        "example..com",
        "*.example.com",
    ];
    for rp in invalid {
        assert_eq!(
            phone
                .core
                .passkey_ios_candidates(rp.to_owned(), Vec::new())
                .err(),
            Some(MobileError::RpIdInvalid),
            "candidates {rp:?}"
        );
        assert_eq!(
            assert_with(rp, made.credential_id.clone()).err(),
            Some(MobileError::RpIdInvalid),
            "assert {rp:?}"
        );
        assert_eq!(
            register(rp, Vec::new()).err(),
            Some(MobileError::RpIdInvalid),
            "register {rp:?}"
        );
    }
    // Valid but other relying parties: nothing of example.com answers.
    for rp in [
        "other.example",
        "login.example.com",
        "xn--80ak6aa92e.com",
        "example.co",
        "examp1e.com",
    ] {
        assert!(
            phone
                .core
                .passkey_ios_candidates(rp.to_owned(), vec![made.credential_id.clone()])
                .unwrap()
                .is_empty(),
            "{rp}"
        );
        assert_eq!(
            assert_with(rp, made.credential_id.clone()).err(),
            Some(MobileError::PasskeyNotFound),
            "{rp}"
        );
    }
    // The same relying party in another spelling is the same one.
    for rp in ["EXAMPLE.COM", "example.com."] {
        assert_eq!(
            phone
                .core
                .passkey_ios_candidates(rp.to_owned(), Vec::new())
                .unwrap()
                .len(),
            1,
            "{rp}"
        );
    }
    // The exclude list of other.example names example.com's credential: no probe and no refusal.
    let other = register("other.example", vec![made.credential_id.clone()]).unwrap();
    assert_ne!(other.credential_id, made.credential_id);
    assert_eq!(
        register("other.example", vec![other.credential_id.clone()]).err(),
        Some(MobileError::PasskeyExcluded)
    );
    // A list filtered by another site's credential ID names nothing.
    assert!(
        phone
            .core
            .passkey_ios_candidates("example.com".to_owned(), vec![other.credential_id])
            .unwrap()
            .is_empty()
    );
    // No user verification: no signature and no new passkey.
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: hash.clone(),
                credential_id: made.credential_id,
                user_verified: false,
            })
            .err(),
        Some(MobileError::ReauthRequired)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_register(IosRegisterRequest {
                rp_id: "example.com".to_owned(),
                user_name: "bob".to_owned(),
                user_handle: USER_HANDLE.to_vec(),
                client_data_hash: hash,
                algorithms: vec![-7],
                exclude_credential_ids: Vec::new(),
                vault_id: phone.vault_id(),
                user_verified: false,
            })
            .err(),
        Some(MobileError::ReauthRequired)
    );
    assert_eq!(phone.core.passkey_identities().unwrap().len(), 2);
}

// ---- The identity store -----------------------------------------------------------------------

/// Attack: a login whose URL is not a plain `https://host` (a port, `http`,
/// userinfo, an Android app URL first, a trashed login) in the hope that its
/// `QuickType` identity names a site the login does not have, or that a tap on
/// the identity fills it where the saved URL would not match. Defense: the
/// identity's domain is the ASCII host of the login's first web origin, and
/// nothing else; the provide request then targets `https://<domain>`, which
/// the login must match exactly. A login on another port or on `http` gets
/// an identity that never fills (a gap in usefulness, never in safety).
#[test]
fn g3i_password_identities_name_the_host_only_and_fill_only_an_exact_origin() {
    let (phone, _) = Phone::with_account();
    let ok = login(
        &phone,
        "OK",
        "dave",
        vec![url("https://user:pw@ok.example/login?x#y", false, false)],
    );
    let ported = login(
        &phone,
        "Ported",
        "bob",
        vec![url("https://site.example:8443", false, false)],
    );
    let plain = login(
        &phone,
        "Plain",
        "carol",
        vec![url("http://plain.example", false, true)],
    );
    let multi = login(
        &phone,
        "Multi",
        "erin",
        vec![
            url("androidapp://com.example.app?cert=AAAA", false, false),
            url("https://multi.example", false, false),
            url("https://second.example", false, false),
        ],
    );
    let idn = login(
        &phone,
        "IDN",
        "frank",
        vec![url("https://b\u{fc}cher.example", false, false)],
    );
    let trashed = login(
        &phone,
        "Trashed",
        "gone",
        vec![url("https://trashed.example", false, false)],
    );
    phone
        .core
        .trash_item(&trashed.vault_id, &trashed.item_id)
        .unwrap();
    login(
        &phone,
        "No user",
        "",
        vec![url("https://nouser.example", false, false)],
    );

    let mut update = SettingsUpdate::default();
    update.password_identities = Some(true);
    update.reauth_password = Some(PASSWORD.as_bytes().to_vec());
    phone.core.update_settings(update).unwrap();
    let identities = phone.core.password_identities().unwrap();
    let mut domains: Vec<(String, String)> = identities
        .iter()
        .map(|i| (i.domain.clone(), i.user_name.clone()))
        .collect();
    domains.sort();
    assert_eq!(
        domains,
        [
            ("multi.example".to_owned(), "erin".to_owned()),
            ("ok.example".to_owned(), "dave".to_owned()),
            ("plain.example".to_owned(), "carol".to_owned()),
            ("site.example".to_owned(), "bob".to_owned()),
            ("xn--bcher-kva.example".to_owned(), "frank".to_owned()),
        ],
        "one identity per login, the host of its first web URL; no trashed login, none without a user"
    );

    // A tap on each identity: the extension targets `https://<domain>` and asks the core for the record.
    let fill_from_identity = |record: &ItemKey| {
        let identity = identities.iter().find(|i| i.record == *record).unwrap();
        let (target, session, titles) = list(&phone, domain(&identity.domain)).unwrap();
        assert_eq!(target.display, identity.domain);
        let values = phone.core.fill_values(session, record.clone());
        (titles, values.map(|v| v.username.unwrap_or_default()))
    };
    let (titles, filled) = fill_from_identity(&ok);
    assert_eq!(titles, ["OK"]);
    assert_eq!(filled.unwrap(), "dave");
    let (titles, filled) = fill_from_identity(&multi);
    assert_eq!(titles, ["Multi"]);
    assert_eq!(filled.unwrap(), "erin");
    let (titles, filled) = fill_from_identity(&idn);
    assert_eq!(titles, ["IDN"]);
    assert_eq!(filled.unwrap(), "frank");
    // Another port: the identity names the host, the login does not match `https://site.example`.
    let (titles, filled) = fill_from_identity(&ported);
    assert!(titles.is_empty());
    assert_eq!(filled.err(), Some(MobileError::NoMatch));
    // `http`: the identity never puts the password into an `https` page of the same host.
    let (titles, filled) = fill_from_identity(&plain);
    assert!(titles.is_empty());
    assert_eq!(filled.err(), Some(MobileError::NoMatch));
    // The identities are gone with the setting.
    let mut update = SettingsUpdate::default();
    update.password_identities = Some(false);
    phone.core.update_settings(update).unwrap();
    assert!(phone.core.password_identities().unwrap().is_empty());
}

// ---- Lifecycle ---------------------------------------------------------------------------------

/// Attack (M10-G3i-1): the app goes to the background, or the extension's
/// request ends (a dismissal, the host going away), while the core is in the
/// middle of an unlock. `close_store()` finds the phase `Unlocking`, only
/// requests a lock, and does not close: when the unlock finishes the phase
/// becomes `Locked` with the store open and its App Group file lock held. The
/// app is then suspended holding the lock (`0xdead10cc`) and the extension
/// finds the store busy until the app is killed; in the extension a finished
/// request keeps the store from the app. Defense wanted: a close during an
/// unlock ends closed, as a lock during an unlock ends locked.
#[test]
fn g3i_1_close_store_during_an_unlock_ends_closed() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    phone.core.test_hooks().set_work_delay_ms(400);
    let result = std::thread::scope(|scope| {
        let unlock = scope.spawn(|| {
            phone
                .core
                .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        });
        wait_for_busy(&phone.core);
        // The scene went to the background: ADR-0094 says lock and close.
        phone.core.close_store();
        unlock.join().unwrap()
    });
    assert!(
        !matches!(result, Ok(SessionState::Unlocked { .. })),
        "a close during the unlock must not end unlocked: {result:?}"
    );
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
    // The extension can open the store now.
    let other = phone.second_core(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert!(
        matches!(other.state(), SessionState::Locked { .. }),
        "{:?}",
        other.state()
    );
}

/// The same race with a failed unlock (a wrong password while the app goes to
/// the background): `put_back` restores `Locked`, and the store stays open.
#[test]
fn g3i_1_close_store_during_a_failed_unlock_ends_closed() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    phone.core.test_hooks().set_work_delay_ms(400);
    let result = std::thread::scope(|scope| {
        let unlock = scope.spawn(|| phone.core.unlock_password(b"wrong password".to_vec(), None));
        wait_for_busy(&phone.core);
        phone.core.close_store();
        unlock.join().unwrap()
    });
    assert!(result.is_err(), "{result:?}");
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
}

/// Held: a second request (or the app) finds the store busy while the first
/// request holds it unlocked, gets nothing from it, and opens it only after
/// the first `close_store()`; the first request's fill session is useless
/// afterwards, and the second one must unlock on its own.
#[test]
fn g3i_a_second_request_waits_for_the_first_and_inherits_nothing() {
    let (phone, _) = Phone::with_account();
    let github = login(
        &phone,
        "GitHub",
        "octo",
        vec![url("https://github.com", false, false)],
    );
    // The first request holds the store unlocked with a fill session open.
    let first = &phone.core;
    let (_, session, _) = list(&phone, domain("github.com")).unwrap();
    let second = phone.second_core(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert_eq!(second.state(), SessionState::StoreBusy);
    assert_eq!(
        second.reopen_store().unwrap(),
        SessionState::StoreBusy,
        "the first request holds the file lock"
    );
    assert_eq!(
        second.fill_values(session.clone(), github.clone()).err(),
        Some(MobileError::Locked),
        "a closed store has no unlocked session to fill from"
    );
    first.close_store();
    assert_eq!(
        first.fill_values(session.clone(), github.clone()).err(),
        Some(MobileError::Locked)
    );
    assert!(matches!(
        second.reopen_store().unwrap(),
        SessionState::Locked { .. }
    ));
    // The second process starts locked: the first one's session ID means nothing to it.
    assert_eq!(
        second.fill_values(session, github).err(),
        Some(MobileError::Locked)
    );
    assert_eq!(
        second
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: vec![1; 32],
                credential_id: vec![1],
                user_verified: true,
            })
            .err(),
        Some(MobileError::Locked)
    );
}

//! One-tap linking of an Android app (ADR-0092 amendment): the link offer of a
//! fill session and the session-bound link.

use std::sync::{Arc, Mutex};

use mosaic_mobile::{
    FillTarget, FillTargetKind, ItemKey, LinkOffer, LinkReason, LoginDraft, MAX_LINK_CANDIDATES,
    MobileError, OtpEdit, UrlDraft,
};
use mosaic_passkey::b64url;
use mosaic_sync::TransportError;

use crate::common::Phone;

const CERT: [u8; 32] = [0x11; 32];
const OTHER_CERT: [u8; 32] = [0x22; 32];

fn url(u: &str) -> UrlDraft {
    UrlDraft {
        url: u.to_owned(),
        match_etld1: false,
        allow_http: false,
    }
}

fn login(phone: &Phone, title: &str, urls: Vec<UrlDraft>) -> ItemKey {
    phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: title.to_owned(),
            username: "alice".to_owned(),
            password: Some(b"very-secret-Passw0rd".to_vec()),
            urls,
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap()
}

fn app_target(package: &str, cert: [u8; 32]) -> FillTarget {
    FillTarget {
        kind: FillTargetKind::AndroidApp {
            package: package.to_owned(),
            certs: vec![cert.to_vec()],
        },
        display: String::new(),
        warning: None,
    }
}

fn web(origin: &str) -> FillTarget {
    FillTarget {
        kind: FillTargetKind::Web {
            origin: origin.to_owned(),
        },
        display: String::new(),
        warning: None,
    }
}

fn assetlinks_json(package: &str, cert: &[u8; 32]) -> Vec<u8> {
    let fingerprint = cert
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");
    format!(
        "[{{\"relation\":[\"delegate_permission/common.get_login_creds\"],\
         \"target\":{{\"namespace\":\"android_app\",\"package_name\":\"{package}\",\
         \"sha256_cert_fingerprints\":[\"{fingerprint}\"]}}}}]"
    )
    .into_bytes()
}

fn session(phone: &Phone, package: &str, cert: [u8; 32]) -> String {
    phone
        .core
        .fill_candidates(app_target(package, cert))
        .unwrap()
        .session_id
}

fn offer(phone: &Phone, package: &str, cert: [u8; 32]) -> Result<LinkOffer, MobileError> {
    phone.core.fill_link_offer(session(phone, package, cert))
}

fn imported(phone: &Phone, title: &str, package: &str) -> ItemKey {
    login(phone, title, vec![url(&format!("androidapp://{package}"))])
}

/// Replaces the fetch: `answer` decides per host; every asked host is recorded.
fn hook(
    phone: &Phone,
    answer: impl Fn(&str) -> Result<Vec<u8>, TransportError> + Send + Sync + 'static,
) -> Arc<Mutex<Vec<String>>> {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let log = seen.clone();
    phone.core.test_hooks().set_asset_links_fetch(move |host| {
        log.lock().unwrap().push(host.to_owned());
        answer(host)
    });
    seen
}

fn urls_of(phone: &Phone, key: &ItemKey) -> Vec<String> {
    phone
        .core
        .item(&key.vault_id, &key.item_id)
        .unwrap()
        .urls
        .into_iter()
        .map(|u| u.url)
        .collect()
}

#[test]
fn one_tap_link_offered_only_for_verified_or_exact_package_candidates() {
    let (phone, _) = Phone::android_with_account();
    login(
        &phone,
        "Imported",
        vec![url("androidapp://com.github.android")],
    );
    login(&phone, "GitHub", vec![url("https://github.com")]);
    login(&phone, "Unrelated", vec![url("https://unrelated.example")]);
    login(&phone, "Plain http", vec![url("http://github.com")]);
    let seen = hook(&phone, |host| {
        if host == "github.com" {
            Ok(assetlinks_json("com.github.android", &CERT))
        } else {
            Err(TransportError::NotFound)
        }
    });
    let offer = offer(&phone, "com.github.android", CERT).unwrap();
    let got: Vec<(&str, &LinkReason)> = offer
        .candidates
        .iter()
        .map(|c| (c.title.as_str(), &c.reason))
        .collect();
    assert_eq!(
        got,
        vec![
            ("Imported", &LinkReason::ImportedForApp),
            (
                "GitHub",
                &LinkReason::VerifiedBy {
                    host: "github.com".to_owned()
                }
            ),
        ]
    );
    assert_eq!(*seen.lock().unwrap(), vec!["github.com".to_owned()]);
    assert_eq!(offer.signer_prefix, "11:11:11:11:11:11:11:11");
    assert_eq!(offer.package_name, "com.github.android");
}

#[test]
fn browser_package_never_gets_a_link_offer() {
    let (phone, _) = Phone::android_with_account();
    let key = imported(&phone, "Chrome", "com.android.chrome");
    let before = urls_of(&phone, &key);
    let opened = phone
        .core
        .fill_candidates(app_target("com.android.chrome", CERT));
    if let Ok(opened) = opened {
        let session = opened.session_id;
        assert!(
            phone
                .core
                .fill_link_offer(session.clone())
                .map_or(true, |o| o.candidates.is_empty())
        );
        assert!(matches!(
            phone.core.link_app_in_fill_session(session, key.clone()),
            Err(MobileError::InvalidInput | MobileError::NoMatch)
        ));
    }
    assert_eq!(urls_of(&phone, &key), before);
}

#[test]
fn link_offer_never_matches_a_lookalike_package() {
    let (phone, _) = Phone::android_with_account();
    let key = imported(&phone, "Example", "com.example.app");
    for p in [
        "com.example.app2",
        "com.example.ap",
        "com.Example.app",
        "com.examp1e.app",
    ] {
        assert!(offer(&phone, p, CERT).unwrap().candidates.is_empty(), "{p}");
    }
    let session = session(&phone, "com.example.app", CERT);
    phone.core.link_app_in_fill_session(session, key).unwrap();
    assert!(
        phone
            .core
            .fill_candidates(app_target("com.example.app2", CERT))
            .unwrap()
            .candidates
            .is_empty()
    );
}

#[test]
fn one_tap_link_writes_the_session_signer() {
    let (phone, _) = Phone::android_with_account();
    let key = imported(&phone, "Example", "com.example.app");
    let s = session(&phone, "com.example.app", CERT);
    phone.core.link_app_in_fill_session(s, key.clone()).unwrap();
    let urls = urls_of(&phone, &key);
    let apps: Vec<&String> = urls
        .iter()
        .filter(|u| u.starts_with("androidapp://"))
        .collect();
    assert_eq!(
        apps,
        vec![&format!(
            "androidapp://com.example.app?cert={}",
            b64url(&CERT)
        )]
    );
    assert!(
        phone
            .core
            .fill_candidates(app_target("com.example.app", OTHER_CERT))
            .unwrap()
            .candidates
            .is_empty()
    );
    let s = session(&phone, "com.example.app", CERT);
    let values = phone.core.fill_values(s, key).unwrap();
    assert!(values.password.is_some());
}

#[test]
fn link_app_in_fill_session_refuses_what_is_not_a_strong_candidate() {
    let (phone, _) = Phone::android_with_account();
    hook(&phone, |_| Err(TransportError::NotFound));
    let plain = login(&phone, "Plain", vec![url("https://unrelated.example")]);
    let before = urls_of(&phone, &plain);
    let s = session(&phone, "com.example.app", CERT);
    assert!(matches!(
        phone.core.link_app_in_fill_session(s, plain.clone()),
        Err(MobileError::NoMatch)
    ));
    assert_eq!(urls_of(&phone, &plain), before);

    let web_session = phone
        .core
        .fill_candidates(web("https://example.com"))
        .unwrap()
        .session_id;
    assert!(matches!(
        phone
            .core
            .link_app_in_fill_session(web_session.clone(), plain.clone()),
        Err(MobileError::InvalidInput)
    ));
    assert!(matches!(
        phone.core.fill_link_offer(web_session),
        Err(MobileError::InvalidInput)
    ));

    let two = phone
        .core
        .fill_candidates(FillTarget {
            kind: FillTargetKind::AndroidApp {
                package: "com.example.app".to_owned(),
                certs: vec![CERT.to_vec(), OTHER_CERT.to_vec()],
            },
            display: String::new(),
            warning: None,
        })
        .unwrap()
        .session_id;
    assert!(matches!(
        phone.core.fill_link_offer(two.clone()),
        Err(MobileError::InvalidInput)
    ));
    assert!(matches!(
        phone.core.link_app_in_fill_session(two, plain.clone()),
        Err(MobileError::InvalidInput)
    ));

    assert!(matches!(
        phone.core.fill_link_offer("00".to_owned()),
        Err(MobileError::NotFound)
    ));
    assert!(matches!(
        phone
            .core
            .link_app_in_fill_session("00".to_owned(), plain.clone()),
        Err(MobileError::NotFound)
    ));
    let s = session(&phone, "com.example.app", CERT);
    phone.core.lock();
    assert!(matches!(
        phone.core.fill_link_offer(s.clone()),
        Err(MobileError::Locked)
    ));
    assert!(matches!(
        phone.core.link_app_in_fill_session(s, plain),
        Err(MobileError::Locked)
    ));
}

#[test]
fn link_offer_is_empty_when_a_login_already_matches() {
    let (phone, _) = Phone::android_with_account();
    let seen = hook(&phone, |_| Ok(assetlinks_json("com.example.app", &CERT)));
    login(
        &phone,
        "Bound",
        vec![url(&format!(
            "androidapp://com.example.app?cert={}",
            b64url(&CERT)
        ))],
    );
    imported(&phone, "Old import", "com.example.app");
    login(&phone, "Site", vec![url("https://example.com")]);
    assert!(
        offer(&phone, "com.example.app", CERT)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn link_offer_bounds_checks_and_fetches() {
    let (phone, _) = Phone::android_with_account();
    for n in 0..4 {
        imported(&phone, &format!("Mail {n}"), "com.google.android.gm");
    }
    assert_eq!(
        offer(&phone, "com.google.android.gm", CERT)
            .unwrap()
            .candidates
            .len(),
        MAX_LINK_CANDIDATES
    );

    let (phone, _) = Phone::android_with_account();
    for c in ["a", "b", "c", "d", "e"] {
        login(
            &phone,
            &format!("Google {c}"),
            vec![url(&format!("https://{c}.google.com"))],
        );
    }
    let seen = hook(&phone, |_| {
        Ok(assetlinks_json("com.google.android.gm", &CERT))
    });
    let n = |phone: &Phone| {
        offer(phone, "com.google.android.gm", CERT)
            .unwrap()
            .candidates
            .len()
    };
    assert_eq!(n(&phone), 1);
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(n(&phone), 2);
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(n(&phone), 2);
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn unreachable_asset_links_give_no_candidate_and_are_not_cached() {
    let (phone, _) = Phone::android_with_account();
    login(&phone, "GitHub", vec![url("https://github.com")]);
    let seen = hook(&phone, |_| Err(TransportError::Unreachable));
    assert!(
        offer(&phone, "com.github.android", CERT)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert!(
        offer(&phone, "com.github.android", CERT)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert_eq!(seen.lock().unwrap().len(), 2);

    let seen = hook(&phone, |_| Err(TransportError::NotFound));
    assert!(
        offer(&phone, "com.github.android", CERT)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert!(
        offer(&phone, "com.github.android", CERT)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn link_offer_skips_trashed_and_deleted_logins() {
    let (phone, _) = Phone::android_with_account();
    let key = imported(&phone, "A", "com.example.app");
    let s = session(&phone, "com.example.app", CERT);
    phone.core.trash_item(&key.vault_id, &key.item_id).unwrap();
    assert!(
        phone
            .core
            .fill_link_offer(s.clone())
            .unwrap()
            .candidates
            .is_empty()
    );
    assert!(matches!(
        phone.core.link_app_in_fill_session(s, key),
        Err(MobileError::NotFound)
    ));
}

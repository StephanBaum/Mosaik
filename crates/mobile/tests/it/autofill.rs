//! Autofill: targets, fields, candidates, values, app linking and asset links.

use mosaic_mobile::{
    AndroidCaller, AssetLinksStatus, Attr, Core, FieldNode, FieldRefusal, FillMatch, FillTarget,
    FillTargetKind, FillValues, ItemFilter, ItemKey, LoginDraft, MobileError, OtpEdit,
    ServiceIdentifier, ServiceIdentifierKind, UrlDraft,
};
use mosaic_passkey::b64url;
use mosaic_sync::TransportError;

use crate::common::{PASSWORD, Phone};

const CERT: [u8; 32] = [0x11; 32];
const OTHER_CERT: [u8; 32] = [0x22; 32];
const SECRET: &str = "very-secret-Passw0rd";

fn url(u: &str, etld1: bool, allow_http: bool) -> UrlDraft {
    UrlDraft {
        url: u.to_owned(),
        match_etld1: etld1,
        allow_http,
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
            password: Some(SECRET.as_bytes().to_vec()),
            urls,
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap()
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

fn caller(package: &str, cert: [u8; 32]) -> AndroidCaller {
    AndroidCaller {
        package: package.to_owned(),
        current_signers: vec![cert.to_vec()],
        web_domain: None,
        web_scheme: None,
    }
}

/// The old one-call form: opens a fill session for `target`, then asks the
/// values of `key` with it (G1-12).
trait FillExt {
    fn fill_values_t(&self, target: FillTarget, key: ItemKey) -> Result<FillValues, MobileError>;
    fn fill_otp_t(
        &self,
        target: FillTarget,
        key: ItemKey,
    ) -> Result<std::sync::Arc<mosaic_mobile::SecretValue>, MobileError>;
}

impl FillExt for Core {
    fn fill_values_t(&self, target: FillTarget, key: ItemKey) -> Result<FillValues, MobileError> {
        let session = self.fill_candidates(target)?.session_id;
        self.fill_values(session, key)
    }

    fn fill_otp_t(
        &self,
        target: FillTarget,
        key: ItemKey,
    ) -> Result<std::sync::Arc<mosaic_mobile::SecretValue>, MobileError> {
        let session = self.fill_candidates(target)?.session_id;
        self.fill_otp(session, key)
    }
}

fn titles(list: &[mosaic_mobile::FillCandidate]) -> Vec<&str> {
    list.iter().map(|c| c.title.as_str()).collect()
}

#[test]
fn fill_candidates_exact_origin_only() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "Example",
        vec![url("https://example.com/login", false, false)],
    );
    login(
        &phone,
        "Other host",
        vec![url("https://www.example.com", false, false)],
    );
    login(
        &phone,
        "Other port",
        vec![url("https://example.com:8443", false, false)],
    );
    login(
        &phone,
        "Plain http",
        vec![url("http://example.com", false, false)],
    );
    let list = phone
        .core
        .fill_candidates(web("https://example.com"))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Example"]);
    assert_eq!(list[0].match_kind, FillMatch::Exact);
    assert_eq!(list[0].username.as_deref(), Some("alice"));
    assert!(!list[0].has_otp);
    // A look-alike or a sibling is not the same origin.
    assert!(
        phone
            .core
            .fill_candidates(web("https://example.com.evil.test"))
            .unwrap()
            .candidates
            .is_empty()
    );
    assert!(
        phone
            .core
            .fill_candidates(web("https://sub.example.com"))
            .unwrap()
            .candidates
            .is_empty()
    );
    // A target that is not an origin is refused.
    assert_eq!(
        phone
            .core
            .fill_candidates(web("https://example.com/path"))
            .err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn fill_candidates_etld1_opt_in() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "Strict",
        vec![url("https://www.example.com", false, false)],
    );
    login(
        &phone,
        "Opted in",
        vec![url("https://www.example.com", true, false)],
    );
    let list = phone
        .core
        .fill_candidates(web("https://login.example.com"))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Opted in"]);
    assert_eq!(list[0].match_kind, FillMatch::Etld1);
    // The exact origin lists both, the exact match first.
    let list = phone
        .core
        .fill_candidates(web("https://www.example.com"))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Opted in", "Strict"]);
    // The registrable domain of another site never matches.
    assert!(
        phone
            .core
            .fill_candidates(web("https://www.example.org"))
            .unwrap()
            .candidates
            .is_empty()
    );
}

#[test]
fn fill_candidates_http_refused_without_allow_http() {
    let (phone, _) = Phone::with_account();
    let plain = login(
        &phone,
        "Plain",
        vec![url("http://intranet.example.com", false, false)],
    );
    assert_eq!(
        phone
            .core
            .fill_candidates(web("http://intranet.example.com"))
            .err(),
        Some(MobileError::InsecureOrigin)
    );
    assert_eq!(
        phone
            .core
            .fill_values_t(web("http://intranet.example.com"), plain.clone())
            .err()
            .map(mosaic_mobile::MobileError::code),
        Some("insecureOrigin")
    );
    // An https page is not matched by an http URL, and loopback is allowed.
    assert!(
        phone
            .core
            .fill_candidates(web("https://intranet.example.com"))
            .unwrap()
            .candidates
            .is_empty()
    );
    login(
        &phone,
        "Local",
        vec![url("http://127.0.0.1:8080", false, false)],
    );
    assert_eq!(
        titles(
            &phone
                .core
                .fill_candidates(web("http://127.0.0.1:8080"))
                .unwrap()
                .candidates
        ),
        vec!["Local"]
    );
    // The login itself allows http.
    login(
        &phone,
        "Allowed",
        vec![url("http://legacy.example.com", false, true)],
    );
    assert_eq!(
        titles(
            &phone
                .core
                .fill_candidates(web("http://legacy.example.com"))
                .unwrap()
                .candidates
        ),
        vec!["Allowed"]
    );
}

#[test]
fn fill_candidates_android_app_requires_cert_binding() {
    let (phone, _) = Phone::with_account();
    let bound = format!("androidapp://com.example.app?cert={}", b64url(&CERT));
    login(&phone, "Bound", vec![url(&bound, false, false)]);
    // An imported URL without a certificate never matches.
    login(
        &phone,
        "Imported",
        vec![url("androidapp://com.example.app", false, false)],
    );
    let list = phone
        .core
        .fill_candidates(app_target("com.example.app", CERT))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Bound"]);
    // Another signer, another package, or no signer: nothing.
    for target in [
        app_target("com.example.app", OTHER_CERT),
        app_target("com.example.other", CERT),
        FillTarget {
            kind: FillTargetKind::AndroidApp {
                package: "com.example.app".to_owned(),
                certs: Vec::new(),
            },
            display: String::new(),
            warning: None,
        },
    ] {
        assert!(
            phone
                .core
                .fill_candidates(target)
                .unwrap()
                .candidates
                .is_empty()
        );
    }
    // The detail tells which URL still needs linking.
    let items = phone.core.items(ItemFilter::default()).unwrap();
    let imported = items.iter().find(|i| i.title == "Imported").unwrap();
    let detail = phone
        .core
        .item(&imported.vault_id, &imported.item_id)
        .unwrap();
    assert!(detail.urls[0].is_app && detail.urls[0].needs_linking);
    // A web page is never an app, and an app is never a page.
    assert!(
        phone
            .core
            .fill_candidates(web("https://com.example.app"))
            .unwrap()
            .candidates
            .is_empty()
    );
}

#[test]
fn fill_candidates_skip_trashed_and_hardware_protected() {
    let (phone, sk) = Phone::with_account();
    let trashed = login(
        &phone,
        "Trashed",
        vec![url("https://example.com", false, false)],
    );
    login(
        &phone,
        "Live",
        vec![url("https://example.com", false, false)],
    );
    // A secure note and a passkey-less item never fill a password.
    phone
        .core
        .save_note(mosaic_mobile::NoteDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: "A note".to_owned(),
            notes: Some(b"text".to_vec()),
            favorite: false,
        })
        .unwrap();
    phone
        .core
        .trash_item(&trashed.vault_id, &trashed.item_id)
        .unwrap();
    let list = phone
        .core
        .fill_candidates(web("https://example.com"))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Live"]);
    // A trashed login is refused when someone names it anyway.
    assert_eq!(
        phone
            .core
            .fill_values_t(web("https://example.com"), trashed)
            .err(),
        Some(MobileError::NotFound)
    );
    // A hardware-protected vault holds one-time codes only and is never a
    // source of fills, whatever its items say.
    phone.core.close_store();
    {
        use mosaic_core::MasterPassword;
        use mosaic_core::SecretBytes;
        use mosaic_core::model::{ItemType, LoginUrl, OtpConfig};
        use mosaic_core::unlock::PrfOutput;
        use mosaic_vault::session::{DEFAULT_RP_ID, LockedSession, NewHardwareKey};
        let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
        let mut session = LockedSession::open(store)
            .unlock(
                &MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::SecretKey::parse(&sk).unwrap(),
            )
            .unwrap();
        let key = NewHardwareKey::new(vec![9; 32], DEFAULT_RP_ID.to_owned(), [0x3a; 32]).unwrap();
        let prf = PrfOutput::from_secret(SecretBytes::from_array(&mut [0xA1; 32]));
        let vault = session.create_hardware_vault("Codes", &key, &prf).unwrap();
        session
            .add_item(vault, &ItemType::Totp, |e| {
                e.set_title("Hardware code")?;
                e.set_otp(&OtpConfig::totp(mosaic_core::SecretVec::from_slice(
                    b"12345678901234567890",
                )?))?;
                e.add_url(&LoginUrl::new("https://example.com"))?;
                Ok(())
            })
            .unwrap();
        drop(session);
    }
    phone.core.reopen_store().unwrap();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let list = phone
        .core
        .fill_candidates(web("https://example.com"))
        .unwrap()
        .candidates;
    assert_eq!(titles(&list), vec!["Live"]);
}

#[test]
fn fill_candidates_have_no_secret_fields() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "Example",
        vec![url("https://example.com", false, false)],
    );
    let list = phone
        .core
        .fill_candidates(web("https://example.com"))
        .unwrap()
        .candidates;
    assert_eq!(list.len(), 1);
    let shown = format!("{list:?}");
    assert!(!shown.contains(SECRET), "{shown}");
}

#[test]
fn fill_values_rechecks_the_match() {
    let (phone, _) = Phone::with_account();
    let key = login(
        &phone,
        "Example",
        vec![url("https://example.com", false, false)],
    );
    let target = web("https://example.com");
    let list = phone
        .core
        .fill_candidates(target.clone())
        .unwrap()
        .candidates;
    assert_eq!(list[0].key, key);
    // The tap hands over the user name and the password, as secret values.
    let values = phone
        .core
        .fill_values_t(target.clone(), key.clone())
        .unwrap();
    assert_eq!(values.username.as_deref(), Some("alice"));
    let password = values.password.unwrap();
    assert_eq!(password.expose_bytes(), SECRET.as_bytes());
    password.wipe();
    // The login's URL changes before the tap: the match is checked again.
    phone
        .core
        .save_login(LoginDraft {
            vault_id: key.vault_id.clone(),
            item_id: Some(key.item_id.clone()),
            title: "Example".to_owned(),
            username: "alice".to_owned(),
            password: None,
            urls: vec![url("https://example.net", false, false)],
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap();
    assert_eq!(
        phone.core.fill_values_t(target, key.clone()).err(),
        Some(MobileError::NoMatch)
    );
    // And for an app: not linked.
    assert_eq!(
        phone
            .core
            .fill_values_t(app_target("com.example.app", CERT), key)
            .err(),
        Some(MobileError::AppNotLinked)
    );
}

#[test]
fn fill_values_refuses_other_origin_candidate() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "Mine",
        vec![url("https://example.com", false, false)],
    );
    let theirs = login(
        &phone,
        "Theirs",
        vec![url("https://other.example", false, false)],
    );
    // A fabricated request names a login of another site.
    assert_eq!(
        phone
            .core
            .fill_values_t(web("https://example.com"), theirs.clone())
            .err(),
        Some(MobileError::NoMatch)
    );
    assert_eq!(
        phone
            .core
            .fill_otp_t(web("https://example.com"), theirs)
            .err(),
        Some(MobileError::NoMatch)
    );
    // Unknown IDs and malformed ones.
    let nobody = ItemKey {
        vault_id: phone.vault_id(),
        item_id: "00000000-0000-7000-8000-000000000001".to_owned(),
    };
    assert_eq!(
        phone
            .core
            .fill_values_t(web("https://example.com"), nobody)
            .err(),
        Some(MobileError::NotFound)
    );
    let bad = ItemKey {
        vault_id: "x".to_owned(),
        item_id: "y".to_owned(),
    };
    assert_eq!(
        phone
            .core
            .fill_values_t(web("https://example.com"), bad)
            .err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn fill_values_locked_is_locked() {
    let (phone, _) = Phone::android_with_account();
    let key = login(
        &phone,
        "Example",
        vec![url("https://example.com", false, false)],
    );
    phone.core.lock();
    let target = web("https://example.com");
    assert_eq!(
        phone.core.fill_values_t(target.clone(), key.clone()).err(),
        Some(MobileError::Locked)
    );
    assert_eq!(
        phone.core.fill_candidates(target.clone()).err(),
        Some(MobileError::Locked)
    );
    assert_eq!(
        phone.core.fill_otp_t(target, key).err(),
        Some(MobileError::Locked)
    );
    // The pure calls work while locked.
    assert!(
        phone
            .core
            .fill_target_android(caller("com.example.app", CERT))
            .is_ok()
    );
}

#[test]
fn fill_otp_for_matching_item_only() {
    let (phone, _) = Phone::with_account();
    let preview = phone
        .core
        .otp_from_text(b"JBSWY3DPEHPK3PXP".to_vec())
        .unwrap();
    let with_otp = phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: "With OTP".to_owned(),
            username: "alice".to_owned(),
            password: None,
            urls: vec![url("https://example.com", false, false)],
            notes: None,
            otp: OtpEdit::SetPending {
                id: preview.pending_id,
            },
            favorite: false,
        })
        .unwrap();
    let without = login(
        &phone,
        "Without",
        vec![url("https://example.com", false, false)],
    );
    let target = web("https://example.com");
    let list = phone
        .core
        .fill_candidates(target.clone())
        .unwrap()
        .candidates;
    assert!(list.iter().find(|c| c.key == with_otp).unwrap().has_otp);
    let code = phone
        .core
        .fill_otp_t(target.clone(), with_otp.clone())
        .unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.expose_bytes().iter().all(u8::is_ascii_digit));
    assert_eq!(
        phone.core.fill_otp_t(target.clone(), without).err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        phone
            .core
            .fill_otp_t(web("https://example.net"), with_otp)
            .err(),
        Some(MobileError::NoMatch)
    );
}

#[test]
fn link_android_app_refuses_a_browser_package() {
    let (phone, _) = Phone::android_with_account();
    let key = login(
        &phone,
        "Example",
        vec![url("https://example.com", false, false)],
    );
    for package in ["com.android.chrome", "org.mozilla.firefox"] {
        assert_eq!(
            phone
                .core
                .link_android_app(&key.vault_id, &key.item_id, caller(package, CERT))
                .err(),
            Some(MobileError::InvalidInput),
            "{package}"
        );
    }
    assert_eq!(
        phone
            .core
            .item(&key.vault_id, &key.item_id)
            .unwrap()
            .urls
            .len(),
        1
    );
}

#[test]
fn link_android_app_writes_cert_bound_url() {
    let (phone, _) = Phone::android_with_account();
    let key = login(
        &phone,
        "Example",
        vec![
            url("https://example.com", false, false),
            url("androidapp://com.example.app", false, false),
        ],
    );
    let target = app_target("com.example.app", CERT);
    assert!(
        phone
            .core
            .fill_candidates(target.clone())
            .unwrap()
            .candidates
            .is_empty()
    );
    phone
        .core
        .link_android_app(&key.vault_id, &key.item_id, caller("com.example.app", CERT))
        .unwrap();
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    let urls: Vec<&str> = detail.urls.iter().map(|u| u.url.as_str()).collect();
    let bound = format!("androidapp://com.example.app?cert={}", b64url(&CERT));
    assert_eq!(urls.len(), 2, "{urls:?}");
    assert!(urls.contains(&"https://example.com"));
    assert!(urls.contains(&bound.as_str()), "{urls:?}");
    assert!(detail.urls.iter().all(|u| !u.needs_linking));
    assert_eq!(
        titles(&phone.core.fill_candidates(target).unwrap().candidates),
        vec!["Example"]
    );
    // Linking again changes nothing; a new signer replaces the old binding.
    phone
        .core
        .link_android_app(&key.vault_id, &key.item_id, caller("com.example.app", CERT))
        .unwrap();
    assert_eq!(
        phone
            .core
            .item(&key.vault_id, &key.item_id)
            .unwrap()
            .urls
            .len(),
        2
    );
    phone
        .core
        .link_android_app(
            &key.vault_id,
            &key.item_id,
            caller("com.example.app", OTHER_CERT),
        )
        .unwrap();
    let detail = phone.core.item(&key.vault_id, &key.item_id).unwrap();
    assert_eq!(detail.urls.len(), 2);
    assert!(
        phone
            .core
            .fill_candidates(app_target("com.example.app", CERT))
            .unwrap()
            .candidates
            .is_empty()
    );
    // Not a login, bad package, bad certificate.
    assert_eq!(
        phone
            .core
            .link_android_app(&key.vault_id, &key.item_id, caller("bad package!", CERT))
            .err(),
        Some(MobileError::InvalidInput)
    );
    let mut short = caller("com.example.app", CERT);
    short.current_signers = vec![vec![1, 2, 3]];
    assert_eq!(
        phone
            .core
            .link_android_app(&key.vault_id, &key.item_id, short)
            .err(),
        Some(MobileError::InvalidInput)
    );
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

#[test]
fn asset_links_status_verified_not_listed_unavailable() {
    let (phone, _) = Phone::android_with_account();
    let key = login(
        &phone,
        "Example",
        vec![
            url("https://example.com", false, false),
            url("http://plain.example.com", false, false),
        ],
    );
    let hosts = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let seen = hosts.clone();
    phone.core.test_hooks().set_asset_links_fetch(move |host| {
        seen.lock().unwrap().push(host.to_owned());
        Ok(assetlinks_json("com.example.app", &CERT))
    });
    let status = |cert: [u8; 32], package: &str| {
        phone
            .core
            .asset_links_status(&key.vault_id, &key.item_id, caller(package, cert))
            .unwrap()
    };
    assert_eq!(status(CERT, "com.example.app"), AssetLinksStatus::Verified);
    // Only the https host is asked.
    assert_eq!(*hosts.lock().unwrap(), vec!["example.com".to_owned()]);
    // The file does not list this signer or this package.
    assert_eq!(
        status(OTHER_CERT, "com.example.app"),
        AssetLinksStatus::NotListed
    );
    assert_eq!(
        status(CERT, "com.example.other"),
        AssetLinksStatus::NotListed
    );
    // No file at all counts as "not listed" (the site answered).
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Err(TransportError::NotFound));
    assert_eq!(status(CERT, "com.example.app"), AssetLinksStatus::NotListed);
    // A site that cannot be reached is "could not check", never "verified".
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Err(TransportError::Unreachable));
    assert_eq!(
        status(CERT, "com.example.app"),
        AssetLinksStatus::Unavailable
    );
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(b"not json".to_vec()));
    assert_eq!(status(CERT, "com.example.app"), AssetLinksStatus::NotListed);
}

#[test]
fn targets_and_fields_are_pure() {
    let (phone, _) = Phone::android_with_account();
    phone.core.lock();
    // A web domain counts only for a trusted browser.
    let mut app = caller("com.example.app", CERT);
    app.web_domain = Some("example.com".to_owned());
    let target = phone.core.fill_target_android(app).unwrap();
    assert_eq!(
        target.kind,
        FillTargetKind::AndroidApp {
            package: "com.example.app".to_owned(),
            certs: vec![CERT.to_vec()]
        }
    );
    let browsers: serde_json::Value =
        serde_json::from_str(&phone.core.privileged_browsers_json()).unwrap();
    let first = &browsers["apps"][0]["info"];
    let package = first["package_name"].as_str().unwrap();
    let fingerprint = first["signatures"][0]["cert_fingerprint_sha256"]
        .as_str()
        .unwrap();
    let cert: Vec<u8> = fingerprint
        .split(':')
        .map(|h| u8::from_str_radix(h, 16).unwrap())
        .collect();
    let browser = AndroidCaller {
        package: package.to_owned(),
        current_signers: vec![cert.clone()],
        web_domain: Some("Example.COM".to_owned()),
        web_scheme: None,
    };
    let target = phone.core.fill_target_android(browser).unwrap();
    assert_eq!(
        target.kind,
        FillTargetKind::Web {
            origin: "https://example.com".to_owned()
        }
    );
    assert_eq!(target.display, "example.com");
    // The same browser without a webDomain is no target at all (M10-G2 H1).
    let no_page = AndroidCaller {
        package: package.to_owned(),
        current_signers: vec![cert],
        web_domain: None,
        web_scheme: None,
    };
    assert_eq!(
        phone.core.fill_target_android(no_page).err(),
        Some(MobileError::NoMatch)
    );
    // iOS identifiers.
    let (ios_phone, _) = Phone::with_account();
    let ios = ios_phone
        .core
        .fill_target_ios(ServiceIdentifier {
            kind: ServiceIdentifierKind::Url,
            value: "https://user:pw@example.com/login?x=1".to_owned(),
        })
        .unwrap();
    assert_eq!(
        ios.kind,
        FillTargetKind::Web {
            origin: "https://example.com".to_owned()
        }
    );
    let idn = ios_phone
        .core
        .fill_target_ios(ServiceIdentifier {
            kind: ServiceIdentifierKind::Domain,
            value: "xn--bcher-kva.example".to_owned(),
        })
        .unwrap();
    assert!(idn.warning.is_some());

    // Fields: a username and a password field in the root frame.
    let node = |id: u32, hints: &[&str], input_type: u32| FieldNode {
        id,
        focused: id == 2,
        visible: true,
        web_domain: Some("example.com".to_owned()),
        input_type,
        autofill_hints: hints.iter().map(|h| (*h).to_owned()).collect(),
        html_tag: Some("input".to_owned()),
        html_attrs: vec![Attr {
            name: "autocomplete".to_owned(),
            value: String::new(),
        }],
        id_entry: None,
        hint: None,
    };
    let selection = phone.core.select_fill_fields(
        vec![node(1, &["username"], 1), node(2, &[], 0x81)],
        &web("https://example.com"),
    );
    assert_eq!(selection.username, Some(1));
    assert_eq!(selection.password, Some(2));
    assert_eq!(selection.refused, None);
    // A focused field of another frame refuses the request.
    let mut foreign = node(3, &["password"], 0x81);
    foreign.web_domain = Some("ads.example.net".to_owned());
    foreign.focused = true;
    let mut root = node(1, &["username"], 1);
    root.focused = false;
    let selection = phone
        .core
        .select_fill_fields(vec![root, foreign], &web("https://example.com"));
    assert_eq!(selection.refused, Some(FieldRefusal::ForeignFrame));
    assert_eq!(selection.password, None);
    // Nothing fillable.
    let selection = phone
        .core
        .select_fill_fields(vec![node(1, &["name"], 1)], &web("https://example.com"));
    assert_eq!(selection.refused, Some(FieldRefusal::NoFillableField));
}

/// A native field of an app, or a field of the same app inside a `WebView`.
fn native_or_web(id: u32, domain: Option<&str>, hints: &[&str], focused: bool) -> FieldNode {
    FieldNode {
        id,
        focused,
        visible: true,
        web_domain: domain.map(str::to_owned),
        input_type: 1,
        autofill_hints: hints.iter().map(|h| (*h).to_owned()).collect(),
        html_tag: None,
        html_attrs: vec![],
        id_entry: None,
        hint: None,
    }
}

#[test]
fn an_app_target_fills_native_fields_and_never_a_webview_above_or_below() {
    // M10-G2 round 2, N1: native login fields above a WebView of a third party.
    let (phone, _) = Phone::with_account();
    let app = app_target("com.example.app", [1; 32]);
    let selection = phone.core.select_fill_fields(
        vec![
            native_or_web(1, None, &["username"], false),
            native_or_web(2, None, &["password"], false),
            native_or_web(3, Some("ads.example.net"), &["username"], false),
            native_or_web(4, Some("ads.example.net"), &["password"], false),
            native_or_web(5, Some("ads.example.net"), &["one-time-code"], false),
        ],
        &app,
    );
    assert_eq!(selection.username, Some(1));
    assert_eq!(selection.password, Some(2));
    assert_eq!(selection.otp, None);
    assert_eq!(selection.refused, None);
}

#[test]
fn a_focused_webview_field_in_an_app_is_refused() {
    let (phone, _) = Phone::with_account();
    let app = app_target("com.example.app", [1; 32]);
    let selection = phone.core.select_fill_fields(
        vec![
            native_or_web(1, None, &["username"], false),
            native_or_web(2, Some("ads.example.net"), &["password"], true),
        ],
        &app,
    );
    assert_eq!(selection.refused, Some(FieldRefusal::ForeignFrame));
    assert_eq!(selection.password, None);
    assert_eq!(selection.username, None);
}

#[test]
fn a_page_in_a_browser_that_is_not_allowlisted_gives_nothing() {
    // Brave stays an app (ADR-0092): every field of the page carries a webDomain.
    let (phone, _) = Phone::with_account();
    let app = app_target("com.brave.browser", [1; 32]);
    let selection = phone.core.select_fill_fields(
        vec![
            native_or_web(1, Some("bank.example"), &["username"], false),
            native_or_web(2, Some("bank.example"), &["password"], false),
        ],
        &app,
    );
    assert_eq!(selection.refused, Some(FieldRefusal::NoFillableField));
    assert_eq!(selection.username, None);
}

#[test]
fn a_web_target_starts_at_the_page_root_and_drops_the_browser_screen() {
    let (phone, _) = Phone::with_account();
    let selection = phone.core.select_fill_fields(
        vec![
            native_or_web(1, None, &["username"], false),
            native_or_web(2, Some("example.com"), &["username"], false),
            native_or_web(3, Some("example.com"), &["password"], false),
        ],
        &web("https://example.com"),
    );
    assert_eq!(selection.username, Some(2));
    assert_eq!(selection.password, Some(3));
}

#[test]
fn fill_warning_names_the_lookalike() {
    let (phone, _) = Phone::with_account();
    login(
        &phone,
        "Example",
        vec![url("https://example.com", false, false)],
    );
    let warning = phone.core.fill_warning(web("https://examp1e.com")).unwrap();
    assert!(matches!(
        warning,
        Some(mosaic_mobile::OriginWarning::Lookalike { .. })
    ));
    assert_eq!(
        phone.core.fill_warning(web("https://example.com")).unwrap(),
        None
    );
    assert_eq!(
        phone
            .core
            .fill_warning(web("https://unrelated.test"))
            .unwrap(),
        None
    );
}

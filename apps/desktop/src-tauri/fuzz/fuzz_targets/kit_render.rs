//! The Emergency Kit renderer with hostile labels and emails. The first
//! byte picks the mode: `L` + KitLabels JSON, or `E` + an email, with fixed
//! values for the rest. A rendered PDF starts with `%PDF-`, fits the
//! preallocated buffer and holds no active content; every refusal is
//! `invalidInput`.
#![no_main]
#![allow(dead_code, unused_imports)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/contract/mod.rs"]
mod contract;
#[path = "../../src/emergency_kit.rs"]
mod emergency_kit;
#[path = "../../src/persist.rs"]
mod persist;

use contract::errors::ErrorCode;
use contract::session::KitLabels;
use emergency_kit::{KIT_CAPACITY, KitValues};

const FORBIDDEN: [&str; 4] = ["/URI", "/JavaScript", "/Launch", "/EmbeddedFile"];

fn labels() -> KitLabels {
    KitLabels {
        title: "Mosaic Emergency Kit".into(),
        intro: "Keep this sheet safe. With your master password and Secret Key you can open your vault even if your device is lost.".into(),
        email: "Account email".into(),
        account_id: "Account ID".into(),
        sign_in_url: "Sign-in address".into(),
        sign_in_url_value: "This device only".into(),
        secret_key: "Secret Key".into(),
        master_password: "Master password".into(),
        master_password_hint: "Write it here by hand, or keep it in your head.".into(),
        footer: "Mosaic has not been externally audited (pre-audit).".into(),
    }
}

fn texts(labels: &KitLabels) -> String {
    [
        &labels.title,
        &labels.intro,
        &labels.email,
        &labels.account_id,
        &labels.sign_in_url,
        &labels.sign_in_url_value,
        &labels.secret_key,
        &labels.master_password,
        &labels.master_password_hint,
        &labels.footer,
    ]
    .iter()
    .map(|s| s.as_str())
    .collect::<Vec<_>>()
    .join("\n")
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let (labels, email) = match mode {
        b'L' => match serde_json::from_slice::<KitLabels>(rest) {
            Ok(labels) => (labels, "alex@example.com".to_owned()),
            Err(_) => return,
        },
        b'E' => (labels(), String::from_utf8_lossy(rest).into_owned()),
        _ => return,
    };
    let values = KitValues {
        email: &email,
        account_id: "019a2b3c-4d5e-7000-8000-000000000001",
        secret_key: "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2",
    };
    match emergency_kit::render(&labels, &values) {
        Ok(pdf) => {
            let pdf = pdf.expose_secret();
            assert!(pdf.starts_with(b"%PDF-"));
            assert!(pdf.len() <= KIT_CAPACITY);
            // Text the input itself carries may legitimately spell a name.
            let input = format!("{}\n{email}", texts(&labels));
            for forbidden in FORBIDDEN {
                if !input.contains(forbidden) {
                    assert!(
                        !pdf.windows(forbidden.len())
                            .any(|w| w == forbidden.as_bytes())
                    );
                }
            }
        }
        Err(error) => assert_eq!(error.code, ErrorCode::InvalidInput),
    }
});

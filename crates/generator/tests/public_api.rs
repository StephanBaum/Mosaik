//! The public API with the real OS CSPRNG: every generator produces fresh,
//! well-formed output and never prints it.

use std::collections::HashSet;

use mosaic_generator::{
    Error, PassphraseOptions, PasswordOptions, PasswordRules, Strength, WORDLIST_LEN,
    generate_passphrase, generate_password, rules_for_domain,
};

#[test]
fn outputs_are_fresh_and_redacted() {
    let mut seen = HashSet::new();
    for _ in 0..50 {
        let p = generate_password(&PasswordOptions::default()).unwrap();
        assert!(
            seen.insert(p.expose_secret().to_owned()),
            "repeated password"
        );
        let dbg = format!("{p:?}");
        assert!(!dbg.contains(p.expose_secret()), "{dbg}");
    }
    let a = generate_passphrase(&PassphraseOptions::default()).unwrap();
    let b = generate_passphrase(&PassphraseOptions::default()).unwrap();
    assert_ne!(a.expose_secret(), b.expose_secret());
    assert!(!format!("{a:?}").contains(a.expose_secret()));
}

#[test]
fn live_entropy_display() {
    let e = PasswordOptions::default().entropy().unwrap();
    assert_eq!(e.strength(), Strength::VeryStrong);
    assert!(e.is_upper_bound());
    let e = PassphraseOptions::default().entropy().unwrap();
    assert!((e.bits() - 6.0 * 12.925).abs() < 1e-9);
    assert_eq!(e.strength(), Strength::Strong);
    assert_eq!(WORDLIST_LEN, 7776);
}

#[test]
fn site_rules_end_to_end() {
    let site = rules_for_domain("account.xiaomi.com").unwrap();
    assert_eq!(site.domain(), "account.xiaomi.com");
    let rules = site.rules();
    for _ in 0..20 {
        let p = rules.generate_default().unwrap();
        assert!(rules.is_satisfied_by(p.expose_secret()));
        assert!(p.entropy().is_upper_bound());
    }
    assert_eq!(
        PasswordRules::parse("minlength: 9; maxlength: 3")
            .generate(10)
            .unwrap_err(),
        Error::RulesUnsatisfiable
    );
}

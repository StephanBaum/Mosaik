//! Secret Key display format properties (CRYPTO_SPEC §5.2).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] functions"
)]

use mosaic_core::{Error, SecretBytes, SecretKey};
use proptest::prelude::*;
use sha2::{Digest, Sha256};

const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn key(bytes: [u8; 16]) -> SecretKey {
    SecretKey::from_secret(SecretBytes::from_slice(&bytes).unwrap())
}

/// Canonical value of a display character, with the parser's aliases.
fn canon(c: char) -> char {
    match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        o => o,
    }
}

/// Reference for `matches_last_group`, written from §5.2: drop `-` and
/// whitespace, fold case, map the Crockford aliases (`O -> 0`, `I`/`L -> 1`),
/// require every remaining character to be in the alphabet, and compare the
/// result with the 4-character last group.
fn reference_matches(last_group: &str, input: &str) -> bool {
    let normalized: Option<String> = input
        .chars()
        .filter(|&c| c != '-' && !c.is_whitespace())
        .map(|c| {
            let k = canon(c);
            ALPHABET.contains(k).then_some(k)
        })
        .collect();
    normalized.is_some_and(|n| n == last_group)
}

/// Rewrites `group` with the allowed leniency: per character a case and
/// alias choice, and a separator (none, `-`, space, tab, newline) before it.
fn lenient_variant(group: &str, forms: &[u8], seps: &[u8]) -> String {
    const SEPS: [&str; 5] = ["", "-", " ", "\t", "\n"];
    let mut out = String::new();
    for ((c, &f), &s) in group.chars().zip(forms).zip(seps) {
        out.push_str(SEPS[usize::from(s) % SEPS.len()]);
        let options: &[char] = match c {
            '0' => &['0', 'O', 'o'],
            '1' => &['1', 'I', 'i', 'L', 'l'],
            _ => &[c, c.to_ascii_lowercase()],
        };
        out.push(options[usize::from(f) % options.len()]);
    }
    out.push_str(SEPS[usize::from(seps[4]) % SEPS.len()]);
    out
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Arbitrary short inputs over the relevant characters (alphabet, lower
    /// case, aliases, `U`, separators, punctuation, non-ASCII) agree with the
    /// reference normalization.
    #[test]
    fn matches_last_group_agrees_with_reference_on_arbitrary_input(
        bytes in any::<[u8; 16]>(),
        input in "[0-9A-Za-z\\- \\t\\n!é]{0,10}",
    ) {
        let sk = key(bytes);
        let last = sk.last_group().unwrap();
        prop_assert_eq!(
            sk.matches_last_group(&input),
            reference_matches(last.expose_secret(), &input),
            "input {:?}, last group {}", input, last.expose_secret()
        );
    }

    /// Lenient rewrites of the real last group match, and so does the
    /// reference; a single substituted character changes both answers the
    /// same way.
    #[test]
    fn matches_last_group_accepts_lenient_variants(
        bytes in any::<[u8; 16]>(),
        forms in any::<[u8; 4]>(),
        seps in any::<[u8; 5]>(),
        pos in 0usize..4,
        sym in 0usize..32,
    ) {
        let sk = key(bytes);
        let last = sk.last_group().unwrap();
        let last = last.expose_secret();
        let variant = lenient_variant(last, &forms, &seps);
        prop_assert!(reference_matches(last, &variant));
        prop_assert!(sk.matches_last_group(&variant), "variant {:?}", variant);

        let mut typo: Vec<char> = last.chars().collect();
        typo[pos] = ALPHABET.chars().nth(sym).unwrap();
        let typo: String = typo.into_iter().collect();
        let variant = lenient_variant(&typo, &forms, &seps);
        prop_assert_eq!(sk.matches_last_group(&variant), typo == last);
        prop_assert_eq!(reference_matches(last, &variant), typo == last);
    }
}

#[test]
fn matches_last_group_examples() {
    let mut zeros = 0u32;
    let mut ones = 0u32;
    for n in 0u32..512 {
        let digest = Sha256::digest((n + 5000).to_be_bytes());
        let sk = key(digest[..16].try_into().unwrap());
        let shown = sk.to_display().unwrap();
        let shown = shown.expose_secret();
        let last = sk.last_group().unwrap();
        let last = last.expose_secret();
        assert_eq!(&shown[shown.len() - 4..], last);
        assert!(sk.matches_last_group(last));

        // Aliases and case.
        assert!(sk.matches_last_group(&last.to_lowercase()));
        if last.contains('0') {
            zeros += 1;
            assert!(sk.matches_last_group(&last.replace('0', "O")));
            assert!(sk.matches_last_group(&last.replace('0', "o")));
        }
        if last.contains('1') {
            ones += 1;
            for alias in ["I", "i", "L", "l"] {
                assert!(sk.matches_last_group(&last.replace('1', alias)));
            }
        }

        // Split groups and surrounding whitespace.
        let (a, b) = last.split_at(2);
        for split in [
            format!("{a}-{b}"),
            format!("{a} {b}"),
            format!(" {a}\t{b}\n"),
            format!("-{a}--{b}-"),
            last.chars().flat_map(|c| [c, ' ']).collect(),
        ] {
            assert!(sk.matches_last_group(&split), "{split:?}");
        }

        // Not the last group: the full display string, the last group with
        // its separator or prefix glued on, too short, too long, invalid.
        for wrong in [
            shown.to_string(),
            shown.replace('-', ""),
            format!("M1-{last}"),
            shown[shown.len() - 9..].to_string(),
            last[..3].to_string(),
            format!("{last}0"),
            format!("0{last}"),
            format!("{}U", &last[..3]),
            String::new(),
            "-".to_string(),
        ] {
            assert!(!sk.matches_last_group(&wrong), "{wrong:?} for {last}");
        }
    }
    // The examples exercised both alias families.
    assert!(zeros > 0 && ones > 0, "zeros {zeros}, ones {ones}");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn display_parse_round_trip(bytes in any::<[u8; 16]>()) {
        let sk = key(bytes);
        let shown = sk.to_display().unwrap();
        let s = shown.expose_secret();
        prop_assert_eq!(s.len(), 37);
        prop_assert!(s.starts_with("M1-"));
        prop_assert!(s[3..].split('-').all(|g| g.len() == 4 && g.chars().all(|c| ALPHABET.contains(c))));
        prop_assert_eq!(SecretKey::parse(s).unwrap().expose_secret().to_vec(), &bytes);
        prop_assert_eq!(SecretKey::parse(&s.to_lowercase()).unwrap().expose_secret().to_vec(), &bytes);
        let last = sk.last_group().unwrap();
        prop_assert!(s.ends_with(last.expose_secret()));
        prop_assert!(sk.matches_last_group(last.expose_secret()));
    }

    /// A single substituted character is never silently accepted as the
    /// original key; it is reported as mistyped or malformed, or (with
    /// probability about 1/256, when the checksum collides) parses as a
    /// different key.
    #[test]
    fn single_typo_never_returns_original(bytes in any::<[u8; 16]>(), pos in 0usize..28, sym in 0usize..32) {
        let sk = key(bytes);
        let shown = sk.to_display().unwrap();
        let s: Vec<char> = shown.expose_secret().chars().collect();
        let data_positions: Vec<usize> = (3..s.len()).filter(|&i| s[i] != '-').collect();
        let i = data_positions[pos];
        let replacement = ALPHABET.chars().nth(sym).unwrap();
        prop_assume!(canon(replacement) != canon(s[i]));
        let mut typo = s.clone();
        typo[i] = replacement;
        let typo: String = typo.into_iter().collect();
        match SecretKey::parse(&typo) {
            Ok(other) => prop_assert_ne!(other.expose_secret(), &bytes),
            Err(e) => prop_assert!(e == Error::SecretKeyMistyped || e == Error::SecretKeyMalformed),
        }
    }

    #[test]
    fn arbitrary_strings_never_panic(s in "\\PC{0,64}") {
        let _ = SecretKey::parse(&s);
    }
}

/// Exhaustive single-substitution typo detection over many keys: at least
/// 99 % of single-character typos must be rejected (the 8-bit checksum
/// gives about 255/256 for data characters; checksum and padding characters
/// are always detected).
#[test]
fn single_typo_detection_rate() {
    let mut total = 0u32;
    let mut detected = 0u32;
    let mut mistyped = 0u32;
    for n in 0u32..64 {
        // Deterministic, well-spread keys.
        let digest = Sha256::digest(n.to_be_bytes());
        let bytes: [u8; 16] = digest[..16].try_into().unwrap();
        let shown = key(bytes).to_display().unwrap();
        let s: Vec<char> = shown.expose_secret().chars().collect();
        for i in (3..s.len()).filter(|&i| s[i] != '-') {
            for r in ALPHABET.chars().filter(|&r| canon(r) != canon(s[i])) {
                let mut t = s.clone();
                t[i] = r;
                let t: String = t.into_iter().collect();
                total += 1;
                match SecretKey::parse(&t) {
                    Err(Error::SecretKeyMistyped) => {
                        detected += 1;
                        mistyped += 1;
                    }
                    Err(_) => detected += 1,
                    Ok(other) => assert_ne!(other.expose_secret(), &bytes),
                }
            }
        }
    }
    let rate = f64::from(detected) / f64::from(total);
    assert!(rate >= 0.99, "typo detection rate {rate}");
    // Most typos are reported as mistyped (checksum), not malformed.
    assert!(f64::from(mistyped) / f64::from(total) > 0.9);
}

#[test]
fn transposition_of_adjacent_characters_is_detected() {
    let mut total = 0u32;
    let mut detected = 0u32;
    for n in 0u32..64 {
        let digest = Sha256::digest((n + 1000).to_be_bytes());
        let bytes: [u8; 16] = digest[..16].try_into().unwrap();
        let shown = key(bytes).to_display().unwrap();
        let data: Vec<char> = shown.expose_secret()[3..]
            .chars()
            .filter(|&c| c != '-')
            .collect();
        for i in 0..data.len() - 1 {
            if data[i] == data[i + 1] {
                continue;
            }
            let mut t = data.clone();
            t.swap(i, i + 1);
            let t: String = format!("M1-{}", t.into_iter().collect::<String>());
            total += 1;
            if SecretKey::parse(&t).is_err() {
                detected += 1;
            }
        }
    }
    assert!(f64::from(detected) / f64::from(total) >= 0.99);
}

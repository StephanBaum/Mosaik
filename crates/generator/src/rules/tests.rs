//! Tests for the `passwordrules` parser and rule-based generation.
//!
//! The conformance cases are ported from Apple's reference test suite
//! (`tools/PasswordRulesParser/PasswordRulesParserTest.js`, MIT, Copyright
//! 2026 Apple Inc.), function by function, with the same inputs and
//! expected values. The one deliberate difference is integer overflow:
//! JavaScript numbers are unbounded there, this parser saturates at
//! `u32::MAX`.

use super::*;
use crate::rng::SplitMix64;
use crate::rng::testing::assert_uniform;
use proptest::prelude::*;

fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn describe_class(c: &RawClass) -> String {
    match c {
        RawClass::Named(n) => match n {
            Named::Upper => "upper",
            Named::Lower => "lower",
            Named::Digit => "digit",
            Named::Special => "special",
            Named::AsciiPrintable => "ascii-printable",
            Named::Unicode => "unicode",
        }
        .to_owned(),
        RawClass::Custom(chars) => format!("[{}]", chars.iter().collect::<String>()),
    }
}

fn describe_values(v: &[RawClass]) -> Vec<String> {
    v.iter().map(describe_class).collect()
}

/// `{ name, value }` of the reference tests, as a string.
fn describe_rule(rule: &RawRule) -> String {
    match rule {
        RawRule::Required(v) => format!("required: {:?}", describe_values(v)),
        RawRule::Allowed(v) => format!("allowed: {:?}", describe_values(v)),
        RawRule::MaxConsecutive(n) => format!("max-consecutive: {n}"),
        RawRule::MaxRepeating(n) => format!("max-repeating: {n}"),
        RawRule::MaxSequential(n) => format!("max-sequential: {n}"),
        RawRule::MinLength(n) => format!("minlength: {n}"),
        RawRule::MaxLength(n) => format!("maxlength: {n}"),
    }
}

#[test]
fn reference_skipping_whitespace() {
    for (input, offset, expected) in [
        ("", 0, None),
        (" ", 0, None),
        ("a", 0, Some('a')),
        ("  ", 0, None),
        ("  a", 0, Some('a')),
        ("\t  \t", 0, None),
        ("\n\n\ta", 0, Some('a')),
        ("\r\na", 0, Some('a')),
        ("\r\na", 1, Some('a')),
        ("\r\na", 2, Some('a')),
        ("\r\na", 3, None),
    ] {
        let input = chars(input);
        assert_eq!(at(&input, skip_whitespace(&input, offset)), expected);
    }
    // Only ASCII whitespace counts, as in the reference.
    let input = chars("\u{a0}a");
    assert_eq!(skip_whitespace(&input, 0), 0);
    let input = chars("\x0ca");
    assert_eq!(skip_whitespace(&input, 0), 1);
}

#[test]
fn reference_parsing_integer() {
    for (input, offset, expected) in [
        ("a", 0, None),
        ("0", 0, Some(0)),
        ("12", 0, Some(12)),
        ("127", 0, Some(127)),
        ("1a", 0, None),
        ("12a", 0, None),
        ("1;", 0, Some(1)),
        ("12;", 1, Some(2)),
        ("abc1234;", 3, Some(1234)),
        // Reference: 9007199254740991. Saturates here.
        ("9007199254740991", 0, Some(u32::MAX)),
        ("4294967295", 0, Some(u32::MAX)),
        ("4294967294", 0, Some(u32::MAX - 1)),
    ] {
        assert_eq!(
            parse_integer(&chars(input), offset).0,
            expected,
            "{input:?}"
        );
    }
}

#[test]
fn reference_parsing_custom_character_class() {
    let cases: &[(&str, usize, Option<&str>)] = &[
        ("[]", 0, Some("")),
        ("[ ]", 0, Some(" ")),
        ("[  ]", 0, Some("  ")),
        ("[a]", 0, Some("a")),
        ("[ab]", 0, Some("ab")),
        ("[abc]", 0, Some("abc")),
        ("[abc01A]", 0, Some("abc01A")),
        ("[12]", 0, Some("12")),
        ("[bab]", 0, Some("bab")),
        ("[abc];", 0, Some("abc")),
        ("[-]", 0, Some("-")),
        ("[a-]", 0, Some("a")),
        ("[a-b]", 0, Some("ab")),
        ("[-a]", 0, Some("-a")),
        ("[-ab]", 0, Some("-ab")),
        ("abcdefg[-ab];", 7, Some("-ab")),
        ("[]]", 0, Some("]")),
        ("[a]]", 0, Some("a]")),
        ("[abc]]", 0, Some("abc]")),
        ("[-a]]", 0, Some("-a]")),
        ("[", 0, None),
        ("[a", 0, None),
        ("[]a]", 0, Some("")),
        ("[a]b]", 0, Some("a")),
        ("[☃]", 0, Some("")),
        ("[a☃]", 0, Some("a")),
        ("[☃a]", 0, Some("a")),
        ("[a☃b]", 0, Some("ab")),
    ];
    for &(input, offset, expected) in cases {
        let (value, _) = parse_custom_class(&chars(input), offset);
        let value: Option<String> = value.map(|v| v.into_iter().collect());
        assert_eq!(value.as_deref(), expected, "{input:?}");
    }
    // Positions after the class.
    assert_eq!(parse_custom_class(&chars("[abc];"), 0).1, 5);
    assert_eq!(parse_custom_class(&chars("[abc]]x"), 0).1, 6);
    assert_eq!(parse_custom_class(&chars("[]a]"), 0).1, 2);
}

#[test]
fn reference_parsing_required_or_allowed_value() {
    let cases: &[(&str, Option<&[&str]>)] = &[
        ("ascii-printable", Some(&["ascii-printable"])),
        ("digit", Some(&["digit"])),
        ("lower", Some(&["lower"])),
        ("upper", Some(&["upper"])),
        ("special", Some(&["special"])),
        ("unicode", Some(&["unicode"])),
        ("UPPER", Some(&["upper"])),
        ("UPpER", Some(&["upper"])),
        ("upper, lower", Some(&["upper", "lower"])),
        ("upper,  lower", Some(&["upper", "lower"])),
        ("upper,  \tlower", Some(&["upper", "lower"])),
        ("[a]", Some(&["[a]"])),
        ("[a], upper", Some(&["[a]", "upper"])),
        ("upper, [a]", Some(&["upper", "[a]"])),
        ("upper, [a], lower", Some(&["upper", "[a]", "lower"])),
        ("[a], [b]", Some(&["[a]", "[b]"])),
        ("[], [b]", Some(&["[b]"])),
        ("[a]], [b]", Some(&["[a]]", "[b]"])),
        ("[a], lower, [b]", Some(&["[a]", "lower", "[b]"])),
        ("lower;", Some(&["lower"])),
        ("lower ;", Some(&["lower"])),
        ("lower \t;", Some(&["lower"])),
        (" ", None),
        ("☃", None),
        ("dummy", None),
        ("upper, dummy", None),
        ("upper,", None),
        (";", None),
        ("upper d", None),
    ];
    for &(input, expected) in cases {
        let (value, _) = parse_class_list(&chars(input), 0);
        let got = value.as_deref().map(describe_values);
        let expected = expected.map(|e| e.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        assert_eq!(got, expected, "{input:?}");
    }
}

#[test]
fn reference_parsing_single_rule() {
    // "-": no rule (unrecognized name or no colon). "null": the reference's
    // `value: null`.
    let cases: &[(&str, &str)] = &[
        ("required:", "null"),
        ("required: ", "null"),
        ("required:;", "null"),
        ("required: ;", "null"),
        ("allowed:", "null"),
        ("allowed: ;", "null"),
        ("max-consecutive:", "null"),
        ("max-consecutive: ;", "null"),
        ("max-repeating:", "null"),
        ("max-repeating: ;", "null"),
        ("max-sequential:", "null"),
        ("max-sequential: ;", "null"),
        ("required: upper", r#"required: ["upper"]"#),
        ("required: upper, lower", r#"required: ["upper", "lower"]"#),
        (
            "required: upper, [*], lower",
            r#"required: ["upper", "[*]", "lower"]"#,
        ),
        ("allowed: upper", r#"allowed: ["upper"]"#),
        (
            "allowed: upper, [*], lower",
            r#"allowed: ["upper", "[*]", "lower"]"#,
        ),
        ("max-consecutive:2", "max-consecutive: 2"),
        ("max-repeating:2", "max-repeating: 2"),
        ("max-sequential:2", "max-sequential: 2"),
        ("minlength: 12;", "minlength: 12"),
        ("maxlength: 73", "maxlength: 73"),
        ("dummy", "-"),
        ("required", "-"),
        // Not in the reference suite: 0 is falsy there, so it is dropped.
        ("minlength: 0", "null"),
        ("maxlength: x", "null"),
        ("required: dummy", "null"),
    ];
    for &(input, expected) in cases {
        let got = match parse_rule(&chars(input), 0).0 {
            ParsedRule::NotARule => "-".to_owned(),
            ParsedRule::NoValue => "null".to_owned(),
            ParsedRule::Rule(r) => describe_rule(&r),
        };
        assert_eq!(got, expected, "{input:?}");
    }
}

#[test]
fn reference_parsing_rules_internal() {
    let cases: &[(&str, Option<&[&str]>)] = &[
        ("required: upper", Some(&[r#"required: ["upper"]"#])),
        ("required: upper;", Some(&[r#"required: ["upper"]"#])),
        ("required: upper; ", Some(&[r#"required: ["upper"]"#])),
        ("required: upper;;", Some(&[r#"required: ["upper"]"#])),
        ("required: upper; ;", Some(&[r#"required: ["upper"]"#])),
        ("required: upper;;;", Some(&[r#"required: ["upper"]"#])),
        ("required: upper ", Some(&[r#"required: ["upper"]"#])),
        ("required: upper ; ", Some(&[r#"required: ["upper"]"#])),
        (
            "required: upper; required: lower",
            Some(&[r#"required: ["upper"]"#, r#"required: ["lower"]"#]),
        ),
        (
            "required: upper; allowed: [ab]",
            Some(&[r#"required: ["upper"]"#, r#"allowed: ["[ab]"]"#]),
        ),
        (
            "allowed: upper; allowed: lower",
            Some(&[r#"allowed: ["upper"]"#, r#"allowed: ["lower"]"#]),
        ),
        (
            "max-consecutive: 2; max-repeating: 3; max-sequential: 4",
            Some(&[
                "max-consecutive: 2",
                "max-repeating: 3",
                "max-sequential: 4",
            ]),
        ),
        ("minlength: 12", Some(&["minlength: 12"])),
        ("minlength:        12", Some(&["minlength: 12"])),
        (
            "minlength: 12; maxlength: 73",
            Some(&["minlength: 12", "maxlength: 73"]),
        ),
        ("required: ;", Some(&[])),
        ("☃", Some(&[])),
        ("required: upper d", None),
        // Not in the reference suite: a non-identifier at the start of a
        // later property keeps what was parsed so far.
        ("required: upper; ☃", Some(&[r#"required: ["upper"]"#])),
        // "minlength: 8 ;" fails the integer (a space before ';'), and the
        // rest still parses.
        ("minlength: 8 ; maxlength: 9", Some(&["maxlength: 9"])),
    ];
    for &(input, expected) in cases {
        let got = parse_rules_internal(&chars(input))
            .map(|rules| rules.iter().map(describe_rule).collect::<Vec<_>>());
        let expected = expected.map(|e| e.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        assert_eq!(got, expected, "{input:?}");
    }
}

fn custom(s: &str) -> RawClass {
    RawClass::Custom(s.chars().collect())
}

fn named(n: Named) -> RawClass {
    RawClass::Named(n)
}

#[test]
fn reference_canonicalizing_character_classes() {
    let upper: String = ('A'..='Z').collect();
    let lower: String = ('a'..='z').collect();
    let digits: String = ('0'..='9').collect();
    let special: String = (' '..='~').filter(|c| !c.is_ascii_alphanumeric()).collect();
    let printable: String = (' '..='~').collect();
    let cases: Vec<(Vec<RawClass>, &str)> = vec![
        (vec![named(Named::Upper)], "upper"),
        (vec![named(Named::Upper), named(Named::Upper)], "upper"),
        (vec![named(Named::Upper), custom("abc")], "upper, [abc]"),
        (vec![named(Named::Upper), custom(&upper)], "upper"),
        (vec![custom(&upper), named(Named::Upper)], "upper"),
        (vec![custom("WABCXYZDEFGHIJKLMNOPQRSTUV")], "upper"),
        (vec![custom(&upper), custom(&lower)], "upper, lower"),
        (vec![custom(&special), named(Named::Special)], "special"),
        (
            vec![custom(&printable), named(Named::AsciiPrintable)],
            "ascii-printable",
        ),
        (vec![custom(&digits), named(Named::Digit)], "digit"),
        (
            vec![
                custom(&upper),
                custom(&digits),
                custom(&lower),
                custom(&special),
            ],
            "ascii-printable",
        ),
        (vec![named(Named::Unicode)], "unicode"),
        // Beyond the reference suite: format-compliant custom classes.
        (vec![custom("]-a")], "[-a]]"),
        (vec![custom("A-")], "[-A]"),
        (vec![custom("]")], "[]]"),
        (vec![custom("[")], "[[]"),
        (vec![named(Named::Lower), custom("A ")], "lower, [A ]"),
        (vec![named(Named::Unicode), named(Named::Upper)], "unicode"),
        (vec![named(Named::Upper), named(Named::Unicode)], "unicode"),
    ];
    for (values, expected) in cases {
        let class = canonical_class(&values);
        assert_eq!(class.to_string(), expected);
        // And the display form parses back to the same set.
        let reparsed = PasswordRules::parse(&format!("allowed: {class}"));
        assert_eq!(reparsed.allowed(), class, "{expected}");
    }
}

#[test]
#[allow(clippy::too_many_lines, reason = "one table of reference cases")]
fn reference_parsing_password_rules() {
    for (input, expected) in [
        ("", "allowed: ascii-printable"),
        ("    required: upper", "required: upper; allowed: upper"),
        ("required: uPPeR", "required: upper; allowed: upper"),
        ("maxLENGTH: 7", "allowed: ascii-printable"),
        ("allowed:upper", "allowed: upper"),
        (
            "required: upper; allowed: upper; allowed: lower",
            "required: upper; allowed: upper, lower",
        ),
        (
            "max-consecutive: 5; max-consecutive: 3",
            "allowed: ascii-printable; max-consecutive: 3",
        ),
        (
            "max-consecutive: 3; max-consecutive: 1; max-consecutive: 5",
            "allowed: ascii-printable; max-consecutive: 1",
        ),
        (
            "max-sequential:5",
            "allowed: ascii-printable; max-sequential: 5",
        ),
        (
            "max-sequential: 3; max-sequential: 1; max-sequential: 5",
            "allowed: ascii-printable; max-sequential: 1",
        ),
        (
            "max-repeating:5",
            "allowed: ascii-printable; max-repeating: 5",
        ),
        (
            "max-repeating: 5; max-repeating: 3; max-repeating: 1",
            "allowed: ascii-printable; max-repeating: 1",
        ),
        (
            "max-consecutive: 2; max-sequential: 4",
            "allowed: ascii-printable; max-consecutive: 2; max-sequential: 4",
        ),
        (
            "max-consecutive: 1; max-repeating: 3; max-sequential: 2",
            "allowed: ascii-printable; max-consecutive: 1; max-repeating: 3; max-sequential: 2",
        ),
        (
            "required: ascii-printable; max-consecutive: 5; max-consecutive: 3",
            "required: ascii-printable; allowed: ascii-printable; max-consecutive: 3",
        ),
        (
            "required: [*&^]; allowed: upper",
            "required: [&*^]; allowed: upper, [&*^]",
        ),
        (
            "required: unicode; required: digit",
            "required: unicode; required: digit; allowed: unicode",
        ),
        (
            "required: ; required: upper",
            "required: upper; allowed: upper",
        ),
        ("allowed: [供应商责任进展]", "allowed: ascii-printable"),
        ("allowed: [供应A商B责任C进展]", "allowed: [ABC]"),
        (
            "minlength: 12; minlength: 7; minlength: 23",
            "allowed: ascii-printable; minlength: 23",
        ),
        (
            "maxlength: 10; maxlength: 7; maxlength: 12",
            "allowed: ascii-printable; maxlength: 7",
        ),
        (
            "required: upper; allowed: upper; allowed: lower; minlength: 12; maxlength: 73;",
            "required: upper; allowed: upper, lower; minlength: 12; maxlength: 73",
        ),
        // Beyond the reference suite.
        ("required: []; allowed: digit", "allowed: digit"),
    ] {
        let rules = PasswordRules::parse(input);
        assert_eq!(rules.to_string(), expected, "{input:?}");
        assert_eq!(PasswordRules::parse(&rules.to_string()), rules, "{input:?}");
    }
    for malformed in [
        "allowed: upper,,",
        "allowed: upper,;",
        "allowed: upper [a]",
        "dummy: upper",
        "upper: lower",
        "max-consecutive: [ABC]",
        "max-consecutive: upper",
        "max-consecutive: 1+1",
        "required: 1",
        "required: A",
        "allowed: 1+1",
    ] {
        assert_eq!(
            PasswordRules::parse(malformed),
            PasswordRules::default(),
            "{malformed:?}"
        );
    }
    assert_eq!(
        PasswordRules::default().to_string(),
        "allowed: ascii-printable"
    );
}

#[test]
fn accessors() {
    let r = PasswordRules::parse(
        "minlength: 8; maxlength: 20; max-consecutive: 2; max-repeating: 3; max-sequential: 4; required: digit; allowed: lower",
    );
    assert_eq!(r.min_length(), Some(8));
    assert_eq!(r.max_length(), Some(20));
    assert_eq!(r.max_consecutive(), Some(2));
    assert_eq!(r.max_repeating(), Some(3));
    assert_eq!(r.max_sequential(), Some(4));
    assert_eq!(r.required().len(), 1);
    assert!(r.required()[0].contains('7'));
    assert!(!r.required()[0].contains('a'));
    assert!(r.allowed().contains('a'));
    assert!(!r.allowed().contains('A'));
    assert!(!r.allowed().contains('ä'));
    assert!(!r.allowed().is_unicode());
    let u = PasswordRules::parse("allowed: unicode");
    assert!(u.allowed().is_unicode());
    assert!(u.allowed().contains('ä'));
}

#[test]
fn is_satisfied_by() {
    let r = PasswordRules::parse(
        "minlength: 4; maxlength: 6; required: upper; required: digit; allowed: lower; max-consecutive: 2",
    );
    assert!(r.is_satisfied_by("Ab1c"));
    assert!(!r.is_satisfied_by("Ab1"), "too short");
    assert!(!r.is_satisfied_by("Ab1cdef"), "too long");
    assert!(!r.is_satisfied_by("abcd1"), "no upper");
    assert!(!r.is_satisfied_by("Abcde"), "no digit");
    assert!(!r.is_satisfied_by("Ab1c!"), "! not allowed");
    assert!(!r.is_satisfied_by("Ab1ccc"), "run of three");
    assert!(r.is_satisfied_by("Ab1cc"));
    let seq = PasswordRules::parse("max-sequential: 2; max-repeating: 2");
    assert!(seq.is_satisfied_by("abxba"));
    assert!(!seq.is_satisfied_by("abc"));
    assert!(!seq.is_satisfied_by("cba"));
    assert!(!seq.is_satisfied_by("axaxa"));
    let uni = PasswordRules::parse("required: unicode; allowed: unicode");
    assert!(uni.is_satisfied_by("ä"));
    assert!(!uni.is_satisfied_by(""));
}

#[test]
fn generation_basics() {
    let mut rng = SplitMix64::new(1);
    let r = PasswordRules::parse("minlength: 8; maxlength: 12; required: lower; required: digit");
    let p = r.generate_with(20, &mut rng).unwrap();
    assert_eq!(p.expose_secret().len(), 12, "clamped to maxlength");
    assert!(r.is_satisfied_by(p.expose_secret()));
    let p = r.generate_with(0, &mut rng).unwrap();
    assert_eq!(p.expose_secret().len(), 8, "clamped to minlength");
    let e = r.entropy(10).unwrap();
    assert!((e.bits() - 36f64.log2() * 10.0).abs() < 1e-9);
    assert!(e.is_upper_bound());

    let unconstrained = PasswordRules::parse("allowed: lower");
    let e = unconstrained.entropy(16).unwrap();
    assert!(!e.is_upper_bound());
    assert!((e.bits() - 26f64.log2() * 16.0).abs() < 1e-9);

    let p = PasswordRules::default().generate_default().unwrap();
    assert_eq!(p.expose_secret().len(), DEFAULT_LENGTH as usize);
    assert!(
        p.expose_secret()
            .bytes()
            .all(|b| (b'!'..=b'~').contains(&b))
    );
    let p = PasswordRules::default().generate(500).unwrap();
    assert_eq!(p.expose_secret().len(), MAX_LENGTH as usize);
    assert_eq!(p.capacity(), MAX_LENGTH as usize);
}

#[test]
fn space_is_left_out_unless_required() {
    let mut rng = SplitMix64::new(2);
    let r = PasswordRules::parse("allowed: [ ab]; minlength: 64; maxlength: 64");
    for _ in 0..20 {
        let p = r.generate_with(64, &mut rng).unwrap();
        assert!(!p.expose_secret().contains(' '));
    }
    assert!(
        (r.entropy(64).unwrap().bits() - 64.0).abs() < 1e-9,
        "two letters"
    );

    let needs_space = PasswordRules::parse("required: [ ]; allowed: lower; minlength: 10");
    let p = needs_space.generate_with(10, &mut rng).unwrap();
    assert!(p.expose_secret().contains(' '));
    assert!(needs_space.is_satisfied_by(p.expose_secret()));

    let only_space = PasswordRules::parse("allowed: [ ]; maxlength: 3");
    let p = only_space.generate_with(10, &mut rng).unwrap();
    assert_eq!(p.expose_secret(), "   ");
    assert_eq!(only_space.entropy(3).unwrap().bits(), 0.0);
}

#[test]
fn unicode_generates_printable_ascii() {
    let mut rng = SplitMix64::new(3);
    let r = PasswordRules::parse("allowed: unicode; required: unicode; required: digit");
    let p = r.generate_with(30, &mut rng).unwrap();
    assert!(
        p.expose_secret()
            .bytes()
            .all(|b| (b'!'..=b'~').contains(&b))
    );
    assert!(r.is_satisfied_by(p.expose_secret()));
    assert!((r.entropy(30).unwrap().bits() - 94f64.log2() * 30.0).abs() < 1e-9);
}

#[test]
fn unsatisfiable_and_strict_rules() {
    let mut rng = SplitMix64::new(4);
    for (rule, err) in [
        ("minlength: 10; maxlength: 5", Error::RulesUnsatisfiable),
        ("minlength: 200", Error::RulesUnsatisfiable),
        // One character, no repeats, two positions.
        (
            "allowed: [a]; max-consecutive: 1; minlength: 2",
            Error::ConstraintsTooStrict,
        ),
        // Two characters, each at most twice, five positions.
        (
            "allowed: [ab]; max-repeating: 2; minlength: 5; maxlength: 5",
            Error::ConstraintsTooStrict,
        ),
        // Four disjoint single characters do not fit in three positions.
        (
            "required: [a]; required: [b]; required: [c]; required: [d]; maxlength: 3",
            Error::ConstraintsTooStrict,
        ),
        // Eight specific characters in at most 30 positions are too unlikely.
        (
            "allowed: ascii-printable; required: [a]; required: [b]; required: [c]; required: [d]; required: [e]; required: [f]; required: [g]; required: [h]; maxlength: 30",
            Error::ConstraintsTooStrict,
        ),
        // No string over {a, b} of length 3 avoids both runs and sequences.
        (
            "allowed: [ab]; max-sequential: 1; max-consecutive: 1; minlength: 3; maxlength: 3",
            Error::RetryLimit,
        ),
    ] {
        let r = PasswordRules::parse(rule);
        assert_eq!(r.generate_with(20, &mut rng).unwrap_err(), err, "{rule}");
    }
    // Too many distinct requirements for the estimate.
    let many = ('a'..='m')
        .map(|c| format!("required: [{c}]"))
        .collect::<Vec<_>>()
        .join("; ");
    let r = PasswordRules::parse(&many);
    assert_eq!(r.required().len(), 13);
    assert_eq!(r.entropy(128).unwrap_err(), Error::ConstraintsTooStrict);
}

#[test]
fn length_grows_until_requirements_are_likely() {
    let rule = "allowed: ascii-printable; required: [a]; required: [b]; required: [c]; required: [d]; required: [e]; required: [f]; required: [g]; required: [h]";
    let r = PasswordRules::parse(rule);
    let policy = r.policy(20).unwrap();
    assert!(
        policy.length() > 20 && policy.length() < 80,
        "{}",
        policy.length()
    );
    let p = r.generate_with(20, &mut SplitMix64::new(5)).unwrap();
    assert!(r.is_satisfied_by(p.expose_secret()));
}

#[test]
fn redundant_requirements_are_pruned() {
    // `lower` is implied by `[ab]`, `digit` is implied by `[1]`.
    let r = PasswordRules::parse(
        "required: lower; required: [ab]; required: digit; required: [1]; required: [1]",
    );
    let policy = r.policy(12).unwrap();
    let p = r.generate_with(12, &mut SplitMix64::new(6)).unwrap();
    assert!(r.is_satisfied_by(p.expose_secret()));
    assert_eq!(policy.length(), 12);
    // Requirements that every character satisfies are dropped too.
    let all = PasswordRules::parse("required: ascii-printable");
    assert!(!all.entropy(10).unwrap().is_upper_bound());
}

/// Rule-based generation is uniform over the qualifying strings.
#[test]
fn chi_squared_uniform_under_rules() {
    let r = PasswordRules::parse(
        "required: [ab]; required: [01]; allowed: [ab01]; minlength: 3; maxlength: 3",
    );
    let mut all = Vec::new();
    for x in "ab01".chars() {
        for y in "ab01".chars() {
            for z in "ab01".chars() {
                let s: String = [x, y, z].into_iter().collect();
                if r.is_satisfied_by(&s) {
                    all.push(s);
                }
            }
        }
    }
    assert_eq!(all.len(), 48);
    let mut counts = vec![0u64; all.len()];
    let mut rng = SplitMix64::new(0x7275_6c65);
    for _ in 0..all.len() * 2_000 {
        let p = r.generate_with(3, &mut rng).unwrap();
        let i = all.iter().position(|s| s == p.expose_secret()).unwrap();
        counts[i] += 1;
    }
    assert_uniform(&counts, "rules: 48 qualifying strings");
}

fn class_token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("upper".to_owned()),
        Just("lower".to_owned()),
        Just("digit".to_owned()),
        Just("special".to_owned()),
        Just("ascii-printable".to_owned()),
        Just("unicode".to_owned()),
        "[-]?[!-~]{1,6}\\]?".prop_map(|s| format!("[{s}]")),
    ]
}

fn class_list() -> impl Strategy<Value = String> {
    prop::collection::vec(class_token(), 1..4).prop_map(|v| v.join(", "))
}

fn property() -> impl Strategy<Value = String> {
    prop_oneof![
        class_list().prop_map(|l| format!("required: {l}")),
        class_list().prop_map(|l| format!("allowed: {l}")),
        (1u32..6).prop_map(|n| format!("max-consecutive: {n}")),
        (1u32..12).prop_map(|n| format!("max-repeating: {n}")),
        (2u32..8).prop_map(|n| format!("max-sequential: {n}")),
        (1u32..40).prop_map(|n| format!("minlength: {n}")),
        (1u32..140).prop_map(|n| format!("maxlength: {n}")),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Parsing never panics and the canonical display is a fixed point.
    #[test]
    fn parse_display_round_trip(input in "\\PC{0,80}") {
        let rules = PasswordRules::parse(&input);
        let shown = rules.to_string();
        prop_assert_eq!(PasswordRules::parse(&shown), rules);
    }

    #[test]
    fn parse_display_round_trip_ascii(input in "[ -~]{0,80}") {
        let rules = PasswordRules::parse(&input);
        prop_assert_eq!(PasswordRules::parse(&rules.to_string()), rules);
    }

    /// Well-formed rule sets: generation either succeeds with a password
    /// that satisfies the rules, or fails with a rules error.
    #[test]
    fn generated_passwords_satisfy_rules(
        props in prop::collection::vec(property(), 0..6),
        preferred in 0u32..150,
        seed in any::<u64>(),
    ) {
        let text = props.join("; ");
        let rules = PasswordRules::parse(&text);
        prop_assert_eq!(PasswordRules::parse(&rules.to_string()), rules.clone());
        match rules.generate_with(preferred, &mut SplitMix64::new(seed)) {
            Ok(p) => {
                prop_assert!(rules.is_satisfied_by(p.expose_secret()), "{} -> violates {}", text, rules);
                prop_assert_eq!(p.capacity(), p.expose_secret().len());
            }
            Err(e) => prop_assert!(
                matches!(e, Error::RulesUnsatisfiable | Error::ConstraintsTooStrict | Error::RetryLimit),
                "{e:?}"
            ),
        }
    }
}

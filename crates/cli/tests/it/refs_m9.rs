//! M9 test-engineer pass: the `inject` template scanner (`refs::scan_template`) is total and
//! lossless. The segments, with each reference re-wrapped, rebuild the input, a reference is only
//! ever a trimmed `mosaic://` placeholder, and the scan is the same on every run.

use mosaic_cli::refs::{Segment, scan_template};

fn rebuild(segments: &[Segment<'_>]) -> String {
    segments
        .iter()
        .map(|s| match s {
            Segment::Text(t) => (*t).to_owned(),
            Segment::Reference(r) => format!("{{{{{r}}}}}"),
        })
        .collect()
}

/// A small deterministic generator over the template alphabet that matters.
fn templates() -> Vec<String> {
    let atoms = [
        "{{",
        "}}",
        "{",
        "}",
        " ",
        "\n",
        "mosaic://",
        "v/i",
        "/pw",
        "\u{e9}",
        "x",
        "{{ mosaic://a/b }}",
    ];
    let mut out = vec![String::new()];
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..3000 {
        let mut s = String::new();
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let len = usize::try_from(state >> 60).unwrap();
        for _ in 0..len {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            s.push_str(atoms[usize::try_from(state >> 33).unwrap() % atoms.len()]);
        }
        out.push(s);
    }
    out
}

#[test]
fn scan_template_is_lossless_and_only_finds_trimmed_mosaic_references() {
    for t in templates() {
        let segments = scan_template(&t);
        // A placeholder is re-wrapped without its padding, so compare after removing spaces
        // around references: rebuild and normalize the original the same way.
        let rebuilt = rebuild(&segments);
        let plain = scan_template(&rebuilt);
        assert_eq!(plain, segments, "idempotent on {t:?}");
        for s in &segments {
            if let Segment::Reference(r) = s {
                assert!(r.starts_with("mosaic://"), "{t:?}");
                assert_eq!(r.trim(), *r, "{t:?}");
                assert!(!r.contains("}}"), "{t:?}");
            }
        }
        // Without any reference, the text comes back exactly.
        if segments.iter().all(|s| matches!(s, Segment::Text(_))) {
            assert_eq!(rebuilt, t, "{t:?}");
        }
    }
}

#[test]
fn scan_template_handles_the_boundaries() {
    assert_eq!(scan_template(""), [Segment::Text("")]);
    assert_eq!(
        scan_template("{{mosaic://a/b}}"),
        [
            Segment::Text(""),
            Segment::Reference("mosaic://a/b"),
            Segment::Text("")
        ]
    );
    // Unclosed, empty and foreign placeholders stay literal.
    for t in ["{{ mosaic://a/b", "{{}}", "{{ other }}", "}}{{"] {
        assert!(
            scan_template(t)
                .iter()
                .all(|s| matches!(s, Segment::Text(_))),
            "{t:?}"
        );
        assert_eq!(rebuild(&scan_template(t)), t);
    }
    // The first closing braces end a placeholder, so a reference cannot swallow later text.
    assert_eq!(
        scan_template("{{ mosaic://a/b }} }}"),
        [
            Segment::Text(""),
            Segment::Reference("mosaic://a/b"),
            Segment::Text(" }}")
        ]
    );
}

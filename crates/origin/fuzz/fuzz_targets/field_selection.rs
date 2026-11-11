//! Field selection (ADR-0092, brief §4.3): any view tree never panics the
//! selector, no selected field is invisible or in another web domain than the
//! root, and a focus in a foreign frame refuses the request with no fields.
//!
//! The input is read as nodes of 5 bytes (flags, domain, two hints and an
//! input type) followed by nothing else; bytes that do not fill a node are
//! ignored.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_origin::fields::{FieldNode, FieldRefusal, InputKind, select_fill_fields};

const DOMAINS: [Option<&str>; 4] = [None, Some("a.com"), Some("b.com"), Some("A.com")];
const HINTS: [&str; 8] = [
    "username",
    "password",
    "newPassword",
    "smsOTPCode",
    "emailAddress",
    "login_email",
    "kennwort",
    "",
];
const AUTOCOMPLETE: [Option<&str>; 4] = [
    None,
    Some("one-time-code"),
    Some("section-x current-password"),
    Some("name"),
];

fuzz_target!(|data: &[u8]| {
    let nodes: Vec<FieldNode> = data
        .chunks_exact(5)
        .map(|c| FieldNode {
            visible: c[0] & 1 != 0,
            focused: c[0] & 2 != 0,
            web_domain: DOMAINS[usize::from(c[1]) % DOMAINS.len()].map(str::to_owned),
            hints: vec![HINTS[usize::from(c[2]) % HINTS.len()].to_owned()],
            autocomplete: AUTOCOMPLETE[usize::from(c[3]) % AUTOCOMPLETE.len()].map(str::to_owned),
            input_kind: match c[4] % 3 {
                0 => InputKind::Text,
                1 => InputKind::Password,
                _ => InputKind::Other,
            },
            id_entry: (c[4] & 0x80 != 0).then(|| "pwd_field".to_owned()),
        })
        .collect();
    let selection = select_fill_fields(&nodes);
    let root = nodes.first().and_then(|n| n.web_domain.clone());
    assert_eq!(selection.root_domain, root);
    if selection.refused == Some(FieldRefusal::ForeignFrame) {
        assert!(selection.fields.is_empty());
        assert!(nodes.iter().any(|n| n.focused && n.web_domain != root));
    }
    for field in &selection.fields {
        let node = &nodes[field.index];
        assert!(node.visible);
        assert_eq!(node.web_domain, root);
    }
});

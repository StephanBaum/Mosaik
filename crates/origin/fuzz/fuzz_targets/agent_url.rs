//! Agent destinations (ADR-0159): any saved string and any request URL never
//! panic, and an allowed destination keeps its invariants.
//!
//! The input is split at the first NUL byte into a saved destination and a
//! request URL.
//!
//! - A saved destination that parses is canonical: it parses again, unchanged,
//!   with `parse_canonical`.
//! - When the request URL parses and `agent_destination_allowed` accepts it,
//!   the parsed `Url` that would be sent has `https`, no userinfo and exactly
//!   the saved origin.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_origin::{AgentOrigin, agent_destination_allowed, parse_agent_url};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (saved_text, url_text) = text.split_once('\0').unwrap_or((text, text));

    let saved = AgentOrigin::from_input(saved_text);
    if let Ok(saved) = &saved {
        let again = AgentOrigin::parse_canonical(saved.as_str())
            .expect("a saved destination re-parses as canonical");
        assert_eq!(&again, saved, "canonical round trip");
        assert!(!saved.host().is_empty() && !saved.host().ends_with('.'));
    }
    let _ = AgentOrigin::parse_canonical(saved_text);

    let url = parse_agent_url(url_text);
    if let (Ok(saved), Ok(u)) = (&saved, &url)
        && agent_destination_allowed(saved, u).is_ok()
    {
        assert!(
            u.username().is_empty() && u.password().is_none() && u.scheme() == "https",
            "no userinfo, https only"
        );
        assert_eq!(
            u.origin().ascii_serialization(),
            saved.as_str(),
            "the sent origin is the saved origin"
        );
        assert_eq!(u.host_str(), Some(saved.host()), "the sent host is the saved host");
    }
});

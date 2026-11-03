//! Characters that make a name look like another one (ADR-0030 red-team
//! follow-up): the Unicode general category Cf (format) and the
//! `Default_Ignorable_Code_Point` property, as one table of ranges (Unicode
//! 16; no dependency). They are invisible or reorder text, so a vault name or
//! an item title could hide or disguise part of itself in a list, the
//! quick-search palette or an approval prompt.
//!
//! The table is deliberately a superset of the bidirectional controls and
//! zero-width characters. It includes the variation selectors U+FE00 to
//! U+FE0F, so an emoji written with U+FE0F is refused too.

/// Whether `c` is in Cf or `Default_Ignorable_Code_Point`.
#[must_use]
pub const fn is_spoofing_char(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// Whether `text` contains a [`is_spoofing_char`].
#[must_use]
pub fn has_spoofing_char(text: &str) -> bool {
    text.chars().any(is_spoofing_char)
}

/// `text` without its [`is_spoofing_char`] characters, and how many were
/// removed (for imports, which clean a title instead of refusing the file).
#[must_use]
pub fn strip_spoofing(text: &str) -> (String, usize) {
    let kept: String = text.chars().filter(|c| !is_spoofing_char(*c)).collect();
    let removed = text.chars().count() - kept.chars().count();
    (kept, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_listed_characters_are_refused() {
        for c in [
            '\u{00AD}',
            '\u{034F}',
            '\u{061C}',
            '\u{115F}',
            '\u{1160}',
            '\u{180E}',
            '\u{200B}',
            '\u{200F}',
            '\u{202A}',
            '\u{202E}',
            '\u{2060}',
            '\u{2066}',
            '\u{2069}',
            '\u{206F}',
            '\u{3164}',
            '\u{FE00}',
            '\u{FE0F}',
            '\u{FEFF}',
            '\u{FFF9}',
            '\u{FFFB}',
            '\u{E0000}',
            '\u{E0001}',
            '\u{E0020}',
            '\u{E0FFF}',
        ] {
            assert!(is_spoofing_char(c), "{c:?}");
        }
    }

    #[test]
    fn ordinary_text_passes() {
        for text in [
            "Privat \u{fc}\u{df}",
            "\u{4f60}\u{597d}",
            "Caf\u{e9} \u{1F510}",
            "a b-c_d",
        ] {
            assert!(!has_spoofing_char(text), "{text:?}");
        }
        for c in [
            '\u{00AC}',
            '\u{00AE}',
            '\u{2010}',
            '\u{3163}',
            '\u{E1000}',
            'a',
            ' ',
        ] {
            assert!(!is_spoofing_char(c), "{c:?}");
        }
    }

    #[test]
    fn stripping_counts_what_it_removed() {
        assert_eq!(
            strip_spoofing("Ba\u{202E}nk\u{200B}"),
            ("Bank".to_owned(), 2)
        );
        assert_eq!(strip_spoofing("Bank"), ("Bank".to_owned(), 0));
        assert_eq!(strip_spoofing("\u{FEFF}"), (String::new(), 1));
    }
}

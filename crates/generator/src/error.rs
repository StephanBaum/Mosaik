//! The crate's single error type. Messages never contain generated secrets.

/// Errors from password and passphrase generation.
///
/// Every message says what went wrong and what to change. No variant carries
/// generated material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The operating system's CSPRNG failed, or returned values so skewed
    /// that rejection sampling could not finish. There is no fallback
    /// generator.
    #[error("the system random number generator failed; try again")]
    Randomness,
    /// The requested password length is outside the supported range.
    #[error("the password length must be between {min} and {max} characters")]
    InvalidLength {
        /// Smallest allowed length.
        min: u32,
        /// Largest allowed length.
        max: u32,
    },
    /// No character class is enabled.
    #[error("enable at least one character class")]
    NoCharacterClasses,
    /// The symbol set is empty or contains something other than ASCII
    /// punctuation.
    #[error("the symbol set must contain only ASCII punctuation characters")]
    InvalidSymbolSet,
    /// Excluding ambiguous characters left an enabled class empty.
    #[error("an enabled character class has no characters left after excluding ambiguous ones")]
    EmptyCharacterClass,
    /// The per-class minimums add up to more than the length.
    #[error("the minimum counts add up to more than the password length")]
    MinimumsExceedLength,
    /// The constraints are satisfiable in principle, but so few strings meet
    /// them that generate-and-retry would rarely succeed.
    #[error(
        "the settings are too strict to generate a password; lower the minimums or allow more characters"
    )]
    ConstraintsTooStrict,
    /// The requested number of passphrase words is outside the supported
    /// range.
    #[error("a passphrase must have between {min} and {max} words")]
    InvalidWordCount {
        /// Fewest allowed words.
        min: u32,
        /// Most allowed words.
        max: u32,
    },
    /// The passphrase separator is too long or contains a control character.
    #[error("the separator must be at most {max} characters and contain no control characters")]
    InvalidSeparator {
        /// Longest allowed separator, in characters.
        max: u32,
    },
    /// The site's password rules cannot be satisfied at all (for example,
    /// `minlength` above `maxlength`).
    #[error("the site's password rules contradict each other; generate a password without them")]
    RulesUnsatisfiable,
    /// No candidate met the constraints within the retry budget.
    #[error("no password meeting the requirements was found; relax the settings and try again")]
    RetryLimit,
    /// The embedded word list did not load. Indicates a corrupted build.
    #[error("the built-in word list is damaged; reinstall the application")]
    Wordlist,
}

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_limits() {
        assert_eq!(
            Error::InvalidLength { min: 4, max: 128 }.to_string(),
            "the password length must be between 4 and 128 characters"
        );
        assert_eq!(
            Error::InvalidWordCount { min: 3, max: 20 }.to_string(),
            "a passphrase must have between 3 and 20 words"
        );
        assert!(
            Error::InvalidSeparator { max: 8 }
                .to_string()
                .contains("at most 8")
        );
        for e in [
            Error::Randomness,
            Error::NoCharacterClasses,
            Error::InvalidSymbolSet,
            Error::EmptyCharacterClass,
            Error::MinimumsExceedLength,
            Error::ConstraintsTooStrict,
            Error::RulesUnsatisfiable,
            Error::RetryLimit,
            Error::Wordlist,
        ] {
            assert_ne!(e.to_string(), "");
        }
    }
}

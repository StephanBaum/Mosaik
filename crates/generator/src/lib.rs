//! Password and passphrase generation for Mosaic (CRYPTO_SPEC §13).
//!
//! - **Random characters** ([`generate_password`], [`PasswordOptions`]):
//!   lowercase, uppercase, digits and a configurable symbol set, optional
//!   exclusion of look-alike characters, per-class minimums, 4 to 128
//!   characters.
//! - **Passphrases** ([`generate_passphrase`], [`PassphraseOptions`]): words
//!   from the EFF large wordlist (7776 words, 12.925 bits each), with a
//!   separator, optional capitalization and an optional digit.
//! - **Site rules** ([`PasswordRules`], [`rules_for_domain`]): Apple's
//!   `passwordrules` language and the vendored Password Manager Resources
//!   quirks, to generate passwords a given site accepts.
//! - **Entropy** ([`Entropy`], [`Strength`]) for a live strength display.
//!
//! # Randomness
//!
//! Every draw comes from the operating system's CSPRNG (`getrandom`); the
//! API has no way to supply another generator. Uniform choices use
//! rejection sampling, so there is no modulo bias. Constraints (minimums,
//! site rules) are met by generating a whole candidate uniformly and
//! retrying until it qualifies, never by forcing characters into positions,
//! so every qualifying output is equally likely.
//!
//! Generated text is returned in [`GeneratedPassword`], which zeroizes on
//! drop and never prints its contents.
//!
//! ```
//! use mosaic_generator::{PasswordOptions, generate_password, rules_for_domain};
//!
//! # fn main() -> Result<(), mosaic_generator::Error> {
//! let password = generate_password(&PasswordOptions::default())?;
//! assert_eq!(password.expose_secret().chars().count(), 20);
//!
//! if let Some(site) = rules_for_domain("www.163.com") {
//!     let p = site.rules().generate_default()?;
//!     assert!(site.rules().is_satisfied_by(p.expose_secret()));
//! }
//! # Ok(())
//! # }
//! ```
#![forbid(unsafe_code)]

mod entropy;
mod error;
mod passphrase;
mod password;
mod policy;
mod rng;
mod rules;
mod secret;
mod sites;

pub use entropy::{BITS_PER_WORD, Entropy, Strength};
pub use error::{Error, Result};
pub use passphrase::{
    DEFAULT_SEPARATOR, DEFAULT_WORDS, MAX_SEPARATOR_CHARS, MAX_WORDS, MIN_WORDS, PassphraseOptions,
    WORDLIST_LEN, generate_passphrase,
};
pub use password::{
    AMBIGUOUS_CHARACTERS, DEFAULT_LENGTH, DEFAULT_SYMBOLS, MAX_LENGTH, MIN_LENGTH, PasswordOptions,
    generate_password,
};
pub use rules::{CharacterClass, MAX_REQUIRED_CLASSES, MAX_RULES_LEN, PasswordRules};
pub use secret::GeneratedPassword;
pub use sites::{SiteRules, rules_for_domain};

/// Deterministic generation for the fuzz targets only. Compiled only under
/// `cfg(fuzzing)` (set by cargo-fuzz); normal builds do not contain it.
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing {
    use crate::rng::SplitMix64;
    use crate::{GeneratedPassword, PasswordRules, Result};

    /// [`PasswordRules::generate`] with a seeded, **insecure** generator, so
    /// a crash reproduces from its input.
    ///
    /// # Errors
    /// As for [`PasswordRules::generate`].
    pub fn generate_with_rules_seeded(
        rules: &PasswordRules,
        preferred_length: u32,
        seed: u64,
    ) -> Result<GeneratedPassword> {
        rules.generate_with(preferred_length, &mut SplitMix64::new(seed))
    }
}

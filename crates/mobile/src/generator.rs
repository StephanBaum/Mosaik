//! The password and passphrase generator (`mosaic-generator`). It needs no
//! session: it works while locked too.

use std::sync::Arc;

use mosaic_generator::{
    PassphraseOptions, PasswordOptions, generate_passphrase, generate_password,
};

use crate::Core;
use crate::error::Result;
use crate::secret::SecretValue;

/// What to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratorMode {
    /// Random characters.
    Password,
    /// Random words.
    Passphrase,
}

/// Generator settings. Fields of the other mode are ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorOptions {
    /// Password or passphrase.
    pub mode: GeneratorMode,
    /// Password length in characters.
    pub length: u32,
    /// Require at least one lowercase letter.
    pub lowercase: bool,
    /// Require at least one uppercase letter.
    pub uppercase: bool,
    /// Require at least one digit.
    pub digits: bool,
    /// Require at least one symbol.
    pub symbols: bool,
    /// Leave out look-alike characters.
    pub exclude_ambiguous: bool,
    /// Passphrase word count.
    pub words: u32,
    /// Passphrase separator.
    pub separator: String,
    /// Capitalize every word.
    pub capitalize: bool,
    /// Append a digit to one word.
    pub append_digit: bool,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            mode: GeneratorMode::Password,
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
            words: 6,
            separator: "-".to_owned(),
            capitalize: false,
            append_digit: false,
        }
    }
}

/// A generated secret and its entropy.
pub struct GeneratedSecret {
    /// The value.
    pub value: Arc<SecretValue>,
    /// Entropy in bits (an upper bound for site-rule passwords).
    pub entropy_bits: f64,
}

impl Core {
    /// Generates a password or passphrase.
    ///
    /// # Errors
    /// `invalidInput` for options out of range, `internal` if the system
    /// randomness fails.
    pub fn generate_password(&self, options: GeneratorOptions) -> Result<GeneratedSecret> {
        let generated = match options.mode {
            GeneratorMode::Password => {
                let mut o = PasswordOptions::default();
                let min = |on: bool| on.then_some(1);
                o.length = options.length;
                o.lowercase = min(options.lowercase);
                o.uppercase = min(options.uppercase);
                o.digits = min(options.digits);
                o.symbols = min(options.symbols);
                o.exclude_ambiguous = options.exclude_ambiguous;
                generate_password(&o)?
            }
            GeneratorMode::Passphrase => {
                let mut o = PassphraseOptions::default();
                o.words = options.words;
                o.separator = options.separator;
                o.capitalize = options.capitalize;
                o.append_digit = options.append_digit;
                generate_passphrase(&o)?
            }
        };
        Ok(GeneratedSecret {
            entropy_bits: generated.entropy().bits(),
            value: self.issue(SecretValue::from_text(generated.expose_secret())),
        })
    }
}

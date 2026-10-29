//! Generator command (mosaic-generator). Works locked and unlocked: it
//! needs no vault key.

use mosaic_core::SecretString;
use mosaic_generator::{
    Error as GeneratorError, GeneratedPassword, PassphraseOptions, PasswordOptions,
    Strength as CoreStrength, generate_passphrase, generate_password as generate_chars,
    rules_for_domain,
};
use tauri::State;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::generator::{GenerateRequest, Generated, Strength};
use crate::contract::secret::SecretOut;
use crate::state::{AppState, Backend, guard};

/// `generate_password`.
#[tauri::command]
pub async fn generate_password(
    state: State<'_, AppState>,
    request: GenerateRequest,
) -> IpcResult<Generated> {
    state.run(move |b| b.generate(request)).await
}

const fn strength(strength: CoreStrength) -> Strength {
    match strength {
        CoreStrength::VeryWeak => Strength::VeryWeak,
        CoreStrength::Weak => Strength::Weak,
        CoreStrength::Fair => Strength::Fair,
        CoreStrength::Strong => Strength::Strong,
        CoreStrength::VeryStrong => Strength::VeryStrong,
    }
}

fn generator_error(error: GeneratorError) -> IpcError {
    IpcError::new(match error {
        GeneratorError::Randomness | GeneratorError::Wordlist => ErrorCode::Internal,
        _ => ErrorCode::InvalidInput,
    })
}

fn generate(request: GenerateRequest) -> Result<GeneratedPassword, GeneratorError> {
    match request {
        GenerateRequest::Password {
            length,
            lowercase,
            uppercase,
            digits,
            symbols,
            exclude_ambiguous,
        } => {
            let mut options = PasswordOptions::default();
            options.length = length;
            options.lowercase = lowercase.then_some(1);
            options.uppercase = uppercase.then_some(1);
            options.digits = digits.then_some(1);
            options.symbols = symbols.then_some(1);
            options.exclude_ambiguous = exclude_ambiguous;
            generate_chars(&options)
        }
        GenerateRequest::Passphrase {
            words,
            separator,
            capitalize,
            append_digit,
        } => {
            let mut options = PassphraseOptions::default();
            options.words = words;
            options.separator = separator;
            options.capitalize = capitalize;
            options.append_digit = append_digit;
            generate_passphrase(&options)
        }
        GenerateRequest::ForSite { host } => match rules_for_domain(&host) {
            Some(site) => site.rules().generate_default(),
            None => generate_chars(&PasswordOptions::default()),
        },
    }
}

impl Backend {
    /// Generates a password and keeps it for `copy_to_clipboard`.
    ///
    /// # Errors
    /// `invalidInput` for options the generator refuses.
    pub fn generate(&self, request: GenerateRequest) -> IpcResult<Generated> {
        self.touch();
        let generated = generate(request).map_err(generator_error)?;
        let entropy = generated.entropy();
        let kept = SecretString::from_str_copy(generated.expose_secret())?;
        let shown = SecretString::from_str_copy(generated.expose_secret())?;
        *guard(&self.last_generated)? = Some(kept);
        Ok(Generated {
            value: SecretOut::new(shown),
            entropy_bits: entropy.bits(),
            strength: strength(entropy.strength()),
        })
    }
}

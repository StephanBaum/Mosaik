//! Password generator (mosaic-generator, ADR-0011). The generated value is
//! shown in the generator, so it is returned (`SecretOut`); Rust also keeps
//! it for `copy_to_clipboard` with `{ kind: "generated" }`.

use serde::{Deserialize, Serialize};

use super::macros::wire_enum;
use super::secret::SecretOut;

wire_enum!(
    /// Strength buckets of the entropy display (`mosaic_generator::Strength`).
    pub enum Strength {
        /// Below 36 bits.
        VeryWeak,
        /// 36 to under 60 bits.
        Weak,
        /// 60 to under 75 bits.
        Fair,
        /// 75 to under 100 bits.
        Strong,
        /// 100 bits or more.
        VeryStrong,
    }
);

/// `generate_password`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GenerateRequest {
    /// Random characters.
    Password {
        /// Length in characters.
        length: u32,
        /// Include a to z.
        lowercase: bool,
        /// Include A to Z.
        uppercase: bool,
        /// Include 0 to 9.
        digits: bool,
        /// Include the default symbol set.
        symbols: bool,
        /// Leave out characters that look alike.
        exclude_ambiguous: bool,
    },
    /// Words from the EFF large wordlist.
    Passphrase {
        /// Number of words.
        words: u32,
        /// Between the words.
        separator: String,
        /// Capitalize each word.
        capitalize: bool,
        /// Append one digit.
        append_digit: bool,
    },
    /// Following a site's password rules (`passwordrules` and the vendored
    /// quirks data), or the defaults if the site has none.
    ForSite {
        /// The site's host name.
        host: String,
    },
}

/// The `generate_password` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    /// The password, for display.
    pub value: SecretOut,
    /// Its entropy in bits (an upper bound for site rules).
    pub entropy_bits: f64,
    /// The strength bucket, for the meter's text label.
    pub strength: Strength,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn requests() -> Vec<GenerateRequest> {
        vec![
            GenerateRequest::Password {
                length: 20,
                lowercase: true,
                uppercase: true,
                digits: true,
                symbols: true,
                exclude_ambiguous: false,
            },
            GenerateRequest::Passphrase {
                words: 5,
                separator: "-".into(),
                capitalize: true,
                append_digit: false,
            },
            GenerateRequest::ForSite {
                host: "example.com".into(),
            },
        ]
    }

    pub(crate) fn generated() -> Generated {
        Generated {
            value: SecretOut::test("q7#Vt2!mZp9@Lx4$Nw6&"),
            entropy_bits: 131.1,
            strength: Strength::VeryStrong,
        }
    }
}

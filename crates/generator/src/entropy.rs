//! Entropy estimates and strength buckets for the UI (CRYPTO_SPEC §13).

/// Bits per word of the EFF large wordlist, `log2(7776) = 12.925` (rounded
/// as in CRYPTO_SPEC §13).
pub const BITS_PER_WORD: f64 = 12.925;

/// An entropy estimate in bits.
///
/// Random characters: `log2(|alphabet|) × length`. Passphrases:
/// `12.925 × words`. When constraints reject some candidates (per-class
/// minimums, site rules), the true entropy is lower and the value is an
/// upper bound; [`Entropy::is_upper_bound`] says so and the UI must show it
/// as "up to".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entropy {
    bits: f64,
    upper_bound: bool,
}

impl Entropy {
    pub(crate) fn new(bits: f64, upper_bound: bool) -> Self {
        Self { bits, upper_bound }
    }

    /// `log2(alphabet) × length`.
    pub(crate) fn for_characters(alphabet: u32, length: u32, upper_bound: bool) -> Self {
        Self::new(f64::from(alphabet).log2() * f64::from(length), upper_bound)
    }

    /// The estimate in bits.
    #[must_use]
    pub fn bits(&self) -> f64 {
        self.bits
    }

    /// Whether [`Entropy::bits`] is only an upper bound, because constraints
    /// reject some otherwise possible outputs.
    #[must_use]
    pub fn is_upper_bound(&self) -> bool {
        self.upper_bound
    }

    /// The strength bucket for these bits.
    #[must_use]
    pub fn strength(&self) -> Strength {
        Strength::from_bits(self.bits)
    }
}

/// Coarse strength buckets for the UI, by entropy in bits.
///
/// | Bucket | Bits |
/// |---|---|
/// | `VeryWeak` | < 36 |
/// | `Weak` | 36 to < 60 |
/// | `Fair` | 60 to < 75 |
/// | `Strong` | 75 to < 100 |
/// | `VeryStrong` | ≥ 100 |
///
/// For orientation: 8 random characters from 94 are 52 bits (`Weak`), 12
/// are 79 bits (`Strong`), 16 are 105 bits (`VeryStrong`); a 5-word EFF
/// passphrase is 64.6 bits (`Fair`), 6 words are 77.6 bits (`Strong`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strength {
    /// Below 36 bits: guessable online.
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

impl Strength {
    /// The bucket for `bits`. `NaN` counts as `VeryWeak`.
    #[must_use]
    pub fn from_bits(bits: f64) -> Self {
        if bits >= 100.0 {
            Self::VeryStrong
        } else if bits >= 75.0 {
            Self::Strong
        } else if bits >= 60.0 {
            Self::Fair
        } else if bits >= 36.0 {
            Self::Weak
        } else {
            Self::VeryWeak
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_per_word_matches_the_wordlist() {
        assert!((7776f64.log2() - BITS_PER_WORD).abs() < 0.001);
    }

    #[test]
    fn buckets() {
        assert_eq!(Strength::from_bits(f64::NAN), Strength::VeryWeak);
        assert_eq!(Strength::from_bits(0.0), Strength::VeryWeak);
        assert_eq!(Strength::from_bits(35.99), Strength::VeryWeak);
        assert_eq!(Strength::from_bits(36.0), Strength::Weak);
        assert_eq!(Strength::from_bits(59.9), Strength::Weak);
        assert_eq!(Strength::from_bits(60.0), Strength::Fair);
        assert_eq!(Strength::from_bits(74.9), Strength::Fair);
        assert_eq!(Strength::from_bits(75.0), Strength::Strong);
        assert_eq!(Strength::from_bits(99.9), Strength::Strong);
        assert_eq!(Strength::from_bits(100.0), Strength::VeryStrong);
        assert_eq!(Strength::from_bits(f64::INFINITY), Strength::VeryStrong);
        assert!(Strength::Weak < Strength::Strong);
    }

    #[test]
    fn character_entropy() {
        let e = Entropy::for_characters(94, 16, false);
        assert!((e.bits() - 104.87).abs() < 0.01);
        assert!(!e.is_upper_bound());
        assert_eq!(e.strength(), Strength::VeryStrong);
        let e = Entropy::for_characters(94, 8, true);
        assert!(e.is_upper_bound());
        assert_eq!(e.strength(), Strength::Weak);
        assert_eq!(Entropy::for_characters(1, 50, false).bits(), 0.0);
    }

    #[test]
    fn documented_examples() {
        assert_eq!(Strength::from_bits(BITS_PER_WORD * 5.0), Strength::Fair);
        assert_eq!(Strength::from_bits(BITS_PER_WORD * 6.0), Strength::Strong);
        assert_eq!(
            Entropy::for_characters(94, 12, false).strength(),
            Strength::Strong
        );
    }
}

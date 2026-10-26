//! HOTP (RFC 4226), TOTP (RFC 6238) and Steam Guard code generation
//! (CRYPTO_SPEC §13).
//!
//! # Steam Guard
//!
//! Valve does not publish the algorithm. Every interoperable implementation
//! (Valve's mobile app as reverse-engineered by `SteamKit`, `steamguard-cli`
//! `steamguard/src/token.rs`, `node-steam-totp` `generateAuthCode`, Aegis,
//! `KeePassXC`) computes it the same way, and this module follows them:
//!
//! 1. Compute the RFC 6238 TOTP counter `T = floor(unix_time / 30)` and the
//!    RFC 4226 dynamic truncation of `HMAC-SHA-1(K, T)`, a 31-bit integer
//!    `v` (no modulo `10^digits`).
//! 2. Five times: output `ALPHABET[v mod 26]`, then `v = floor(v / 26)`,
//!    with `ALPHABET = "23456789BCDFGHJKMNPQRTVWXY"`.
//!
//! The known-answer test below uses the vector from `steamguard-cli`'s
//! `test_generate_code` (secret `zvIayp3JPvtvX/QGHqsqKBk/44s=` in base64,
//! time 1616374841, code `2F9J5`), which was also recomputed independently
//! with Python's `hmac` module. Steam Guard always uses SHA-1 and 30 s in
//! practice; the algorithm and period stay configurable because
//! `otpauth://` URIs may state them.

use core::fmt;

use hmac::{Hmac, KeyInit, Mac};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{Error, Result};
use crate::secret::OtpSecret;

/// The Steam Guard alphabet (CRYPTO_SPEC §13).
pub const STEAM_ALPHABET: &[u8; 26] = b"23456789BCDFGHJKMNPQRTVWXY";

/// Length of a Steam Guard code.
pub const STEAM_CODE_LEN: usize = 5;

/// HMAC hash function (RFC 6238 §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Algorithm {
    /// HMAC-SHA-1, the default of RFC 4226 and of `otpauth://`.
    #[default]
    Sha1,
    /// HMAC-SHA-256.
    Sha256,
    /// HMAC-SHA-512.
    Sha512,
}

impl Algorithm {
    /// The `otpauth://` spelling: `SHA1`, `SHA256` or `SHA512`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }

    /// Parses the `otpauth://` spelling, ignoring ASCII case.
    ///
    /// # Errors
    /// [`Error::UnsupportedAlgorithm`] for anything else.
    pub fn parse(s: &str) -> Result<Self> {
        [Self::Sha1, Self::Sha256, Self::Sha512]
            .into_iter()
            .find(|a| a.as_str().eq_ignore_ascii_case(s))
            .ok_or(Error::UnsupportedAlgorithm)
    }
}

/// Number of decimal digits of an HOTP/TOTP code: 6, 7 or 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digits(u8);

impl Digits {
    /// Fewest digits accepted (RFC 4226 §5.3 requires at least 6).
    pub const MIN: u8 = 6;
    /// Most digits accepted.
    pub const MAX: u8 = 8;
    /// Six digits, the usual default.
    pub const DEFAULT: Self = Self(6);

    /// A code length of `digits` digits.
    ///
    /// # Errors
    /// [`Error::InvalidDigits`] outside [`Self::MIN`]..=[`Self::MAX`].
    pub const fn new(digits: u8) -> Result<Self> {
        if digits >= Self::MIN && digits <= Self::MAX {
            Ok(Self(digits))
        } else {
            Err(Error::InvalidDigits)
        }
    }

    /// The number of digits.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for Digits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TOTP time step `X` in seconds: 1 to 300.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Period(u32);

impl Period {
    /// Shortest period accepted.
    pub const MIN: u32 = 1;
    /// Longest period accepted.
    pub const MAX: u32 = 300;
    /// 30 seconds, the RFC 6238 default.
    pub const DEFAULT: Self = Self(30);

    /// A time step of `seconds` seconds.
    ///
    /// # Errors
    /// [`Error::InvalidPeriod`] outside [`Self::MIN`]..=[`Self::MAX`].
    pub const fn new(seconds: u32) -> Result<Self> {
        if seconds >= Self::MIN && seconds <= Self::MAX {
            Ok(Self(seconds))
        } else {
            Err(Error::InvalidPeriod)
        }
    }

    /// The period in seconds.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The RFC 6238 counter `T = floor((unix_time - T0) / X)` with `T0 = 0`.
    #[must_use]
    pub const fn time_step(self, unix_time: u64) -> u64 {
        // `self.0 >= 1` by construction.
        unix_time / self.0 as u64
    }

    /// Seconds until the code for `unix_time` expires, in `1..=period`.
    #[must_use]
    pub fn seconds_remaining(self, unix_time: u64) -> u32 {
        let p = u64::from(self.0);
        // `unix_time % p < p <= 300`, so the conversion cannot fail.
        self.0 - u32::try_from(unix_time % p).unwrap_or(0)
    }
}

impl Default for Period {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A generated one-time code: 5 to 8 ASCII characters, zeroized on drop.
///
/// A code grants access for as long as it is valid, so it is treated as a
/// short-lived secret: no `Display`, no `Clone`, redacted `Debug`.
pub struct Code {
    buf: [u8; 8],
    len: u8,
}

impl Code {
    fn new() -> Self {
        Self {
            buf: [0; 8],
            len: 0,
        }
    }

    fn push(&mut self, c: u8) {
        if let Some(slot) = self.buf.get_mut(usize::from(self.len)) {
            *slot = c;
            self.len += 1;
        }
    }

    /// The code as text. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        let bytes = self.buf.get(..usize::from(self.len)).unwrap_or_default();
        // Only ASCII digits and Steam alphabet characters are ever pushed.
        core::str::from_utf8(bytes).unwrap_or_default()
    }

    /// Number of characters.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Always `false` for a generated code.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for Code {
    fn drop(&mut self) {
        self.buf.zeroize();
    }
}

impl ZeroizeOnDrop for Code {}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// RFC 4226 §5.3 dynamic truncation: the 31-bit value at the offset given
/// by the low nibble of the last MAC byte.
fn dynamic_truncation(mac: &[u8]) -> u32 {
    let offset = usize::from(mac.last().copied().unwrap_or(0) & 0x0F);
    // Every supported MAC is at least 20 bytes, so `offset + 3 <= 18` is in
    // bounds; the fallback is never taken.
    let word: [u8; 4] = core::array::from_fn(|i| mac.get(offset + i).copied().unwrap_or(0));
    u32::from_be_bytes(word) & 0x7FFF_FFFF
}

fn truncated_with<M: Mac + KeyInit>(key: &[u8], counter: u64) -> Result<u32> {
    // HMAC accepts keys of any length; the error arm is unreachable.
    let mut mac = <M as KeyInit>::new_from_slice(key).map_err(|_| Error::InvalidSecret)?;
    Mac::update(&mut mac, &counter.to_be_bytes());
    // `CtOutput` zeroizes the MAC on drop (`digest/zeroize`).
    let out = mac.finalize();
    Ok(dynamic_truncation(out.as_bytes().as_slice()))
}

/// `DT(HMAC(K, C))` for the chosen algorithm (RFC 4226 §5.3 step 1 and 2).
fn truncated(secret: &OtpSecret, algorithm: Algorithm, counter: u64) -> Result<u32> {
    let key = secret.expose_secret();
    match algorithm {
        Algorithm::Sha1 => truncated_with::<Hmac<sha1::Sha1>>(key, counter),
        Algorithm::Sha256 => truncated_with::<Hmac<sha2::Sha256>>(key, counter),
        Algorithm::Sha512 => truncated_with::<Hmac<sha2::Sha512>>(key, counter),
    }
}

fn decimal_code(mut value: u32, digits: Digits) -> Code {
    value %= 10u32.pow(u32::from(digits.get()));
    let mut rev = [0u8; 8];
    for slot in rev.iter_mut().take(usize::from(digits.get())) {
        *slot = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
    }
    let mut code = Code::new();
    for &c in rev.iter().take(usize::from(digits.get())).rev() {
        code.push(c);
    }
    rev.zeroize();
    value.zeroize();
    code
}

/// `HOTP(K, C)` (RFC 4226 §5.3).
///
/// # Errors
/// None in practice; the `Result` covers HMAC key setup.
pub fn hotp(
    secret: &OtpSecret,
    algorithm: Algorithm,
    digits: Digits,
    counter: u64,
) -> Result<Code> {
    let mut v = truncated(secret, algorithm, counter)?;
    let code = decimal_code(v, digits);
    v.zeroize();
    Ok(code)
}

/// `TOTP(K, T)` with `T = floor(unix_time / period)` (RFC 6238 §4).
///
/// # Errors
/// None in practice; see [`hotp`].
pub fn totp(
    secret: &OtpSecret,
    algorithm: Algorithm,
    digits: Digits,
    period: Period,
    unix_time: u64,
) -> Result<Code> {
    hotp(secret, algorithm, digits, period.time_step(unix_time))
}

/// A Steam Guard code (see the module documentation).
///
/// # Errors
/// None in practice; see [`hotp`].
pub fn steam_guard(
    secret: &OtpSecret,
    algorithm: Algorithm,
    period: Period,
    unix_time: u64,
) -> Result<Code> {
    let mut v = truncated(secret, algorithm, period.time_step(unix_time))?;
    let mut code = Code::new();
    for _ in 0..STEAM_CODE_LEN {
        let idx = usize::try_from(v % 26).unwrap_or(0);
        code.push(STEAM_ALPHABET.get(idx).copied().unwrap_or(b'2'));
        v /= 26;
    }
    v.zeroize();
    Ok(code)
}

/// The current Unix time in whole seconds, from the system clock.
///
/// # Errors
/// [`Error::TimeBeforeEpoch`] if the clock is set before 1970.
pub fn unix_time_now() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| Error::TimeBeforeEpoch)
}

/// Which code an [`Otp`] produces, with its parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OtpKind {
    /// Time-based (RFC 6238).
    Totp {
        /// Code length.
        digits: Digits,
        /// Time step.
        period: Period,
    },
    /// Counter-based (RFC 4226). The caller stores and advances `counter`.
    Hotp {
        /// Code length.
        digits: Digits,
        /// The moving factor `C` for the next code.
        counter: u64,
    },
    /// Steam Guard: time-based, 5 characters from [`STEAM_ALPHABET`].
    Steam {
        /// Time step (30 s for Steam).
        period: Period,
    },
}

impl OtpKind {
    /// TOTP with 6 digits and 30 s.
    pub const DEFAULT_TOTP: Self = Self::Totp {
        digits: Digits::DEFAULT,
        period: Period::DEFAULT,
    };
    /// Steam Guard with 30 s.
    pub const STEAM: Self = Self::Steam {
        period: Period::DEFAULT,
    };
}

/// A complete one-time-password configuration: secret, hash and kind.
#[derive(Debug, PartialEq, Eq)]
pub struct Otp {
    /// The shared secret.
    pub secret: OtpSecret,
    /// The HMAC hash function.
    pub algorithm: Algorithm,
    /// TOTP, HOTP or Steam Guard, with parameters.
    pub kind: OtpKind,
}

impl Otp {
    /// The code at `unix_time`. HOTP ignores the time and uses its counter.
    ///
    /// # Errors
    /// None in practice; see [`hotp`].
    pub fn code_at(&self, unix_time: u64) -> Result<Code> {
        match self.kind {
            OtpKind::Totp { digits, period } => {
                totp(&self.secret, self.algorithm, digits, period, unix_time)
            }
            OtpKind::Hotp { digits, counter } => {
                hotp(&self.secret, self.algorithm, digits, counter)
            }
            OtpKind::Steam { period } => {
                steam_guard(&self.secret, self.algorithm, period, unix_time)
            }
        }
    }

    /// Seconds until the code for `unix_time` expires, in `1..=period`;
    /// `None` for HOTP.
    #[must_use]
    pub fn seconds_remaining(&self, unix_time: u64) -> Option<u32> {
        match self.kind {
            OtpKind::Totp { period, .. } | OtpKind::Steam { period } => {
                Some(period.seconds_remaining(unix_time))
            }
            OtpKind::Hotp { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use proptest::prelude::*;

    const RFC_SHA1: &[u8] = b"12345678901234567890";
    const RFC_SHA256: &[u8] = b"12345678901234567890123456789012";
    const RFC_SHA512: &[u8] = b"1234567890123456789012345678901234567890123456789012345678901234";

    fn secret(b: &[u8]) -> OtpSecret {
        OtpSecret::from_bytes(b).unwrap()
    }

    /// RFC 4226 Appendix D: truncated values and 6-digit HOTP for counters
    /// 0 to 9.
    #[test]
    fn rfc4226_appendix_d() {
        let k = secret(RFC_SHA1);
        let table: [(u32, &str); 10] = [
            (1_284_755_224, "755224"),
            (1_094_287_082, "287082"),
            (137_359_152, "359152"),
            (1_726_969_429, "969429"),
            (1_640_338_314, "338314"),
            (868_254_676, "254676"),
            (1_918_287_922, "287922"),
            (82_162_583, "162583"),
            (673_399_871, "399871"),
            (645_520_489, "520489"),
        ];
        for (counter, (dec, code)) in (0u64..).zip(table) {
            assert_eq!(truncated(&k, Algorithm::Sha1, counter).unwrap(), dec);
            let c = hotp(&k, Algorithm::Sha1, Digits::DEFAULT, counter).unwrap();
            assert_eq!(c.expose_secret(), code, "counter {counter}");
            assert_eq!(c.len(), 6);
            assert!(!c.is_empty());
        }
    }

    /// RFC 4226 §5.4 worked example of dynamic truncation.
    #[test]
    fn rfc4226_section_5_4() {
        let mac = hex!("1f8698690e02ca16618550ef7f19da8e945b555a");
        assert_eq!(dynamic_truncation(&mac), 0x50ef_7f19);
        assert_eq!(
            decimal_code(0x50ef_7f19, Digits::DEFAULT).expose_secret(),
            "872921"
        );
    }

    /// RFC 6238 Appendix B, every row, with the per-algorithm seeds.
    #[test]
    fn rfc6238_appendix_b() {
        let rows: [(u64, u64, &str, &str, &str); 6] = [
            (59, 0x1, "94287082", "46119246", "90693936"),
            (
                1_111_111_109,
                0x0235_23EC,
                "07081804",
                "68084774",
                "25091201",
            ),
            (
                1_111_111_111,
                0x0235_23ED,
                "14050471",
                "67062674",
                "99943326",
            ),
            (
                1_234_567_890,
                0x0273_EF07,
                "89005924",
                "91819424",
                "93441116",
            ),
            (
                2_000_000_000,
                0x03F9_40AA,
                "69279037",
                "90698825",
                "38618901",
            ),
            (
                20_000_000_000,
                0x27BC_86AA,
                "65353130",
                "77737706",
                "47863826",
            ),
        ];
        let eight = Digits::new(8).unwrap();
        let p = Period::DEFAULT;
        let keys = [
            (Algorithm::Sha1, secret(RFC_SHA1)),
            (Algorithm::Sha256, secret(RFC_SHA256)),
            (Algorithm::Sha512, secret(RFC_SHA512)),
        ];
        for (t, step, c1, c256, c512) in rows {
            assert_eq!(p.time_step(t), step);
            for ((alg, k), want) in keys.iter().zip([c1, c256, c512]) {
                let code = totp(k, *alg, eight, p, t).unwrap();
                assert_eq!(code.expose_secret(), want, "{alg:?} at {t}");
            }
        }
    }

    /// `steamguard-cli` `test_generate_code`; see the module docs.
    #[test]
    fn steam_guard_vector() {
        // base64 "zvIayp3JPvtvX/QGHqsqKBk/44s="
        let k = secret(&hex!("cef21aca9dc93efb6f5ff4061eab2a28193fe38b"));
        let code = steam_guard(&k, Algorithm::Sha1, Period::DEFAULT, 1_616_374_841).unwrap();
        assert_eq!(code.expose_secret(), "2F9J5");
        let otp = Otp {
            secret: k,
            algorithm: Algorithm::Sha1,
            kind: OtpKind::STEAM,
        };
        assert_eq!(otp.code_at(1_616_374_841).unwrap().expose_secret(), "2F9J5");
        assert_eq!(otp.seconds_remaining(1_616_374_841), Some(19));
    }

    #[test]
    fn otp_dispatch() {
        let hotp_otp = Otp {
            secret: secret(RFC_SHA1),
            algorithm: Algorithm::Sha1,
            kind: OtpKind::Hotp {
                digits: Digits::DEFAULT,
                counter: 9,
            },
        };
        assert_eq!(hotp_otp.code_at(123).unwrap().expose_secret(), "520489");
        assert_eq!(hotp_otp.seconds_remaining(123), None);
        let totp_otp = Otp {
            secret: secret(RFC_SHA1),
            algorithm: Algorithm::Sha1,
            kind: OtpKind::DEFAULT_TOTP,
        };
        // 6-digit truncation of the RFC 6238 SHA-1 row at t = 59.
        assert_eq!(totp_otp.code_at(59).unwrap().expose_secret(), "287082");
        assert_eq!(totp_otp.seconds_remaining(59), Some(1));
        assert_eq!(totp_otp.seconds_remaining(60), Some(30));
        assert_eq!(format!("{:?}", totp_otp.code_at(0).unwrap()), "[REDACTED]");
    }

    #[test]
    fn parameter_bounds() {
        assert_eq!(Digits::new(5).err(), Some(Error::InvalidDigits));
        assert_eq!(Digits::new(9).err(), Some(Error::InvalidDigits));
        assert_eq!(Digits::new(7).unwrap().get(), 7);
        assert_eq!(Digits::default(), Digits::DEFAULT);
        assert_eq!(Period::new(0).err(), Some(Error::InvalidPeriod));
        assert_eq!(Period::new(301).err(), Some(Error::InvalidPeriod));
        assert_eq!(Period::new(1).unwrap().get(), 1);
        assert_eq!(Period::new(300).unwrap().get(), 300);
        assert_eq!(Period::default(), Period::DEFAULT);
        assert_eq!(Algorithm::default(), Algorithm::Sha1);
        assert_eq!(Algorithm::parse("sha256"), Ok(Algorithm::Sha256));
        assert_eq!(Algorithm::parse("SHA512"), Ok(Algorithm::Sha512));
        assert_eq!(Algorithm::parse("Sha1"), Ok(Algorithm::Sha1));
        assert_eq!(Algorithm::parse("MD5"), Err(Error::UnsupportedAlgorithm));
        assert_eq!(Algorithm::parse("SHA-1"), Err(Error::UnsupportedAlgorithm));
    }

    #[test]
    fn extreme_times() {
        let k = secret(RFC_SHA1);
        let one = Period::new(1).unwrap();
        let max = Period::new(300).unwrap();
        for t in [0, u64::MAX] {
            for p in [one, max] {
                assert_eq!(
                    totp(&k, Algorithm::Sha1, Digits::DEFAULT, p, t)
                        .unwrap()
                        .len(),
                    6
                );
                let r = p.seconds_remaining(t);
                assert!((1..=p.get()).contains(&r));
            }
        }
        assert_eq!(one.seconds_remaining(12345), 1);
        assert!(unix_time_now().unwrap() > 1_700_000_000);
    }

    proptest! {
        #[test]
        fn totp_is_hotp_of_time_step(
            key in proptest::collection::vec(any::<u8>(), 1..=128),
            t in any::<u64>(),
            p in 1u32..=300,
            d in 6u8..=8,
            alg in prop_oneof![Just(Algorithm::Sha1), Just(Algorithm::Sha256), Just(Algorithm::Sha512)],
        ) {
            let k = secret(&key);
            let p = Period::new(p).unwrap();
            let d = Digits::new(d).unwrap();
            let a = totp(&k, alg, d, p, t).unwrap();
            let b = hotp(&k, alg, d, t / u64::from(p.get())).unwrap();
            prop_assert_eq!(a.expose_secret(), b.expose_secret());
            prop_assert_eq!(a.len(), usize::from(d.get()));
            prop_assert!(a.expose_secret().bytes().all(|c| c.is_ascii_digit()));
            let s = steam_guard(&k, alg, p, t).unwrap();
            prop_assert_eq!(s.len(), STEAM_CODE_LEN);
            prop_assert!(s.expose_secret().bytes().all(|c| STEAM_ALPHABET.contains(&c)));
        }

        #[test]
        fn seconds_remaining_hits_next_boundary(t in any::<u32>(), p in 1u32..=300) {
            let p = Period::new(p).unwrap();
            let t = u64::from(t);
            let r = p.seconds_remaining(t);
            prop_assert!((1..=p.get()).contains(&r));
            prop_assert_eq!((t + u64::from(r)) % u64::from(p.get()), 0);
            prop_assert_eq!(p.time_step(t + u64::from(r)), p.time_step(t) + 1);
            prop_assert_eq!(p.time_step(t + u64::from(r) - 1), p.time_step(t));
        }
    }
}

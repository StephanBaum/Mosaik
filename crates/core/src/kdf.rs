//! Password preparation, Argon2id stretching and the two-secret key
//! derivation (CRYPTO_SPEC §5.1, §5.3, §5.4).
//!
//! ```text
//! MP'  = NFC(MP)                                        (§5.1)
//! P    = Argon2id(MP', account_salt, profile, len = 32) (§5.3)
//! S    = HKDF(ikm = SK, salt = account_id, info = ctx("secret-key"), 32)
//! R    = HKDF(ikm = P || S, salt = account_id, info = ctx("root"), 32)
//! KEK  = HKDF(ikm = R, salt = "", info = ctx("kek"), 32)
//! OPW  = HKDF(ikm = R, salt = "", info = ctx("opaque-password"), 32)
//! ```

use core::fmt;
use core::time::Duration;

use argon2::{Algorithm, Argon2, AssociatedData, Block, ParamsBuilder, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroize;

use crate::encoding::{CTX_PREFIX, Label, label};
use crate::error::{Error, Result};
use crate::ids::AccountId;
use crate::rng;
use crate::secret::{Kek, MasterPassword, OpaquePassword, SecretBytes, SecretKey, SecretVec};

/// The per-account Argon2id salt `account_salt = random(16)` (§5.3). It is
/// not secret: the server stores it in plaintext.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountSalt([u8; 16]);

impl AccountSalt {
    /// A fresh random salt.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn generate() -> Result<Self> {
        Ok(Self(rng::array()?))
    }

    /// Wraps a stored salt.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The raw salt bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Debug for AccountSalt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountSalt(")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

/// The fixed Argon2id parameter profiles of §5.3. Only these are accepted, so
/// a hostile server cannot make a client allocate more than 1 GiB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Argon2Profile {
    /// `0x01`: m = 128 MiB, t = 3, p = 4.
    Baseline = 0x01,
    /// `0x02` (default): m = 256 MiB, t = 3, p = 4.
    Standard = 0x02,
    /// `0x03`: m = 512 MiB, t = 3, p = 4.
    Strong = 0x03,
    /// `0x04`: m = 1 GiB, t = 4, p = 4.
    Max = 0x04,
}

impl Argon2Profile {
    /// All profiles, weakest first.
    pub const ALL: [Self; 4] = [Self::Baseline, Self::Standard, Self::Strong, Self::Max];

    /// The default profile.
    pub const DEFAULT: Self = Self::Standard;

    /// The profile ID stored per account.
    #[must_use]
    pub const fn id(self) -> u8 {
        self as u8
    }

    /// Looks up a stored profile ID.
    ///
    /// # Errors
    /// [`Error::UnknownProfile`] for any ID not in the §5.3 table.
    pub const fn from_id(id: u8) -> Result<Self> {
        match id {
            0x01 => Ok(Self::Baseline),
            0x02 => Ok(Self::Standard),
            0x03 => Ok(Self::Strong),
            0x04 => Ok(Self::Max),
            _ => Err(Error::UnknownProfile),
        }
    }

    /// Memory cost in KiB.
    #[must_use]
    pub const fn m_cost_kib(self) -> u32 {
        match self {
            Self::Baseline => 128 * 1024,
            Self::Standard => 256 * 1024,
            Self::Strong => 512 * 1024,
            Self::Max => 1024 * 1024,
        }
    }

    /// Time cost (passes).
    #[must_use]
    pub const fn t_cost(self) -> u32 {
        match self {
            Self::Baseline | Self::Standard | Self::Strong => 3,
            Self::Max => 4,
        }
    }

    /// Parallelism (lanes).
    #[must_use]
    pub const fn p_cost(self) -> u32 {
        4
    }

    pub(crate) const fn cost(self) -> Argon2Cost {
        Argon2Cost {
            m_kib: self.m_cost_kib(),
            t: self.t_cost(),
            p: self.p_cost(),
        }
    }
}

/// Raw Argon2id cost parameters. Public code only reaches these through an
/// [`Argon2Profile`]; tests use small values to stay fast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Argon2Cost {
    pub(crate) m_kib: u32,
    pub(crate) t: u32,
    pub(crate) p: u32,
}

/// Cheap parameters for unit tests only. Never used outside `cfg(test)`.
#[cfg(test)]
pub(crate) const TEST_COST: Argon2Cost = Argon2Cost {
    m_kib: 64,
    t: 1,
    p: 1,
};

/// The prepared master password `MP' = NFC(MP)` as UTF-8 (§5.1).
pub struct PreparedPassword(SecretVec);

impl fmt::Debug for PreparedPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// `MP' = NFC(MP)` (§5.1). No trimming, no case folding.
///
/// # Errors
/// [`Error::EmptyPassword`] for an empty password; [`Error::Allocation`] or
/// [`Error::SizeLimit`] if the buffer cannot be reserved.
pub fn prepare_password(password: &MasterPassword) -> Result<PreparedPassword> {
    let mp = password.expose_secret();
    if mp.is_empty() {
        return Err(Error::EmptyPassword);
    }
    // NFC expands UTF-8 by at most a factor of 3; reserving that up front
    // means the buffer never reallocates and leaves no stray copies.
    let cap = mp.len().checked_mul(3).ok_or(Error::SizeLimit)?;
    let mut out = SecretVec::with_capacity(cap)?;
    let mut buf = [0u8; 4];
    for c in mp.nfc() {
        out.as_mut_vec()
            .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }
    buf.zeroize();
    Ok(PreparedPassword(out))
}

/// The stretched password `P` (§5.3). It cannot be inspected; it only feeds
/// [`derive_keys`], which consumes and zeroizes it.
pub struct StretchedPassword(SecretBytes<32>);

impl fmt::Debug for StretchedPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

fn map_argon2_error(e: argon2::Error) -> Error {
    match e {
        argon2::Error::OutOfMemory => Error::Allocation,
        argon2::Error::PwdTooLong => Error::SizeLimit,
        _ => Error::Malformed,
    }
}

/// Argon2's working memory of `block_count` zeroed blocks, reserved
/// fallibly: a device that cannot provide it gets [`Error::Allocation`]
/// (which calibration steps down from) instead of an abort.
fn argon2_memory(block_count: usize) -> Result<Vec<Block>> {
    let mut blocks: Vec<Block> = Vec::new();
    blocks
        .try_reserve_exact(block_count)
        .map_err(|_| Error::Allocation)?;
    blocks.resize(block_count, Block::new());
    Ok(blocks)
}

/// Argon2id v1.3 into `out`, with a caller-owned memory buffer that is
/// reserved fallibly (allocation failure is an error, not an abort) and
/// zeroized afterwards. `secret` and `ad` are Argon2's optional `K` and `X`
/// inputs; Mosaic passes them empty, the RFC 9106 KAT does not.
pub(crate) fn argon2id(
    pwd: &[u8],
    salt: &[u8],
    secret: &[u8],
    ad: &[u8],
    cost: Argon2Cost,
    out: &mut [u8],
) -> Result<()> {
    let mut builder = ParamsBuilder::new();
    builder
        .m_cost(cost.m_kib)
        .t_cost(cost.t)
        .p_cost(cost.p)
        .output_len(out.len());
    if !ad.is_empty() {
        builder.data(AssociatedData::new(ad).map_err(map_argon2_error)?);
    }
    let params = builder.build().map_err(map_argon2_error)?;
    let block_count = params.block_count();
    let ctx = if secret.is_empty() {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    } else {
        Argon2::new_with_secret(secret, Algorithm::Argon2id, Version::V0x13, params)
            .map_err(map_argon2_error)?
    };

    let mut blocks = argon2_memory(block_count)?;
    let result = ctx
        .hash_password_into_with_memory(pwd, salt, out, blocks.as_mut_slice())
        .map_err(map_argon2_error);
    blocks.zeroize();
    result
}

pub(crate) fn stretch_with_cost(
    password: &PreparedPassword,
    salt: &AccountSalt,
    cost: Argon2Cost,
) -> Result<StretchedPassword> {
    let mut p = SecretBytes::<32>::zero();
    argon2id(
        password.0.expose_secret(),
        salt.as_bytes(),
        &[],
        &[],
        cost,
        p.expose_secret_mut(),
    )?;
    Ok(StretchedPassword(p))
}

/// `P = Argon2id(MP', account_salt, m, t, p, len = 32)` with the parameters
/// of `profile` (§5.3).
///
/// # Errors
/// [`Error::Allocation`] if the profile's memory cannot be reserved.
pub fn stretch(
    password: &PreparedPassword,
    salt: &AccountSalt,
    profile: Argon2Profile,
) -> Result<StretchedPassword> {
    stretch_with_cost(password, salt, profile.cost())
}

/// HKDF-SHA-256 (RFC 5869), Extract-then-Expand. `info` is the
/// concatenation of `info_parts`.
///
/// Extract is called explicitly so that the returned PRK can be zeroized:
/// `Hkdf::new` drops it without wiping, and for `R` the PRK is
/// `HMAC(account_id, P || S)`. The HMAC states inside `Hkdf` are wiped on
/// drop through `digest/zeroize`. What stays out of reach is upstream stack
/// residue: the `key ^ ipad/opad` block in `hmac`'s key setup and the
/// expanded output block in `expand_multi_info` (threat model R11).
pub(crate) fn hkdf_sha256(
    ikm: &[u8],
    salt: &[u8],
    info_parts: &[&[u8]],
    out: &mut [u8],
) -> Result<()> {
    let (mut prk, hkdf) = Hkdf::<Sha256>::extract(Some(salt), ikm);
    prk.as_mut_slice().zeroize();
    hkdf.expand_multi_info(info_parts, out)
        .map_err(|_| Error::SizeLimit)
}

/// The item ID of the conflict copy that holds the value one edit (`counter`
/// and `device`, its stamp) wrote under `key` in record `item_id`, after an
/// oversized split (CRYPTO_SPEC §10, ADR-0016):
///
/// `UUIDv8(HKDF(VK, "", ctx("conflict-copy-id") || item_id || u32(len(key))
/// || key || counter:u64 || device_id, 16))`
///
/// Every device that holds the vault key and moves the same value out
/// derives the same ID, so their copies are one record instead of
/// duplicates; without the key nobody can link a copy to its record (§12).
/// The UUID version nibble is 8 and the variant is RFC 9562's.
pub(crate) fn conflict_copy_id(
    vk: &[u8; 32],
    item_id: &[u8; 16],
    key: &[u8],
    counter: u64,
    device: &[u8; 16],
) -> Result<[u8; 16]> {
    let key_len = u32::try_from(key.len()).map_err(|_| Error::SizeLimit)?;
    let mut out = [0u8; 16];
    hkdf_sha256(
        vk,
        &[],
        &[
            CTX_PREFIX.as_bytes(),
            label::CONFLICT_COPY_ID.as_bytes(),
            item_id,
            &key_len.to_be_bytes(),
            key,
            &counter.to_be_bytes(),
            device,
        ],
        &mut out,
    )?;
    if let Some(b) = out.get_mut(6) {
        *b = (*b & 0x0F) | 0x80;
    }
    if let Some(b) = out.get_mut(8) {
        *b = (*b & 0x3F) | 0x80;
    }
    Ok(out)
}

/// `HKDF(ikm, salt, ctx(label), 32)` into a fresh secret.
fn hkdf_ctx(ikm: &[u8], salt: &[u8], label: Label) -> Result<SecretBytes<32>> {
    let mut out = SecretBytes::<32>::zero();
    hkdf_sha256(
        ikm,
        salt,
        &[CTX_PREFIX.as_bytes(), label.as_bytes()],
        out.expose_secret_mut(),
    )?;
    Ok(out)
}

/// The outputs of the two-secret derivation (§5.4).
#[derive(Debug)]
pub struct DerivedKeys {
    /// `KEK`: wraps the account key for password unlock (§6.1).
    pub kek: Kek,
    /// `OPW`: the OPAQUE password input (§9.1).
    pub opaque_password: OpaquePassword,
}

/// Derives `KEK` and `OPW` from `P`, `SK` and `account_id` (§5.4). `P`, `S`
/// and `R` are zeroized before this returns.
///
/// # Errors
/// Only on internal HKDF length errors, which cannot occur for 32-byte
/// outputs.
pub fn derive_keys(
    stretched: StretchedPassword,
    secret_key: &SecretKey,
    account_id: &AccountId,
) -> Result<DerivedKeys> {
    let salt = account_id.as_bytes();
    let s = hkdf_ctx(secret_key.expose_secret(), salt, label::SECRET_KEY)?;

    let mut ikm = SecretBytes::<64>::zero();
    let (left, right) = ikm.expose_secret_mut().split_at_mut(32);
    left.copy_from_slice(stretched.0.expose_secret());
    right.copy_from_slice(s.expose_secret());
    drop(stretched);
    drop(s);

    let r = hkdf_ctx(ikm.expose_secret(), salt, label::ROOT)?;
    drop(ikm);

    let kek = hkdf_ctx(r.expose_secret(), &[], label::KEK)?;
    let opw = hkdf_ctx(r.expose_secret(), &[], label::OPAQUE_PASSWORD)?;
    drop(r);

    Ok(DerivedKeys {
        kek: Kek(kek),
        opaque_password: OpaquePassword(opw),
    })
}

/// Calibration target: the strongest profile that derives in about this long
/// or less is chosen (§5.3).
pub const CALIBRATION_TARGET: Duration = Duration::from_secs(1);

/// `standard` is preferred whenever it takes this long or less (§5.3).
pub const STANDARD_CEILING: Duration = Duration::from_secs(2);

/// Chooses the account's profile at account creation (§5.3), using `measure`
/// to time one derivation under a profile.
///
/// Rules: the strongest profile that the device can allocate and that
/// derives within [`CALIBRATION_TARGET`]; `standard` whenever it takes at
/// most [`STANDARD_CEILING`]; never below `baseline`.
///
/// `measure` returns [`Error::Allocation`] when the device cannot allocate
/// the profile's memory; that is treated as "too expensive". Any other error
/// is passed through. Profiles are measured weakest first, starting at
/// `standard`, and stronger ones are only tried while the previous one met
/// the target.
///
/// # Errors
/// [`Error::Allocation`] if not even `baseline` can be allocated; any
/// non-allocation error from `measure`.
pub fn calibrate_with<F>(mut measure: F) -> Result<Argon2Profile>
where
    F: FnMut(Argon2Profile) -> Result<Duration>,
{
    match measure(Argon2Profile::Standard) {
        Ok(d) if d <= CALIBRATION_TARGET => {
            let mut best = Argon2Profile::Standard;
            for candidate in [Argon2Profile::Strong, Argon2Profile::Max] {
                match measure(candidate) {
                    Ok(d) if d <= CALIBRATION_TARGET => best = candidate,
                    Ok(_) | Err(Error::Allocation) => break,
                    Err(e) => return Err(e),
                }
            }
            Ok(best)
        }
        Ok(d) if d <= STANDARD_CEILING => Ok(Argon2Profile::Standard),
        // `standard` was allocatable, so the smaller `baseline` is too.
        Ok(_) => Ok(Argon2Profile::Baseline),
        Err(Error::Allocation) => {
            measure(Argon2Profile::Baseline)?;
            Ok(Argon2Profile::Baseline)
        }
        Err(e) => Err(e),
    }
}

/// Times one real derivation under `profile` with a random throwaway
/// password and salt, using the monotonic clock.
///
/// # Errors
/// [`Error::Allocation`] if the profile's memory cannot be reserved;
/// [`Error::Randomness`] if the CSPRNG fails.
pub fn measure_profile(profile: Argon2Profile) -> Result<Duration> {
    let pwd: [u8; 16] = rng::array()?;
    let salt = AccountSalt::generate()?;
    let mut out = [0u8; 32];
    let start = std::time::Instant::now();
    argon2id(&pwd, salt.as_bytes(), &[], &[], profile.cost(), &mut out)?;
    let elapsed = start.elapsed();
    out.zeroize();
    Ok(elapsed)
}

/// [`calibrate_with`] using the real clock ([`measure_profile`]). This runs
/// Argon2id with up to 1 GiB of memory and takes several seconds.
///
/// # Errors
/// See [`calibrate_with`].
pub fn calibrate() -> Result<Argon2Profile> {
    calibrate_with(measure_profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    /// RFC 9106 §5.3 Argon2id test vector (m = 32 KiB, t = 3, p = 4, with
    /// secret `K` and associated data `X`). Bytes as in the RFC text and in
    /// the `argon2` crate's `tests/kat.rs`.
    #[test]
    fn kat_rfc9106_argon2id() {
        let mut out = [0u8; 32];
        argon2id(
            &[0x01; 32],
            &[0x02; 16],
            &[0x03; 8],
            &[0x04; 12],
            Argon2Cost {
                m_kib: 32,
                t: 3,
                p: 4,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(
            out,
            hex!("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659")
        );
    }

    /// RFC 5869 Appendix A, test cases 1-3 (HKDF-SHA-256). Cross-checked
    /// against the `hkdf` crate's `tests/data/rfc5869_sha256.blb`.
    #[test]
    fn kat_rfc5869_hkdf_sha256() {
        struct Tc {
            ikm: Vec<u8>,
            salt: Vec<u8>,
            info: Vec<u8>,
            okm: Vec<u8>,
        }
        let range = |a: u8, b: u8| (a..=b).collect::<Vec<u8>>();
        let cases = [
            Tc {
                ikm: vec![0x0b; 22],
                salt: range(0x00, 0x0c),
                info: range(0xf0, 0xf9),
                okm: hex!(
                    "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
                    "34007208d5b887185865"
                )
                .to_vec(),
            },
            Tc {
                ikm: range(0x00, 0x4f),
                salt: range(0x60, 0xaf),
                info: range(0xb0, 0xff),
                okm: hex!(
                    "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c"
                    "59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71"
                    "cc30c58179ec3e87c14c01d5c1f3434f1d87"
                )
                .to_vec(),
            },
            Tc {
                ikm: vec![0x0b; 22],
                salt: vec![],
                info: vec![],
                okm: hex!(
                    "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d"
                    "9d201395faa4b61a96c8"
                )
                .to_vec(),
            },
        ];
        for tc in cases {
            let mut okm = vec![0u8; tc.okm.len()];
            hkdf_sha256(&tc.ikm, &tc.salt, &[&tc.info], &mut okm).unwrap();
            assert_eq!(okm, tc.okm);
            // Splitting `info` into parts must not change the result.
            let (a, b) = tc.info.split_at(tc.info.len() / 2);
            let mut okm2 = vec![0u8; tc.okm.len()];
            hkdf_sha256(&tc.ikm, &tc.salt, &[a, b], &mut okm2).unwrap();
            assert_eq!(okm2, tc.okm);
        }
        let mut too_long = vec![0u8; 255 * 32 + 1];
        assert_eq!(
            hkdf_sha256(b"k", b"", &[], &mut too_long),
            Err(Error::SizeLimit)
        );
    }

    #[test]
    fn profiles_match_spec_table() {
        let table = [
            (0x01, 131_072, 3, 4),
            (0x02, 262_144, 3, 4),
            (0x03, 524_288, 3, 4),
            (0x04, 1_048_576, 4, 4),
        ];
        for (p, (id, m, t, par)) in Argon2Profile::ALL.iter().zip(table) {
            assert_eq!(p.id(), id);
            assert_eq!(Argon2Profile::from_id(id).unwrap(), *p);
            assert_eq!((p.m_cost_kib(), p.t_cost(), p.p_cost()), (m, t, par));
        }
        for id in [0x00, 0x05, 0x80, 0xFF] {
            assert_eq!(Argon2Profile::from_id(id), Err(Error::UnknownProfile));
        }
        assert_eq!(Argon2Profile::DEFAULT, Argon2Profile::Standard);
    }

    #[test]
    fn prepare_applies_nfc_and_rejects_empty() {
        assert_eq!(
            prepare_password(&MasterPassword::new(String::new())).unwrap_err(),
            Error::EmptyPassword
        );
        // "é" as e + combining acute (NFD) becomes U+00E9 (NFC).
        let nfd = prepare_password(&MasterPassword::new("Caf\u{0065}\u{0301}".into())).unwrap();
        let nfc = prepare_password(&MasterPassword::new("Caf\u{00E9}".into())).unwrap();
        assert_eq!(nfd.0.expose_secret(), "Caf\u{00E9}".as_bytes());
        assert_eq!(nfd.0, nfc.0);
        // No trimming or case folding.
        let spaced = prepare_password(&MasterPassword::new(" Pw ".into())).unwrap();
        assert_eq!(spaced.0.expose_secret(), b" Pw ");
        assert_eq!(format!("{spaced:?}"), "[REDACTED]");
    }

    fn prep(s: &str) -> PreparedPassword {
        prepare_password(&MasterPassword::new(s.to_string())).unwrap()
    }

    /// NFC invariants of `prepare_password` for one input: canonically
    /// equivalent inputs agree, the output is a fixed point, it equals the
    /// library's NFC, and the 3x reservation was enough (no reallocation,
    /// so no stray unzeroized copy).
    fn check_nfc_invariants(s: &str) {
        let mut p = prep(s);
        let nfd: String = s.nfd().collect();
        assert_eq!(prep(&nfd).0, p.0, "NFD input of {s:?}");
        let out = String::from_utf8(p.0.expose_secret().to_vec()).unwrap();
        assert_eq!(prep(&out).0, p.0, "idempotence for {s:?}");
        assert_eq!(out, s.nfc().collect::<String>());
        assert!(p.0.len() <= 3 * s.len());
        assert_eq!(
            p.0.as_mut_vec().capacity(),
            3 * s.len(),
            "buffer reallocated for {s:?}"
        );
    }

    #[test]
    fn nfc_worst_case_expansions() {
        // U+1D160 MUSICAL SYMBOL EIGHTH NOTE: 4 UTF-8 bytes -> three 4-byte
        // code points under NFC (composition exclusion), the 3x maximum.
        assert_eq!(prep("\u{1D160}").0.len(), 12);
        for s in [
            "\u{1D160}",
            "\u{1D160}\u{1D160}x",
            "\u{FB2C}",
            "\u{0958}",
            "\u{0958}\u{FB2C}\u{1D160}",
            "e\u{0301}\u{0323}",
        ] {
            check_nfc_invariants(s);
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 256,
            ..proptest::prelude::ProptestConfig::default()
        })]

        #[test]
        fn nfc_invariants_hold(s in "\\PC{1,64}") {
            check_nfc_invariants(&s);
        }
    }

    #[test]
    fn nfc_equivalent_passwords_derive_equal_keys() {
        let salt = AccountSalt::from_bytes([5; 16]);
        let a = prepare_password(&MasterPassword::new("A\u{030A}ngstr\u{00F6}m".into())).unwrap();
        let b = prepare_password(&MasterPassword::new("\u{00C5}ngstro\u{0308}m".into())).unwrap();
        let pa = stretch_with_cost(&a, &salt, TEST_COST).unwrap();
        let pb = stretch_with_cost(&b, &salt, TEST_COST).unwrap();
        assert_eq!(pa.0, pb.0);
        assert_eq!(format!("{pa:?}"), "[REDACTED]");
    }

    #[test]
    fn derivation_separates_inputs() {
        let pw = prepare_password(&MasterPassword::new("correct horse".into())).unwrap();
        let salt = AccountSalt::from_bytes([1; 16]);
        let sk = SecretKey(SecretBytes::from_array(&mut [2; 16]));
        let id = AccountId::from_bytes([3; 16]).unwrap();
        let derive = |pw: &PreparedPassword, salt: &AccountSalt, sk: &SecretKey, id: &AccountId| {
            derive_keys(stretch_with_cost(pw, salt, TEST_COST).unwrap(), sk, id).unwrap()
        };
        let base = derive(&pw, &salt, &sk, &id);
        assert_ne!(
            base.kek.0.expose_secret(),
            base.opaque_password.expose_secret(),
            "KEK and OPW are independent outputs"
        );
        let again = derive(&pw, &salt, &sk, &id);
        assert_eq!(base.kek, again.kek);
        assert_eq!(base.opaque_password, again.opaque_password);

        let other_sk = SecretKey(SecretBytes::from_array(&mut [9; 16]));
        assert_ne!(derive(&pw, &salt, &other_sk, &id).kek, base.kek);
        let other_id = AccountId::from_bytes([4; 16]).unwrap();
        assert_ne!(derive(&pw, &salt, &sk, &other_id).kek, base.kek);
        let other_salt = AccountSalt::from_bytes([7; 16]);
        assert_ne!(derive(&pw, &other_salt, &sk, &id).kek, base.kek);
        let other_pw = prepare_password(&MasterPassword::new("correct horsf".into())).unwrap();
        assert_ne!(derive(&other_pw, &salt, &sk, &id).kek, base.kek);
    }

    /// Recomputes §5.4 step by step from the primitives, so that the
    /// composition (labels, salts, concatenation order) is pinned.
    #[test]
    fn derivation_matches_spec_composition() {
        let pw = prepare_password(&MasterPassword::new("pw".into())).unwrap();
        let salt = AccountSalt::from_bytes([1; 16]);
        let sk_bytes = [2u8; 16];
        let id = AccountId::from_bytes([3; 16]).unwrap();
        let mut p = [0u8; 32];
        argon2id(b"pw", &[1; 16], &[], &[], TEST_COST, &mut p).unwrap();
        let mut s = [0u8; 32];
        hkdf_sha256(&sk_bytes, &[3; 16], &[b"mosaic/v1/secret-key"], &mut s).unwrap();
        let mut ikm = p.to_vec();
        ikm.extend_from_slice(&s);
        let mut r = [0u8; 32];
        hkdf_sha256(&ikm, &[3; 16], &[b"mosaic/v1/root"], &mut r).unwrap();
        let mut kek = [0u8; 32];
        hkdf_sha256(&r, b"", &[b"mosaic/v1/kek"], &mut kek).unwrap();
        let mut opw = [0u8; 32];
        hkdf_sha256(&r, b"", &[b"mosaic/v1/opaque-password"], &mut opw).unwrap();

        let keys = derive_keys(
            stretch_with_cost(&pw, &salt, TEST_COST).unwrap(),
            &SecretKey(SecretBytes::from_array(&mut sk_bytes.clone())),
            &id,
        )
        .unwrap();
        assert_eq!(keys.kek.0.expose_secret(), &kek);
        assert_eq!(keys.opaque_password.expose_secret(), &opw);
    }

    #[test]
    fn argon2_rejects_bad_params() {
        let mut out = [0u8; 32];
        let bad = Argon2Cost {
            m_kib: 1,
            t: 1,
            p: 1,
        };
        assert_eq!(
            argon2id(b"pw", &[0; 16], &[], &[], bad, &mut out),
            Err(Error::Malformed)
        );
    }

    /// An unsatisfiable reservation is `Error::Allocation`, not an abort or
    /// a panic: `calibrate_with` relies on it to step down. `usize::MAX`
    /// blocks overflow the capacity computation on every platform, so this
    /// fails deterministically without asking the OS for memory.
    #[test]
    fn argon2_memory_failure_is_an_error() {
        assert_eq!(argon2_memory(usize::MAX).unwrap_err(), Error::Allocation);
        assert_eq!(
            argon2_memory(usize::MAX / core::mem::size_of::<Block>() + 1).unwrap_err(),
            Error::Allocation
        );
        let small = argon2_memory(8).unwrap();
        assert_eq!((small.len(), small.capacity()), (8, 8));
    }

    #[test]
    fn calibration_rules() {
        use Argon2Profile::{Baseline, Max, Standard, Strong};
        let ms = Duration::from_millis;
        let run = |table: &[(Argon2Profile, Result<Duration>)]| {
            let mut seen = Vec::new();
            let r = calibrate_with(|p| {
                seen.push(p);
                table
                    .iter()
                    .find(|(q, _)| *q == p)
                    .map_or(Err(Error::Allocation), |(_, d)| *d)
            });
            (r, seen)
        };
        // Fast device: everything within 1 s -> max.
        let (r, seen) = run(&[
            (Standard, Ok(ms(200))),
            (Strong, Ok(ms(400))),
            (Max, Ok(ms(900))),
        ]);
        assert_eq!(r, Ok(Max));
        assert_eq!(seen, vec![Standard, Strong, Max]);
        // Max too slow -> strong.
        let (r, _) = run(&[
            (Standard, Ok(ms(300))),
            (Strong, Ok(ms(700))),
            (Max, Ok(ms(1500))),
        ]);
        assert_eq!(r, Ok(Strong));
        // Max cannot be allocated -> strong.
        let (r, _) = run(&[(Standard, Ok(ms(300))), (Strong, Ok(ms(700)))]);
        assert_eq!(r, Ok(Strong));
        // Strong too slow -> standard, and max is never tried.
        let (r, seen) = run(&[(Standard, Ok(ms(600))), (Strong, Ok(ms(1200)))]);
        assert_eq!(r, Ok(Standard));
        assert_eq!(seen, vec![Standard, Strong]);
        // Exactly 1 s counts as within target.
        let (r, _) = run(&[(Standard, Ok(ms(1000))), (Strong, Ok(ms(2000)))]);
        assert_eq!(r, Ok(Standard));
        // Standard between 1 s and 2 s is still preferred.
        let (r, seen) = run(&[(Standard, Ok(ms(1800)))]);
        assert_eq!(r, Ok(Standard));
        assert_eq!(seen, vec![Standard]);
        let (r, _) = run(&[(Standard, Ok(ms(2000)))]);
        assert_eq!(r, Ok(Standard));
        // Standard slower than 2 s -> baseline (never below).
        let (r, seen) = run(&[(Standard, Ok(ms(2001)))]);
        assert_eq!(r, Ok(Baseline));
        assert_eq!(seen, vec![Standard]);
        // Standard not allocatable -> baseline, after checking it allocates.
        let (r, seen) = run(&[(Baseline, Ok(ms(5000)))]);
        assert_eq!(r, Ok(Baseline));
        assert_eq!(seen, vec![Standard, Baseline]);
        // Nothing allocatable -> error.
        let (r, _) = run(&[]);
        assert_eq!(r, Err(Error::Allocation));
        // Other errors propagate.
        let (r, _) = run(&[(Standard, Err(Error::Randomness))]);
        assert_eq!(r, Err(Error::Randomness));
        let (r, _) = run(&[(Standard, Ok(ms(10))), (Strong, Err(Error::Randomness))]);
        assert_eq!(r, Err(Error::Randomness));
    }

    #[test]
    fn salt_and_debug() {
        let s = AccountSalt::from_bytes([0xAB; 16]);
        assert_eq!(s.as_bytes(), &[0xAB; 16]);
        assert_eq!(
            format!("{s:?}"),
            format!("AccountSalt({})", "ab".repeat(16))
        );
        assert_ne!(
            AccountSalt::generate().unwrap(),
            AccountSalt::generate().unwrap()
        );
    }
}

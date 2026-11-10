//! The breach check, pure part (M7b, CRYPTO_SPEC §2 "Breach check",
//! ADR-0067): which passwords to look up, and how to read the answers of
//! the Have I Been Pwned (HIBP) range API.
//!
//! **No network here.** The transport (`mosaic_sync::http::pwned`) implements
//! [`RangeSource`]; this module never opens a socket, so tests use a fake.
//!
//! # Protocol (k-anonymity)
//!
//! 1. The password's UTF-8 bytes are hashed with SHA-1 (no normalization, as
//!    HIBP does), written as 40 uppercase hex characters in a zeroizing
//!    buffer. The first 5 characters are the [`RangePrefix`], the only part
//!    that ever leaves the device; the other 35 are the suffix.
//! 2. The transport asks for `GET /range/{prefix}` with `Add-Padding: true`
//!    and gets lines `SUFFIX:COUNT` for every hash with that prefix, plus
//!    padding lines with count 0.
//! 3. The wanted suffix is compared with **every** line in constant time
//!    ([`subtle`]); only "found" is kept, never the count. No hash is
//!    stored: [`BreachResult`] holds the item, the field, an opaque
//!    [`ValueMark`] and the answer.
//!
//! # Run order
//!
//! [`plan`] deduplicates prefixes (many passwords, one request) and skips
//! rows that are fresh. [`run`] shuffles the prefixes (Fisher-Yates with the
//! caller's CSPRNG), asks one at a time with a random 100 to 400 ms pause,
//! stops at once on lock/cancel (the plan is zeroized on drop), keeps the
//! results finished before an offline error, waits `Retry-After` (at most
//! 60 s) once after a 429 and then stops.
//!
//! The SHA-1 here is HIBP's identifier of a leaked password, not a Mosaic
//! security primitive: nothing relies on its collision resistance.

use core::sync::atomic::{AtomicBool, Ordering};
use std::collections::BTreeMap;

use mosaic_core::model::{Item, ItemType, SecretField, ValueMark};
use mosaic_core::{SecretString, Timestamp};
use sha1::{Digest, Sha1};
use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, Zeroizing};

use crate::error::{Error, Result};
use crate::{Candidate, DAY_MS, ItemKey};

/// The most bytes of one range response the parser reads (256 KiB; HIBP's
/// padded answers are about 40 KiB).
pub const MAX_BODY_LEN: usize = 256 * 1024;
/// Length of the SHA-1 suffix in hex characters.
pub const SUFFIX_LEN: usize = 35;
/// The longest `Retry-After` [`run`] waits for, in seconds.
pub const MAX_RETRY_AFTER_S: u32 = 60;
/// The wait after a 429 without a usable `Retry-After`, in seconds.
const DEFAULT_RETRY_AFTER_S: u32 = 10;
/// The random pause between requests, in milliseconds.
const PAUSE_MIN_MS: u32 = 100;
const PAUSE_SPAN_MS: u32 = 301;

/// A SHA-1 prefix: 5 uppercase hex characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RangePrefix([u8; 5]);

impl core::fmt::Debug for RangePrefix {
    /// A prefix is 20 bits of a password hash: never printed.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RangePrefix(..)")
    }
}

impl RangePrefix {
    /// Reads a prefix; `None` unless it is 5 hex characters (lowercase is
    /// uppercased).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() != 5 || !b.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        let mut out = [0u8; 5];
        out.copy_from_slice(b);
        out.make_ascii_uppercase();
        Some(Self(out))
    }

    /// The prefix as it goes into the request path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // The bytes are ASCII hex by construction.
        core::str::from_utf8(&self.0).unwrap_or("")
    }
}

/// Why a range request did not give an answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum RangeError {
    /// No connection (or the connection failed): stop, keep what is done.
    #[error("offline")]
    Offline,
    /// HTTP 429.
    #[error("rate limited")]
    RateLimited {
        /// The `Retry-After` header in seconds, if the server sent a number.
        retry_after_s: Option<u32>,
    },
    /// The answer is not what the API sends (status, size, a malformed
    /// line, TLS name).
    #[error("unexpected response")]
    Protocol,
    /// The user cancelled, or the vault locked.
    #[error("cancelled")]
    Cancelled,
}

/// Where range answers come from: the HTTPS client in production, a fake in
/// tests. One request at a time.
pub trait RangeSource {
    /// The body of `GET /range/{prefix}` (padded), at most
    /// [`MAX_BODY_LEN`] bytes. The body holds no secret of the user, but it
    /// is returned zeroizing so that it never lingers next to a plan.
    ///
    /// # Errors
    /// [`RangeError`].
    fn range(
        &mut self,
        prefix: &RangePrefix,
    ) -> core::result::Result<Zeroizing<Vec<u8>>, RangeError>;
}

/// Which secret of an item was checked. The numbers are the store's
/// `field` column.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BreachField {
    /// `login.password`.
    LoginPassword,
    /// `wifi.password`.
    WifiPassword,
}

impl BreachField {
    /// The store's code: 1 login password, 2 Wi-Fi password.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::LoginPassword => 1,
            Self::WifiPassword => 2,
        }
    }

    /// The field of a store code.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::LoginPassword),
            2 => Some(Self::WifiPassword),
            _ => None,
        }
    }

    /// The item field.
    #[must_use]
    pub const fn secret_field(self) -> SecretField {
        match self {
            Self::LoginPassword => SecretField::Password,
            Self::WifiPassword => SecretField::WifiPassword,
        }
    }
}

/// A stored answer for one secret of one item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BreachRecord {
    /// [`Item::secret_mark`] of the value that was checked.
    pub mark: ValueMark,
    /// When it was checked.
    pub checked_at: Timestamp,
    /// Whether the password was in the data set.
    pub found: bool,
}

/// What the store holds from earlier scans.
#[derive(Clone, Default, Debug)]
pub struct BreachStates(BTreeMap<(ItemKey, BreachField), BreachRecord>);

impl BreachStates {
    /// No rows.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Adds or replaces a row.
    pub fn insert(&mut self, key: ItemKey, field: BreachField, record: BreachRecord) {
        self.0.insert((key, field), record);
    }

    /// The row of one secret.
    #[must_use]
    pub fn get(&self, key: ItemKey, field: BreachField) -> Option<&BreachRecord> {
        self.0.get(&(key, field))
    }

    /// The items among `items` with a row that says "found" **for the
    /// current value**: a row whose mark differs from the item's
    /// [`Item::secret_mark`] (the password changed, a rotation was
    /// confirmed) does not count, so a confirmed rotation clears the finding.
    #[must_use]
    pub fn breached_items<'a, I>(&self, items: I) -> std::collections::BTreeSet<ItemKey>
    where
        I: IntoIterator<Item = Candidate<'a>>,
    {
        let mut out = std::collections::BTreeSet::new();
        for c in items {
            let key = ItemKey {
                vault_id: c.vault_id,
                item_id: c.item_id,
            };
            for field in [BreachField::LoginPassword, BreachField::WifiPassword] {
                let current = c.item.secret_mark(field.secret_field());
                if self
                    .get(key, field)
                    .is_some_and(|r| r.found && Some(r.mark) == current)
                {
                    out.insert(key);
                }
            }
        }
        out
    }
}

/// One secret to look up.
struct PlanEntry {
    key: ItemKey,
    field: BreachField,
    mark: ValueMark,
    /// 35 bytes in their own heap block: moving the entry (a `Vec` that
    /// grows) moves only the pointer and leaves no copy behind.
    suffix: Zeroizing<Vec<u8>>,
}

/// The passwords to look up, grouped by prefix. Holds hash suffixes: they
/// are zeroized when the plan is dropped (also on cancel).
pub struct BreachPlan {
    by_prefix: BTreeMap<RangePrefix, Vec<PlanEntry>>,
    checked_at: Timestamp,
}

impl BreachPlan {
    /// How many requests [`run`] makes.
    #[must_use]
    pub fn prefix_count(&self) -> usize {
        self.by_prefix.len()
    }

    /// How many secrets are looked up.
    #[must_use]
    pub fn secret_count(&self) -> usize {
        self.by_prefix.values().map(Vec::len).sum()
    }

    /// Whether there is nothing to look up.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_prefix.is_empty()
    }
}

impl core::fmt::Debug for BreachPlan {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BreachPlan")
            .field("prefixes", &self.prefix_count())
            .field("secrets", &self.secret_count())
            .finish_non_exhaustive()
    }
}

/// The answer for one secret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BreachResult {
    /// The item.
    pub key: ItemKey,
    /// Which secret.
    pub field: BreachField,
    /// The mark of the value that was checked.
    pub mark: ValueMark,
    /// When ([`plan`]'s `now`).
    pub checked_at: Timestamp,
    /// Whether the password is in the data set.
    pub found: bool,
}

/// The upper-case hex SHA-1 of `secret`, in a zeroizing buffer.
fn sha1_hex_upper(secret: &[u8]) -> Zeroizing<[u8; 40]> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut hasher = Sha1::new();
    hasher.update(secret);
    let mut digest = hasher.finalize();
    let mut out = Zeroizing::new([0u8; 40]);
    for (byte, [hi, lo]) in digest.iter().zip(out.as_chunks_mut::<2>().0.iter_mut()) {
        *hi = HEX.get(usize::from(byte >> 4)).copied().unwrap_or(b'0');
        *lo = HEX.get(usize::from(byte & 0x0f)).copied().unwrap_or(b'0');
    }
    AsMut::<[u8]>::as_mut(&mut digest).zeroize();
    out
}

/// The secret an item is checked for, with its field.
fn checked_secret(item: &Item) -> Result<Option<(BreachField, SecretString)>> {
    let field = match item.item_type() {
        Some(ItemType::Login) => BreachField::LoginPassword,
        Some(ItemType::Wifi) => BreachField::WifiPassword,
        _ => return Ok(None),
    };
    let secret = item
        .secret(field.secret_field())
        .map_err(|_| Error::Allocation)?
        .filter(|s| !s.expose_secret().is_empty());
    Ok(secret.map(|s| (field, s)))
}

/// Chooses what to look up: live logins and Wi-Fi items with a password
/// (outside the trash, no conflict copies) whose row in `states` is missing,
/// belongs to another value (the mark differs), or is `recheck_days` old.
/// Attempt values of a rotation are not checked. Prefixes are deduplicated.
///
/// # Errors
/// [`Error::Allocation`] if a secret cannot be copied into zeroizing memory.
pub fn plan<'a, I>(
    items: I,
    states: &BreachStates,
    now: Timestamp,
    recheck_days: u32,
) -> Result<BreachPlan>
where
    I: IntoIterator<Item = Candidate<'a>>,
{
    let max_age = i64::from(recheck_days).saturating_mul(DAY_MS);
    let mut by_prefix: BTreeMap<RangePrefix, Vec<PlanEntry>> = BTreeMap::new();
    for c in items {
        let item = c.item;
        if item.is_deleted() || item.trashed_at().is_some() || item.conflict_copy_of().is_some() {
            continue;
        }
        let Some((field, secret)) = checked_secret(item)? else {
            continue;
        };
        let Some(mark) = item.secret_mark(field.secret_field()) else {
            continue;
        };
        let key = ItemKey {
            vault_id: c.vault_id,
            item_id: c.item_id,
        };
        let fresh = states.get(key, field).is_some_and(|r| {
            let age = now
                .as_unix_millis()
                .saturating_sub(r.checked_at.as_unix_millis());
            // A row from the future (a clock that jumped back) is stale.
            r.mark == mark && (0..max_age).contains(&age)
        });
        if fresh {
            continue;
        }
        let hex = sha1_hex_upper(secret.expose_secret().as_bytes());
        let mut prefix = [0u8; 5];
        let (p, s) = hex.split_at(5);
        prefix.copy_from_slice(p);
        let suffix = Zeroizing::new(s.to_vec());
        by_prefix
            .entry(RangePrefix(prefix))
            .or_default()
            .push(PlanEntry {
                key,
                field,
                mark,
                suffix,
            });
    }
    Ok(BreachPlan {
        by_prefix,
        checked_at: now,
    })
}

/// Parses one range response and answers, for each wanted suffix, whether a
/// line with that suffix and a **non-zero** count is in it (count 0 is
/// padding). The count is read and dropped.
///
/// Lines are `SUFFIX:COUNT`, separated by LF or CRLF, a trailing line break
/// and empty lines are fine; a suffix is 35 hex characters (either case) and
/// the count 1 to 20 ASCII digits. The comparison with every wanted suffix
/// touches every line and takes the same time whether or not it matches.
///
/// The request carries `Add-Padding: true`, so every genuine answer has at
/// least one padding line (count 0). A body without one is an empty or cut-off
/// answer, not "nothing found". The transport must **reject** a body above
/// [`MAX_BODY_LEN`], never truncate it (a cut body could hide the match).
///
/// # Errors
/// [`RangeError::Protocol`] if the body is longer than [`MAX_BODY_LEN`], has no
/// padding line, or any line is not of that form: nothing from such a body is
/// trusted.
pub fn parse_range(
    body: &[u8],
    wanted: &[[u8; SUFFIX_LEN]],
) -> core::result::Result<Vec<bool>, RangeError> {
    if body.len() > MAX_BODY_LEN {
        return Err(RangeError::Protocol);
    }
    let mut found: Vec<Choice> = vec![Choice::from(0); wanted.len()];
    let mut padding = false;
    for raw in body.split(|&b| b == b'\n') {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        if line.is_empty() {
            continue;
        }
        let (suffix, count) = line
            .split_at_checked(SUFFIX_LEN)
            .ok_or(RangeError::Protocol)?;
        let count = count.strip_prefix(b":").ok_or(RangeError::Protocol)?;
        if !suffix.iter().all(u8::is_ascii_hexdigit)
            || count.is_empty()
            || count.len() > 20
            || !count.iter().all(u8::is_ascii_digit)
        {
            return Err(RangeError::Protocol);
        }
        let mut upper = [0u8; SUFFIX_LEN];
        upper.copy_from_slice(suffix);
        upper.make_ascii_uppercase();
        let nonzero = count.iter().any(|&d| d != b'0');
        padding |= !nonzero;
        let real = Choice::from(u8::from(nonzero));
        for (hit, want) in found.iter_mut().zip(wanted) {
            *hit |= want.ct_eq(&upper) & real;
        }
    }
    if !padding {
        return Err(RangeError::Protocol);
    }
    Ok(found.into_iter().map(bool::from).collect())
}

/// What [`run`] needs from its caller.
pub struct RunHooks<'a> {
    /// A uniform number below the argument (the generator's CSPRNG).
    pub rng: &'a mut dyn FnMut(u32) -> u32,
    /// Waits this many milliseconds (the random pause between requests and
    /// the `Retry-After` wait) **on the cancel flag**, not in a plain sleep,
    /// and returns `false` as soon as it is set: [`run`] then stops at once
    /// and drops the plan, which holds the hash suffixes. A test records the
    /// time instead and returns `true`.
    pub pause: &'a mut dyn FnMut(u32) -> bool,
    /// Called with `(done, total)` prefixes after each answer.
    pub progress: &'a mut dyn FnMut(u32, u32),
    /// Set by cancel and by lock: [`run`] stops before the next request.
    pub cancel: &'a AtomicBool,
}

/// Looks up every prefix of `plan` through `source` (see the module
/// documentation for the order and the pacing). Returns the answers that
/// were finished and the error that stopped the run, if any. A prefix whose
/// answer is malformed is left unchecked (no result for its secrets) and the
/// run goes on; its [`RangeError::Protocol`] is returned at the end if
/// nothing else stopped the run.
pub fn run(
    plan: BreachPlan,
    source: &mut dyn RangeSource,
    hooks: &mut RunHooks<'_>,
) -> (Vec<BreachResult>, Option<RangeError>) {
    let checked_at = plan.checked_at;
    let mut groups: Vec<(RangePrefix, Vec<PlanEntry>)> = plan.by_prefix.into_iter().collect();
    // Fisher-Yates: the order of the requests says nothing about the vault.
    for i in (1..groups.len()).rev() {
        let bound = u32::try_from(i.saturating_add(1)).unwrap_or(u32::MAX);
        let j = usize::try_from((hooks.rng)(bound)).unwrap_or(0).min(i);
        groups.swap(i, j);
    }
    let total = u32::try_from(groups.len()).unwrap_or(u32::MAX);
    let mut results = Vec::new();
    let mut protocol = false;
    // One `Retry-After` wait per run, not per prefix.
    let mut retried = false;
    for (n, (prefix, entries)) in groups.into_iter().enumerate() {
        if hooks.cancel.load(Ordering::Acquire) {
            return (results, Some(RangeError::Cancelled));
        }
        if n > 0 {
            let ms = PAUSE_MIN_MS.saturating_add((hooks.rng)(PAUSE_SPAN_MS).min(PAUSE_SPAN_MS - 1));
            if !(hooks.pause)(ms) {
                return (results, Some(RangeError::Cancelled));
            }
        }
        let body = match fetch(source, prefix, hooks, &mut retried) {
            Ok(body) => body,
            Err(e) => return (results, Some(e)),
        };
        if hooks.cancel.load(Ordering::Acquire) {
            return (results, Some(RangeError::Cancelled));
        }
        let wanted: Zeroizing<Vec<[u8; SUFFIX_LEN]>> = Zeroizing::new(
            entries
                .iter()
                .filter_map(|e| suffix_array(&e.suffix))
                .collect(),
        );
        match parse_range(&body, &wanted) {
            Ok(hits) => {
                results.extend(entries.iter().zip(hits).map(|(e, found)| BreachResult {
                    key: e.key,
                    field: e.field,
                    mark: e.mark,
                    checked_at,
                    found,
                }));
            }
            Err(_) => protocol = true,
        }
        (hooks.progress)(
            u32::try_from(n.saturating_add(1)).unwrap_or(u32::MAX),
            total,
        );
    }
    (results, protocol.then_some(RangeError::Protocol))
}

/// The suffix as an array (always 35 bytes).
fn suffix_array(suffix: &[u8]) -> Option<[u8; SUFFIX_LEN]> {
    <[u8; SUFFIX_LEN]>::try_from(suffix).ok()
}

/// One request, with the single `Retry-After` retry of the whole run
/// (`retried`).
fn fetch(
    source: &mut dyn RangeSource,
    prefix: RangePrefix,
    hooks: &mut RunHooks<'_>,
    retried: &mut bool,
) -> core::result::Result<Zeroizing<Vec<u8>>, RangeError> {
    match source.range(&prefix) {
        Err(RangeError::RateLimited { retry_after_s }) => {
            let wait = retry_after_s.unwrap_or(DEFAULT_RETRY_AFTER_S);
            if *retried || wait > MAX_RETRY_AFTER_S || hooks.cancel.load(Ordering::Acquire) {
                return Err(RangeError::RateLimited { retry_after_s });
            }
            *retried = true;
            if !(hooks.pause)(wait.saturating_mul(1000)) || hooks.cancel.load(Ordering::Acquire) {
                return Err(RangeError::Cancelled);
            }
            source.range(&prefix)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::assert_is_empty, reason = "tests")]

    use std::collections::{BTreeSet, VecDeque};

    use mosaic_core::{DeviceId, ItemId, VaultId};

    use super::*;

    const DAY: i64 = DAY_MS;
    const NOW: i64 = 1_800_000_000_000;
    const PASSWORD_SUFFIX: &str = "1E4C9B93F3F0682250B6CF8331B7EE68FD8";
    /// The padding line every genuine (padded) answer has.
    const PAD: &str = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:0\r\n";

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_unix_millis(ms)
    }

    fn suffix(s: &str) -> [u8; SUFFIX_LEN] {
        let mut out = [0u8; SUFFIX_LEN];
        out.copy_from_slice(s.as_bytes());
        out
    }

    fn login(password: &str, device: DeviceId) -> Item {
        let mut item = Item::new(&ItemType::Login, device, at(NOW - 100 * DAY)).unwrap();
        item.edit(device, at(NOW - 100 * DAY))
            .set_secret(SecretField::Password, password)
            .unwrap();
        item
    }

    struct Fixture {
        vault: VaultId,
        device: DeviceId,
        items: Vec<(ItemId, Item)>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                vault: VaultId::new_v7().unwrap(),
                device: DeviceId::new_v7().unwrap(),
                items: Vec::new(),
            }
        }

        fn add(&mut self, password: &str) -> ItemKey {
            let id = ItemId::new_v7().unwrap();
            self.items.push((id, login(password, self.device)));
            ItemKey {
                vault_id: self.vault,
                item_id: id,
            }
        }

        fn candidates(&self) -> Vec<Candidate<'_>> {
            self.items
                .iter()
                .map(|(id, item)| Candidate {
                    vault_id: self.vault,
                    item_id: *id,
                    item,
                })
                .collect()
        }

        fn plan(&self, states: &BreachStates) -> BreachPlan {
            plan(self.candidates(), states, at(NOW), 30).unwrap()
        }

        fn mark(&self, key: ItemKey) -> ValueMark {
            let (_, item) = self
                .items
                .iter()
                .find(|(id, _)| *id == key.item_id)
                .unwrap();
            item.secret_mark(SecretField::Password).unwrap()
        }
    }

    /// Answers from a script, records the prefixes in request order.
    struct Fake {
        asked: Vec<String>,
        answers: VecDeque<core::result::Result<Vec<u8>, RangeError>>,
        default: Vec<u8>,
        after: Option<(usize, &'static AtomicBool)>,
    }

    impl Fake {
        fn new(default: &str) -> Self {
            Self {
                asked: Vec::new(),
                answers: VecDeque::new(),
                default: format!("{default}{PAD}").into_bytes(),
                after: None,
            }
        }
    }

    impl RangeSource for Fake {
        fn range(
            &mut self,
            prefix: &RangePrefix,
        ) -> core::result::Result<Zeroizing<Vec<u8>>, RangeError> {
            self.asked.push(prefix.as_str().to_owned());
            if let Some((n, flag)) = self.after
                && self.asked.len() == n
            {
                flag.store(true, Ordering::Release);
            }
            match self.answers.pop_front() {
                Some(r) => r.map(Zeroizing::new),
                None => Ok(Zeroizing::new(self.default.clone())),
            }
        }
    }

    struct Env {
        pauses: Vec<u32>,
        progress: Vec<(u32, u32)>,
        seed: u32,
        cancel: AtomicBool,
        pause_ok: bool,
    }

    impl Env {
        fn new() -> Self {
            Self {
                pauses: Vec::new(),
                progress: Vec::new(),
                seed: 12345,
                cancel: AtomicBool::new(false),
                pause_ok: true,
            }
        }

        fn run(
            &mut self,
            plan: BreachPlan,
            source: &mut Fake,
        ) -> (Vec<BreachResult>, Option<RangeError>) {
            let mut seed = self.seed;
            let mut rng = move |bound: u32| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 8) % bound.max(1)
            };
            let pauses = &mut self.pauses;
            let progress = &mut self.progress;
            let pause_ok = self.pause_ok;
            let mut pause = |ms: u32| {
                pauses.push(ms);
                pause_ok
            };
            let mut prog = |d: u32, t: u32| progress.push((d, t));
            let mut hooks = RunHooks {
                rng: &mut rng,
                pause: &mut pause,
                progress: &mut prog,
                cancel: &self.cancel,
            };
            run(plan, source, &mut hooks)
        }
    }

    #[test]
    fn sha1_password_prefix_5baa6() {
        let hex = sha1_hex_upper(b"password");
        assert_eq!(
            core::str::from_utf8(&*hex).unwrap(),
            "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8"
        );
        let mut fx = Fixture::new();
        fx.add("password");
        let p = fx.plan(&BreachStates::new());
        assert_eq!(p.prefix_count(), 1);
        assert_eq!(p.by_prefix.keys().next().unwrap().as_str(), "5BAA6");
        let entry = &p.by_prefix.values().next().unwrap()[0];
        assert_eq!(&entry.suffix[..], PASSWORD_SUFFIX.as_bytes());
        assert_eq!(RangePrefix::parse("5baa6"), RangePrefix::parse("5BAA6"));
        assert_eq!(RangePrefix::parse("5BAA"), None);
        assert_eq!(RangePrefix::parse("5BAAG"), None);
    }

    #[test]
    fn suffix_found_and_count_dropped() {
        let other = "0000000000000000000000000000000000A";
        let body = format!(
            "{other}:7\r\n{}:37615\r\n{PAD}",
            PASSWORD_SUFFIX.to_ascii_lowercase()
        );
        let hits = parse_range(
            body.as_bytes(),
            &[
                suffix(PASSWORD_SUFFIX),
                suffix("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"),
            ],
        )
        .unwrap();
        assert_eq!(hits, [true, false]);
        // The result type cannot carry a count.
        let mut fx = Fixture::new();
        let key = fx.add("password");
        let mut src = Fake::new(&body);
        let (results, err) = Env::new().run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!(err, None);
        assert_eq!(results.len(), 1);
        assert!(results[0].found);
        assert_eq!(results[0].key, key);
        assert_eq!(results[0].mark, fx.mark(key));
        assert_eq!(results[0].checked_at, at(NOW));
    }

    #[test]
    fn padding_lines_ignored() {
        let body = format!("{PASSWORD_SUFFIX}:0\r\nABCDEF0123456789ABCDEF0123456789ABC:0\r\n");
        assert_eq!(
            parse_range(body.as_bytes(), &[suffix(PASSWORD_SUFFIX)]).unwrap(),
            [false]
        );
        let body = format!("{PASSWORD_SUFFIX}:00\n");
        assert_eq!(
            parse_range(body.as_bytes(), &[suffix(PASSWORD_SUFFIX)]).unwrap(),
            [false]
        );
        let body = format!("{PASSWORD_SUFFIX}:0\n{PASSWORD_SUFFIX}:3\n");
        assert_eq!(
            parse_range(body.as_bytes(), &[suffix(PASSWORD_SUFFIX)]).unwrap(),
            [true]
        );
    }

    #[test]
    fn malformed_line_marks_prefix_unchecked() {
        let w = [suffix(PASSWORD_SUFFIX)];
        for bad in [
            "garbage\r\n",
            "1E4C9B93F3F0682250B6CF8331B7EE68FD8\r\n",
            "1E4C9B93F3F0682250B6CF8331B7EE68FD8:\r\n",
            "1E4C9B93F3F0682250B6CF8331B7EE68FD8:12x\r\n",
            "1E4C9B93F3F0682250B6CF8331B7EE68FD:5\r\n",
            "ZE4C9B93F3F0682250B6CF8331B7EE68FD8:5\r\n",
            "1E4C9B93F3F0682250B6CF8331B7EE68FD8:111111111111111111111\r\n",
        ] {
            assert_eq!(
                parse_range(bad.as_bytes(), &w),
                Err(RangeError::Protocol),
                "{bad:?}"
            );
        }
        // One bad line spoils the body even after a good match.
        let body = format!("{PASSWORD_SUFFIX}:5\r\nnope\r\n");
        assert_eq!(parse_range(body.as_bytes(), &w), Err(RangeError::Protocol));
        // run(): the secret gets no result, the error comes back at the end.
        let mut fx = Fixture::new();
        fx.add("password");
        let mut src = Fake::new("nope\r\n");
        let (results, err) = Env::new().run(fx.plan(&BreachStates::new()), &mut src);
        assert!(results.is_empty());
        assert_eq!(err, Some(RangeError::Protocol));
    }

    #[test]
    fn oversize_body_rejected() {
        let w = [suffix(PASSWORD_SUFFIX)];
        let mut ok = PAD.as_bytes().to_vec();
        ok.resize(MAX_BODY_LEN, b'\n');
        assert_eq!(parse_range(&ok, &w).unwrap(), [false]);
        ok.push(b'\n');
        assert_eq!(parse_range(&ok, &w), Err(RangeError::Protocol));
    }

    /// An empty or cut-off 200 body is not "nothing found".
    #[test]
    fn empty_or_unpadded_body_is_protocol() {
        let w = [suffix(PASSWORD_SUFFIX)];
        assert_eq!(parse_range(b"", &w), Err(RangeError::Protocol));
        assert_eq!(parse_range(b"\r\n\r\n", &w), Err(RangeError::Protocol));
        let cut = format!("{}:3\r\n", "A".repeat(35));
        assert_eq!(parse_range(cut.as_bytes(), &w), Err(RangeError::Protocol));
        // run(): the secret gets no result and the run reports Protocol.
        let mut fx = Fixture::new();
        fx.add("password");
        let mut src = Fake::new("");
        src.answers.push_back(Ok(Vec::new()));
        let (results, err) = Env::new().run(fx.plan(&BreachStates::new()), &mut src);
        assert!(results.is_empty());
        assert_eq!(err, Some(RangeError::Protocol));
    }

    #[test]
    fn crlf_and_lf_accepted() {
        let w = [suffix(PASSWORD_SUFFIX)];
        for sep in ["\r\n", "\n"] {
            let body = format!("{}:1{sep}{PASSWORD_SUFFIX}:2{sep}{PAD}", "A".repeat(35));
            assert_eq!(parse_range(body.as_bytes(), &w).unwrap(), [true], "{sep:?}");
        }
        let no_trailing = format!("{PAD}{PASSWORD_SUFFIX}:2");
        assert_eq!(parse_range(no_trailing.as_bytes(), &w).unwrap(), [true]);
    }

    #[test]
    fn plan_skips_fresh_rows_and_rechecks_stale_marks() {
        let mut fx = Fixture::new();
        let fresh = fx.add("alpha-1");
        let old = fx.add("alpha-2");
        let changed = fx.add("alpha-3");
        let missing = fx.add("alpha-4");
        let future = fx.add("alpha-5");
        let mut states = BreachStates::new();
        let rec = |mark, days_ago: i64| BreachRecord {
            mark,
            checked_at: at(NOW - days_ago * DAY),
            found: false,
        };
        states.insert(fresh, BreachField::LoginPassword, rec(fx.mark(fresh), 3));
        states.insert(old, BreachField::LoginPassword, rec(fx.mark(old), 31));
        // A row from the future is stale.
        states.insert(future, BreachField::LoginPassword, rec(fx.mark(future), -2));
        // A row for the value before the password changed.
        let before = fx.mark(changed);
        states.insert(changed, BreachField::LoginPassword, rec(before, 1));
        let device = fx.device;
        let (_, item) = fx
            .items
            .iter_mut()
            .find(|(id, _)| *id == changed.item_id)
            .unwrap();
        item.edit(device, at(NOW - DAY))
            .set_secret(SecretField::Password, "alpha-3b")
            .unwrap();
        assert_ne!(fx.mark(changed), before);
        let p = fx.plan(&states);
        let keys: BTreeSet<ItemKey> = p.by_prefix.values().flatten().map(|e| e.key).collect();
        assert_eq!(keys, BTreeSet::from([old, changed, missing, future]));
        assert_eq!(p.secret_count(), 4);
    }

    #[test]
    fn plan_skips_what_is_not_checked() {
        let mut fx = Fixture::new();
        fx.add("");
        let trashed = fx.add("x-trashed");
        let note_id = ItemId::new_v7().unwrap();
        let mut note = Item::new(&ItemType::SecureNote, fx.device, at(NOW)).unwrap();
        note.edit(fx.device, at(NOW)).set_title("n").unwrap();
        fx.items.push((note_id, note));
        let (_, t) = fx
            .items
            .iter_mut()
            .find(|(id, _)| *id == trashed.item_id)
            .unwrap();
        t.edit(fx.device, at(NOW)).trash().unwrap();
        // A pending rotation value is not checked, the password is.
        let live = fx.add("live-pw");
        let (_, l) = fx
            .items
            .iter_mut()
            .find(|(id, _)| *id == live.item_id)
            .unwrap();
        l.edit(fx.device, at(NOW))
            .begin_rotation("pending-pw", None)
            .unwrap();
        let p = fx.plan(&BreachStates::new());
        assert_eq!(p.secret_count(), 1);
        let hex = sha1_hex_upper(b"live-pw");
        assert_eq!(
            p.by_prefix.keys().next().unwrap().as_str().as_bytes(),
            &hex[..5]
        );
    }

    #[test]
    fn plan_dedupes_prefixes() {
        let mut fx = Fixture::new();
        fx.add("same-pw");
        fx.add("same-pw");
        fx.add("other-pw");
        let p = fx.plan(&BreachStates::new());
        assert_eq!(p.secret_count(), 3);
        assert_eq!(p.prefix_count(), 2);
    }

    #[test]
    fn run_shuffles_prefix_order() {
        let mut fx = Fixture::new();
        for i in 0..12 {
            fx.add(&format!("pw-{i}"));
        }
        let sorted: Vec<String> = fx
            .plan(&BreachStates::new())
            .by_prefix
            .keys()
            .map(|p| p.as_str().to_owned())
            .collect();
        let mut orders = BTreeSet::new();
        for seed in 1..6 {
            let mut env = Env::new();
            env.seed = seed;
            let mut src = Fake::new("");
            let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
            assert_eq!((results.len(), err), (12, None));
            let mut asked = src.asked.clone();
            asked.sort();
            assert_eq!(asked, sorted);
            // Pauses between requests only, each 100 to 400 ms.
            assert_eq!(env.pauses.len(), 11);
            assert!(env.pauses.iter().all(|&p| (100..=400).contains(&p)));
            assert_eq!(env.progress.last(), Some(&(12, 12)));
            orders.insert(src.asked);
        }
        assert!(orders.len() > 1, "the request order depends on the rng");
    }

    #[test]
    fn offline_keeps_finished_results() {
        let mut fx = Fixture::new();
        for i in 0..6 {
            fx.add(&format!("pw-{i}"));
        }
        let mut src = Fake::new("");
        src.answers.extend([
            Ok(PAD.as_bytes().to_vec()),
            Ok(PAD.as_bytes().to_vec()),
            Err(RangeError::Offline),
        ]);
        let (results, err) = Env::new().run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!(err, Some(RangeError::Offline));
        assert_eq!(results.len(), 2);
        assert_eq!(src.asked.len(), 3);
    }

    #[test]
    fn rate_limited_waits_once_then_stops() {
        let mut fx = Fixture::new();
        fx.add("pw-a");
        fx.add("pw-b");
        let limited = RangeError::RateLimited {
            retry_after_s: Some(7),
        };
        // One 429, then the retry works.
        let mut src = Fake::new("");
        src.answers.push_back(Err(limited));
        let mut env = Env::new();
        let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!((results.len(), err), (2, None));
        assert!(env.pauses.contains(&7000));
        // Two in a row: wait once, then stop with the error.
        let mut src = Fake::new("");
        src.answers.extend([Err(limited), Err(limited)]);
        let mut env = Env::new();
        let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        assert!(results.is_empty());
        assert_eq!(err, Some(limited));
        assert_eq!(env.pauses, [7000]);
        assert_eq!(src.asked.len(), 2);
        // A Retry-After above the cap is not waited for.
        let long = RangeError::RateLimited {
            retry_after_s: Some(600),
        };
        let mut src = Fake::new("");
        src.answers.push_back(Err(long));
        let mut env = Env::new();
        let (_, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!(err, Some(long));
        assert!(env.pauses.is_empty());
        assert_eq!(src.asked.len(), 1);
    }

    #[test]
    fn cancel_stops_and_zeroizes() {
        static FLAG: AtomicBool = AtomicBool::new(false);
        let mut fx = Fixture::new();
        for i in 0..5 {
            fx.add(&format!("pw-{i}"));
        }
        let mut src = Fake::new("");
        src.after = Some((2, &FLAG));
        let mut env = Env::new();
        let plan = fx.plan(&BreachStates::new());
        let cancel_view: &AtomicBool = &FLAG;
        let mut seed = 7u32;
        let mut rng = move |b: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % b.max(1)
        };
        let mut pause = |_: u32| true;
        let mut prog = |d: u32, t: u32| env.progress.push((d, t));
        let mut hooks = RunHooks {
            rng: &mut rng,
            pause: &mut pause,
            progress: &mut prog,
            cancel: cancel_view,
        };
        let (results, err) = run(plan, &mut src, &mut hooks);
        // The flag was set during the second request: that answer is not
        // used and no third request is made. The plan was moved into `run`
        // and dropped there (its suffixes are `Zeroizing`).
        assert_eq!(err, Some(RangeError::Cancelled));
        assert_eq!(src.asked.len(), 2);
        assert!(results.len() <= 1);
    }

    /// A pause that sees the cancel flag stops the run at once.
    #[test]
    fn cancel_during_a_pause_stops_at_once() {
        let mut fx = Fixture::new();
        for i in 0..4 {
            fx.add(&format!("pw-{i}"));
        }
        let mut src = Fake::new("");
        let mut env = Env::new();
        env.pause_ok = false;
        let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        // The first request is made, the pause before the second says stop.
        assert_eq!(err, Some(RangeError::Cancelled));
        assert_eq!(src.asked.len(), 1);
        assert_eq!(results.len(), 1);
        assert_eq!(env.pauses.len(), 1);
        // The same during the Retry-After wait.
        let mut src = Fake::new("");
        src.answers.push_back(Err(RangeError::RateLimited {
            retry_after_s: Some(30),
        }));
        let mut env = Env::new();
        env.pause_ok = false;
        let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!(err, Some(RangeError::Cancelled));
        assert!(results.is_empty());
        assert_eq!(src.asked.len(), 1);
        assert_eq!(env.pauses, [30_000]);
    }

    /// The 429 retry is spent once per run, not once per prefix.
    #[test]
    fn retry_budget_is_per_run() {
        let mut fx = Fixture::new();
        for i in 0..3 {
            fx.add(&format!("pw-{i}"));
        }
        let limited = RangeError::RateLimited {
            retry_after_s: Some(1),
        };
        let mut src = Fake::new("");
        // First prefix: 429 then fine; second prefix: 429 again.
        src.answers
            .extend([Err(limited), Ok(PAD.as_bytes().to_vec()), Err(limited)]);
        let mut env = Env::new();
        let (results, err) = env.run(fx.plan(&BreachStates::new()), &mut src);
        assert_eq!(err, Some(limited));
        assert_eq!(results.len(), 1);
        assert_eq!(env.pauses.iter().filter(|&&p| p == 1000).count(), 1);
        assert_eq!(src.asked.len(), 3);
    }

    #[test]
    fn breached_items_follow_the_current_mark() {
        let mut fx = Fixture::new();
        let hit = fx.add("hit-pw");
        let other = fx.add("other-pw");
        let mut states = BreachStates::new();
        let rec = |mark| BreachRecord {
            mark,
            checked_at: at(NOW),
            found: true,
        };
        states.insert(hit, BreachField::LoginPassword, rec(fx.mark(hit)));
        let mut stale = rec(fx.mark(other));
        stale.found = false;
        states.insert(other, BreachField::LoginPassword, stale);
        assert_eq!(
            states.breached_items(fx.candidates()),
            BTreeSet::from([hit])
        );
        // A confirmed rotation changes the mark: the finding goes away.
        let device = fx.device;
        let (_, item) = fx
            .items
            .iter_mut()
            .find(|(id, _)| *id == hit.item_id)
            .unwrap();
        let id = item
            .edit(device, at(NOW))
            .begin_rotation("new-pw", None)
            .unwrap();
        assert_eq!(
            states.breached_items(fx.candidates()),
            BTreeSet::from([hit])
        );
        let (_, item) = fx
            .items
            .iter_mut()
            .find(|(id, _)| *id == hit.item_id)
            .unwrap();
        item.edit(device, at(NOW)).confirm_rotation(id).unwrap();
        assert!(states.breached_items(fx.candidates()).is_empty());
    }

    #[test]
    fn prefix_debug_is_redacted() {
        let p = RangePrefix::parse("5BAA6").unwrap();
        assert!(!format!("{p:?}").contains("5BAA6"));
    }
}

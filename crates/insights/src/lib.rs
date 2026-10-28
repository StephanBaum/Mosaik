//! Security insights for the dashboard (brief §5 "Security dashboard",
//! offline parts): weak, reused and old passwords, logins without a second
//! factor, and URLs that allow plain HTTP.
//!
//! Mosaic is pre-audit. Nothing here has been externally reviewed.
//!
//! # Rules
//!
//! - Everything runs on the device over decrypted items the session already
//!   holds. Nothing leaves the device and nothing is fetched: the 2FA
//!   Directory data is embedded ([`directory`]). The breach check (Have I
//!   Been Pwned, k-anonymity) is M7 and lives elsewhere.
//! - A [`Report`] carries item IDs, finding kinds, severities, action keys,
//!   the 2FA Directory entry of a site and the host of a plain-HTTP URL,
//!   never a password, a hash of one, a strength score or any other value
//!   derived from a secret beyond the finding itself, so it can cross the
//!   IPC boundary. The hosts are vault content: a report is never logged.
//! - Secrets are read into zeroizing buffers for the duration of
//!   [`analyze`] and dropped (zeroized) before it returns. Reuse detection
//!   (below) compares keyed hashes under a per-call random
//!   key and then the secrets themselves in constant time; no lookup table
//!   keyed by a secret outlives the call.
//!
//! # What is checked
//!
//! Items in the trash, deleted items and conflict copies
//! (`Item::conflict_copy_of`, which the vault already flags for review) are
//! skipped. For the rest:
//!
//! | Finding | Items | Rule | Severity | Action |
//! |---|---|---|---|---|
//! | [`FindingKind::Weak`] | logins (password), Wi-Fi (password) | [`estimate_strength`] below the generator's `Fair` bucket (60 bits) | `High` below 36 bits (`VeryWeak`), else `Medium` | change password |
//! | [`FindingKind::Reused`] | logins, Wi-Fi | the same non-empty password as another checked item (exact, byte for byte) | `High` | change password |
//! | [`FindingKind::Old`] | logins with a password | the current password was written more than [`Options::max_password_age_days`] (365) days before `now` | `Low` | change password |
//! | [`FindingKind::MissingTotp`] | logins without a one-time code (neither the item's own nor a custom field of the OTP kind) | a URL's host is a 2FA Directory TOTP site, and no checked item with a one-time code (own or custom field) names that site by URL or issuer | `Medium`; `Low` if a checked item holds a passkey for that site (relying party ID) | set up 2FA |
//! | [`FindingKind::InsecureUrl`] | logins | a URL with the `http` scheme, except loopback hosts | `Medium`; `Low` if every such URL is on a local network ([`url::is_local_network`]) | review URL |
//!
//! The password's age comes from the time its current value was written
//! (the winning entry of the password field, by the writer's clock), else
//! from the newest password-history entry, else from the item's creation.
//!
//! # Reuse
//!
//! See the `reuse` module and ADR-0020: `HMAC-SHA-256(K, password)` under a key `K` drawn
//! from the OS CSPRNG for each call, candidate groups by equal tags, then
//! exact classes by constant-time comparison. The groups are exactly the
//! equivalence classes of equal passwords (a property test checks this).
#![forbid(unsafe_code)]

pub mod directory;
mod error;
mod report;
mod reuse;
mod strength;
pub mod url;
mod words;

use std::collections::{BTreeMap, HashSet};

use mosaic_core::model::{CustomFieldValue, Item, ItemType, OtpConfig, SecretField};
use mosaic_core::{ItemId, SecretString, Timestamp, VaultId};
pub use mosaic_generator::Strength;

pub use directory::{TotpSite, totp_site, totp_sites_for_issuer};
pub use error::{Error, Result};
pub use report::{Action, Counts, Finding, FindingKind, ItemFindings, ItemKey, Report, Severity};
pub use strength::{MAX_ANALYZED_CHARS, StrengthEstimate, brute_force_bits, estimate_strength};
pub use words::COMMON_WORDS;

/// Milliseconds per day.
const DAY_MS: i64 = 86_400_000;

/// One decrypted item to check, as `mosaic_vault::session::ItemRef` holds it.
#[derive(Clone, Copy, Debug)]
pub struct Candidate<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// The decrypted item.
    pub item: &'a Item,
}

/// Settings for [`analyze`]. Start from [`Options::default`] and set fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    /// A password older than this many days is [`FindingKind::Old`].
    /// Default 365 (brief §5: "old (> 1 year)").
    pub max_password_age_days: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_password_age_days: 365,
        }
    }
}

/// A checked item and the secret the checks look at.
struct Checked<'a> {
    key: ItemKey,
    item: &'a Item,
    item_type: Option<ItemType>,
    /// The login or Wi-Fi password, if non-empty. Zeroized on drop.
    secret: Option<SecretString>,
}

impl Checked<'_> {
    fn is_login(&self) -> bool {
        self.item_type == Some(ItemType::Login)
    }
}

/// Checks `items` at time `now` (see the crate documentation for the rules).
///
/// # Errors
/// [`Error::Random`] if the per-call reuse key cannot be drawn,
/// [`Error::Allocation`] if a secret cannot be copied into zeroizing memory.
pub fn analyze<'a, I>(items: I, now: Timestamp, options: &Options) -> Result<Report>
where
    I: IntoIterator<Item = Candidate<'a>>,
{
    let checked = collect(items)?;
    let mut found = Found::new(&checked);
    check_weak(&mut found);
    let groups = check_reused(&mut found)?;
    check_old(&mut found, now, options.max_password_age_days);
    check_urls(&mut found);
    check_totp(&mut found);
    Ok(found.finish(&groups))
}

/// Findings collected so far, by index into the checked items.
struct Found<'c, 'a> {
    checked: &'c [Checked<'a>],
    map: BTreeMap<usize, ItemFindings>,
}

impl<'c, 'a> Found<'c, 'a> {
    fn new(checked: &'c [Checked<'a>]) -> Self {
        Self {
            checked,
            map: BTreeMap::new(),
        }
    }

    fn key(&self, index: usize) -> Option<ItemKey> {
        self.checked.get(index).map(|c| c.key)
    }

    /// Adds one finding (at most one per kind and item) and returns the
    /// item's entry.
    fn add(
        &mut self,
        index: usize,
        kind: FindingKind,
        severity: Severity,
    ) -> Option<&mut ItemFindings> {
        let item = self.key(index)?;
        let entry = self.map.entry(index).or_insert_with(|| ItemFindings {
            item,
            findings: Vec::new(),
            reused_with: Vec::new(),
            totp_site: None,
            insecure_host: None,
        });
        if !entry.has(kind) {
            entry.findings.push(finding(kind, severity));
        }
        Some(entry)
    }

    /// Sorts everything and computes the counts.
    fn finish(self, groups: &[Vec<usize>]) -> Report {
        let mut reuse_groups: Vec<Vec<ItemKey>> = groups
            .iter()
            .map(|g| g.iter().filter_map(|&i| self.key(i)).collect::<Vec<_>>())
            .collect();
        for group in &mut reuse_groups {
            group.sort_unstable();
        }
        reuse_groups.sort_unstable();

        let mut items: Vec<ItemFindings> = self.map.into_values().collect();
        for item in &mut items {
            item.findings
                .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.kind.cmp(&b.kind)));
            item.reused_with.sort_unstable();
        }
        items.sort_by(|a, b| {
            b.severity()
                .cmp(&a.severity())
                .then(b.findings.len().cmp(&a.findings.len()))
                .then(a.item.cmp(&b.item))
        });
        Report {
            items_checked: saturating_u32(self.checked.len()),
            counts: count(&items, reuse_groups.len()),
            items,
            reuse_groups,
        }
    }
}

fn check_weak(found: &mut Found<'_, '_>) {
    let checked = found.checked;
    for (index, c) in checked.iter().enumerate() {
        let Some(secret) = &c.secret else {
            continue;
        };
        let estimate = estimate_strength(secret.expose_secret());
        if estimate.is_weak() {
            let severity = if estimate.strength() == Strength::VeryWeak {
                Severity::High
            } else {
                Severity::Medium
            };
            found.add(index, FindingKind::Weak, severity);
        }
    }
}

/// Marks every member of a reuse group and returns the groups.
fn check_reused(found: &mut Found<'_, '_>) -> Result<Vec<Vec<usize>>> {
    let secrets: Vec<&[u8]> = found
        .checked
        .iter()
        .map(|c| {
            c.secret
                .as_ref()
                .map_or(&[][..], |s| s.expose_secret().as_bytes())
        })
        .collect();
    let groups = reuse::groups(&secrets)?;
    for group in &groups {
        let keys: Vec<ItemKey> = group.iter().filter_map(|&i| found.key(i)).collect();
        for &index in group {
            let me = found.key(index);
            if let Some(entry) = found.add(index, FindingKind::Reused, Severity::High) {
                entry.reused_with = keys.iter().copied().filter(|k| Some(*k) != me).collect();
            }
        }
    }
    Ok(groups)
}

fn check_old(found: &mut Found<'_, '_>, now: Timestamp, max_age_days: u32) {
    let max_age = i64::from(max_age_days).saturating_mul(DAY_MS);
    let checked = found.checked;
    for (index, c) in checked.iter().enumerate() {
        if c.is_login()
            && c.secret.is_some()
            && let Some(changed) = password_changed_at(c.item)
            && now
                .as_unix_millis()
                .saturating_sub(changed.as_unix_millis())
                > max_age
        {
            found.add(index, FindingKind::Old, Severity::Low);
        }
    }
}

fn check_urls(found: &mut Found<'_, '_>) {
    let checked = found.checked;
    for (index, c) in checked.iter().enumerate() {
        if !c.is_login() {
            continue;
        }
        let http_hosts: Vec<String> = hosts(c.item)
            .into_iter()
            .filter(|p| p.is_http() && !url::is_loopback(p.host()))
            .map(|p| p.host().to_owned())
            .collect();
        if http_hosts.is_empty() {
            continue;
        }
        let severity = if http_hosts.iter().all(|h| url::is_local_network(h)) {
            Severity::Low
        } else {
            Severity::Medium
        };
        if let Some(entry) = found.add(index, FindingKind::InsecureUrl, severity) {
            // The host that sets the severity: a public one if any.
            let shown = http_hosts
                .iter()
                .position(|h| !url::is_local_network(h))
                .unwrap_or(0);
            entry.insecure_host = http_hosts.into_iter().nth(shown);
        }
    }
}

fn check_totp(found: &mut Found<'_, '_>) {
    let covered = totp_coverage(found.checked);
    let checked = found.checked;
    for (index, c) in checked.iter().enumerate() {
        if !c.is_login() || !otp_configs(c.item).is_empty() {
            continue;
        }
        let uncovered: Vec<TotpSite> = hosts(c.item)
            .iter()
            .filter_map(|p| totp_site(p.host()))
            .filter(|s| !covered.totp.contains(s.domain()))
            .collect();
        // A site without a passkey first: a passkey for one site must not
        // lower the finding for another.
        let site = uncovered
            .iter()
            .find(|s| !covered.passkey.contains(s.domain()))
            .or_else(|| uncovered.first())
            .copied();
        let Some(site) = site else {
            continue;
        };
        let severity = if covered.passkey.contains(site.domain()) {
            Severity::Low
        } else {
            Severity::Medium
        };
        if let Some(entry) = found.add(index, FindingKind::MissingTotp, severity) {
            entry.totp_site = Some(site);
        }
    }
}

fn finding(kind: FindingKind, severity: Severity) -> Finding {
    let action = match kind {
        FindingKind::Weak | FindingKind::Reused | FindingKind::Old => Action::ChangePassword,
        FindingKind::MissingTotp => Action::SetUpTotp,
        FindingKind::InsecureUrl => Action::ReviewUrl,
    };
    Finding {
        kind,
        severity,
        action,
    }
}

/// The items to check and their secrets.
fn collect<'a, I>(items: I) -> Result<Vec<Checked<'a>>>
where
    I: IntoIterator<Item = Candidate<'a>>,
{
    let mut out = Vec::new();
    for c in items {
        let item = c.item;
        if item.is_deleted() || item.trashed_at().is_some() || item.conflict_copy_of().is_some() {
            continue;
        }
        let item_type = item.item_type();
        let field = match item_type {
            Some(ItemType::Login) => Some(SecretField::Password),
            Some(ItemType::Wifi) => Some(SecretField::WifiPassword),
            _ => None,
        };
        let secret = match field {
            Some(field) => item
                .secret(field)
                .map_err(|_| Error::Allocation)?
                .filter(|s| !s.expose_secret().is_empty()),
            None => None,
        };
        out.push(Checked {
            key: ItemKey {
                vault_id: c.vault_id,
                item_id: c.item_id,
            },
            item,
            item_type,
            secret,
        });
    }
    Ok(out)
}

/// When the current password was written (see the crate documentation).
fn password_changed_at(item: &Item) -> Option<Timestamp> {
    item.entries(SecretField::Password.key())
        .last()
        .map(mosaic_core::model::Entry::at)
        .or_else(|| {
            item.password_history()
                .iter()
                .find(|h| h.field == SecretField::Password)
                .map(|h| h.changed_at)
        })
        .or_else(|| item.created_at())
}

/// The parsed URLs of an item.
fn hosts(item: &Item) -> Vec<url::UrlParts> {
    item.urls()
        .iter()
        .filter_map(|(_, u)| url::parse(u.url.as_str()))
        .collect()
}

/// Directory domains the vault already protects with a second factor.
struct Coverage {
    /// Domains some checked item holds a one-time code for, by its URLs or
    /// its OTP issuer.
    totp: HashSet<&'static str>,
    /// Domains some checked item holds a passkey for, by its relying party
    /// ID.
    passkey: HashSet<&'static str>,
}

/// Every OTP configuration of an item: its own one-time code and any
/// custom field of the OTP kind (imports often put one there). The
/// secrets are dropped (zeroized) with the returned values.
fn otp_configs(item: &Item) -> Vec<OtpConfig> {
    item.otp()
        .into_iter()
        .chain(
            item.custom_fields()
                .into_iter()
                .filter_map(|(_, field)| match field.value {
                    CustomFieldValue::Otp(config) => Some(config),
                    _ => None,
                }),
        )
        .collect()
}

fn totp_coverage(checked: &[Checked<'_>]) -> Coverage {
    let mut covered = Coverage {
        totp: HashSet::new(),
        passkey: HashSet::new(),
    };
    for c in checked {
        if let Some(passkey) = c.item.passkey()
            && let Some(site) = totp_site(passkey.rp_id.as_str())
        {
            covered.passkey.insert(site.domain());
        }
        let configs = otp_configs(c.item);
        if configs.is_empty() {
            continue;
        }
        covered.totp.extend(
            hosts(c.item)
                .iter()
                .filter_map(|p| totp_site(p.host()))
                .map(|s| s.domain()),
        );
        for issuer in configs.iter().filter_map(|otp| otp.issuer.as_ref()) {
            covered.totp.extend(
                totp_sites_for_issuer(issuer.as_str())
                    .iter()
                    .map(TotpSite::domain),
            );
        }
    }
    covered
}

fn count(items: &[ItemFindings], reuse_groups: usize) -> Counts {
    let mut counts = Counts {
        reused_groups: saturating_u32(reuse_groups),
        ..Counts::default()
    };
    for item in items {
        for f in &item.findings {
            let slot = match f.kind {
                FindingKind::Weak => &mut counts.weak,
                FindingKind::Reused => &mut counts.reused,
                FindingKind::Old => &mut counts.old,
                FindingKind::MissingTotp => &mut counts.missing_totp,
                FindingKind::InsecureUrl => &mut counts.insecure_url,
            };
            *slot = slot.saturating_add(1);
        }
        let slot = match item.severity() {
            Some(Severity::High) => &mut counts.high,
            Some(Severity::Medium) => &mut counts.medium,
            Some(Severity::Low) => &mut counts.low,
            None => continue,
        };
        *slot = slot.saturating_add(1);
    }
    counts
}

fn saturating_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

//! Security dashboard, offline parts (mosaic-insights). Findings name items
//! and kinds only, never a secret or anything derived from one.

use serde::Serialize;

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::macros::wire_enum;

wire_enum!(
    /// What is wrong with an item.
    pub enum InsightKind {
        /// The password is weak (below the generator's `Fair` bucket).
        Weak,
        /// The same password is used by another item.
        Reused,
        /// The password has not changed in more than a year.
        Old,
        /// A login without a one-time password where the site supports one.
        MissingTotp,
        /// A login URL that allows plain HTTP.
        InsecureUrl,
    }
);

/// How many items have each finding.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightCounts {
    /// `weak`.
    pub weak: u32,
    /// `reused`.
    pub reused: u32,
    /// `old`.
    pub old: u32,
    /// `missingTotp`.
    pub missing_totp: u32,
    /// `insecureUrl`.
    pub insecure_url: u32,
}

/// One item and its findings.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightFinding {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Its findings.
    pub kinds: Vec<InsightKind>,
    /// Items sharing the password, for `reused` (empty otherwise).
    pub reused_with: Vec<WireItemId>,
}

/// The `security_report` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityReport {
    /// When it was computed.
    pub generated_at: UnixMillis,
    /// Items checked.
    pub items_checked: u32,
    /// Per-kind counts.
    pub counts: InsightCounts,
    /// Items with at least one finding.
    pub findings: Vec<InsightFinding>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn report() -> SecurityReport {
        SecurityReport {
            generated_at: 1_761_480_000_000,
            items_checked: 42,
            counts: InsightCounts {
                weak: 1,
                reused: 2,
                old: 0,
                missing_totp: 1,
                insecure_url: 0,
            },
            findings: vec![
                InsightFinding {
                    vault_id: ids::vault(1),
                    item_id: ids::item(1),
                    kinds: InsightKind::ALL.to_vec(),
                    reused_with: vec![ids::item(2)],
                },
                InsightFinding {
                    vault_id: ids::vault(1),
                    item_id: ids::item(2),
                    kinds: vec![InsightKind::Reused],
                    reused_with: vec![ids::item(1)],
                },
            ],
        }
    }
}

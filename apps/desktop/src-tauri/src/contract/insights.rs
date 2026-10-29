//! Security dashboard, offline parts (mosaic-insights). Findings name items,
//! kinds, severities and actions, plus the 2FA Directory site and the
//! plain-HTTP host the dashboard text needs; never a secret or anything
//! derived from one.

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

wire_enum!(
    /// How urgent a finding is.
    pub enum InsightSeverity {
        /// Worth fixing when convenient.
        Low,
        /// Fix soon.
        Medium,
        /// Fix first.
        High,
    }
);

wire_enum!(
    /// What the dashboard offers to do about a finding.
    pub enum InsightAction {
        /// Open the item to change its password.
        ChangePassword,
        /// Open the item to add a one-time code.
        SetUpTotp,
        /// Open the item to review its URL.
        ReviewUrl,
    }
);

/// Summary counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightCounts {
    /// Items with a weak password.
    pub weak: u32,
    /// Items whose password another item also uses.
    pub reused: u32,
    /// Groups of items sharing one password.
    pub reused_groups: u32,
    /// Items with an old password.
    pub old: u32,
    /// Logins for sites that support one-time codes, without one.
    pub missing_totp: u32,
    /// Logins with a plain-HTTP URL.
    pub insecure_url: u32,
    /// Items whose most severe finding is `high`.
    pub high: u32,
    /// Items whose most severe finding is `medium`.
    pub medium: u32,
    /// Items whose most severe finding is `low`.
    pub low: u32,
}

/// One finding on an item.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightIssue {
    /// What is wrong.
    pub kind: InsightKind,
    /// How urgent it is.
    pub severity: InsightSeverity,
    /// What the dashboard offers.
    pub action: InsightAction,
}

/// An item, by vault and item ID.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightItemRef {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
}

/// A site from the bundled 2FA Directory data.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpSiteInfo {
    /// The service name, as the directory spells it.
    pub name: String,
    /// The service's `https://` page about setting up 2FA, if known. Open it
    /// only on a user action.
    pub documentation: Option<String>,
}

/// One item and its findings.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightFinding {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Its findings, most severe first, at most one per kind.
    pub issues: Vec<InsightIssue>,
    /// Items sharing the password, for `reused` (empty otherwise).
    pub reused_with: Vec<InsightItemRef>,
    /// The site, for `missingTotp` (`null` otherwise).
    pub totp_site: Option<TotpSiteInfo>,
    /// The plain-HTTP host, for `insecureUrl` (`null` otherwise).
    pub insecure_host: Option<String>,
}

/// The `security_report` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityReport {
    /// When it was computed.
    pub generated_at: UnixMillis,
    /// Items checked.
    pub items_checked: u32,
    /// Summary counts.
    pub counts: InsightCounts,
    /// Items with at least one finding: most severe first, then more
    /// findings first, then by vault and item ID.
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
                reused_groups: 1,
                old: 1,
                missing_totp: 2,
                insecure_url: 1,
                high: 2,
                medium: 0,
                low: 0,
            },
            findings: vec![
                InsightFinding {
                    vault_id: ids::vault(1),
                    item_id: ids::item(1),
                    issues: vec![
                        InsightIssue {
                            kind: InsightKind::Weak,
                            severity: InsightSeverity::High,
                            action: InsightAction::ChangePassword,
                        },
                        InsightIssue {
                            kind: InsightKind::Reused,
                            severity: InsightSeverity::High,
                            action: InsightAction::ChangePassword,
                        },
                        InsightIssue {
                            kind: InsightKind::MissingTotp,
                            severity: InsightSeverity::Medium,
                            action: InsightAction::SetUpTotp,
                        },
                        InsightIssue {
                            kind: InsightKind::InsecureUrl,
                            severity: InsightSeverity::Medium,
                            action: InsightAction::ReviewUrl,
                        },
                        InsightIssue {
                            kind: InsightKind::Old,
                            severity: InsightSeverity::Low,
                            action: InsightAction::ChangePassword,
                        },
                    ],
                    reused_with: vec![InsightItemRef {
                        vault_id: ids::vault(1),
                        item_id: ids::item(2),
                    }],
                    totp_site: Some(TotpSiteInfo {
                        name: "GitHub".into(),
                        documentation: Some(
                            "https://docs.github.com/en/authentication/securing-your-account-with-two-factor-authentication-2fa".into(),
                        ),
                    }),
                    insecure_host: Some("shop.example.com".into()),
                },
                InsightFinding {
                    vault_id: ids::vault(1),
                    item_id: ids::item(2),
                    issues: vec![
                        InsightIssue {
                            kind: InsightKind::Reused,
                            severity: InsightSeverity::High,
                            action: InsightAction::ChangePassword,
                        },
                        InsightIssue {
                            kind: InsightKind::MissingTotp,
                            severity: InsightSeverity::Low,
                            action: InsightAction::SetUpTotp,
                        },
                    ],
                    reused_with: vec![InsightItemRef {
                        vault_id: ids::vault(1),
                        item_id: ids::item(1),
                    }],
                    totp_site: Some(TotpSiteInfo {
                        name: "Example".into(),
                        documentation: None,
                    }),
                    insecure_host: None,
                },
            ],
        }
    }
}

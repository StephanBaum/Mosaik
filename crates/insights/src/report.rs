//! The report: item IDs, finding kinds, severities and action keys. Nothing
//! in it is a secret or derived from one beyond the finding itself (a weak
//! password is reported as `Weak`, never with its value, score or hash), so
//! a report may cross the IPC boundary and be logged.

use mosaic_core::{ItemId, VaultId};

use crate::directory::TotpSite;

/// What is wrong with an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FindingKind {
    /// The password is weak: below the generator's `Fair` bucket.
    Weak,
    /// Another item has the same password.
    Reused,
    /// The password has not changed for longer than the configured age.
    Old,
    /// The site supports one-time codes (2FA Directory), but no item holds
    /// a one-time code for it.
    MissingTotp,
    /// A login URL uses plain HTTP.
    InsecureUrl,
}

impl FindingKind {
    /// Every kind, in the order of the dashboard summary.
    pub const ALL: [Self; 5] = [
        Self::Weak,
        Self::Reused,
        Self::Old,
        Self::MissingTotp,
        Self::InsecureUrl,
    ];

    /// The i18n key of the kind's label (`packages/i18n`).
    #[must_use]
    pub const fn i18n_key(self) -> &'static str {
        match self {
            Self::Weak => "insights.kind.weak",
            Self::Reused => "insights.kind.reused",
            Self::Old => "insights.kind.old",
            Self::MissingTotp => "insights.kind.missingTotp",
            Self::InsecureUrl => "insights.kind.insecureUrl",
        }
    }
}

/// How urgent a finding is. Ordered: `Low < Medium < High`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth fixing when convenient.
    Low,
    /// Fix soon.
    Medium,
    /// Fix first.
    High,
}

impl Severity {
    /// The i18n key of the severity's label.
    #[must_use]
    pub const fn i18n_key(self) -> &'static str {
        match self {
            Self::Low => "insights.severity.low",
            Self::Medium => "insights.severity.medium",
            Self::High => "insights.severity.high",
        }
    }
}

/// What the dashboard offers to do about a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Action {
    /// Open the item to change its password (with the generator).
    ChangePassword,
    /// Open the item to add a one-time code.
    SetUpTotp,
    /// Open the item to review its URL (switch to `https://`, or keep it
    /// for a device on the local network).
    ReviewUrl,
}

impl Action {
    /// The i18n key of the action's button label.
    #[must_use]
    pub const fn i18n_key(self) -> &'static str {
        match self {
            Self::ChangePassword => "insights.action.changePassword",
            Self::SetUpTotp => "insights.action.setUpTotp",
            Self::ReviewUrl => "insights.action.reviewUrl",
        }
    }
}

/// One finding on one item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    /// What is wrong.
    pub kind: FindingKind,
    /// How urgent it is.
    pub severity: Severity,
    /// What the dashboard offers.
    pub action: Action,
}

/// An item, by vault and item ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemKey {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
}

/// One item and everything found on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemFindings {
    /// The item.
    pub item: ItemKey,
    /// Its findings, most severe first, at most one per kind.
    pub findings: Vec<Finding>,
    /// For [`FindingKind::Reused`]: the other items with the same password,
    /// sorted; empty otherwise.
    pub reused_with: Vec<ItemKey>,
    /// For [`FindingKind::MissingTotp`]: the directory entry of the site,
    /// for a "how to set up 2FA" link; `None` otherwise.
    pub totp_site: Option<TotpSite>,
}

impl ItemFindings {
    /// The most severe finding's severity.
    #[must_use]
    pub fn severity(&self) -> Option<Severity> {
        self.findings.iter().map(|f| f.severity).max()
    }

    /// Whether the item has a finding of `kind`.
    #[must_use]
    pub fn has(&self, kind: FindingKind) -> bool {
        self.findings.iter().any(|f| f.kind == kind)
    }
}

/// Summary counts for the dashboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Items with a weak password.
    pub weak: u32,
    /// Items whose password another item also uses.
    pub reused: u32,
    /// Groups of items sharing one password.
    pub reused_groups: u32,
    /// Items with an old password.
    pub old: u32,
    /// Items for sites that support one-time codes, without one.
    pub missing_totp: u32,
    /// Items with a plain-HTTP URL.
    pub insecure_url: u32,
    /// Items whose most severe finding is `High`.
    pub high: u32,
    /// Items whose most severe finding is `Medium`.
    pub medium: u32,
    /// Items whose most severe finding is `Low`.
    pub low: u32,
}

impl Counts {
    /// The count of `kind`.
    #[must_use]
    pub const fn of(&self, kind: FindingKind) -> u32 {
        match kind {
            FindingKind::Weak => self.weak,
            FindingKind::Reused => self.reused,
            FindingKind::Old => self.old,
            FindingKind::MissingTotp => self.missing_totp,
            FindingKind::InsecureUrl => self.insecure_url,
        }
    }
}

/// The result of [`crate::analyze`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// How many items were checked (live, outside the trash, not conflict
    /// copies).
    pub items_checked: u32,
    /// Summary counts.
    pub counts: Counts,
    /// Items with at least one finding: most severe first, then more
    /// findings first, then by vault and item ID.
    pub items: Vec<ItemFindings>,
    /// Groups of items sharing one password, each sorted, the groups sorted
    /// by their first member.
    pub reuse_groups: Vec<Vec<ItemKey>>,
}

impl Report {
    /// The findings of one item, if it has any.
    #[must_use]
    pub fn item(&self, key: ItemKey) -> Option<&ItemFindings> {
        self.items.iter().find(|i| i.item == key)
    }
}

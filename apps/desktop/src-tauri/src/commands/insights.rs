//! Security dashboard command, on top of `mosaic-insights`. The report names
//! items and finding kinds only; the passwords it looks at stay in Rust and
//! are dropped (zeroized) before the command returns.

use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_core::Timestamp;
use mosaic_insights::{Action, Candidate, Finding, FindingKind, Options, Severity, analyze};
use mosaic_vault::session::ItemFilter;
use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::insights::{
    InsightAction, InsightCounts, InsightFinding, InsightIssue, InsightItemRef, InsightKind,
    InsightSeverity, SecurityReport, TotpSiteInfo,
};
use crate::state::{AppState, Backend};

/// `security_report`.
#[tauri::command]
pub async fn security_report(state: State<'_, AppState>) -> IpcResult<SecurityReport> {
    state.run(|b| b.security_report(now())).await
}

impl Backend {
    /// Runs the offline checks over every live item outside the trash.
    ///
    /// # Errors
    /// `locked`; `internal` if the checks fail (the OS random number
    /// generator or an allocation).
    pub fn security_report(&self, now: Timestamp) -> IpcResult<SecurityReport> {
        self.touch();
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let entries = session.items(&ItemFilter::default());
        let report = analyze(
            entries.iter().map(|e| Candidate {
                vault_id: e.vault_id,
                item_id: e.item_id,
                item: e.item,
            }),
            now,
            &Options::default(),
        )
        .map_err(|_| IpcError::INTERNAL)?;
        Ok(to_wire(&report, now))
    }
}

fn to_wire(report: &mosaic_insights::Report, now: Timestamp) -> SecurityReport {
    let c = report.counts;
    SecurityReport {
        generated_at: now.as_unix_millis(),
        items_checked: report.items_checked,
        counts: InsightCounts {
            weak: c.weak,
            reused: c.reused,
            reused_groups: c.reused_groups,
            old: c.old,
            missing_totp: c.missing_totp,
            insecure_url: c.insecure_url,
            high: c.high,
            medium: c.medium,
            low: c.low,
        },
        // Same order as the report: most severe first, then more findings,
        // then by vault and item ID (the mock follows it too).
        findings: report
            .items
            .iter()
            .map(|item| InsightFinding {
                vault_id: item.item.vault_id.into(),
                item_id: item.item.item_id.into(),
                issues: item.findings.iter().copied().filter_map(issue).collect(),
                reused_with: item
                    .reused_with
                    .iter()
                    .map(|k| InsightItemRef {
                        vault_id: k.vault_id.into(),
                        item_id: k.item_id.into(),
                    })
                    .collect(),
                totp_site: item.totp_site.map(|site| TotpSiteInfo {
                    name: site.name().to_owned(),
                    documentation: site.documentation().map(str::to_owned),
                }),
                insecure_host: item.insecure_host.clone(),
            })
            .collect(),
    }
}

/// The wire finding. `FindingKind` and `Action` are non-exhaustive: a value
/// the contract does not know yet is left out rather than mislabeled.
fn issue(finding: Finding) -> Option<InsightIssue> {
    Some(InsightIssue {
        kind: kind(finding.kind)?,
        severity: severity(finding.severity),
        action: action(finding.action)?,
    })
}

fn kind(kind: FindingKind) -> Option<InsightKind> {
    match kind {
        FindingKind::Weak => Some(InsightKind::Weak),
        FindingKind::Reused => Some(InsightKind::Reused),
        FindingKind::Old => Some(InsightKind::Old),
        FindingKind::MissingTotp => Some(InsightKind::MissingTotp),
        FindingKind::InsecureUrl => Some(InsightKind::InsecureUrl),
        _ => None,
    }
}

fn severity(severity: Severity) -> InsightSeverity {
    match severity {
        Severity::Low => InsightSeverity::Low,
        Severity::Medium => InsightSeverity::Medium,
        Severity::High => InsightSeverity::High,
    }
}

fn action(action: Action) -> Option<InsightAction> {
    match action {
        Action::ChangePassword => Some(InsightAction::ChangePassword),
        Action::SetUpTotp => Some(InsightAction::SetUpTotp),
        Action::ReviewUrl => Some(InsightAction::ReviewUrl),
        _ => None,
    }
}

/// Wall-clock time for the report.
fn now() -> Timestamp {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    Timestamp::from_unix_millis(ms)
}

#[cfg(test)]
mod tests {
    use mosaic_core::model::{ItemType, LoginUrl, SecretField};
    use mosaic_core::{Argon2Profile, VaultId};
    use mosaic_vault::session::LockedSession;
    use mosaic_vault::storage::Store;

    use super::*;
    use crate::contract::secret::SecretIn;
    use crate::contract::session::CreateAccountRequest;
    use crate::keystore::MemoryKeystore;
    use crate::state::Slot;

    fn unlocked() -> (Backend, VaultId) {
        let backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        );
        backend
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test("correct horse battery staple"),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        let vault = backend.list_vaults().unwrap()[0].vault_id.get();
        (backend, vault)
    }

    fn add_login(backend: &Backend, vault: VaultId, password: &str, url: &str) -> String {
        let mut slot = backend.slot().unwrap();
        let session = slot.unlocked_mut().unwrap();
        session
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title("Sample")?;
                e.set_secret(SecretField::Password, password)?;
                e.add_url(&LoginUrl::new(url))?;
                Ok(())
            })
            .unwrap()
            .to_string()
    }

    #[test]
    fn report_over_the_session() {
        let (backend, vault) = unlocked();
        let reused_a = add_login(&backend, vault, "Sommer2024!", "https://a.example");
        let reused_b = add_login(&backend, vault, "Sommer2024!", "http://b.example");
        let fine = add_login(&backend, vault, "Vw9!rT2#kL5$pM8&nQ4*", "https://c.example");

        let now = Timestamp::from_unix_millis(1_800_000_000_000);
        let report = backend.security_report(now).unwrap();
        assert_eq!(report.generated_at, 1_800_000_000_000);
        assert_eq!(report.items_checked, 3);
        assert_eq!(
            report.counts,
            InsightCounts {
                weak: 2,
                reused: 2,
                reused_groups: 1,
                old: 0,
                missing_totp: 0,
                insecure_url: 1,
                high: 2,
                medium: 0,
                low: 0,
            }
        );
        let json = serde_json::to_value(&report).unwrap();
        let text = json.to_string();
        assert!(!text.contains("Sommer2024"), "no secret on the wire");
        let findings = json["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 2);
        let b = findings
            .iter()
            .find(|f| f["itemId"] == reused_b.as_str())
            .unwrap();
        assert_eq!(
            b["issues"],
            serde_json::json!([
                { "kind": "weak", "severity": "high", "action": "changePassword" },
                { "kind": "reused", "severity": "high", "action": "changePassword" },
                { "kind": "insecureUrl", "severity": "medium", "action": "reviewUrl" },
            ])
        );
        assert_eq!(
            b["reusedWith"],
            serde_json::json!([{ "vaultId": vault.to_string(), "itemId": reused_a }])
        );
        assert_eq!(b["insecureHost"], "b.example");
        assert_eq!(b["totpSite"], serde_json::Value::Null);
        // Most severe first, then more findings first: b (3) before a (2).
        assert_eq!(findings[0]["itemId"], reused_b.as_str());
        assert!(!text.contains(&fine));
    }

    #[test]
    fn missing_totp_names_the_site() {
        let (backend, vault) = unlocked();
        let login = add_login(
            &backend,
            vault,
            "Vw9!rT2#kL5$pM8&nQ4*",
            "https://github.com",
        );
        let report = backend
            .security_report(Timestamp::from_unix_millis(1_800_000_000_000))
            .unwrap();
        let json = serde_json::to_value(&report).unwrap();
        let f = &json["findings"][0];
        assert_eq!(f["itemId"], login.as_str());
        assert_eq!(f["totpSite"]["name"], "GitHub");
        assert!(
            f["totpSite"]["documentation"]
                .as_str()
                .unwrap()
                .starts_with("https://")
        );
        assert_eq!(report.counts.medium, 1);
    }

    #[test]
    fn locked_sessions_get_no_report() {
        let backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        );
        assert_eq!(
            backend.security_report(now()).unwrap_err(),
            IpcError::LOCKED
        );
    }

    #[test]
    fn every_kind_has_a_wire_kind() {
        for k in FindingKind::ALL {
            assert!(kind(k).is_some(), "{k:?}");
        }
        for a in [Action::ChangePassword, Action::SetUpTotp, Action::ReviewUrl] {
            assert!(action(a).is_some(), "{a:?}");
        }
        assert_eq!(severity(Severity::Low), InsightSeverity::Low);
        assert_eq!(severity(Severity::Medium), InsightSeverity::Medium);
        assert_eq!(severity(Severity::High), InsightSeverity::High);
        assert!(now().as_unix_millis() > 1_700_000_000_000);
    }
}

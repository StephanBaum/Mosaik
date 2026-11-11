/**
 * "A browser connection points somewhere else" (`browserConnectorForeign`, ADR-0083): the
 * native-messaging entry of a browser starts another program than the installed Mosaic host.
 * Shown at the top of the vault and in Settings > Browsers, with a Repair button. The findings
 * come from `browser_connector_audit` (run when the screen opens); Repair rewrites the per-user
 * entries (`browser_connector_repair`) and the banner goes away with its button, so focus moves
 * to the page.
 */
import { useCallback, useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { UserText } from "../components/UserText.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { BrowserConnectorFinding, IpcError } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { focusPageIfLost } from "./focusPage.ts";

/** The findings of the audit, and the repair. */
export function useConnectorAudit() {
  const ipc = useIpc();
  const [findings, setFindings] = useState<readonly BrowserConnectorFinding[]>([]);
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("browser_connector_audit", null).then((result) => {
      if (current && result.ok) {
        setFindings(result.value.findings);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  const repair = useCallback(async (): Promise<boolean> => {
    const result = await ipc.call("browser_connector_repair", null);
    if (result.ok) {
      setFindings(result.value.findings);
      setError(null);
      return result.value.findings.length === 0;
    }
    setError(result.error);
    return false;
  }, [ipc]);
  return { findings, error, repair };
}

export function BrowserConnectorBanner() {
  const { t } = useI18n();
  const toast = useToast();
  const { findings, error, repair } = useConnectorAudit();
  const first = findings[0];
  // The live region stays mounted, so a finding that appears after the audit is announced.
  return (
    <div role="status">
      {first !== undefined && (
        <Callout tone="warning">
          <p className="body">
            <strong>{t("browser.connector.title")}</strong>
          </p>
          <p className="body">
            {t("browser.connector.body", {
              browser: t(`browser.family.${first.browser}`),
              path: first.path,
            })}
          </p>
          {findings.length > 1 && (
            <ul className="caption">
              {findings.slice(1).map((finding) => (
                <li key={finding.location}>
                  <UserText>
                    {t("browser.connector.item", {
                      browser: t(`browser.family.${finding.browser}`),
                      path: finding.path,
                    })}
                  </UserText>
                </li>
              ))}
            </ul>
          )}
          <ErrorAlert error={error} />
          <div className="actions">
            <Button
              onClick={() => {
                void repair().then((repaired) => {
                  if (repaired) {
                    toast(t("browser.connector.repaired"));
                    focusPageIfLost();
                  }
                });
              }}
            >
              {t("browser.connector.repair")}
            </Button>
          </div>
        </Callout>
      )}
    </div>
  );
}

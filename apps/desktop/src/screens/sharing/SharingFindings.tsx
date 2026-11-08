/**
 * What sharing wants the user to look at, at the top of the app so it is not missed: a member
 * nobody verified, a member the server dropped, a vault key that was refused, a vault the server
 * no longer gives this device. Each says what to do. The findings that point at the server
 * (a key that is not the account's, a damaged member list, a refused key) are cleared with the
 * same "I've checked…" as the sync warnings, which asks in a native dialog; the member findings
 * clear when the member is verified or removed.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SharingFindingView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate, useSyncOverview } from "../sync/hooks.ts";
import { VerifyContactDialog } from "./VerifyContactDialog.tsx";

/** The findings that need the user to have looked at the server. */
const ACKNOWLEDGEABLE: ReadonlySet<SharingFindingView["kind"]> = new Set([
  "identityMismatch",
  "rosterInvalid",
  "shareUntrusted",
]);

function useFindingText() {
  const { t } = useI18n();
  const vault = (name: string | null) =>
    name === null ? t("sync.warning.unknownVault") : isolate(name);
  const person = (email: string | null) =>
    email === null ? t("sharing.finding.unknownEmail") : isolate(email);
  return (finding: SharingFindingView): string => {
    switch (finding.kind) {
      case "identityMismatch":
        return t("sharing.finding.identityMismatch");
      case "rosterInvalid":
        return t("sharing.finding.rosterInvalid", { vault: vault(finding.vaultName) });
      case "shareUntrusted":
        return t("sharing.finding.shareUntrusted", {
          vault: vault(finding.vaultName),
          sender: person(finding.senderEmail),
        });
      case "memberUnverified":
        return t("sharing.finding.memberUnverified", {
          vault: vault(finding.vaultName),
          email: person(finding.email),
        });
      case "memberDropped":
        return t("sharing.finding.memberDropped", {
          vault: vault(finding.vaultName),
          email: person(finding.email),
        });
      case "accessLost":
        return t("sharing.finding.accessLost", { vault: vault(finding.vaultName) });
    }
  };
}

type MemberFinding = Extract<SharingFindingView, { kind: "memberUnverified" | "memberDropped" }>;

/** One finding with what to do about it: a member can be verified or removed. */
function FindingCard({
  finding,
  text,
  onVerify,
  onRemove,
}: {
  finding: SharingFindingView;
  text: string;
  onVerify: (email: string) => void;
  onRemove: (finding: MemberFinding) => void;
}) {
  const { t } = useI18n();
  return (
    <Callout tone="warning">
      <p className="body">{text}</p>
      {(finding.kind === "memberUnverified" || finding.kind === "memberDropped") && (
        <div className="actions">
          {finding.kind === "memberUnverified" && finding.email !== null && (
            <Button
              size="sm"
              onClick={() => {
                onVerify(finding.email ?? "");
              }}
            >
              {t("sharing.finding.verify")}
            </Button>
          )}
          <Button
            size="sm"
            variant="danger"
            onClick={() => {
              onRemove(finding);
            }}
          >
            {t("sharing.finding.remove")}
          </Button>
        </div>
      )}
    </Callout>
  );
}

export function SharingFindings() {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const text = useFindingText();
  const { overview, setOverview, refresh } = useSyncOverview();
  const [error, setError] = useState<IpcError | null>(null);
  const [verifying, setVerifying] = useState<string | null>(null);
  const findings = overview?.sharingFindings ?? [];
  if (findings.length === 0) {
    return null;
  }

  async function acknowledge() {
    setError(null);
    // Rust asks first; "no" returns the overview unchanged.
    const result = await ipc.call("sync_acknowledge", {});
    if (result.ok) {
      setOverview(result.value);
    } else {
      setError(result.error);
    }
  }

  async function remove(finding: MemberFinding) {
    setError(null);
    // Rust asks in a native dialog; "no" answers `cancelled`.
    const result = await ipc.call("vault_member_remove", {
      vaultId: finding.vaultId,
      accountId: finding.accountId,
    });
    if (result.ok) {
      toast(t("sharing.members.removed"));
      refresh();
    } else if (result.error.code !== "cancelled") {
      setError(result.error);
    }
  }

  return (
    <div className="stack sharing-findings" role="group" aria-label={t("sharing.finding.region")}>
      {findings.map((finding, index) => (
        <FindingCard
          key={`${finding.kind}-${String(index)}`}
          finding={finding}
          text={text(finding)}
          onVerify={setVerifying}
          onRemove={(member) => {
            void remove(member);
          }}
        />
      ))}
      {findings.some((f) => ACKNOWLEDGEABLE.has(f.kind)) && (
        <div className="actions">
          <Button
            onClick={() => {
              void acknowledge();
            }}
          >
            {t("sync.warning.acknowledge")}
          </Button>
        </div>
      )}
      <ErrorAlert error={error} />
      {verifying !== null && (
        <VerifyContactDialog
          email={verifying}
          onVerified={refresh}
          onClose={() => {
            setVerifying(null);
          }}
        />
      )}
    </div>
  );
}

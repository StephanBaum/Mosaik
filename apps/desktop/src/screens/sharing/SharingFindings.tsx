/**
 * What sharing wants the user to look at, at the top of the app so it is not missed: a member
 * nobody verified, a member the server dropped or whose key did not match, a vault key that was
 * refused, a vault held until another device confirms its owner, a vault the server no longer
 * gives this device. It is one line ("3 sharing warnings", announced politely) that expands to
 * the list, so it never covers the page. Each finding says what to do. The findings that point at the server
 * (a key that is not the account's, a damaged member list, a refused key) are cleared with the
 * same "I've checked…" as the sync warnings, which asks in a native dialog; the member findings
 * clear when the member is verified or removed.
 */
import { useId, useRef, useState, type RefObject } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SharingFindingView, SyncOverview } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate, useSyncOverview } from "../sync/hooks.ts";
import { VerifyContactDialog } from "./VerifyContactDialog.tsx";

/** The findings that need the user to have looked at the server. */
const ACKNOWLEDGEABLE: ReadonlySet<SharingFindingView["kind"]> = new Set([
  "identityMismatch",
  "rosterInvalid",
  "shareUntrusted",
  "memberKeyMismatch",
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
      case "memberKeyMismatch":
        return t("sharing.finding.memberKeyMismatch", {
          vault: vault(finding.vaultName),
          email: person(finding.email),
        });
      case "vaultHeld":
        return t("sharing.finding.vaultHeld", { vault: vault(finding.vaultName) });
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

/** One finding with what to do about it: a member can be verified or removed, a lost vault dismissed. */
function FindingCard({
  finding,
  text,
  onVerify,
  onRemove,
  onDismiss,
}: {
  finding: SharingFindingView;
  text: string;
  onVerify: (email: string) => void;
  onRemove: (finding: MemberFinding) => void;
  onDismiss: (finding: Extract<SharingFindingView, { kind: "accessLost" }>) => void;
}) {
  const { t } = useI18n();
  const member = finding.kind === "memberUnverified" || finding.kind === "memberDropped" ? finding : null;
  const email = member === null ? "" : isolate(member.email ?? t("sharing.finding.unknownEmail"));
  const vault = member === null ? "" : isolate(member.vaultName ?? t("sync.warning.unknownVault"));
  return (
    <Callout tone="warning">
      <p className="body">{text}</p>
      {member !== null && (
        <div className="actions">
          {member.kind === "memberUnverified" && member.email !== null && (
            <Button
              size="sm"
              aria-label={t("sharing.members.verifyOf", { email })}
              onClick={() => {
                onVerify(member.email ?? "");
              }}
            >
              {t("sharing.finding.verify")}
            </Button>
          )}
          <Button
            size="sm"
            variant="danger"
            aria-label={t("sharing.finding.removeOf", { email, vault })}
            onClick={() => {
              onRemove(member);
            }}
          >
            {t("sharing.finding.remove")}
          </Button>
        </div>
      )}
      {finding.kind === "accessLost" && (
        <div className="actions">
          <Button
            size="sm"
            onClick={() => {
              onDismiss(finding);
            }}
          >
            {t("sharing.finding.dismiss")}
          </Button>
        </div>
      )}
    </Callout>
  );
}

/** The two things a finding card can ask Rust to do, with the error either one ends in. */
function useFindingActions(setOverview: (overview: SyncOverview) => void, refresh: () => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [error, setError] = useState<IpcError | null>(null);

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

  return { error, acknowledge, remove };
}

/** The one line a collapsed list shows: how many warnings, announced politely, and the toggle. */
function FindingsBar({
  count,
  expanded,
  listId,
  toggle,
  onToggle,
}: {
  count: number;
  expanded: boolean;
  listId: string;
  toggle: RefObject<HTMLButtonElement | null>;
  onToggle: () => void;
}) {
  const { t } = useI18n();
  return (
    <Callout tone="warning">
      <div className="sharing-findings__bar">
        <p className="body" role="status">
          {t("sharing.finding.summary", { count })}
        </p>
        <Button
          size="sm"
          ref={toggle}
          aria-expanded={expanded}
          aria-controls={listId}
          onClick={onToggle}
        >
          {expanded ? t("sharing.finding.collapse") : t("sharing.finding.review")}
        </Button>
      </div>
    </Callout>
  );
}

export function SharingFindings() {
  const { t } = useI18n();
  const text = useFindingText();
  const listId = useId();
  const toggle = useRef<HTMLButtonElement>(null);
  const { overview, setOverview, refresh } = useSyncOverview();
  const { error, acknowledge, remove } = useFindingActions(setOverview, refresh);
  const [verifying, setVerifying] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);
  // "Dismiss" on a lost vault hides it until the app starts again; the vault stays lost.
  const [dismissed, setDismissed] = useState<ReadonlySet<string>>(new Set());
  const findings = (overview?.sharingFindings ?? []).filter(
    (f) => !(f.kind === "accessLost" && dismissed.has(f.vaultId)),
  );
  if (findings.length === 0) {
    return null;
  }

  return (
    <div className="stack sharing-findings" role="group" aria-label={t("sharing.finding.region")}>
      <FindingsBar
        count={findings.length}
        expanded={expanded}
        listId={listId}
        toggle={toggle}
        onToggle={() => {
          setExpanded(!expanded);
        }}
      />
      {expanded && (
        <div id={listId} className="stack">
          {findings.map((finding, index) => (
            <FindingCard
              key={`${finding.kind}-${String(index)}`}
              finding={finding}
              text={text(finding)}
              onVerify={setVerifying}
              onRemove={(member) => {
                void remove(member);
              }}
              onDismiss={(lost) => {
                // The button goes away: focus stays on the page's nearest control.
                (findings.length > 1 ? toggle.current : document.getElementById("main"))?.focus();
                setDismissed((set) => new Set(set).add(lost.vaultId));
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

/**
 * Looks an address up and gets its key verified: the one path every share and every grant takes
 * (verification before sharing is required, CRYPTO_SPEC §8.2). A key that is verified already
 * continues at once; a new one goes through the safety-code comparison; a key that changed (or
 * that devices disagree on) goes through the key-change warning first.
 */
import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { AccountId, IpcError, LookupResult } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { KeyChangedPanel } from "./KeyChangedPanel.tsx";
import { SafetyCodePanel } from "./SafetyCodePanel.tsx";

type Phase =
  | { kind: "loading" }
  | { kind: "failed"; error: IpcError }
  | { kind: "ready"; found: LookupResult; oldCode: string | null };

/** The lookup, and for a key that moved the code that was pinned before. */
async function lookUp(ipc: MosaicIpc, email: string): Promise<Phase> {
  const found = await ipc.call("sharing_lookup", { email });
  if (!found.ok) {
    return { kind: "failed", error: found.error };
  }
  const moved = found.value.status === "changed" || found.value.status === "conflicting";
  const contacts = moved ? await ipc.call("sharing_contacts", null) : null;
  const pinned = contacts?.ok
    ? contacts.value.contacts.find((c) => c.accountId === found.value.accountId)
    : undefined;
  return { kind: "ready", found: found.value, oldCode: pinned?.safetyCode ?? null };
}

function useLookup(email: string): Phase {
  const ipc = useIpc();
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  useEffect(() => {
    const live = { current: true };
    void lookUp(ipc, email).then((next) => {
      if (live.current) {
        setPhase(next);
      }
    });
    return () => {
      live.current = false;
    };
  }, [ipc, email]);
  return phase;
}

/** The comparison of a key that is new (pinned, not verified yet). */
function CompareCode({
  found,
  onVerified,
  onCancel,
}: {
  found: LookupResult;
  onVerified: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function verify() {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sharing_verify_contact", {
      accountId: found.accountId,
      safetyCode: found.safetyCode,
    });
    setBusy(false);
    if (result.ok) {
      onVerified();
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="stack">
      <SafetyCodePanel
        email={found.email}
        code={found.safetyCode}
        busy={busy}
        error={error}
        onMatches={() => void verify()}
        onCancel={onCancel}
        hint={t("sharing.lookup.notFoundHint")}
      />
    </div>
  );
}

export function ContactVerifier({
  email,
  onVerified,
  onCancel,
}: {
  email: string;
  /** The contact is verified (it was, or the user just compared the code). */
  onVerified: (accountId: AccountId, email: string) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const phase = useLookup(email);
  const advanced = useRef(false);
  const alreadyVerified = phase.kind === "ready" && phase.found.status === "verified";

  useEffect(() => {
    if (phase.kind === "ready" && alreadyVerified && !advanced.current) {
      advanced.current = true;
      onVerified(phase.found.accountId, phase.found.email);
    }
  }, [phase, alreadyVerified, onVerified]);

  if (phase.kind === "loading") {
    return (
      <p className="body muted" role="status">
        {t("sharing.lookup.searching")}
      </p>
    );
  }
  if (phase.kind === "failed") {
    return (
      <div className="stack">
        <ErrorAlert error={phase.error} />
        <div className="dialog__actions">
          <Button onClick={onCancel}>{t("action.back")}</Button>
        </div>
      </div>
    );
  }
  const { found, oldCode } = phase;
  if (alreadyVerified) {
    return (
      <p className="body" role="status">
        {t("sharing.code.verified", { email: isolate(found.email) })}
      </p>
    );
  }
  const verified = () => {
    onVerified(found.accountId, found.email);
  };
  return found.status === "changed" || found.status === "conflicting" ? (
    <KeyChangedPanel
      accountId={found.accountId}
      email={found.email}
      oldCode={oldCode}
      newCode={found.safetyCode}
      conflicting={found.status === "conflicting"}
      onAccepted={verified}
      onCancel={onCancel}
    />
  ) : (
    <CompareCode found={found} onVerified={verified} onCancel={onCancel} />
  );
}

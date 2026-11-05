/** Everything before the vault is reachable: no helper, forbidden, no desktop, not connected, pairing, version mismatch. */
import { useState } from "react";
import { errorCodeOf, type ErrorCode } from "../../shared/errors.ts";
import type { ConnectionState, PopupState } from "../../shared/messages.ts";
import { Button } from "../components/Button.tsx";
import { Icon } from "../components/Icon.tsx";
import { useI18n } from "../i18n.tsx";
import { useServices } from "../usePopup.ts";

type Titles = "hostMissing" | "hostForbidden" | "desktopNotRunning" | "notPaired" | "pairing" | "protocolMismatch" | "error";

function viewOf(connection: ConnectionState): Titles {
  switch (connection) {
    case "hostMissing":
    case "hostForbidden":
    case "desktopNotRunning":
    case "notPaired":
    case "pairing":
    case "protocolMismatch":
      return connection;
    default:
      return "error";
  }
}

/** "Open Mosaic" cannot be done from here: the helper never starts the desktop. */
function Action({ view, busy, onConnect, onCheck }: { view: Titles; busy: boolean; onConnect: () => void; onCheck: () => void }) {
  const { t } = useI18n();
  if (view === "notPaired") {
    return (
      <Button variant="primary" block disabled={busy} onClick={onConnect}>
        {t("ext.connect.start")}
      </Button>
    );
  }
  if (view === "pairing") {
    return null;
  }
  return (
    <Button block onClick={onCheck}>
      {t("ext.connect.check")}
    </Button>
  );
}

function PairCode({ pairCode }: { pairCode: PopupState["pairCode"] }) {
  const { t } = useI18n();
  const code = pairCode?.code ?? "";
  const shown = code.length === 6 ? `${code.slice(0, 3)}-${code.slice(3)}` : code;
  return (
    <div className="pair" role="status">
      <span className="caption">{t("ext.connect.pairing.code")}</span>
      <span className="pair__code" aria-label={shown.split("").join(" ")}>
        {shown}
      </span>
      <span className="caption">{t("ext.connect.pairing.waiting")}</span>
    </div>
  );
}

export function ConnectView({ state }: { state: PopupState }) {
  const { t, errorText } = useI18n();
  const { api, env, refresh } = useServices();
  const [failure, setFailure] = useState<ErrorCode | null>(null);
  const [busy, setBusy] = useState(false);
  const view = viewOf(state.connection);

  async function connect() {
    setBusy(true);
    setFailure(null);
    try {
      await api.request({ op: "pair" });
    } catch (error) {
      setFailure(errorCodeOf(error));
    } finally {
      setBusy(false);
      await refresh();
    }
  }

  return (
    <section className="center" aria-labelledby="connect-title">
      <Icon name={view === "pairing" ? "shield" : "lock"} size="lg" />
      <h1 id="connect-title" className="title">
        {t(`ext.connect.${view}.title`)}
      </h1>
      <p className="muted">{t(`ext.connect.${view}.body`)}</p>
      {view === "hostMissing" && (
        <p className="caption">
          {t("ext.connect.hostMissing.dev", { command: env.hostInstallCommand })}
        </p>
      )}
      {view === "pairing" && <PairCode pairCode={state.pairCode} />}
      {failure !== null && (
        <p className="alert" role="alert">
          {errorText(failure)}
        </p>
      )}
      <Action view={view} busy={busy} onConnect={() => void connect()} onCheck={() => void refresh()} />
    </section>
  );
}

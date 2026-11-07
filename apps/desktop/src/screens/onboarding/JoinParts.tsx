/** The tabs and the code step of the join screen ("I already use Mosaic"). */
import { type KeyboardEvent, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";

export type Tab = "server" | "folder";
const TABS: readonly Tab[] = ["server", "folder"];

/** The tab an arrow, Home or End key moves to (`null` for any other key). */
function tabForKey(key: string, tab: Tab): Tab | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowLeft":
      return tab === "server" ? "folder" : "server";
    case "Home":
      return "server";
    case "End":
      return "folder";
    default:
      return null;
  }
}

export function Tabs({ id, tab, onTab }: { id: string; tab: Tab; onTab: (tab: Tab) => void }) {
  const { t } = useI18n();
  function onKeyDown(event: KeyboardEvent) {
    const next = tabForKey(event.key, tab);
    if (next !== null) {
      event.preventDefault();
      onTab(next);
      document.getElementById(`${id}-${next}`)?.focus();
    }
  }
  return (
    <div className="segmented" role="tablist" aria-label={t("onboarding.join.tabs")} onKeyDown={onKeyDown}>
      {TABS.map((each) => (
        <button
          key={each}
          id={`${id}-${each}`}
          type="button"
          role="tab"
          className="segmented__tab"
          aria-selected={each === tab}
          aria-controls={`${id}-panel`}
          tabIndex={each === tab ? 0 : -1}
          onClick={() => {
            onTab(each);
          }}
        >
          {t(`onboarding.join.tab.${each}`)}
        </button>
      ))}
    </div>
  );
}

export function CodeForm({
  busy,
  error,
  onSubmit,
  onBack,
}: {
  busy: boolean;
  error: IpcError | null;
  onSubmit: (code: string) => void;
  onBack: () => void;
}) {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  return (
    <form
      className="form-stack"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(code);
        setCode("");
      }}
      noValidate
    >
      <div className="auth__intro">
        <h1 className="display">{t("onboarding.join.twoFactor.title")}</h1>
        <p className="lede">{t("onboarding.join.twoFactor.body")}</p>
      </div>
      <TextField
        size="lg"
        label={t("onboarding.join.twoFactor.code")}
        inputMode="numeric"
        autoComplete="one-time-code"
        mono
        autoFocus
        value={code}
        onChange={(event) => {
          setCode(event.target.value);
        }}
      />
      <ErrorAlert error={error} />
      <HoloButton type="submit" size="lg" block disabled={busy}>
        {t("onboarding.join.twoFactor.submit")}
      </HoloButton>
      <Button variant="ghost" size="lg" block className="button--muted" onClick={onBack}>
        {t("action.back")}
      </Button>
    </form>
  );
}

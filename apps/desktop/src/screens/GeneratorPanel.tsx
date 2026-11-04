/** The password generator (D12): characters or words; copies through Rust. */
import { useEffect, useId, useMemo, useState } from "react";
import { Button } from "../components/Button.tsx";
import { useCopy } from "../components/CopyButton.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { StrengthMeter } from "../components/StrengthMeter.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { Generated, IpcError } from "../ipc/contract/index.ts";
import { splitRuns } from "./generatorRuns.ts";
import {
  ModeTabs,
  panelId,
  PassphraseOptions,
  PasswordOptions,
  type Mode,
  type Options,
  type Phrase,
} from "./GeneratorOptions.tsx";

const PASSWORD: Options = {
  kind: "password",
  length: 20,
  lowercase: true,
  uppercase: true,
  digits: true,
  symbols: true,
  excludeAmbiguous: true,
};
const PASSPHRASE: Phrase = {
  kind: "passphrase",
  words: 5,
  separator: "-",
  capitalize: false,
  appendDigit: false,
};

function useGenerated(request: Options | Phrase, round: number) {
  const ipc = useIpc();
  const [result, setResult] = useState<Generated | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("generate_password", request).then((response) => {
      if (!current) {
        return;
      }
      setResult(response.ok ? response.value : null);
      setError(response.ok ? null : response.error);
    });
    return () => {
      current = false;
    };
  }, [ipc, request, round]);
  return { result, error };
}

function Output({ result, mode, onRegenerate }: { result: Generated; mode: Mode; onRegenerate: () => void }) {
  const { t } = useI18n();
  const copy = useCopy();
  return (
    <div className="generator__output">
      {/* aria-live off: the value changes with every slider step and must not be announced each time. */}
      <output className="generated" data-secret="" aria-label={t("generator.result")} aria-live="off">
        {mode === "password"
          ? splitRuns(result.value).map((run, index) => (
              <span key={index} className={run.minor ? "generated__minor" : undefined}>
                {run.text}
              </span>
            ))
          : result.value}
      </output>
      <div className="generator__meta">
        <StrengthMeter strength={result.strength} bits={result.entropyBits} />
        {mode === "password" && <span className="caption">{t("generator.minorHint")}</span>}
      </div>
      <div className="actions">
        <Button onClick={onRegenerate}>
          <Icon name="refresh" />
          {t("action.regenerate")}
        </Button>
        <HoloButton
          onClick={() => {
            void copy({ kind: "generated" });
          }}
        >
          <Icon name="copy" />
          {t("action.copy")}
        </HoloButton>
      </div>
    </div>
  );
}

export function GeneratorPanel() {
  const { t } = useI18n();
  const tabsId = useId();
  const [mode, setMode] = useState<Mode>("password");
  const [password, setPassword] = useState(PASSWORD);
  const [phrase, setPhrase] = useState(PASSPHRASE);
  const [round, setRound] = useState(0);
  const request = useMemo(() => (mode === "password" ? password : phrase), [mode, password, phrase]);
  const { result, error } = useGenerated(request, round);

  return (
    <section className="generator" aria-label={t("generator.title")}>
      <h1 className="display">{t("generator.title")}</h1>
      <ModeTabs id={tabsId} mode={mode} onMode={setMode} />
      <ErrorAlert error={error} />
      {result !== null && (
        <Output
          result={result}
          mode={mode}
          onRegenerate={() => {
            setRound((n) => n + 1);
          }}
        />
      )}
      <div
        className="generator__options"
        role="tabpanel"
        id={panelId(tabsId)}
        aria-labelledby={`${tabsId}-${mode}`}
      >
        {mode === "password" ? (
          <PasswordOptions value={password} onChange={setPassword} />
        ) : (
          <PassphraseOptions value={phrase} onChange={setPhrase} />
        )}
      </div>
    </section>
  );
}

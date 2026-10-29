/** The password generator (D12): characters or words; copies through Rust. */
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/Button.tsx";
import { CopyButton } from "../components/CopyButton.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { Select } from "../components/Select.tsx";
import { StrengthMeter } from "../components/StrengthMeter.tsx";
import { Switch } from "../components/Switch.tsx";
import { TextField } from "../components/TextField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { Generated, GenerateRequest, IpcError } from "../ipc/contract/index.ts";

type Options = Extract<GenerateRequest, { kind: "password" }>;
type Phrase = Extract<GenerateRequest, { kind: "passphrase" }>;

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

type Flag = "lowercase" | "uppercase" | "digits" | "symbols" | "excludeAmbiguous";
const FLAGS: readonly Flag[] = ["lowercase", "uppercase", "digits", "symbols", "excludeAmbiguous"];

function PasswordOptions({ value, onChange }: { value: Options; onChange: (next: Options) => void }) {
  const { t } = useI18n();
  return (
    <>
      <TextField
        label={t("generator.length")}
        type="number"
        min={4}
        max={128}
        value={value.length}
        onChange={(event) => {
          onChange({ ...value, length: Number(event.target.value) });
        }}
      />
      {FLAGS.map((flag) => (
        <Switch
          key={flag}
          label={t(`generator.${flag}`)}
          checked={value[flag]}
          onChange={(event) => {
            onChange({ ...value, [flag]: event.target.checked });
          }}
        />
      ))}
    </>
  );
}

function PassphraseOptions({ value, onChange }: { value: Phrase; onChange: (next: Phrase) => void }) {
  const { t } = useI18n();
  return (
    <>
      <TextField
        label={t("generator.words")}
        type="number"
        min={3}
        max={20}
        value={value.words}
        onChange={(event) => {
          onChange({ ...value, words: Number(event.target.value) });
        }}
      />
      <Switch
        label={t("generator.capitalize")}
        checked={value.capitalize}
        onChange={(event) => {
          onChange({ ...value, capitalize: event.target.checked });
        }}
      />
      <Switch
        label={t("generator.appendDigit")}
        checked={value.appendDigit}
        onChange={(event) => {
          onChange({ ...value, appendDigit: event.target.checked });
        }}
      />
    </>
  );
}

export function GeneratorPanel() {
  const { t } = useI18n();
  const ipc = useIpc();
  const [mode, setMode] = useState<"password" | "passphrase">("password");
  const [password, setPassword] = useState(PASSWORD);
  const [phrase, setPhrase] = useState(PASSPHRASE);
  const [round, setRound] = useState(0);
  const [result, setResult] = useState<Generated | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const request = useMemo(() => (mode === "password" ? password : phrase), [mode, password, phrase]);

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

  return (
    <section className="stack" aria-label={t("generator.title")}>
      <h1 className="title">{t("generator.title")}</h1>
      <Select
        label={t("generator.mode")}
        value={mode}
        options={[
          { value: "password", label: t("generator.mode.password") },
          { value: "passphrase", label: t("generator.mode.passphrase") },
        ]}
        onChange={(event) => {
          setMode(event.target.value === "passphrase" ? "passphrase" : "password");
        }}
      />
      {mode === "password" ? (
        <PasswordOptions value={password} onChange={setPassword} />
      ) : (
        <PassphraseOptions value={phrase} onChange={setPhrase} />
      )}
      <ErrorAlert error={error} />
      {result !== null && (
        <div className="generated">
          <p className="generated__value mono" aria-label={t("generator.result")}>
            {result.value}
          </p>
          <StrengthMeter strength={result.strength} bits={result.entropyBits} />
        </div>
      )}
      <div className="actions">
        <Button
          onClick={() => {
            setRound((n) => n + 1);
          }}
        >
          {t("action.generate")}
        </Button>
        <CopyButton source={{ kind: "generated" }} field={t("generator.result")} />
      </div>
    </section>
  );
}

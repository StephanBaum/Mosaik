/**
 * Comparing a safety code (CRYPTO_SPEC §8.1): the code Mosaic shows for the person, side by side
 * with a field for the code they read out. Typing is optional and only a help (the code is a
 * public value); the attestation is the click on "It matches". The backend checks the code
 * against the pinned key again, so a key that moved meanwhile is refused.
 */
import { useId, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";
import { groupCode, normalizeCode, sameCode } from "../../ipc/safetyCode.ts";
import { isolate } from "../sync/hooks.ts";

const CODE_LENGTH = 26;

/** A safety code in its groups, in the mono face, readable by assistive technology as one text. */
export function CodeDisplay({ code, labelledBy }: { code: string; labelledBy: string }) {
  return (
    <p className="safety__code mono" aria-labelledby={labelledBy}>
      {groupCode(code).map((group, index) => (
        <span key={index} className="safety__group">
          {group}
        </span>
      ))}
    </p>
  );
}

type Typed = "empty" | "partial" | "match" | "mismatch";

function typedState(code: string, typed: string): Typed {
  const length = normalizeCode(typed).length;
  if (length === 0) {
    return "empty";
  }
  if (length < CODE_LENGTH) {
    return "partial";
  }
  return sameCode(code, typed) ? "match" : "mismatch";
}

export function SafetyCodePanel({
  email,
  code,
  busy,
  error,
  onMatches,
  onCancel,
  codeLabel,
}: {
  email: string;
  code: string;
  busy: boolean;
  error: IpcError | null;
  /** The user compared the codes and they match. */
  onMatches: () => void;
  onCancel: () => void;
  /** A different caption for the shown code (the owner's code of an invite). */
  codeLabel?: string;
}) {
  const { t } = useI18n();
  const [typed, setTyped] = useState("");
  const headId = useId();
  const shownId = useId();
  const state = typedState(code, typed);
  const who = isolate(email);
  return (
    <section className="safety stack" aria-labelledby={headId}>
      <h3 id={headId} className="subtitle">
        {t("sharing.code.title")}
      </h3>
      <p className="body muted">{t("sharing.code.compare", { email: who })}</p>
      <div className="safety__sides">
        <div className="safety__side">
          <p id={shownId} className="row__label">
            {codeLabel ?? t("sharing.code.shown", { email: who })}
          </p>
          <CodeDisplay code={code} labelledBy={shownId} />
        </div>
        <div className="safety__side">
          <TextField
            label={t("sharing.code.typed")}
            mono
            autoComplete="off"
            spellCheck={false}
            value={typed}
            onChange={(event) => {
              setTyped(event.target.value);
            }}
          />
        </div>
      </div>
      {state === "match" && (
        <p className="field__hint" role="status">
          {t("sharing.code.typedMatch")}
        </p>
      )}
      {state === "mismatch" && (
        <Callout tone="warning">
          <p className="body">{t("sharing.code.mismatch", { email: who })}</p>
          <p className="body">{t("sharing.code.mismatchHelp", { email: who })}</p>
        </Callout>
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton disabled={busy || state === "mismatch"} onClick={onMatches}>
          {t("sharing.code.matches")}
        </HoloButton>
      </div>
    </section>
  );
}

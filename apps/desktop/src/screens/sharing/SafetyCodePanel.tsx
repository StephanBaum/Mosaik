/**
 * Comparing a safety code (CRYPTO_SPEC §8.1): the code Mosaic shows for the person, side by side
 * with a field for the code they read out. Typing is optional and only a help (the code is a
 * public value); the attestation is the checkbox "I compared this code…" (or a typed code that
 * matches), which "Codes match" waits for, so the button is no reflex click. The backend checks
 * the code against the pinned key again, so a key that moved meanwhile is refused.
 */
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";
import { groupCode, normalizeCode, sameCode } from "../../ipc/safetyCode.ts";
import { isolate } from "../sync/hooks.ts";
import { StepHeading } from "./StepHeading.tsx";

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

/** The code Mosaic shows next to the field for the code the person reads out. */
function CodeSides({
  code,
  caption,
  typed,
  onTyped,
}: {
  code: string;
  caption: string;
  typed: string;
  onTyped: (typed: string) => void;
}) {
  const { t } = useI18n();
  const shownId = useId();
  return (
    <div className="safety__sides">
      <div className="safety__side">
        <p id={shownId} className="row__label safety__text">
          {caption}
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
            onTyped(event.target.value);
          }}
        />
      </div>
    </div>
  );
}

/** What the typed code says: it matches, or it differs (announced at once, nothing is shared). */
function TypedVerdict({ state, who }: { state: Typed; who: string }) {
  const { t } = useI18n();
  if (state === "match") {
    return (
      <p className="field__hint" role="status">
        {t("sharing.code.typedMatch")}
      </p>
    );
  }
  return state === "mismatch" ? (
    <Callout tone="warning" role="alert">
      <p className="body safety__text">{t("sharing.code.mismatch", { email: who })}</p>
      <p className="body safety__text">{t("sharing.code.mismatchHelp", { email: who })}</p>
    </Callout>
  ) : null;
}

export function SafetyCodePanel({
  email,
  code,
  busy,
  error,
  onMatches,
  onCancel,
  codeLabel,
  heading,
  intro,
  before,
  hint,
  matchLabel,
  focusCancel = false,
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
  /** A different heading (a key change). */
  heading?: string;
  /** A different instruction under the heading (a key change). */
  intro?: string;
  /** Notes between the heading and the instruction (what a key change can mean). */
  before?: ReactNode;
  /** A note above the buttons. */
  hint?: string;
  /** A different label for the confirming button. */
  matchLabel?: string;
  /** Starts with focus on "Cancel", so Enter can't confirm (a key change). */
  focusCancel?: boolean;
}) {
  const { t } = useI18n();
  const [typed, setTyped] = useState("");
  const [attested, setAttested] = useState(false);
  const headId = useId();
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (focusCancel) {
      cancelRef.current?.focus();
    }
  }, [focusCancel]);
  const state = typedState(code, typed);
  const who = isolate(email);
  const confirmed = attested || state === "match";
  return (
    <section className="safety stack" aria-labelledby={headId}>
      <StepHeading id={headId} focus={!focusCancel}>
        {heading ?? t("sharing.code.title")}
      </StepHeading>
      {before}
      <p className="body muted safety__text">{intro ?? t("sharing.code.compare", { email: who })}</p>
      <CodeSides
        code={code}
        caption={codeLabel ?? t("sharing.code.shown", { email: who })}
        typed={typed}
        onTyped={setTyped}
      />
      <TypedVerdict state={state} who={who} />
      <label className="check check--top">
        <input
          type="checkbox"
          checked={attested}
          onChange={(event) => {
            setAttested(event.target.checked);
          }}
        />
        <span className="safety__text">{t("sharing.code.attest", { email: who })}</span>
      </label>
      {hint !== undefined && <p className="field__hint">{hint}</p>}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button ref={cancelRef} onClick={onCancel}>
          {t("action.cancel")}
        </Button>
        <HoloButton disabled={busy || state === "mismatch" || !confirmed} onClick={onMatches}>
          {matchLabel ?? t("sharing.code.matches")}
        </HoloButton>
      </div>
    </section>
  );
}

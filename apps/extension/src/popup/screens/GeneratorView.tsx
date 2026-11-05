/**
 * E03: a password for the site in front. The desktop generates it (and keeps it for "Copy");
 * the background holds it for a Fill on this tab and origin only. Fill needs a new-password
 * field on the page, and a confirmed frame where that field is in a frame.
 */
import { useEffect, useMemo, useState } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import type { FrameRef, PopupResults, PopupState } from "../../shared/messages.ts";
import { useAnnounce } from "../components/Announcer.tsx";
import { Button, IconButton } from "../components/Button.tsx";
import { useFrameGate } from "../components/FrameGate.tsx";
import {
  DEFAULT_PASSPHRASE,
  DEFAULT_PASSWORD,
  PassphraseControls,
  RandomControls,
  TypeSwitch,
  type Mode,
  type PassphraseOptions,
  type PasswordOptions,
} from "../components/GeneratorOptions.tsx";
import { useI18n } from "../i18n.tsx";
import type { GenerateRequest } from "../../shared/protocol.ts";
import { useActions } from "../useActions.ts";
import { useServices } from "../usePopup.ts";

type Generated = PopupResults["generate"];
const STRENGTHS = ["veryWeak", "weak", "fair", "strong", "veryStrong"] as const;
const BARS = [0, 1, 2, 3] as const;
const REGENERATE_DELAY_MS = 250;

function Strength({ strength, bits }: { strength: string; bits: number }) {
  const { t } = useI18n();
  const level = STRENGTHS.findIndex((each) => each === strength);
  const known = STRENGTHS.find((each) => each === strength);
  return (
    <span className="strength">
      <span className="strength__bar" aria-hidden="true">
        {BARS.map((bar) => (
          <span key={bar} className={bar < level ? "strength__step is-on" : "strength__step"} />
        ))}
      </span>
      <span className="strength__label">
        {t("generator.strengthShort", {
          strength: known === undefined ? strength : t(`generator.strength.${known}`),
          bits: Math.round(bits),
        })}
      </span>
    </span>
  );
}

function requestFor(
  mode: Mode,
  touched: boolean,
  host: string | null,
  options: { password: PasswordOptions; phrase: PassphraseOptions },
): GenerateRequest {
  if (mode === "passphrase") {
    return { ...options.phrase };
  }
  return touched || host === null ? { ...options.password } : { kind: "forSite", host, rules: null };
}

/** The generated value, asked of the desktop again whenever the request or the round changes. */
function useGenerated(request: GenerateRequest, round: number): Generated | null {
  const { api } = useServices();
  const { errorText } = useI18n();
  const announce = useAnnounce();
  const [result, setResult] = useState<Generated | null>(null);
  useEffect(() => {
    let current = true;
    const timer = setTimeout(() => {
      api.request({ op: "generate", request }).then(
        (generated) => {
          if (current) {
            setResult(generated);
          }
        },
        (error: unknown) => {
          if (current) {
            announce(errorText(errorCodeOf(error)));
          }
        },
      );
    }, REGENERATE_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [api, request, round, announce, errorText]);
  return result;
}

/** Fill the shown password into the page; a password that is no longer held is replaced, not substituted silently. */
function useFillGenerated(onExpired: () => void): (confirmFrame: FrameRef | null) => Promise<void> {
  const { t } = useI18n();
  const { api, env, refresh } = useServices();
  const actions = useActions();
  const announce = useAnnounce();
  return async (confirmFrame) => {
    try {
      await api.request({ op: "fillGenerated", confirmFrame });
      announce(t("ext.generator.filled"));
      await env.closePopup();
    } catch (error) {
      if (errorCodeOf(error) === "generatedExpired") {
        announce(t("ext.generator.expired"));
        onExpired();
      } else {
        actions.fail(error);
      }
      await refresh();
    }
  };
}

function GeneratedValue({
  result,
  showRules,
  onRegenerate,
}: {
  result: Generated | null;
  showRules: boolean;
  onRegenerate: () => void;
}) {
  const { t } = useI18n();
  const rules = result?.rulesSource ?? null;
  return (
    <>
      <div className="generated-box">
        {/* Not an <output>: that is a live region, and the password must never be announced. */}
        <span className="generated" data-secret="" role="group" aria-label={t("ext.generator.value")}>
          {result?.value ?? ""}
        </span>
        <IconButton icon="refresh" label={t("ext.generator.regenerate")} onClick={onRegenerate} />
      </div>
      <div className="meta">
        {result !== null && <Strength strength={result.strength} bits={result.entropyBits} />}
        {rules !== null && showRules && <span className="chip">{t(`ext.generator.rules.${rules}`)}</span>}
      </div>
    </>
  );
}

export function GeneratorView({ state }: { state: PopupState }) {
  const { t } = useI18n();
  const actions = useActions();
  const gate = useFrameGate(state);
  const [mode, setMode] = useState<Mode>("random");
  // Until an option is touched the desktop's rules for this site (and the page's own) decide.
  const [touched, setTouched] = useState(false);
  const [password, setPassword] = useState(DEFAULT_PASSWORD);
  const [phrase, setPhrase] = useState(DEFAULT_PASSPHRASE);
  const [round, setRound] = useState(0);
  const host = state.tab?.host ?? null;
  const request = useMemo(
    () => requestFor(mode, touched, host, { password, phrase }),
    [mode, touched, host, password, phrase],
  );
  const result = useGenerated(request, round);
  const fill = useFillGenerated(() => {
    setRound((each) => each + 1);
  });

  return (
    <section className="section" aria-labelledby="gen-title">
      <h1 id="gen-title" className="title">
        {host === null ? t("ext.generator.titleGeneric") : t("ext.generator.title", { host })}
      </h1>
      <GeneratedValue
        result={result}
        showRules={!touched && mode === "random"}
        onRegenerate={() => {
          setRound((each) => each + 1);
        }}
      />
      <TypeSwitch mode={mode} onMode={setMode} />
      {mode === "random" ? (
        <RandomControls
          value={password}
          onChange={(next) => {
            setTouched(true);
            setPassword(next);
          }}
        />
      ) : (
        <PassphraseControls value={phrase} onChange={setPhrase} />
      )}
      <div className="actions">
        <Button
          variant="primary"
          disabled={!state.generatorFill || gate.blocked || result === null}
          onClick={() => {
            gate.gate(fill);
          }}
        >
          {t("ext.generator.fill")}
        </Button>
        <Button disabled={result === null} onClick={() => void actions.copy({ kind: "generated" })}>
          {t("ext.generator.copy")}
        </Button>
      </div>
      {!state.generatorFill && <p className="caption">{t("ext.generator.fillHint")}</p>}
      {gate.dialog}
    </section>
  );
}

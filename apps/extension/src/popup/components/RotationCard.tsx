/**
 * The password change on this site: one card per open attempt (at most five) for a login saved for
 * this exact address, and the offer to add a related address the tab was sent to. The card shows
 * no secret: the new password is shown only in the Mosaic app, where the user allowed it.
 * Every button is a click in this popup, which is what lets the background fill anything. A
 * frame of another site is never filled (`gate.blocked`), and a same-origin frame needs the
 * frame confirmation (`gate.gate`).
 */
import { useState } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import type { PopupState, RotationTabOffer } from "../../shared/messages.ts";
import type { RotationMatch } from "../../shared/protocol.ts";
import { useI18n } from "../i18n.tsx";
import { useServices } from "../usePopup.ts";
import { useAnnounce } from "./Announcer.tsx";
import { Avatar, Button } from "./Button.tsx";
import type { FrameGate } from "./FrameGate.tsx";

type Busy = "open" | "fillChange" | "fillPending" | "result" | "fresh";

function startedText(startedAt: number, locale: string): string {
  try {
    return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(startedAt);
  } catch {
    return new Date(startedAt).toISOString();
  }
}

/** Runs one request at a time: says what happened, and refreshes the state (an answered attempt is gone). */
function useRunner() {
  const { errorText } = useI18n();
  const { refresh } = useServices();
  const announce = useAnnounce();
  const [busy, setBusy] = useState<Busy | null>(null);
  async function run(kind: Busy, action: () => Promise<string | null>): Promise<void> {
    setBusy(kind);
    try {
      const said = await action();
      if (said !== null) {
        announce(said);
      }
    } catch (error) {
      announce(errorText(errorCodeOf(error)));
    } finally {
      setBusy(null);
      await refresh();
    }
  }
  return { busy, run };
}

type Runner = ReturnType<typeof useRunner>;

/** The three buttons that act on the page: open the change page, fill the form, fill to sign in. */
function PageActions({ attempt, gate, runner }: { attempt: RotationMatch; gate: FrameGate; runner: Runner }) {
  const { t } = useI18n();
  const { api } = useServices();
  const { busy, run } = runner;
  const ids = { vaultId: attempt.vaultId, itemId: attempt.itemId, attemptId: attempt.attemptId };
  return (
    <div className="actions">
      <Button
        disabled={busy !== null}
        onClick={() => {
          void run("open", async () => {
            await api.request({ op: "rotationOpenPage", ...ids });
            return t("ext.rotation.opened");
          });
        }}
      >
        {t("ext.rotation.openPage")}
      </Button>
      <Button
        variant="primary"
        disabled={busy !== null || gate.blocked}
        onClick={() => {
          gate.gate((confirmFrame) =>
            run("fillChange", async () => {
              await api.request({ op: "fillChange", ...ids, confirmFrame });
              return t("ext.rotation.filled");
            }),
          );
        }}
      >
        {t("ext.rotation.fillChange")}
      </Button>
      <Button
        disabled={busy !== null || gate.blocked}
        onClick={() => {
          gate.gate((confirmFrame) =>
            run("fillPending", async () => {
              await api.request({ op: "fillPending", ...ids, confirmFrame });
              return t("ext.filled");
            }),
          );
        }}
      >
        {t("ext.rotation.fillPending")}
      </Button>
    </div>
  );
}

/** The buttons that say how it went (the Mosaic app decides) and the one that starts over. */
function ResultActions({ attempt, runner }: { attempt: RotationMatch; runner: Runner }) {
  const { t } = useI18n();
  const { api } = useServices();
  const { busy, run } = runner;
  const ids = { vaultId: attempt.vaultId, itemId: attempt.itemId, attemptId: attempt.attemptId };
  const answer = (outcome: "succeeded" | "failed") =>
    run("result", async () => {
      const result = await api.request({ op: "rotationResult", ...ids, outcome });
      if (result.outcome === "confirmed") {
        return t("ext.rotation.confirmed");
      }
      return t(result.outcome === "keptOld" ? "ext.rotation.keptOld" : "ext.rotation.resultCancelled");
    });
  const startAgain = () =>
    run("fresh", async () => {
      const result = await api.request({ op: "rotationBegin", vaultId: attempt.vaultId, itemId: attempt.itemId });
      return t(result.outcome === "started" ? "ext.rotation.started" : "ext.rotation.startCancelled");
    });
  return (
    <div className="actions">
      <Button
        disabled={busy !== null}
        onClick={() => {
          void answer("succeeded");
        }}
      >
        {t("ext.rotation.worked")}
      </Button>
      <Button
        disabled={busy !== null}
        onClick={() => {
          void answer("failed");
        }}
      >
        {t("ext.rotation.failed")}
      </Button>
      <Button
        variant="link"
        disabled={busy !== null}
        onClick={() => {
          void startAgain();
        }}
      >
        {t("ext.rotation.fresh")}
      </Button>
    </div>
  );
}

function AttemptCard({ attempt, gate }: { attempt: RotationMatch; gate: FrameGate }) {
  const { t, locale } = useI18n();
  const runner = useRunner();
  const name = attempt.username ?? attempt.title ?? t("ext.item.untitled");
  const waiting = runner.busy === "result" || runner.busy === "fresh";
  return (
    <section className="card" aria-labelledby={`rotation-${attempt.attemptId}`}>
      <div className="card__head">
        <Avatar text={attempt.title ?? name} />
        <h2 id={`rotation-${attempt.attemptId}`} className="title">
          {t("ext.rotation.card.title")}
        </h2>
      </div>
      <p className="muted">{t("ext.rotation.card.for", { name: attempt.title ?? name })}</p>
      <dl className="facts">
        <dt>{t("ext.save.username")}</dt>
        <dd>{attempt.username ?? t("ext.save.noUsername")}</dd>
      </dl>
      <p className="caption">{t("ext.rotation.card.started", { when: startedText(attempt.startedAt, locale) })}</p>
      {attempt.stage === "delivered" && <p className="caption">{t("ext.rotation.card.delivered")}</p>}
      <p className="caption">{t("ext.rotation.card.note")}</p>
      {waiting && (
        <p className="caption" role="status">
          {t("ext.rotation.waiting")}
        </p>
      )}
      <PageActions attempt={attempt} gate={gate} runner={runner} />
      <ResultActions attempt={attempt} runner={runner} />
    </section>
  );
}

/** "This page is on accounts.example.com, which isn't saved for this login. Add it?" The desktop decides. */
function OfferCard({ offer }: { offer: RotationTabOffer }) {
  const { t, errorText } = useI18n();
  const { api, refresh } = useServices();
  const announce = useAnnounce();
  const [waiting, setWaiting] = useState(false);
  const host = offer.unicodeHost ?? offer.host;
  const title = offer.title ?? t("ext.item.untitled");

  async function add(): Promise<void> {
    setWaiting(true);
    try {
      const { outcome } = await api.request({ op: "rotationAddOrigin" });
      announce(t(outcome === "added" ? "ext.rotation.offer.added" : "ext.rotation.offer.cancelled"));
    } catch (error) {
      announce(errorText(errorCodeOf(error)));
    } finally {
      setWaiting(false);
      await refresh();
    }
  }

  return (
    <section className="card" aria-labelledby="rotation-offer">
      <div className="card__head">
        <Avatar text={host} />
        <h2 id="rotation-offer" className="title">
          {t("ext.rotation.offer.title")}
        </h2>
      </div>
      <p className="muted">{t("ext.rotation.offer.body", { host, title })}</p>
      {waiting && (
        <p className="caption" role="status">
          {t("ext.rotation.waiting")}
        </p>
      )}
      <div className="actions">
        <Button
          variant="primary"
          disabled={waiting}
          onClick={() => {
            void add();
          }}
        >
          {t("ext.rotation.offer.add", { host })}
        </Button>
      </div>
    </section>
  );
}

export function RotationCards({ state, gate }: { state: PopupState; gate: FrameGate }) {
  return (
    <>
      {state.rotations.map((attempt) => (
        <AttemptCard key={attempt.attemptId} attempt={attempt} gate={gate} />
      ))}
      {state.rotationTabOffer !== null && <OfferCard offer={state.rotationTabOffer} />}
    </>
  );
}

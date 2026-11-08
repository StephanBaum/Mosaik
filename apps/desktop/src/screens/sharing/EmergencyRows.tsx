/** The two lists of emergency access: people who can ask (grantor) and people who named this account (grantee). */
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { UserText } from "../../components/UserText.tsx";
import { formatRelative } from "../../i18n/format.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { GrantedView, ReceivedView } from "../../ipc/contract/index.ts";
import { isolate } from "../sync/hooks.ts";
import { useFormatWhen } from "./hooks.ts";

function GrantedRow({ grant, onRemove }: { grant: GrantedView; onRemove: () => void }) {
  const { t } = useI18n();
  const formatWhen = useFormatWhen();
  const email = isolate(grant.email);
  const status = {
    waiting: t("emergency.granted.waitingUntil", { when: formatWhen(grant.releaseAt ?? 0) }),
    approved: t("emergency.granted.approved"),
    released: t("emergency.granted.released"),
    idle: t("emergency.granted.idle"),
  }[grant.state];
  return (
    <li className="row">
      <div className="row__main">
        <span className="row__value">
          <UserText>{grant.email}</UserText>
        </span>
        <span className="row__meta">
          <Chip>
            {grant.scope.kind === "all"
              ? t("emergency.granted.scopeAll")
              : t("emergency.granted.scopeSome", { count: grant.scope.vaultIds.length })}
          </Chip>
          <Chip>{t("emergency.granted.wait", { days: grant.waitDays })}</Chip>
        </span>
        <p className="field__hint" role={grant.state === "waiting" ? "status" : undefined}>
          {status}
        </p>
      </div>
      <Button
        size="sm"
        variant="danger"
        aria-label={`${t("emergency.granted.remove")}: ${email}`}
        onClick={onRemove}
      >
        {t("emergency.granted.remove")}
      </Button>
    </li>
  );
}

/** Where a received grant stands at `now`: asking again is on hold, the wait runs, or it is open. */
function standing(grant: ReceivedView, now: number) {
  const cooling = grant.cooldownUntil !== null && grant.cooldownUntil > now;
  const over = grant.releaseAt !== null && grant.releaseAt <= now;
  const open = grant.state === "released" || grant.state === "approved" || (grant.state === "waiting" && over);
  return { cooling, open, waiting: grant.state === "waiting" && !open };
}

/** What a received grant says about itself: how to ask, how long, what the other side did. */
function ReceivedNotes({ grant, now, needsVerify }: { grant: ReceivedView; now: number; needsVerify: boolean }) {
  const { t, locale } = useI18n();
  const email = isolate(grant.email);
  const { cooling, waiting } = standing(grant, now);
  return (
    <>
      {grant.state === "idle" && !cooling && (
        <p className="field__hint">{t("emergency.received.requestHint", { email, days: grant.waitDays })}</p>
      )}
      {cooling && grant.cooldownUntil !== null && (
        <p className="field__hint">
          {t("emergency.received.cooldown", { email, when: formatRelative(locale, grant.cooldownUntil, now) })}
        </p>
      )}
      {waiting && (
        <p className="field__hint" role="status">
          {t("emergency.received.countdown", { when: formatRelative(locale, grant.releaseAt ?? now, now) })}
        </p>
      )}
      {grant.state === "approved" && <p className="field__hint">{t("emergency.received.approved", { email })}</p>}
      {needsVerify && <p className="field__hint">{t("emergency.received.verifyFirst", { email })}</p>}
    </>
  );
}

function ReceivedRow({
  grant,
  now,
  busy,
  needsVerify,
  onRequest,
  onClaim,
  onVerify,
}: {
  grant: ReceivedView;
  now: number;
  busy: boolean;
  needsVerify: boolean;
  onRequest: () => void;
  onClaim: () => void;
  onVerify: () => void;
}) {
  const { t } = useI18n();
  const email = isolate(grant.email);
  const { cooling, open } = standing(grant, now);
  return (
    <li className="row">
      <div className="row__main">
        <span className="row__value">
          <UserText>{grant.email}</UserText>
        </span>
        <span className="row__meta">
          <Chip>{t("emergency.received.waitDays", { days: grant.waitDays })}</Chip>
        </span>
        <ReceivedNotes grant={grant} now={now} needsVerify={needsVerify} />
      </div>
      <div className="actions">
        {needsVerify && (
          <Button size="sm" onClick={onVerify} aria-label={`${t("emergency.received.verify")} ${email}`}>
            {t("emergency.received.verify")}
          </Button>
        )}
        {grant.state === "idle" && (
          <Button
            size="sm"
            disabled={busy || cooling}
            onClick={onRequest}
            aria-label={`${t("emergency.received.request")}: ${email}`}
          >
            {t("emergency.received.request")}
          </Button>
        )}
        {open && (
          <Button size="sm" disabled={busy} onClick={onClaim} aria-label={`${t("emergency.received.claim")}: ${email}`}>
            {t("emergency.received.claim")}
          </Button>
        )}
      </div>
    </li>
  );
}

/** People who can ask for access to this account's vaults. */
export function GrantedSection({
  granted,
  onAdd,
  onRemove,
}: {
  granted: readonly GrantedView[];
  onAdd: () => void;
  onRemove: (grant: GrantedView) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <h3 className="subtitle">{t("emergency.granted.title")}</h3>
      {granted.length === 0 ? (
        <p className="body muted">{t("emergency.granted.empty")}</p>
      ) : (
        <ul className="rows">
          {granted.map((grant) => (
            <GrantedRow
              key={grant.granteeAccountId}
              grant={grant}
              onRemove={() => {
                onRemove(grant);
              }}
            />
          ))}
        </ul>
      )}
      <div className="setting-row">
        <Button onClick={onAdd}>{t("emergency.granted.add")}</Button>
      </div>
    </>
  );
}

/** People who named this account. */
export function ReceivedSection({
  received,
  now,
  busy,
  unverified,
  onVerify,
  onRequest,
  onClaim,
}: {
  received: readonly ReceivedView[];
  now: number;
  busy: boolean;
  /** The grantors whose claim needs their safety code compared first. */
  unverified: ReadonlySet<string>;
  onVerify: (email: string) => void;
  onRequest: (grant: ReceivedView) => void;
  onClaim: (grant: ReceivedView) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <h3 className="subtitle">{t("emergency.received.title")}</h3>
      {received.length === 0 ? (
        <p className="body muted">{t("emergency.received.empty")}</p>
      ) : (
        <ul className="rows">
          {received.map((grant) => (
            <ReceivedRow
              key={grant.grantorAccountId}
              grant={grant}
              now={now}
              busy={busy}
              needsVerify={unverified.has(grant.grantorAccountId)}
              onVerify={() => {
                onVerify(grant.email);
              }}
              onRequest={() => {
                onRequest(grant);
              }}
              onClaim={() => {
                onClaim(grant);
              }}
            />
          ))}
        </ul>
      )}
    </>
  );
}

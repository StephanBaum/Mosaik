/**
 * E05: what the desktop found odd about the address. There is no "Fill anyway": the only way to
 * fill an address the saved login does not match is to change the login in the Mosaic app.
 */
import type { OriginWarning } from "../../shared/protocol.ts";
import { useI18n } from "../i18n.tsx";
import { Icon } from "./Icon.tsx";

function Idn({ unicodeHost, host }: { unicodeHost: string; host: string }) {
  const { t } = useI18n();
  return (
    <div role="alert" className="callout callout--warning">
      <Icon name="shieldAlert" />
      <div>
        <h2 className="title title--sm">{t("ext.warning.idn.title")}</h2>
        <p>{t("ext.warning.idn.body")}</p>
        <dl className="facts facts--mono">
          <dt>{t("ext.warning.youAreOn")}</dt>
          <dd>{unicodeHost}</dd>
          <dt>{t("ext.warning.writtenAs")}</dt>
          <dd>{host}</dd>
        </dl>
      </div>
    </div>
  );
}

function Lookalike({ savedHost, host }: { savedHost: string; host: string }) {
  const { t } = useI18n();
  return (
    <div role="alert" className="callout callout--warning">
      <Icon name="shieldAlert" />
      <div>
        <h2 className="title title--sm">{t("ext.warning.idn.title")}</h2>
        <p>{t("ext.warning.lookalike.body", { saved: savedHost })}</p>
        <dl className="facts facts--mono">
          <dt>{t("ext.warning.youAreOn")}</dt>
          <dd>{host}</dd>
          <dt>{t("ext.warning.savedFor")}</dt>
          <dd>{savedHost}</dd>
        </dl>
        <p className="caption">{t("ext.warning.noFill")}</p>
      </div>
    </div>
  );
}

export function Warnings({
  warnings,
  host,
  unicodeHost,
}: {
  warnings: readonly OriginWarning[];
  host: string;
  /** The address as the extension decodes it; the desktop's own form is the fallback. */
  unicodeHost: string | null;
}) {
  const { t } = useI18n();
  return (
    <>
      {warnings.map((warning) => {
        switch (warning.kind) {
          case "idn":
            return <Idn key="idn" unicodeHost={unicodeHost ?? warning.unicodeHost} host={host} />;
          case "lookalike":
            return <Lookalike key={`lookalike-${warning.savedHost}`} savedHost={warning.savedHost} host={host} />;
          case "insecure":
            return (
              <div key="insecure" role="alert" className="callout callout--warning">
                <Icon name="alert" />
                <div>
                  <h2 className="title title--sm">{t("ext.warning.insecure.title")}</h2>
                  <p>{t("ext.warning.insecure.body")}</p>
                </div>
              </div>
            );
        }
      })}
    </>
  );
}

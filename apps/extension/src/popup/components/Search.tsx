/** E02, "Other items": a search of the vault. Hits are summaries (no secret); the page never sees them. */
import { useEffect, useId, useState } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import type { CopySource, SearchHit } from "../../shared/protocol.ts";
import { useI18n } from "../i18n.tsx";
import { useServices } from "../usePopup.ts";
import { useAnnounce } from "./Announcer.tsx";
import { Icon } from "./Icon.tsx";
import { ItemRow, type RowItem } from "./ItemRow.tsx";

const DEBOUNCE_MS = 250;

function toRow(hit: SearchHit): RowItem {
  return {
    vaultId: hit.vaultId,
    itemId: hit.itemId,
    title: hit.title,
    username: hit.username,
    hasOtp: hit.hasOtp,
    fillable: hit.fillable,
    blocked: false,
    sameSite: false,
  };
}

/** The hits for the trimmed query; null while the box is empty or the answer is still on its way. */
function useSearch(query: string): readonly SearchHit[] | null {
  const { api } = useServices();
  const { errorText } = useI18n();
  const announce = useAnnounce();
  const trimmed = query.trim();
  const [answer, setAnswer] = useState<{ query: string; items: readonly SearchHit[] } | null>(null);
  useEffect(() => {
    if (trimmed === "") {
      return undefined;
    }
    let current = true;
    const timer = setTimeout(() => {
      api.request({ op: "search", query: trimmed }).then(
        (result) => {
          if (current) {
            setAnswer({ query: trimmed, items: result.items });
          }
        },
        (error: unknown) => {
          if (current) {
            announce(errorText(errorCodeOf(error)));
          }
        },
      );
    }, DEBOUNCE_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [api, trimmed, announce, errorText]);
  return answer !== null && answer.query === trimmed ? answer.items : null;
}

export function Search({
  fillOff,
  onFill,
  onCopy,
}: {
  fillOff: boolean;
  onFill: (item: RowItem) => void;
  onCopy: (source: CopySource) => void;
}) {
  const { t } = useI18n();
  const inputId = useId();
  const [query, setQuery] = useState("");
  const hits = useSearch(query);
  return (
    <section className="search" aria-labelledby={`${inputId}-label`}>
      <h2 id={`${inputId}-label`} className="label">
        <label htmlFor={inputId}>{t("ext.search.label")}</label>
      </h2>
      <label className="search__box">
        <Icon name="search" />
        <input
          id={inputId}
          type="search"
          autoComplete="off"
          placeholder={t("ext.search.placeholder")}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
      </label>
      {hits === null ? (
        <p className="caption">{t("ext.search.note")}</p>
      ) : hits.length === 0 ? (
        <p className="caption" role="status">
          {t("ext.search.none")}
        </p>
      ) : (
        <>
          <p className="visually-hidden" role="status">
            {t("ext.search.results", { count: hits.length })}
          </p>
          <ul className="items">
            {hits.map((hit) => (
              <ItemRow
                key={`${hit.vaultId}/${hit.itemId}`}
                item={toRow(hit)}
                primary={false}
                fillOff={fillOff}
                onFill={() => {
                  onFill(toRow(hit));
                }}
                onCopy={onCopy}
              />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

/**
 * What a click does: ask the background, say the outcome in the live region, refresh. Errors
 * never show codes or page data; they show the translated "what happened and what to do".
 */
import { useMemo } from "react";
import { errorCodeOf } from "../shared/errors.ts";
import type { FrameRef } from "../shared/messages.ts";
import type { CopySource } from "../shared/protocol.ts";
import { useAnnounce } from "./components/Announcer.tsx";
import { useI18n } from "./i18n.tsx";
import { useServices } from "./usePopup.ts";

export interface ItemKey {
  vaultId: string;
  itemId: string;
}

export interface Actions {
  fill: (item: ItemKey, confirm: FrameRef | null) => Promise<void>;
  fillOtp: (item: ItemKey, confirm: FrameRef | null) => Promise<void>;
  copy: (source: CopySource) => Promise<void>;
  openDesktop: () => Promise<void>;
  /** Announces an error code's text. */
  fail: (error: unknown) => void;
}

export function useActions(): Actions {
  const { api, env, refresh } = useServices();
  const { t, errorText } = useI18n();
  const announce = useAnnounce();

  return useMemo<Actions>(() => {
    const fail = (error: unknown): void => {
      announce(errorText(errorCodeOf(error)));
    };
    const filled = async (done: Promise<unknown>, said: "ext.filled" | "ext.filledCode"): Promise<void> => {
      try {
        await done;
        announce(t(said));
        await env.closePopup();
      } catch (error) {
        fail(error);
        await refresh();
      }
    };
    return {
      fill: (item, confirmFrame) => filled(api.request({ op: "fill", vaultId: item.vaultId, itemId: item.itemId, confirmFrame }), "ext.filled"),
      fillOtp: (item, confirmFrame) =>
        filled(api.request({ op: "fillOtp", vaultId: item.vaultId, itemId: item.itemId, confirmFrame }), "ext.filledCode"),
      copy: async (source) => {
        try {
          const { clearAfterSeconds } = await api.request({ op: "copy", source });
          announce(
            clearAfterSeconds === null || clearAfterSeconds === 0
              ? t("ext.copiedNoClear")
              : t("ext.copied", { seconds: clearAfterSeconds }),
          );
        } catch (error) {
          fail(error);
        }
      },
      openDesktop: async () => {
        try {
          await api.request({ op: "focusDesktop" });
        } catch (error) {
          fail(error);
        }
      },
      fail,
    };
  }, [api, env, refresh, announce, t, errorText]);
}

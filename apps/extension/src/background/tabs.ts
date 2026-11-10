/** The browser side of a fill: injecting the content script, probing frames, delivering a credential. */
import { ExtensionError } from "../shared/errors.ts";
import { CONTENT_SCRIPT } from "../shared/ids.ts";
import {
  isFillReply,
  isProbeResult,
  type ContentMessage,
  type FillReply,
  type FrameProbe,
} from "../shared/messages.ts";

export interface FrameTarget {
  frameId: number;
  /** The document the probe saw; delivery fails if the frame has navigated since (Chromium). */
  documentId: string | null;
}

export interface TabApi {
  inject(tabId: number): Promise<void>;
  probe(tabId: number): Promise<FrameProbe[]>;
  send(tabId: number, frame: FrameTarget, message: ContentMessage): Promise<FillReply>;
  /** Sends the tab to `url` (an https address the caller checked). Injects nothing. */
  navigate(tabId: number, url: string): Promise<void>;
}

export function chromeTabs(): TabApi {
  return {
    inject: async (tabId) => {
      try {
        await chrome.scripting.executeScript({ target: { tabId, allFrames: true }, files: [CONTENT_SCRIPT] });
      } catch {
        throw new ExtensionError("unsupportedPage");
      }
    },
    probe: async (tabId) => {
      const results = await chrome.scripting.executeScript({
        target: { tabId, allFrames: true },
        func: () => globalThis.mosaicContent?.probe(),
      });
      return results.flatMap((entry) =>
        isProbeResult(entry.result)
          ? [{ ...entry.result, frameId: entry.frameId, documentId: typeof entry.documentId === "string" ? entry.documentId : null }]
          : [],
      );
    },
    send: async (tabId, frame, message) => {
      const options: chrome.tabs.MessageSendOptions =
        frame.documentId === null
          ? { frameId: frame.frameId }
          : { frameId: frame.frameId, documentId: frame.documentId };
      let reply: unknown;
      try {
        reply = await chrome.tabs.sendMessage(tabId, message, options);
      } catch {
        // The document is gone or never had the script: the page navigated since the probe.
        throw new ExtensionError("originChanged");
      }
      if (!isFillReply(reply)) {
        throw new ExtensionError("internal");
      }
      return reply;
    },
    navigate: async (tabId, url) => {
      await chrome.tabs.update(tabId, { url });
    },
  };
}

/** The tab the popup was opened on. */
export async function activeTab(): Promise<{ id: number; url: string | undefined } | null> {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab?.id === undefined ? null : { id: tab.id, url: tab.url };
}

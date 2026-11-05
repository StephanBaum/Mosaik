import type { ProbeResult } from "./shared/messages.ts";

declare global {
  /** Set by the content script (isolated world); its presence makes injection idempotent. */
  var mosaicContent: { probe(): ProbeResult } | undefined;
}

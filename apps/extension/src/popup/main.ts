/**
 * Placeholder for the popup (M4 track T4 renders the UI). It loads the design tokens and opens
 * the port the background's router expects, so the build and the sender rules have a real page.
 */
import "virtual:mosaic-tokens.css";
import { POPUP_PORT } from "../shared/messages.ts";

const port = chrome.runtime.connect({ name: POPUP_PORT });
port.postMessage({ id: 1, request: { op: "getState" } });

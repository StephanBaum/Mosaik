/**
 * The popup's entry point. Fonts are bundled (SIL OFL 1.1, ADR-0014); nothing loads from a CDN.
 * It opens the port the background's router expects (`popup`, from this page only).
 */
import "@fontsource/dm-sans/latin-400.css";
import "@fontsource/dm-sans/latin-500.css";
import "@fontsource/dm-sans/latin-600.css";
import "@fontsource/dm-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "virtual:mosaic-tokens.css";
import "./popup.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { POPUP_PORT } from "../shared/messages.ts";
import { App } from "./App.tsx";
import { createPopupClient } from "./client.ts";
import { chromeEnv } from "./env.ts";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("popup.html has no #root");
}
const api = createPopupClient(chrome.runtime.connect({ name: POPUP_PORT }));
createRoot(container).render(
  <StrictMode>
    <App api={api} env={chromeEnv()} languages={navigator.languages} />
  </StrictMode>,
);

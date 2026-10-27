/** Entry point. Fonts are bundled (SIL OFL 1.1, ADR-0014); nothing loads from a CDN. */
import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/dm-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "virtual:mosaic-tokens.css";
import "./styles/app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";
import { loadIpc } from "./ipc/index.ts";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("index.html has no #root");
}
const root = createRoot(container);
void loadIpc().then((ipc) => {
  root.render(
    <StrictMode>
      <App ipc={ipc} />
    </StrictMode>,
  );
});

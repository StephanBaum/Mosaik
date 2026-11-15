/** Entry point of the list window (ADR-0164). */
import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/dm-sans/600.css";
import "virtual:mosaic-tokens.css";
import "../styles/app.css";
import "../styles/components.css";
import "./picker.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { loadPickerIpc } from "./ipc.ts";
import { PickerList } from "./PickerList.tsx";
import { PickerShell } from "./PickerShell.tsx";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("picker.html has no #root");
}
const root = createRoot(container);
void loadPickerIpc().then((ipc) => {
  root.render(
    <StrictMode>
      <PickerShell ipc={ipc}>{(state) => <PickerList state={state} ipc={ipc} />}</PickerShell>
    </StrictMode>,
  );
});

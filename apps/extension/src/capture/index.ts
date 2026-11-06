/**
 * The save capture script (ADR-0038). It exists only after the user turned "Offer to save
 * logins" on: the popup asked for the optional site access, and the background registered this
 * script. It watches for the user submitting a form (a click on its submit control, or Enter in
 * one of its boxes) and sends the typed login to the background, which asks the desktop;
 * nothing is written anywhere from here. Only events the browser raised for the user
 * (`isTrusted`) count, so a page cannot make it capture by dispatching events. The `submit`
 * event is deliberately not a trigger: the browser raises it as a trusted event for a script's
 * `requestSubmit()` and `button.click()` as well. The script adds nothing to the page and
 * listens to no page message.
 */
import type { CapturedMessage } from "../shared/messages.ts";
import { containerOf, extract, isTextEntry, mayReport, submitControl, Throttle } from "./capture.ts";

const throttle = new Throttle(Date.now);

function send(scope: Element | null): void {
  // An opaque document, or a frame the user cannot see or whose ancestors are another site's, reports nothing.
  if (scope === null || !mayReport(window) || !throttle.allow()) {
    return;
  }
  const found = extract(scope);
  if (found === null) {
    return;
  }
  const message: CapturedMessage = {
    type: "captured",
    ...found,
    ancestorsSameOrigin: true,
  };
  // The runtime copies the message when it is sent; nothing of it is kept here.
  chrome.runtime.sendMessage(message).catch(() => undefined);
  Object.assign(message, { username: null, password: "", newPassword: null });
}

const firstTarget = (event: Event): EventTarget | undefined => event.composedPath()[0];

function onClick(event: MouseEvent): void {
  const control = event.isTrusted ? submitControl(firstTarget(event)) : null;
  if (control !== null) {
    send(containerOf(control));
  }
}

function onKeyDown(event: KeyboardEvent): void {
  const target = firstTarget(event);
  if (event.isTrusted && event.key === "Enter" && isTextEntry(target)) {
    send(containerOf(target));
  }
}

// Once per world, even if the browser injects the script twice (a registered script and a page reload race).
if (!Object.hasOwn(globalThis, "mosaicCapture")) {
  Object.defineProperty(globalThis, "mosaicCapture", { value: true, enumerable: false });
  window.addEventListener("click", onClick, { capture: true });
  window.addEventListener("keydown", onKeyDown, { capture: true });
}

import { cleanup, configure } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// findBy*/waitFor default to 1 s, which the full parallel suite (30 jsdom workers) exceeds on slow machines.
configure({ asyncUtilTimeout: 5000 });

// jsdom lacks these; the app uses them, so tests get minimal stand-ins.
if (typeof HTMLDialogElement !== "undefined") {
  const proto = HTMLDialogElement.prototype as Partial<HTMLDialogElement>;
  proto.showModal ??= function showModal(this: HTMLDialogElement): void {
    this.setAttribute("open", "");
  };
  proto.close ??= function close(this: HTMLDialogElement): void {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
}

if (typeof window.matchMedia !== "function") {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }) as unknown as MediaQueryList;
}

if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  };
}

Element.prototype.scrollIntoView = (): void => undefined;
window.print = vi.fn();

afterEach(() => {
  cleanup();
});

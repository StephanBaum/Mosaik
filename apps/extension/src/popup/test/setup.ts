import { cleanup, configure } from "@testing-library/react";
import { afterEach } from "vitest";

// findBy*/waitFor default to 1 s, which a busy machine can exceed.
configure({ asyncUtilTimeout: 5000 });

// jsdom has no <dialog> behavior; the popup uses showModal and close.
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

afterEach(() => {
  cleanup();
});

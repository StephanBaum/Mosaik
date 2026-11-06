import { describe, expect, it } from "vitest";
import { ExtensionError } from "../shared/errors.ts";
import { createPopupClient } from "./client.ts";
import { baseState, fakePort } from "./test/harness.tsx";

describe("the popup client", () => {
  it("matches replies to requests by number, in any order", async () => {
    const port = fakePort(baseState());
    const posted: { id: number }[] = [];
    const original = port.postMessage;
    port.postMessage = (message) => {
      posted.push(message as { id: number });
    };
    const client = createPopupClient(port);
    const first = client.request({ op: "search", query: "a" });
    const second = client.request({ op: "search", query: "b" });
    expect(posted.map((message) => message.id)).toEqual([1, 2]);
    port.push({ id: 2, ok: true, result: { items: [] } });
    port.push({ id: 1, ok: true, result: { items: [{ marker: 1 }] } });
    await expect(second).resolves.toEqual({ items: [] });
    await expect(first).resolves.toEqual({ items: [{ marker: 1 }] });
    port.postMessage = original;
  });

  it("turns an error reply into an ExtensionError with the code, and an unknown code into internal", async () => {
    const port = fakePort(baseState());
    port.replies.unpair = () => {
      throw new Error("locked");
    };
    port.replies.pair = () => {
      throw new Error("somethingNew");
    };
    const client = createPopupClient(port);
    await expect(client.request({ op: "unpair" })).rejects.toEqual(new ExtensionError("locked"));
    await expect(client.request({ op: "pair" })).rejects.toEqual(new ExtensionError("internal"));
  });

  it("ignores replies nobody waits for and anything that is not a message", () => {
    const port = fakePort(baseState());
    createPopupClient(port);
    port.push(null);
    port.push("text");
    port.push({ id: 99, ok: true, result: {} });
    port.push({ event: "unknown" });
    expect(port.requests).toEqual([]);
  });

  it("delivers pushed events to subscribers until they stop", () => {
    const port = fakePort(baseState());
    const client = createPopupClient(port);
    const seen: unknown[] = [];
    const stop = client.subscribe((event) => seen.push(event));
    port.push({ event: "connection", connection: "locked" });
    port.push({ event: "pairCode", code: "4F7K2Q", expiresInSeconds: 120 });
    stop();
    port.push({ event: "connection", connection: "unlocked" });
    expect(seen).toEqual([
      { event: "connection", connection: "locked" },
      { event: "pairCode", code: "4F7K2Q", expiresInSeconds: 120 },
    ]);
  });

  it("fails what is in flight, and later requests, when the port closes", async () => {
    const port = fakePort(baseState());
    port.postMessage = () => undefined;
    const client = createPopupClient(port);
    const waiting = client.request({ op: "getState" });
    port.disconnect();
    await expect(waiting).rejects.toEqual(new ExtensionError("hostExited"));
    await expect(client.request({ op: "getState" })).rejects.toEqual(new ExtensionError("hostExited"));
  });
});

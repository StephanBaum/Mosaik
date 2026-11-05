import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ErrorCode } from "../shared/errors.ts";
import type { HostEvent } from "../shared/protocol.ts";
import { createMemoryStore } from "./db.ts";
import { NativeClient, connectionOf, disconnectCode, type LinkState, type NativePort } from "./native.ts";
import { PairingStore } from "./pairing.ts";

const PAIRING = { pairingId: "11".repeat(16), key: "22".repeat(32) };

interface Envelope {
  id: number;
  op: string;
  body: unknown;
}

class FakePort implements NativePort {
  sent: Envelope[] = [];
  disconnected = false;
  #messages: ((m: unknown) => void)[] = [];
  #drops: (() => void)[] = [];
  /** How to answer a request; undefined = stay silent. */
  answer: (request: Envelope) => unknown = () => undefined;

  postMessage(message: unknown): void {
    const request = message as Envelope;
    this.sent.push(request);
    const reply = this.answer(request);
    if (reply !== undefined) {
      queueMicrotask(() => {
        this.emit(reply);
      });
    }
  }
  disconnect(): void {
    this.disconnected = true;
  }
  onMessage = {
    addListener: (listener: (m: unknown) => void): void => {
      this.#messages.push(listener);
    },
  };
  onDisconnect = {
    addListener: (listener: () => void): void => {
      this.#drops.push(listener);
    },
  };
  emit(message: unknown): void {
    for (const listener of this.#messages) {
      listener(message);
    }
  }
  drop(): void {
    for (const listener of this.#drops) {
      listener();
    }
  }
}

const ok = (id: number, body: unknown): unknown => ({ v: 1, id, ok: true, body });
const fail = (id: number, error: string): unknown => ({ v: 1, id, ok: false, error });

function answerLikeHost(request: Envelope): unknown {
  if (request.op === "hello") {
    return ok(request.id, { desktop: "unlocked", paired: true });
  }
  if (request.op === "status") {
    return ok(request.id, { state: "unlocked" });
  }
  return undefined;
}

function setup(answer: (r: Envelope) => unknown = answerLikeHost, withPairing = true) {
  const ports: FakePort[] = [];
  const events: HostEvent[] = [];
  let lastError: string | undefined;
  const pairings = new PairingStore(createMemoryStore());
  const client = new NativeClient({
    connect: () => {
      const port = new FakePort();
      port.answer = answer;
      ports.push(port);
      return port;
    },
    lastError: () => lastError,
    pairings,
    hint: "chrome",
    onEvent: (event) => events.push(event),
  });
  const ready = withPairing ? pairings.save(PAIRING) : Promise.resolve();
  return {
    client,
    ports,
    events,
    pairings,
    ready,
    setLastError: (message: string) => {
      lastError = message;
    },
  };
}

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("NativeClient requests", () => {
  it("opens the port lazily, says hello with the stored pairing, then matches responses by id", async () => {
    const { client, ports, ready } = setup();
    await ready;
    expect(ports).toHaveLength(0);
    const status = await client.request("status", {});
    expect(status).toEqual({ state: "unlocked" });
    expect(ports).toHaveLength(1);
    const [hello, second] = ports[0]?.sent ?? [];
    expect(hello).toMatchObject({ op: "hello", body: { pairing: PAIRING, hint: "chrome" } });
    expect(second).toMatchObject({ op: "status" });
    expect(hello?.id).not.toBe(second?.id);
    expect(client.connection).toBe("unlocked");
  });

  it("says hello once for many requests, and answers each caller", async () => {
    const { client, ports, ready } = setup();
    await ready;
    const both = await Promise.all([client.request("status", {}), client.request("status", {})]);
    expect(both).toEqual([{ state: "unlocked" }, { state: "unlocked" }]);
    expect(ports[0]?.sent.filter((s) => s.op === "hello")).toHaveLength(1);
  });

  it("refuses a response body of the wrong shape", async () => {
    const { client, ready } = setup((r) =>
      r.op === "hello" ? answerLikeHost(r) : ok(r.id, { state: "open" }),
    );
    await ready;
    await expect(client.request("status", {})).rejects.toMatchObject({ code: "internal" });
  });

  it("rejects with the desktop's error code", async () => {
    const { client, ready } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : fail(r.id, "locked")));
    await ready;
    await expect(client.request("listForOrigin", { origin: "https://a.test", topOrigin: "https://a.test" })).rejects.toMatchObject({
      code: "locked",
    });
  });

  it("times out after 20 seconds", async () => {
    const { client, ready } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : undefined));
    await ready;
    const outcome = expect(client.request("status", {})).rejects.toMatchObject({ code: "timeout" });
    await vi.advanceTimersByTimeAsync(19_999);
    await vi.advanceTimersByTimeAsync(1);
    await outcome;
  });

  it("refuses a fifth request while four are in flight", async () => {
    const { client, ready } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : undefined));
    await ready;
    await client.ensureSession();
    const pending = Array.from({ length: 4 }, () => client.request("status", {}));
    for (const p of pending) {
      p.catch(() => undefined);
    }
    await expect(client.request("status", {})).rejects.toMatchObject({ code: "busy" });
    await vi.advanceTimersByTimeAsync(20_000);
    await expect(pending[0]).rejects.toMatchObject({ code: "timeout" });
  });

  it("ignores a response with an unknown id, extra keys or the wrong version", async () => {
    const { client, ports, ready } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : undefined));
    await ready;
    const pending = client.request("status", {});
    pending.catch(() => undefined);
    await vi.advanceTimersByTimeAsync(0);
    const port = ports[0] as FakePort;
    const id = port.sent[1]?.id ?? 0;
    port.emit(ok(id + 100, { state: "locked" }));
    port.emit({ v: 1, id, ok: true, body: { state: "locked" }, extra: true });
    port.emit({ v: 2, id, ok: true, body: { state: "locked" } });
    await vi.advanceTimersByTimeAsync(0);
    port.emit(ok(id, { state: "locked" }));
    await expect(pending).resolves.toEqual({ state: "locked" });
  });
});

describe("NativeClient errors", () => {
  it.each([
    ["Specified native messaging host not found.", "hostMissing"],
    ["Access to the specified native messaging host is forbidden.", "hostForbidden"],
    ["Native host has exited.", "hostExited"],
    [undefined, "hostExited"],
  ] as const)("maps %j to %s", async (message, code) => {
    const { client, ports, ready, setLastError } = setup(() => undefined);
    await ready;
    const request = client.request("status", {});
    const outcome = expect(request).rejects.toMatchObject({ code });
    await vi.advanceTimersByTimeAsync(0);
    if (message !== undefined) {
      setLastError(message);
    }
    ports[0]?.drop();
    await outcome;
    expect(client.state.error).toBe(code);
  });

  it("fails every request in flight when the port drops, and reconnects on the next one", async () => {
    const { client, ports, ready, setLastError } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : undefined));
    await ready;
    await client.ensureSession();
    const a = client.request("status", {});
    const b = client.request("status", {});
    const both = Promise.allSettled([a, b]);
    setLastError("Native host has exited.");
    ports[0]?.drop();
    expect((await both).map((r) => r.status)).toEqual(["rejected", "rejected"]);
    const next = client.request("status", {});
    next.catch(() => undefined);
    await vi.advanceTimersByTimeAsync(0);
    expect(ports).toHaveLength(2);
  });

  it("keeps the pairing when the desktop does not know it", async () => {
    const { client, pairings, ready } = setup((r) => (r.op === "hello" ? ok(r.id, { desktop: "unlocked", paired: false }) : fail(r.id, "notPaired")));
    await ready;
    await expect(client.request("status", {})).rejects.toMatchObject({ code: "notPaired" });
    expect(await pairings.load()).toEqual(PAIRING);
    expect(client.connection).toBe("notPaired");
  });
});

describe("NativeClient idle and events", () => {
  it("closes the port after 60 seconds with nothing in flight and opens a new one later", async () => {
    const { client, ports, ready } = setup();
    await ready;
    await client.request("status", {});
    await vi.advanceTimersByTimeAsync(59_999);
    expect(ports[0]?.disconnected).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(ports[0]?.disconnected).toBe(true);
    await client.request("status", {});
    expect(ports).toHaveLength(2);
  });

  it("does not close the port while a request is in flight", async () => {
    const { client, ports, ready } = setup((r) => (r.op === "hello" ? answerLikeHost(r) : undefined));
    await ready;
    await client.ensureSession();
    const pending = client.request("status", {});
    pending.catch(() => undefined);
    await vi.advanceTimersByTimeAsync(19_000);
    expect(ports[0]?.disconnected).toBe(false);
    await vi.advanceTimersByTimeAsync(1_000);
  });

  it("passes events on, tracks the lock state and forgets the pairing on unpaired", async () => {
    const { client, ports, events, pairings, ready } = setup();
    await ready;
    await client.request("status", {});
    const port = ports[0] as FakePort;
    port.emit({ v: 1, event: "state", state: "locked" });
    expect(client.state.desktop).toBe("locked");
    expect(client.connection).toBe("locked");
    port.emit({ v: 1, event: "pairCode", requestId: 3, code: "4F7K2Q", expiresInSeconds: 120 });
    port.emit({ v: 1, event: "reboot" });
    port.emit({ v: 1, event: "unpaired" });
    await vi.advanceTimersByTimeAsync(0);
    expect(events.map((e) => e.event)).toEqual(["state", "pairCode", "unpaired"]);
    expect(await pairings.load()).toBeNull();
    expect(client.connection).toBe("notPaired");
  });

  it("starts a new session after the host reports the desktop gone", async () => {
    const { client, ports, ready } = setup();
    await ready;
    await client.request("status", {});
    ports[0]?.emit({ v: 1, event: "disconnected" });
    expect(client.connection).toBe("desktopNotRunning");
    await client.request("status", {});
    expect(ports[0]?.sent.filter((s) => s.op === "hello")).toHaveLength(2);
  });
});

describe("NativeClient pairing", () => {
  it("pairs, stores the new pairing and says hello with it", async () => {
    const { client, ports, pairings, events } = setup(
      (r) => {
        if (r.op === "pair") {
          return ok(r.id, PAIRING);
        }
        return r.op === "hello" ? ok(r.id, { desktop: "unlocked", paired: true }) : undefined;
      },
      false,
    );
    const pairing = client.pair();
    await vi.advanceTimersByTimeAsync(0);
    ports[0]?.emit({ v: 1, event: "pairCode", requestId: 1, code: "4F7K2Q", expiresInSeconds: 120 });
    await pairing;
    expect(events[0]).toMatchObject({ event: "pairCode", code: "4F7K2Q" });
    expect(await pairings.load()).toEqual(PAIRING);
    expect(ports[0]?.sent.map((s) => s.op)).toEqual(["pair", "hello"]);
    expect(ports[0]?.sent[1]?.body).toMatchObject({ pairing: PAIRING });
    expect(client.connection).toBe("unlocked");
  });

  it("reports a refused pairing and stores nothing", async () => {
    const { client, pairings } = setup((r) => fail(r.id, "pairingDenied"), false);
    await expect(client.pair()).rejects.toMatchObject({ code: "pairingDenied" });
    expect(await pairings.load()).toBeNull();
    expect(client.state.pairing).toBe(false);
  });
});

describe("connectionOf", () => {
  const link = (over: Partial<LinkState>): LinkState => ({
    checked: true,
    desktop: "unlocked",
    paired: true,
    hasPairing: true,
    pairing: false,
    error: null,
    ...over,
  });

  it.each<[Partial<LinkState>, string]>([
    [{ checked: false }, "checking"],
    [{ pairing: true }, "pairing"],
    [{ error: "hostMissing" }, "hostMissing"],
    [{ error: "hostForbidden" }, "hostForbidden"],
    [{ error: "protocolVersion" }, "protocolMismatch"],
    [{ error: "desktopNotRunning" }, "desktopNotRunning"],
    [{ desktop: "notRunning" }, "desktopNotRunning"],
    [{ hasPairing: false }, "notPaired"],
    [{ error: "notPaired" }, "notPaired"],
    [{ paired: false }, "notPaired"],
    [{ error: "hostExited" }, "error"],
    [{ error: "timeout" }, "error"],
    [{ desktop: "locked" }, "locked"],
    [{}, "unlocked"],
  ])("%j is %s", (over, expected) => {
    expect(connectionOf(link(over))).toBe(expected);
  });

  it("maps the browser's wording", () => {
    const codes: ErrorCode[] = [
      disconnectCode("Specified native messaging host not found."),
      disconnectCode("Access to the specified native messaging host is forbidden."),
      disconnectCode("Native host has exited."),
    ];
    expect(codes).toEqual(["hostMissing", "hostForbidden", "hostExited"]);
  });
});

import { afterEach, describe, expect, it, vi } from "vitest";
import { PasskeySwitch, chromePasskeyRegistry } from "./passkey-registry.ts";

interface Registered {
  id: string;
  [key: string]: unknown;
}

/** A stand-in for the parts of `chrome.scripting` and `chrome.permissions` the registry uses. */
function stubChrome(granted: boolean) {
  const registered: Registered[] = [];
  const chromeStub = {
    scripting: {
      getRegisteredContentScripts: vi.fn(({ ids }: { ids: string[] }) =>
        Promise.resolve(registered.filter((script) => ids.includes(script.id))),
      ),
      registerContentScripts: vi.fn((scripts: Registered[]) => {
        registered.push(...scripts);
        return Promise.resolve();
      }),
      unregisterContentScripts: vi.fn(({ ids }: { ids: string[] }) => {
        for (const id of ids) {
          const at = registered.findIndex((script) => script.id === id);
          if (at >= 0) {
            registered.splice(at, 1);
          }
        }
        return Promise.resolve();
      }),
    },
    permissions: { contains: vi.fn(() => Promise.resolve(granted)) },
  };
  vi.stubGlobal("chrome", chromeStub);
  return { registered, chromeStub };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the passkey scripts", () => {
  it("register for the top frame of https pages only, at document_start, one per world, and nothing else", async () => {
    const { registered } = stubChrome(true);
    expect(await new PasskeySwitch(chromePasskeyRegistry()).set(true)).toBe(true);
    const common = { matches: ["https://*/*"], allFrames: false, runAt: "document_start", persistAcrossSessions: true };
    // Exactly these keys: no matchOriginAsFallback or matchAboutBlank (about:, blob: and data: frames get nothing), no CSS.
    expect(registered).toEqual([
      { id: "passkey-main", js: ["passkey-main.js"], world: "MAIN", ...common },
      { id: "passkey-bridge", js: ["passkey-bridge.js"], world: "ISOLATED", ...common },
    ]);
  });

  it("are not registered without the site access, and go away when the switch goes off", async () => {
    const denied = stubChrome(false);
    await expect(new PasskeySwitch(chromePasskeyRegistry()).set(true)).rejects.toMatchObject({ code: "permissionDenied" });
    expect(denied.registered).toEqual([]);

    const { registered } = stubChrome(true);
    const toggle = new PasskeySwitch(chromePasskeyRegistry());
    await toggle.set(true);
    expect(await toggle.set(false)).toBe(false);
    expect(registered).toEqual([]);
    expect(await toggle.enabled()).toBe(false);
  });

  it("switch themselves off when the browser took the site access back", async () => {
    const { registered, chromeStub } = stubChrome(true);
    const toggle = new PasskeySwitch(chromePasskeyRegistry());
    await toggle.set(true);
    chromeStub.permissions.contains.mockReturnValue(Promise.resolve(false));
    expect(await toggle.enabled()).toBe(false);
    expect(registered).toEqual([]);
  });
});

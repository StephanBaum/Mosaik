/**
 * The opt-in passkey scripts (ADR-0040). They are never in the manifest: turning "Use Mosaic
 * for passkeys" on asks for the optional host permission (in the popup, on a click) and
 * registers `passkey-main.js` (main world) and `passkey-bridge.js` (isolated world) for the top
 * frame of https pages at `document_start`; turning it off, or the browser taking the
 * permission back, unregisters them. Pages that are already open keep the scripts they have
 * until they reload, so the background checks the switch on every request as well.
 */
import { ExtensionError } from "../shared/errors.ts";
import { PASSKEY_MATCHES } from "../shared/ids.ts";

export const PASSKEY_MAIN_ID = "passkey-main";
export const PASSKEY_BRIDGE_ID = "passkey-bridge";
export const PASSKEY_MAIN_FILE = "passkey-main.js";
export const PASSKEY_BRIDGE_FILE = "passkey-bridge.js";
const IDS = [PASSKEY_MAIN_ID, PASSKEY_BRIDGE_ID];

export interface PasskeyRegistry {
  /** Whether the site access (`optional_host_permissions`) for https pages is granted. */
  hasAccess(): Promise<boolean>;
  /** Whether both scripts are registered. */
  isRegistered(): Promise<boolean>;
  register(): Promise<void>;
  unregister(): Promise<void>;
}

export function chromePasskeyRegistry(): PasskeyRegistry {
  const registered = async (): Promise<string[]> =>
    (await chrome.scripting.getRegisteredContentScripts({ ids: IDS })).map((script) => script.id);
  const common = { matches: PASSKEY_MATCHES, allFrames: false, runAt: "document_start", persistAcrossSessions: true } as const;
  return {
    hasAccess: () => chrome.permissions.contains({ origins: PASSKEY_MATCHES }),
    isRegistered: async () => (await registered()).length === IDS.length,
    register: async () => {
      const have = await registered();
      const scripts: chrome.scripting.RegisteredContentScript[] = [
        { id: PASSKEY_MAIN_ID, js: [PASSKEY_MAIN_FILE], world: "MAIN", ...common },
        { id: PASSKEY_BRIDGE_ID, js: [PASSKEY_BRIDGE_FILE], world: "ISOLATED", ...common },
      ];
      const missing = scripts.filter((script) => !have.includes(script.id));
      if (missing.length > 0) {
        await chrome.scripting.registerContentScripts(missing);
      }
    },
    unregister: async () => {
      try {
        await chrome.scripting.unregisterContentScripts({ ids: IDS });
      } catch {
        // Not registered: nothing to undo.
      }
    },
  };
}

/** The setting "Use Mosaic for passkeys": on only while the scripts are registered and the site access is still granted. */
export class PasskeySwitch {
  readonly #registry: PasskeyRegistry;

  constructor(registry: PasskeyRegistry) {
    this.#registry = registry;
  }

  async enabled(): Promise<boolean> {
    if (!(await this.#registry.isRegistered())) {
      return false;
    }
    if (await this.#registry.hasAccess()) {
      return true;
    }
    // The browser took the permission back: the scripts cannot run, so the setting is off.
    await this.#registry.unregister();
    return false;
  }

  /** The popup switch. Turning on needs the site access the popup just asked the user for. */
  async set(enabled: boolean): Promise<boolean> {
    if (!enabled) {
      await this.#registry.unregister();
      return false;
    }
    if (!(await this.#registry.hasAccess())) {
      throw new ExtensionError("permissionDenied");
    }
    await this.#registry.register();
    return true;
  }
}

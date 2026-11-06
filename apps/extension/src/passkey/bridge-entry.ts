/** Entry of `passkey-bridge.js` (isolated world, document_start): installs the bridge. */
import { installBridge } from "./bridge.ts";

installBridge({ win: window, send: (message) => chrome.runtime.sendMessage(message) });

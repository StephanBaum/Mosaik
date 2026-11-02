/** Icons: the component, the names and the icon of each item kind. */
import type { ItemKind } from "../ipc/contract/index.ts";
import type { IconName } from "./icons/paths.ts";

export { Icon, type IconSize } from "./icons/Icon.tsx";
export type { IconName } from "./icons/paths.ts";

const KIND_ICONS: Record<ItemKind, IconName> = {
  login: "key",
  passkey: "passkey",
  totp: "timer",
  secureNote: "note",
  creditCard: "card",
  identity: "note",
  sshKey: "terminal",
  apiKey: "key",
  wifi: "wifi",
  document: "file",
  unsupported: "file",
};

export function kindIcon(kind: ItemKind): IconName {
  return KIND_ICONS[kind];
}

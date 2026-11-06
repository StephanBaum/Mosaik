/** Which shortcut hints to show: Command/Option symbols on a Mac, Ctrl/Alt names elsewhere. */
export function shortcutPlatform(): "mac" | "other" {
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const name = nav.userAgentData?.platform ?? nav.platform;
  return /mac|iphone|ipad/i.test(name) ? "mac" : "other";
}

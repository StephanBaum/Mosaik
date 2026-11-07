/**
 * The download section shows the visitor's system first and larger. Detection reads only what
 * the browser already sends (user agent and platform); nothing leaves the page.
 */
export const PLATFORMS = ["windows", "macos", "linux", "iphone", "android", "chrome", "edge", "firefox", "cli"] as const;

export type Platform = (typeof PLATFORMS)[number];

/** The visitor's operating system as one of the download platforms, or `null`. */
export function detectPlatform(userAgent: string, platform = ""): Platform | null {
  const ua = `${userAgent} ${platform}`;
  if (/iPhone|iPad|iPod/i.test(ua)) {
    return "iphone";
  }
  if (/Android/i.test(ua)) {
    return "android";
  }
  if (/Windows|Win32|Win64/i.test(ua)) {
    return "windows";
  }
  if (/Mac/i.test(ua)) {
    return "macos";
  }
  if (/Linux|X11|CrOS/i.test(ua)) {
    return "linux";
  }
  return null;
}

export function wireDownloads(root: Document, nav: Pick<Navigator, "userAgent">): void {
  const detected = detectPlatform(nav.userAgent);
  const list = root.querySelector<HTMLElement>(".platforms");
  const item = detected === null ? null : list?.querySelector<HTMLElement>(`[data-platform="${detected}"]`);
  if (!list || !item) {
    return;
  }
  item.classList.add("platform--detected");
  list.prepend(item);
}

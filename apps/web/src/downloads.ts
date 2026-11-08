/**
 * The download page shows the visitor's system first (website v2 §5.8). Detection reads only
 * what the browser already sends (user agent and platform); nothing leaves the page. The holo
 * button then names the system and points at its tile. A tile with a release file starts the
 * file and opens the next-steps page a moment later; without JS the visitor just gets the file.
 */
export const PLATFORMS = ["windows", "macos", "linux", "iphone", "android", "chrome", "edge", "firefox", "cli"] as const;

export type Platform = (typeof PLATFORMS)[number];

/** Delay before the next-steps page opens after a file download starts. */
const NEXT_DELAY_MS = 1200;

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

/** A file link opens the next-steps page a moment after the download starts. */
function wireNext(link: HTMLAnchorElement): void {
  link.addEventListener("click", () => {
    const next = link.dataset["next"];
    if (next !== undefined) {
      window.setTimeout(() => {
        location.assign(next);
      }, NEXT_DELAY_MS);
    }
  });
}

/** Points the holo button at the detected system's tile and names the system in its label. */
function pointHolo(root: Document, item: HTMLElement): void {
  const link = item.querySelector<HTMLAnchorElement>("a.platform__link");
  const cta = root.querySelector<HTMLElement>("[data-island='downloads'][data-label]");
  const holo = cta?.querySelector<HTMLAnchorElement>("a.holo");
  const label = holo?.querySelector(".holo__label")?.firstChild;
  const name = item.querySelector(".platform__name")?.textContent;
  if (!link || !cta || !holo || !label || !name) {
    return;
  }
  label.textContent = (cta.dataset["label"] ?? "").replace("%os%", name);
  holo.href = link.href;
  if (link.dataset["next"] !== undefined) {
    holo.dataset["next"] = link.dataset["next"];
    holo.setAttribute("download", "");
    wireNext(holo);
  }
}

export function wireDownloads(root: Document, nav: Pick<Navigator, "userAgent">): void {
  for (const link of root.querySelectorAll<HTMLAnchorElement>("a.platform__link[data-next]")) {
    wireNext(link);
  }
  const detected = detectPlatform(nav.userAgent);
  const item = detected === null ? null : root.querySelector<HTMLElement>(`.platform[data-platform="${detected}"]`);
  if (item === null) {
    return;
  }
  item.classList.add("platform--detected");
  item.parentElement?.prepend(item);
  const group = item.closest<HTMLElement>(".platform-group");
  group?.parentElement?.prepend(group);
  pointHolo(root, item);
}

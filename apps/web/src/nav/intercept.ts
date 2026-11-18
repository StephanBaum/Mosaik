/** Which link clicks get the veil wipe before navigating. Everything else behaves natively. */
type ClickLike = Pick<MouseEvent, "button" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "detail" | "defaultPrevented">;

interface LinkLike {
  href: string;
  target: string;
  hasDownload: boolean;
  isNav: boolean;
}

/** A primary-button pointer click without modifiers (`detail` 0 is keyboard activation). */
export function isPlainPointerClick(e: ClickLike): boolean {
  return e.button === 0 && e.detail > 0 && !e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey;
}

/**
 * True only for a plain pointer click on a `data-nav` link that opens another page of this
 * site in the same tab. Modifier clicks, middle clicks, keyboard activation (`detail` 0),
 * `target="_blank"`, downloads, other origins and same-page hashes never wipe.
 */
export function shouldWipe(e: ClickLike, a: LinkLike, here: { origin: string; pathname: string }): boolean {
  if (!a.isNav || a.hasDownload || e.defaultPrevented) {
    return false;
  }
  if (!isPlainPointerClick(e) || (a.target !== "" && a.target !== "_self")) {
    return false;
  }
  let url: URL;
  try {
    url = new URL(a.href, here.origin);
  } catch {
    return false;
  }
  return url.origin === here.origin && url.pathname !== here.pathname;
}

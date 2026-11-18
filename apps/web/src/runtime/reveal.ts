/**
 * Scroll reveals without a library. Only JS adds `reveal-pending`, and only to elements that are
 * below the fold when the observer first reports, so with JS off or slow every section is visible.
 */
const PENDING = "reveal-pending";
const IN = "reveal-in";

export function wireReveals(doc: Document, allowed: boolean, IO: typeof IntersectionObserver = IntersectionObserver): void {
  if (!allowed) {
    return;
  }
  const targets = doc.querySelectorAll<HTMLElement>("[data-reveal]");
  if (targets.length === 0) {
    return;
  }
  let booted = false;
  const observer = new IO(
    (entries) => {
      for (const entry of entries) {
        const el = entry.target;
        if (!booted && !entry.isIntersecting) {
          el.classList.add(PENDING);
        } else if (entry.isIntersecting) {
          if (el.classList.contains(PENDING)) {
            el.classList.replace(PENDING, IN);
          }
          observer.unobserve(el);
        }
      }
      booted = true;
    },
    { rootMargin: "0px 0px -3% 0px" },
  );
  for (const el of targets) {
    observer.observe(el);
  }
}

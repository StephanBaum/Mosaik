/**
 * The header's scrolled state and the full-screen `<dialog>` menu below 900 px. The dialog
 * owns focus and Escape.
 */
const SCROLLED = "is-scrolled";
/** Scroll distance (px) after which the header turns dark. */
const THRESHOLD = 8;

function wireHeader(root: Document): void {
  const header = root.querySelector<HTMLElement>(".site-header");
  if (!header) {
    return;
  }
  let queued = false;
  const update = (): void => {
    queued = false;
    header.classList.toggle(SCROLLED, window.scrollY > THRESHOLD);
  };
  window.addEventListener(
    "scroll",
    () => {
      if (!queued) {
        queued = true;
        requestAnimationFrame(update);
      }
    },
    { passive: true },
  );
  update();
}

export function wireMenu(root: Document): void {
  wireHeader(root);
  const dialog = root.querySelector<HTMLDialogElement>("dialog.menu");
  const toggle = root.querySelector<HTMLButtonElement>("button.menu-toggle");
  if (!dialog || !toggle) {
    return;
  }
  toggle.addEventListener("click", () => {
    dialog.showModal();
  });
  dialog.querySelector(".menu__close")?.addEventListener("click", () => {
    dialog.close();
  });
  // Following a link closes the menu, so in-page anchors land on a visible page.
  for (const link of dialog.querySelectorAll("a")) {
    link.addEventListener("click", () => {
      dialog.close();
    });
  }
  dialog.addEventListener("close", () => {
    toggle.focus();
  });
}

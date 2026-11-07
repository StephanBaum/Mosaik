/** The full-screen `<dialog>` menu below 900 px (design concept §6). The dialog owns focus and Escape. */
export function wireMenu(root: Document): void {
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

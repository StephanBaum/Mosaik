/**
 * After a notice went away with the button that had focus: focus goes to the page's main region,
 * not to the document. Runs a moment later, once the notice is gone from the page.
 */
export function focusPageIfLost(): void {
  window.setTimeout(() => {
    const lost = document.activeElement === null || document.activeElement === document.body;
    const main = document.querySelector<HTMLElement>("main");
    if (lost && main !== null) {
      if (!main.hasAttribute("tabindex")) {
        main.tabIndex = -1;
      }
      main.focus();
    }
  }, 50);
}

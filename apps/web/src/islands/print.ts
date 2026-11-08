/** The checklist's print button (Switch guide). Without JS the button stays hidden. */
export function bootPrint(root: Document): void {
  for (const button of root.querySelectorAll<HTMLButtonElement>("[data-print]")) {
    button.hidden = false;
    button.addEventListener("click", () => {
      window.print();
    });
  }
}

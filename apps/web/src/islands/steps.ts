/**
 * P8 sticky stage (website v2 §3): on wide screens the stage crossfades to the image of the
 * step card that crosses the middle of the viewport. Below 1024 px every card shows its own
 * image and this island does nothing visible. Without JS the stage shows the first image.
 */
export function bootSteps(root: Document): void {
  for (const region of root.querySelectorAll<HTMLElement>("[data-island='steps']")) {
    const frames = [...region.querySelectorAll<HTMLElement>(".sticky__frame")];
    const cards = [...region.querySelectorAll<HTMLElement>("[data-step]")];
    if (frames.length === 0 || cards.length === 0) {
      continue;
    }
    const show = (step: string): void => {
      for (const frame of frames) {
        frame.classList.toggle("is-active", frame.dataset["frame"] === step);
      }
      for (const card of cards) {
        card.classList.toggle("is-active", card.dataset["step"] === step);
      }
    };
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const step = (entry.target as HTMLElement).dataset["step"];
          if (entry.isIntersecting && step !== undefined) {
            show(step);
          }
        }
      },
      { rootMargin: "-45% 0px -45% 0px" },
    );
    for (const card of cards) {
      observer.observe(card);
    }
  }
}

/** Line icons for the static pages, from the app's icon table (24 x 24, stroke 1.75). */
const STROKE = 1.75;
const ARROW_DOWN = ["M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", "m7 10 5 5 5-5", "M12 15V3"];
const ARROW_RIGHT = ["M5 12h14", "m12 5 7 7-7 7"];

const PATHS = { download: ARROW_DOWN, arrowRight: ARROW_RIGHT } as const;

export function Icon({ name, className }: { name: keyof typeof PATHS; className?: string }) {
  return (
    <svg
      className={["icon", className].filter(Boolean).join(" ")}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={STROKE}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}

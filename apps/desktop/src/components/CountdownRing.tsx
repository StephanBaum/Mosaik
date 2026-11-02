/** A ring that drains with `remaining / total`. Decorative; the number is shown as text next to it. */
const CIRCUMFERENCE = 50.3;
const LOW_SECONDS = 5;

export function CountdownRing({ remaining, total }: { remaining: number; total: number }) {
  const fraction = total > 0 ? Math.min(Math.max(remaining / total, 0), 1) : 0;
  const low = remaining <= LOW_SECONDS;
  return (
    <svg className="ring" viewBox="0 0 20 20" aria-hidden="true" focusable="false">
      <circle className="ring__track" cx="10" cy="10" r="8" fill="none" strokeWidth="2.5" />
      <circle
        className={low ? "ring__value ring__value--low" : "ring__value"}
        cx="10"
        cy="10"
        r="8"
        fill="none"
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeDasharray={String(CIRCUMFERENCE)}
        strokeDashoffset={String(CIRCUMFERENCE * (1 - fraction))}
        transform="rotate(-90 10 10)"
      />
    </svg>
  );
}

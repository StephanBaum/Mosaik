/** One line icon from the path table. Decorative: pair it with a text label or `aria-label` on its button. */
import { ICONS, type IconElement, type IconName } from "./paths.ts";

export type { IconName } from "./paths.ts";

export type IconSize = "sm" | "md" | "lg";

function Element({ element }: { element: IconElement }) {
  if ("d" in element) {
    return <path d={element.d} />;
  }
  if ("circle" in element) {
    const [cx, cy, r] = element.circle;
    return <circle cx={cx} cy={cy} r={r} />;
  }
  if ("rect" in element) {
    const [x, y, width, height, rx] = element.rect;
    return <rect x={x} y={y} width={width} height={height} rx={rx} />;
  }
  const [cx, cy, r] = element.dot;
  return <circle cx={cx} cy={cy} r={r} className="icon__dot" />;
}

export function Icon({
  name,
  size = "md",
  bold = false,
  filled = false,
  className,
}: {
  name: IconName;
  size?: IconSize;
  /** Stroke width 2, for the plus sign. */
  bold?: boolean;
  /** Fills the shape (the favourites star). */
  filled?: boolean;
  className?: string;
}) {
  const classes = ["icon", `icon--${size}`, filled ? "icon--filled" : "", className]
    .filter(Boolean)
    .join(" ");
  const elements: readonly IconElement[] = ICONS[name];
  return (
    <svg
      className={classes}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={bold ? 2 : 1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {elements.map((element, index) => (
        <Element key={index} element={element} />
      ))}
    </svg>
  );
}

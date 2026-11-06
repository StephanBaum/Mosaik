/**
 * One line icon from the desktop's path table (the geometry is copied from docs/design/screens
 * there, once). Decorative: pair it with a text label or an `aria-label` on its button.
 */
import { ICONS, type IconElement, type IconName } from "../../../../desktop/src/components/icons/paths.ts";

export type { IconName };

function Shape({ element }: { element: IconElement }) {
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

export function Icon({ name, size = "md" }: { name: IconName; size?: "sm" | "md" | "lg" }) {
  const elements: readonly IconElement[] = ICONS[name];
  return (
    <svg
      className={`icon icon--${size}`}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {elements.map((element, index) => (
        <Shape key={index} element={element} />
      ))}
    </svg>
  );
}

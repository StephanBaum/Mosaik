/** A letter (or icon) tile for an item or vault. Decorative: the title is next to it. */
import { Icon, type IconName } from "./icons.tsx";

export function avatarInitial(title: string | undefined): string {
  return Array.from((title ?? "").trim())[0]?.toLocaleUpperCase() ?? "•";
}

export function Avatar({
  title,
  icon,
  size = "md",
  selected = false,
}: {
  title?: string;
  icon?: IconName;
  size?: "sm" | "md" | "lg";
  selected?: boolean;
}) {
  return (
    <span className={selected ? "avatar avatar--inverse" : "avatar"} data-size={size} aria-hidden="true">
      {icon === undefined ? avatarInitial(title) : <Icon name={icon} />}
    </span>
  );
}

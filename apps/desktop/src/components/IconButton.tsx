/** An icon-only button. The label is both the accessible name and the tooltip (UX_GUIDELINES §7). */
import type { ButtonHTMLAttributes, Ref } from "react";
import { Icon, type IconName } from "./icons.tsx";

export function IconButton({
  icon,
  label,
  pressed,
  size = "md",
  variant = "ghost",
  className,
  type = "button",
  ref,
  ...rest
}: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "aria-label"> & {
  icon: IconName;
  label: string;
  pressed?: boolean;
  size?: "sm" | "md";
  variant?: "ghost" | "outline";
  ref?: Ref<HTMLButtonElement>;
}) {
  const classes = ["icon-button", `icon-button--${size}`, `icon-button--${variant}`, className]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      ref={ref}
      type={type}
      className={classes}
      aria-label={label}
      title={label}
      aria-pressed={pressed}
      {...rest}
    >
      <Icon name={icon} />
    </button>
  );
}

/**
 * Buttons. The screen's single primary action is a `HoloButton` (ADR-0014); everything
 * else is one of these variants.
 */
import type { ButtonHTMLAttributes, Ref } from "react";

export type ButtonVariant = "secondary" | "ghost" | "danger" | "dangerSolid";
export type ButtonSize = "sm" | "md" | "lg";

export function Button({
  variant = "secondary",
  size = "md",
  block = false,
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  block?: boolean;
  ref?: Ref<HTMLButtonElement>;
}) {
  const classes = ["button", `button--${variant}`, `button--${size}`, block ? "button--block" : "", className]
    .filter(Boolean)
    .join(" ");
  return <button type={type} className={classes} {...props} />;
}

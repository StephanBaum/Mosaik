/**
 * The screen's single primary action (ADR-0014). This renders the static token gradient,
 * which is also the fallback for reduced motion and for systems without WebGL2.
 */
import type { ButtonHTMLAttributes } from "react";

export function HoloButton({
  className,
  type = "button",
  size = "md",
  block = false,
  ambient = false,
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  size?: "sm" | "md" | "lg";
  block?: boolean;
  ambient?: boolean;
}) {
  const classes = ["button", `button--${size}`, "holo", block ? "button--block" : "", className]
    .filter(Boolean)
    .join(" ");
  return (
    <button type={type} className={classes} data-ambient={ambient ? "true" : undefined} {...props}>
      <span className="holo__label">{children}</span>
    </button>
  );
}

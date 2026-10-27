/**
 * Buttons. `primary` is the screen's single primary action (ADR-0014): a round pill that
 * the UI track renders in holo foil (WebGL2, ported from docs/design/holo/); until then,
 * and whenever WebGL2 is unavailable, it uses the flat fallback fill.
 */
import type { ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "ghost";

export function Button({
  variant = "secondary",
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  const classes = ["button", `button--${variant}`, className].filter(Boolean).join(" ");
  return <button type={type} className={classes} {...props} />;
}

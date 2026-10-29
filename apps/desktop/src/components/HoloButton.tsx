/**
 * The screen's single primary action (ADR-0014). The holo-foil WebGL2 renderer
 * (docs/design/holo/holo-button.js) is not ported yet: this renders the static token
 * gradient, which is also the fallback for reduced motion and for systems without WebGL2.
 */
import type { ButtonHTMLAttributes } from "react";

export function HoloButton({
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement>) {
  const classes = ["button", "button--primary", "holo", className].filter(Boolean).join(" ");
  return <button type={type} className={classes} {...props} />;
}

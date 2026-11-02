/**
 * The screen's single primary action (ADR-0014): a pill with a holo foil painted by the
 * shared WebGL2 renderer. Without WebGL2, with a lost context, or without a provider it
 * keeps the static token gradient. The label stays real DOM text.
 */
import { useEffect, useRef, type ButtonHTMLAttributes } from "react";
import { useHolo } from "../holo/HoloProvider.tsx";

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
  /** Keeps the sheen moving. Use on at most one button per screen. */
  ambient?: boolean;
}) {
  const holo = useHolo();
  const button = useRef<HTMLButtonElement>(null);
  const foil = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (button.current === null || foil.current === null) {
      return undefined;
    }
    return holo.register(button.current, foil.current, { ambient });
  }, [holo, ambient]);

  const classes = ["button", `button--${size}`, "holo", block ? "button--block" : "", className]
    .filter(Boolean)
    .join(" ");
  return (
    <button ref={button} type={type} className={classes} {...props}>
      <canvas ref={foil} className="holo__foil" aria-hidden="true" />
      <span className="holo__label">{children}</span>
    </button>
  );
}

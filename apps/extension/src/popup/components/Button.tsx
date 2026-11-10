/**
 * Buttons. The popup's single primary action is `primary`: the static holo gradient (no WebGL
 * in the popup, ADR-0035); everything else is secondary, ghost or a text link-button.
 */
import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";
import { Icon, type IconName } from "./Icon.tsx";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "link";

export function Button({
  variant = "secondary",
  block = false,
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; block?: boolean; ref?: Ref<HTMLButtonElement> }) {
  const classes = ["button", `button--${variant}`, variant === "primary" ? "holo" : "", block ? "button--block" : "", className]
    .filter(Boolean)
    .join(" ");
  return <button type={type} className={classes} {...props} />;
}

/** An icon-only button: the label is its accessible name (and its tooltip). */
export function IconButton({
  icon,
  label,
  pressed,
  current,
  ...props
}: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-label" | "children"> & {
  icon: IconName;
  label: string;
  pressed?: boolean;
  current?: boolean;
  ref?: Ref<HTMLButtonElement>;
}) {
  return (
    <button
      type="button"
      className={current === true ? "icon-button is-current" : "icon-button"}
      aria-label={label}
      title={label}
      aria-pressed={pressed}
      aria-current={current === true ? "page" : undefined}
      {...props}
    >
      <Icon name={icon} />
    </button>
  );
}

/** A small round tile with the first letter of a title, standing in for a site icon. */
export function Avatar({ text, active = false }: { text: string; active?: boolean }): ReactNode {
  const first = text.trim().codePointAt(0);
  const letter = first === undefined ? "?" : String.fromCodePoint(first).toUpperCase();
  return (
    <span className={active ? "avatar is-active" : "avatar"} aria-hidden="true">
      {letter}
    </span>
  );
}

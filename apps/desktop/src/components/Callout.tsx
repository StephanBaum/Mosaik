/** A warning or an info note with an icon. */
import type { ReactNode } from "react";
import { Icon } from "./icons.tsx";

export function Callout({
  tone,
  role,
  children,
}: {
  tone: "warning" | "info";
  /** `alert` announces the note when it appears (a warning that comes from the user's input). */
  role?: "alert";
  children: ReactNode;
}) {
  return (
    <div className={`callout callout--${tone}`} role={role ?? (tone === "warning" ? "note" : undefined)}>
      <Icon name={tone === "warning" ? "alert" : "info"} />
      <div>{children}</div>
    </div>
  );
}

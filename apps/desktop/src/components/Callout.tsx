/** A warning or an info note with an icon. */
import type { ReactNode } from "react";
import { Icon } from "./icons.tsx";

export function Callout({ tone, children }: { tone: "warning" | "info"; children: ReactNode }) {
  return (
    <div className={`callout callout--${tone}`} role={tone === "warning" ? "note" : undefined}>
      <Icon name={tone === "warning" ? "alert" : "info"} />
      <div>{children}</div>
    </div>
  );
}

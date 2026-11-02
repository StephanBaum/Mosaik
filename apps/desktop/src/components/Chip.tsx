/** A small outlined label (item type, vault, tag, severity). */
import type { ReactNode } from "react";

export function Chip({ children }: { children: ReactNode }) {
  return <span className="chip">{children}</span>;
}

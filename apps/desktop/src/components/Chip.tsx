/** A small outlined label (item type, vault, tag, severity). */
import type { ReactNode } from "react";

export function Chip({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "warning";
}) {
  return <span className={tone === "warning" ? "chip chip--warning" : "chip"}>{children}</span>;
}

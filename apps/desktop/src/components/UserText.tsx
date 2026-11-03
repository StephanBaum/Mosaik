import type { ReactNode } from "react";

/**
 * Text a person typed (a vault name, an item title, a key title): shown in its own
 * bidirectional isolate, so right-to-left or direction-changing characters in it cannot
 * reorder the text around it (a list row, the palette, an approval prompt). The backend also
 * refuses such characters in names and titles; this is the second layer.
 */
export function UserText({ children }: { children: ReactNode }) {
  return <bdi className="user-text">{children}</bdi>;
}

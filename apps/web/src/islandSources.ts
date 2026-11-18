/** Island id to source path, so the budget script can attribute lazy chunks to the pages that load them. */
import type { IslandId } from "./pages/routes.ts";

export const ISLAND_SOURCES: Readonly<Record<IslandId, string>> = {
  holo: "src/holo/index.ts",
  downloads: "src/downloads.ts",
  selfcheck: "src/islands/selfcheck.ts",
  print: "src/islands/print.ts",
  demos: "src/demos/bootDemos.ts",
  decode: "src/demos/decode.ts",
};

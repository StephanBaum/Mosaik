import { describe, expect, it } from "vitest";
import { titleOrUntitled } from "./title.ts";

describe("titleOrUntitled", () => {
  it("keeps a real title and replaces a missing or blank one", () => {
    expect(titleOrUntitled("GitHub", "Untitled")).toBe("GitHub");
    expect(titleOrUntitled(null, "Untitled")).toBe("Untitled");
    expect(titleOrUntitled("", "Untitled")).toBe("Untitled");
    expect(titleOrUntitled("   ", "Untitled")).toBe("Untitled");
  });
});

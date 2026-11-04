import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { createMockIpc } from "../ipc/mock.ts";
import { GROUP_SELECTOR, POP_SELECTOR } from "./choreography.ts";

describe("choreography selectors", () => {
  it("match the pieces of a rendered vault", async () => {
    const { container } = render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    const groups = [...container.querySelectorAll(GROUP_SELECTOR)];
    expect(groups.length).toBeGreaterThan(0);
    const has = (selector: string) => groups.some((el) => el.matches(selector));
    for (const selector of [".nav-item", ".sidebar__footer > *", ".topbar", ".list-pane__header", ".item-row"]) {
      expect(has(selector), selector).toBe(true);
    }
    expect(container.querySelectorAll(POP_SELECTOR).length).toBeGreaterThan(0);
    // The brand is deliberately not a group: the logo stays identical throughout.
    expect(groups.some((el) => el.closest(".sidebar__brand") !== null)).toBe(false);
  });

  it("finds the detail pane", async () => {
    const { container } = render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    expect(container.querySelector(".split__detail")?.matches(GROUP_SELECTOR)).toBe(true);
  });
});

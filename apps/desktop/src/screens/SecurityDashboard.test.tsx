import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { InsightFinding, InsightKind } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";
import { listFindings } from "./SecurityDashboard.tsx";

describe("listFindings", () => {
  const finding = (itemId: string, severity: "high" | "low", kind: InsightKind = "old"): InsightFinding =>
    ({
      itemId,
      vaultId: "v",
      issues: [{ kind, severity, action: "changePassword" }],
      reusedWith: [],
      totpSite: null,
      insecureHost: null,
    });

  it("lists only the high-severity findings and offers the rest", () => {
    const all = [finding("a", "low"), finding("b", "high")];
    const { listed, total } = listFindings(all, null, false);
    expect(listed.map((f) => f.itemId)).toEqual(["b"]);
    expect(total).toBe(2);
    expect(listFindings(all, null, true).listed).toHaveLength(2);
  });

  it("falls back to the first five findings when nothing is high severity", () => {
    const all = Array.from({ length: 7 }, (_, i) => finding(String(i), "low"));
    const { listed, total } = listFindings(all, null, false);
    expect(listed).toHaveLength(5);
    expect(total).toBe(7);
  });

  it("applies the kind filter before slicing", () => {
    const all = [finding("a", "high", "weak"), finding("b", "low", "old")];
    expect(listFindings(all, "old", false).listed.map((f) => f.itemId)).toEqual(["b"]);
  });
});

describe("security dashboard", () => {
  it("shows counts and the findings, with the 2FA attribution", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: /^Security/ }));

    await screen.findByRole("heading", { name: "Security" });
    expect(await screen.findByText("Missing 2FA")).toBeTruthy();
    expect(screen.getByText("Unsecured HTTP")).toBeTruthy();
    expect(screen.getByText(/2FA Directory/)).toBeTruthy();

    // Without a high-severity finding the list falls back to the first findings (no empty "Fix these first").
    expect((await screen.findAllByText(/Figma offers two-factor codes/)).length).toBeGreaterThan(0);
    expect(screen.getByRole("heading", { name: "Fix these first" })).toBeTruthy();
    expect(screen.getAllByRole("listitem").length).toBeGreaterThan(0);
  });

  it("opens the item of a finding", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: /^Security/ }));
    await user.click(await screen.findByRole("button", { name: /Figma/ }));
    expect(await screen.findByRole("article", { name: "Figma" })).toBeTruthy();
  });
});

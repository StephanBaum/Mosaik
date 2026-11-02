import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { HoloButton } from "./HoloButton.tsx";

describe("HoloButton without a provider", () => {
  it("renders the static fallback with a real text label", () => {
    const { container, unmount } = render(<HoloButton>Create account</HoloButton>);
    const button = screen.getByRole("button", { name: "Create account" });
    expect(button.classList.contains("holo")).toBe(true);
    expect(button.classList.contains("holo--live")).toBe(false);
    expect(container.querySelector("canvas")?.getAttribute("aria-hidden")).toBe("true");
    expect(() => {
      unmount();
    }).not.toThrow();
  });

  it("passes type, disabled and size through", () => {
    render(
      <HoloButton type="submit" disabled size="lg" block>
        Unlock
      </HoloButton>,
    );
    const button = screen.getByRole("button", { name: "Unlock" });
    expect(button.getAttribute("type")).toBe("submit");
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(button.className).toContain("button--lg");
    expect(button.className).toContain("button--block");
  });
});

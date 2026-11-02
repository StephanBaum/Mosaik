import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MenuButton } from "./Menu.tsx";

function setup() {
  const first = vi.fn();
  const second = vi.fn();
  render(
    <MenuButton
      label="More actions"
      items={[
        { label: "Duplicate", onSelect: first },
        { label: "Delete", onSelect: second, tone: "danger" },
      ]}
    />,
  );
  return { first, second, trigger: screen.getByRole("button", { name: "More actions" }) };
}

describe("MenuButton", () => {
  it("opens with the first item focused and moves with the arrow keys", async () => {
    const user = userEvent.setup();
    const { trigger } = setup();
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    await user.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const items = screen.getAllByRole("menuitem");
    expect(document.activeElement).toBe(items[0]);
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(items[1]);
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(items[0]);
    await user.keyboard("{ArrowUp}");
    expect(document.activeElement).toBe(items[1]);
    await user.keyboard("{Home}");
    expect(document.activeElement).toBe(items[0]);
    await user.keyboard("{End}");
    expect(document.activeElement).toBe(items[1]);
  });

  it("closes on Escape and returns focus to the trigger", async () => {
    const user = userEvent.setup();
    const { trigger } = setup();
    await user.click(trigger);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on a click outside", async () => {
    const user = userEvent.setup();
    const { trigger } = setup();
    await user.click(trigger);
    await user.click(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("runs the selected action and closes", async () => {
    const user = userEvent.setup();
    const { second, first, trigger } = setup();
    await user.click(trigger);
    await user.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(second).toHaveBeenCalledOnce();
    expect(first).not.toHaveBeenCalled();
    expect(screen.queryByRole("menu")).toBeNull();
  });
});

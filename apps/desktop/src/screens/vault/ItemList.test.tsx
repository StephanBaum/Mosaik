import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderedIndices, visibleRange, INITIAL_ROWS, OVERSCAN } from "../../hooks/useWindowedRows.ts";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { ItemSummary } from "../../ipc/contract/index.ts";
import { IpcProvider } from "../../ipc/IpcProvider.tsx";
import { ItemList, WINDOW_AFTER } from "./ItemList.tsx";

function summaries(count: number): ItemSummary[] {
  return Array.from({ length: count }, (_, index) => ({
    vaultId: "v1",
    itemId: `i${String(index)}`,
    kind: "login",
    title: `Site ${String(index)}`,
    subtitle: null,
    host: null,
    favorite: false,
    hasOtp: false,
    tags: [],
    modifiedAt: null,
    trashedAt: null,
    needsReview: false,
  }));
}

function stubIpc(items: ItemSummary[]): MosaicIpc {
  return {
    call: (command) =>
      Promise.resolve(
        command === "list_items"
          ? { ok: true, value: items }
          : { ok: false, error: { code: "internal", message: "not stubbed" } },
      ) as never,
    subscribe: () => Promise.resolve(() => undefined),
  };
}

function renderList(count: number, selected: string | null = null) {
  render(
    <IpcProvider ipc={stubIpc(summaries(count))}>
      <I18nProvider locale="en">
        <ItemList
          vaultId={null}
          trash={false}
          query=""
          category="all"
          findings={[]}
          selected={selected === null ? null : { vaultId: "v1", itemId: selected }}
          reloadKey={0}
          onSelect={() => undefined}
        />
      </I18nProvider>
    </IpcProvider>,
  );
}

describe("visibleRange", () => {
  it("covers the viewport plus the overscan, clamped to the list", () => {
    expect(visibleRange({ top: 0, bottom: 400 }, 50, 1000)).toEqual({ from: 0, to: 8 + OVERSCAN });
    expect(visibleRange({ top: 5000, bottom: 5400 }, 50, 1000)).toEqual({ from: 100 - OVERSCAN, to: 108 + OVERSCAN });
    expect(visibleRange({ top: 49_900, bottom: 50_300 }, 50, 1000)).toEqual({ from: 998 - OVERSCAN, to: 1000 });
  });

  it("is empty when the list is scrolled far out of view, and never inverted", () => {
    const range = visibleRange({ top: -9000, bottom: -8000 }, 50, 1000);
    expect(range).toEqual({ from: 0, to: 0 });
    const past = visibleRange({ top: 90_000, bottom: 91_000 }, 50, 1000);
    expect(past.from).toBeLessThanOrEqual(past.to);
    expect(past.to).toBe(1000);
  });

  it("renders a fixed first page before the row height is known", () => {
    expect(visibleRange({ top: 0, bottom: 800 }, 0, 1000)).toEqual({ from: 0, to: INITIAL_ROWS });
    expect(visibleRange({ top: 0, bottom: 800 }, 0, 5)).toEqual({ from: 0, to: 5 });
  });
});

describe("renderedIndices", () => {
  it("adds pinned rows in order and drops out-of-range pins", () => {
    expect(renderedIndices({ from: 3, to: 6 }, [900, 4, -1, 1, 2000], 1000)).toEqual([1, 3, 4, 5, 900]);
  });
});

describe("item list", () => {
  it("renders every row up to the window threshold", async () => {
    renderList(WINDOW_AFTER);
    await screen.findByRole("list", { name: `${String(WINDOW_AFTER)} items` });
    expect(screen.getAllByRole("listitem")).toHaveLength(WINDOW_AFTER);
    expect(screen.queryByRole("button", { name: /Show all/ })).toBeNull();
  }, 30_000); // 200 jsdom rows are slow when the whole suite runs in parallel

  it("windows a long list, announces its full size and keeps the open item rendered", async () => {
    renderList(5000, "i4321");
    const list = await screen.findByRole("list", { name: "5,000 items" });
    expect(list.classList.contains("item-list--windowed")).toBe(true);
    const rows = screen.getAllByRole("listitem");
    expect(rows.length).toBeLessThanOrEqual(INITIAL_ROWS + 1);
    expect(rows[0]?.getAttribute("aria-setsize")).toBe("5000");
    expect(rows[0]?.getAttribute("aria-posinset")).toBe("1");
    const open = screen.getByRole("button", { name: /Site 4321/ });
    expect(open.getAttribute("aria-current")).toBe("true");
    expect(open.closest("li")?.getAttribute("aria-posinset")).toBe("4322");
  });
});

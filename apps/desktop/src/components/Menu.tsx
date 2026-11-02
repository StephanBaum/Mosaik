/** A small actions menu (WAI-ARIA menu button): arrow keys move, Esc and Tab close. */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import type { IconName } from "./icons.tsx";
import { IconButton } from "./IconButton.tsx";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  tone?: "danger";
}

/** Closes the menu on a pointer press outside `anchor` and focuses the first item on open. */
function useMenuOpen(anchor: React.RefObject<HTMLDivElement | null>, firstItem: () => void) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) {
      return undefined;
    }
    firstItem();
    function onPointerDown(event: PointerEvent) {
      if (event.target instanceof Node && anchor.current?.contains(event.target) !== true) {
        setOpen(false);
      }
    }
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
    };
    // `firstItem` only reads a ref, so it is safe to leave out.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, anchor]);
  return [open, setOpen] as const;
}

/** The index to focus for a menu key, or `undefined` when the key does nothing. */
function targetIndex(key: string, current: number, count: number): number | undefined {
  switch (key) {
    case "ArrowDown":
      return (current + 1) % count;
    case "ArrowUp":
      return (current - 1 + count) % count;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return undefined;
  }
}

export function MenuButton({
  label,
  icon = "more",
  items,
}: {
  label: string;
  icon?: IconName;
  items: readonly MenuItem[];
}) {
  const id = useId();
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const entries = useRef<(HTMLButtonElement | null)[]>([]);
  const [open, setOpen] = useMenuOpen(anchor, () => entries.current[0]?.focus());

  function close(restoreFocus: boolean) {
    setOpen(false);
    if (restoreFocus) {
      trigger.current?.focus();
    }
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const current = entries.current.findIndex((entry) => entry === document.activeElement);
    const target = targetIndex(event.key, current, items.length);
    if (target !== undefined) {
      event.preventDefault();
      entries.current[target]?.focus();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close(true);
    } else if (event.key === "Tab") {
      close(true);
    }
  }

  return (
    <div className="menu-anchor" ref={anchor}>
      <IconButton
        ref={trigger}
        icon={icon}
        label={label}
        variant="outline"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={() => {
          setOpen((current) => !current);
        }}
      />
      {open && (
        <div id={id} role="menu" className="menu" aria-label={label} onKeyDown={onKeyDown}>
          {items.map((item, index) => (
            <button
              key={item.label}
              ref={(node) => {
                entries.current[index] = node;
              }}
              type="button"
              role="menuitem"
              className={item.tone === "danger" ? "menu__item menu__item--danger" : "menu__item"}
              onClick={() => {
                close(true);
                item.onSelect();
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * A menu button: arrow keys move, Home and End jump, Enter or Space choose, Esc closes and
 * gives focus back to the button, Tab closes. Items can be checkable (a switch in a menu).
 */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { IconButton } from "./Button.tsx";
import { Icon, type IconName } from "./Icon.tsx";

export interface MenuItem {
  id: string;
  label: string;
  /** A second line under the label. */
  description?: string;
  /** A keyboard hint shown at the end. */
  hint?: string;
  /** Present for a switch: the menu item is checkable. */
  checked?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}

const MOVES: Readonly<Record<string, (index: number, count: number) => number>> = {
  ArrowDown: (index) => index + 1,
  ArrowUp: (index) => index - 1,
  Home: () => 0,
  End: (_index, count) => count - 1,
};

function MenuList({
  id,
  label,
  items,
  onChoose,
  onKeyDown,
}: {
  id: string;
  label: string;
  items: readonly MenuItem[];
  onChoose: (item: MenuItem) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void;
}) {
  return (
    <div id={id} className="menu__list" role="menu" aria-label={label} onKeyDown={onKeyDown}>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className="menu__item"
          role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
          aria-checked={item.checked}
          // A description explains the item; it is not part of its name.
          aria-labelledby={item.description === undefined ? undefined : `${id}-${item.id}-label`}
          aria-describedby={item.description === undefined ? undefined : `${id}-${item.id}-description`}
          disabled={item.disabled}
          tabIndex={-1}
          onClick={() => {
            onChoose(item);
          }}
        >
          <span className="menu__text">
            <span id={`${id}-${item.id}-label`} className="menu__label">
              {item.label}
            </span>
            {item.description !== undefined && (
              <span id={`${id}-${item.id}-description`} className="menu__description">
                {item.description}
              </span>
            )}
          </span>
          {item.checked !== undefined && (
            <span className={item.checked ? "menu__check is-on" : "menu__check"} aria-hidden="true">
              {item.checked && <Icon name="check" size="sm" />}
            </span>
          )}
          {item.hint !== undefined && <kbd className="kbd">{item.hint}</kbd>}
        </button>
      ))}
    </div>
  );
}

export function Menu({
  icon,
  label,
  items,
  placement = "below",
}: {
  /** The icon of the button that opens the menu; `label` is its name. */
  icon: IconName;
  label: string;
  items: readonly MenuItem[];
  placement?: "below" | "above";
}) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement | null>(null);
  const root = useRef<HTMLDivElement | null>(null);
  const menuId = useId();

  function close(restoreFocus: boolean) {
    setOpen(false);
    if (restoreFocus) {
      button.current?.focus();
    }
  }

  useEffect(() => {
    if (!open) {
      return undefined;
    }
    root.current?.querySelector<HTMLElement>('[role^="menuitem"]:not(:disabled)')?.focus();
    const onPointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && root.current?.contains(event.target) !== true) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
    };
  }, [open]);

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const entries = [...event.currentTarget.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)')];
    const index = entries.indexOf(document.activeElement as HTMLElement);
    const step = MOVES[event.key];
    if (step !== undefined) {
      event.preventDefault();
      entries[(step(index, entries.length) + entries.length) % entries.length]?.focus();
    } else if (event.key === "Escape") {
      // The top layer only: a menu inside a dialog must not close the dialog.
      event.preventDefault();
      event.stopPropagation();
      close(true);
    } else if (event.key === "Tab") {
      close(false);
    }
  }

  return (
    <div className={`menu menu--${placement}`} ref={root}>
      <IconButton
        ref={button}
        icon={icon}
        label={label}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => {
          setOpen((was) => !was);
        }}
      />
      {open && (
        <MenuList
          id={menuId}
          label={label}
          items={items}
          onKeyDown={onKeyDown}
          onChoose={(item) => {
            close(true);
            item.onSelect();
          }}
        />
      )}
    </div>
  );
}

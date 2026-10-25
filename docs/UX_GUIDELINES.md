# Mosaic UX Guidelines

Every UI milestone is checked against this document by the `ux-reviewer`
subagent. Design tokens live in `packages/design-tokens`, and UI strings live
in `packages/i18n`. Hard-coded colors, spacing or strings in app code are
review failures.

## 1. Principles

1. **Calm.** Show one primary action per screen and no clutter. Secrets are
   hidden by default, but always one keystroke away.
2. **Fast.** The command palette opens in under 100 ms and search answers as
   you type. No screen makes the user wait without showing progress.
3. **Keyboard first.** Every action works without a mouse, and the mouse is
   fully supported too.
4. **Honest security.** Say plainly what protects the user and what does not.
   No fear-mongering, no "bank-grade", no "military-grade".
5. **Progressive disclosure.** Defaults are safe and good. Advanced settings
   sit behind an "Advanced" section, never on the first screen.

## 2. Design tokens

Use tokens only. Never use raw values in components.

- **Spacing scale (px):** 0, 2, 4, 8, 12, 16, 24, 32, 48, 64 (`space.0` …
  `space.9`).
- **Radius:** 4 (controls), 8 (cards), 12 (dialogs), and full (pills).
- **Type scale (px / line height):** 12/16, 14/20 (body), 16/24, 20/28,
  24/32, 32/40. System UI font stack, plus a monospace stack for secrets and
  codes.
- **Color:** semantic tokens (`bg`, `bg-subtle`, `surface`, `border`, `text`,
  `text-muted`, `accent`, `accent-text`, `danger`, `warning`, `success`,
  `focus-ring`) for the light and dark themes. Contrast, tested in CI:
  body text ≥ 4.5:1, large text and UI components/focus indicators ≥ 3:1.
- **Themes:** light, dark, and system (default).
- **Motion:** 120 ms (micro) and 200 ms (panels), ease-out.
  `prefers-reduced-motion` disables any non-essential motion.

## 3. Layout

- Desktop: sidebar (vaults, categories, tags), item list, and detail pane.
  Below 900 px it collapses to two panes. Minimum window size: 720 × 480.
- Extension popup: 360 × 600 maximum. It shows the matching items for the
  current site first.
- Lists use virtual scrolling past 200 rows.

## 4. Keyboard

| Action | Windows/Linux | macOS |
|---|---|---|
| Global: open palette / quick search | Ctrl+Shift+Space | ⌥⌘Space |
| Palette (in app) | Ctrl+K | ⌘K |
| New item | Ctrl+N | ⌘N |
| Copy username / password / TOTP | Ctrl+C / Ctrl+Shift+C / Ctrl+Alt+C | ⌘C / ⇧⌘C / ⌥⌘C |
| Lock | Ctrl+Alt+L | ⌥⌘L |
| Fill (extension) | Ctrl+Shift+L in page | ⇧⌘L |
| Settings | Ctrl+, | ⌘, |

Rules: the focus order follows the visual order, `Esc` always closes the
top-most layer, and the palette lists every command together with its
shortcut. Global shortcuts can be remapped, and the app detects conflicts.

## 5. Copy and error messages

- Every error states **what happened** and **what to do next**. Example:
  "Couldn't reach your sync server. Your changes are saved on this device and
  will sync when you're back online. [Retry]"
- Never show stack traces, error codes without explanation, or "Something went
  wrong" on its own. Technical details go behind "Details" and contain no
  secrets.
- Address the user directly, in the active voice. Use sentence case for
  labels.
- German uses "du". It is informal but respectful, and stays consistent
  everywhere.
- Destructive actions name the object ("Delete 'GitHub'?"), the button says
  the verb ("Delete", not "OK"), and they offer undo where possible (trash).

## 6. Internationalization

- Every user-facing string comes from `packages/i18n`. German and English are
  required, and a CI test enforces key and placeholder parity.
- ICU MessageFormat for plurals and variables. No string concatenation, no
  splitting sentences around variables.
- Layouts tolerate text that is 35 % longer (German). No fixed-width buttons
  for text.
- Dates, numbers and relative times use `Intl.*` with the user's locale.

## 7. Accessibility (WCAG 2.2 AA)

- Every interactive element has an accessible name. Icon-only buttons have
  `aria-label` plus a tooltip.
- A visible focus indicator: at least a 2 px `focus-ring` with ≥ 3:1 contrast,
  never obscured by sticky headers (2.4.11).
- Target size at least 24 × 24 px (2.5.8). Primary buttons are 32 px tall.
- Do not rely on color alone. Strength meters and dashboard states also use
  text or icons.
- Live regions announce "Copied, clears in 30 seconds", lock state changes and
  sync errors.
- Screen-reader labels for secrets: "Password, hidden. Press Space to reveal."
  Revealed secrets are read out character by character on request only.
- No time limits on input, except auto-lock, which is configurable and
  announced.
- Tests: `axe-core` via Playwright on every screen in both themes, plus manual
  keyboard-only walkthroughs per milestone.

## 8. Security UX patterns

- **Clipboard:** auto-clear after 30 s (configurable from 10 s to 5 min, or
  off with a warning), with a countdown in a toast. Excluded from clipboard
  history where the OS allows.
- **Reveal:** a per-field toggle that hides again after 60 s, on blur or on
  lock.
- **Re-authentication** before export, master password change, viewing the
  Secret Key, adding a quick-unlock method, or turning off auto-lock.
- **Emergency Kit:** generated during onboarding. The user must download or
  print it, then confirm by typing the last group of the Secret Key.
- **Warnings** (HTTP site, lookalike domain, reused password): one sentence,
  one recommended action, and a secondary "Fill anyway" behind an explicit
  click.
- **Onboarding budget:** under 2 minutes, in this order: create account →
  Emergency Kit → (optional) import from Enpass → install extension. Each step
  is skippable except the Emergency Kit.

## 9. Review checklist (used by `ux-reviewer`)

- [ ] Only tokens are used (no raw colors, sizes or spacing)
- [ ] All strings are in `packages/i18n`, DE and EN, and the screen was
      checked in German
- [ ] Complete keyboard path, visible focus, `Esc` behavior
- [ ] axe-core clean in light and dark
- [ ] Error states designed (offline, server error, wrong password, empty
      vault)
- [ ] Empty states have one clear next action
- [ ] Reduced motion respected
- [ ] No secrets visible without a user action. Clipboard clears.
- [ ] Copy follows §5

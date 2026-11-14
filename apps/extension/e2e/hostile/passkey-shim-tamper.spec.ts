/**
 * The shim's channel is page-visible by design (ADR-0040): the page can forge requests, answers
 * and aborts. None of it gives the page more than `navigator.credentials` gives it for its own
 * origin: origin and frame come from the browser, the request id from the background, and every
 * operation needs the desktop dialog.
 */
import type { Page } from "@playwright/test";
import {
  BANK,
  EVIL,
  bankRp,
  evilRp,
  expect,
  open,
  pendingDialog,
  rpCreate,
  rpGet,
  rpResult,
  rpStart,
  test,
} from "../fixtures.ts";

const CHALLENGE = "AAAAAAAAAAAAAAAAAAAAAA";

/** Posts `messages` to the page's own window and returns what the shim protocol answered within a moment. */
async function post(page: Page, messages: unknown[]): Promise<unknown[]> {
  return page.evaluate(async (list) => {
    const seen: unknown[] = [];
    window.addEventListener("message", (event) => {
      const data = event.data as { type?: string } | null;
      if (data?.type === "mosaic-passkey-res" || data?.type === "mosaic-passkey-ack") {
        seen.push(data);
      }
    });
    for (const message of list) {
      window.postMessage(message, "*");
    }
    await new Promise((resolve) => setTimeout(resolve, 600));
    return seen;
  }, messages);
}

const getRequest = (id: number, rpId: string | null, extra: object = {}) => ({
  type: "mosaic-passkey-req",
  id,
  op: "get",
  options: { rpId, challenge: CHALLENGE, allowCredentials: [], userVerification: "preferred" },
  ...extra,
});

test("a forged request names the browser's origin, not the page's claim", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, evilRp);

  // A claimed origin is not part of the message: the extra member makes it unknown and it is dropped without a word.
  expect(await post(page, [getRequest(41, null, { origin: BANK })])).toEqual([]);
  expect(await fake.passkeyLog()).toEqual([]);

  // A well-formed forgery asking for bank.test's rp id reaches the desktop as evil.test, and is refused there.
  const answers = await post(page, [getRequest(42, "bank.test")]);
  expect(answers).toEqual([
    { type: "mosaic-passkey-ack", id: 42 },
    { type: "mosaic-passkey-res", id: 42, result: { kind: "fallback" } },
  ]);
  expect(await fake.passkeyLog()).toMatchObject([
    { op: "passkeyGet", origin: EVIL, topOrigin: EVIL, rpId: "bank.test", result: "rpIdMismatch" },
  ]);

  // A forged registration for the page's own rp id opens a dialog that names evil.test; the user can say no.
  await post(page, [
    {
      type: "mosaic-passkey-req",
      id: 43,
      op: "create",
      options: {
        rpId: null,
        userId: "AQID",
        userName: "x",
        userDisplayName: "x",
        challenge: CHALLENGE,
        algorithms: [-7],
        excludeCredentials: [],
        userVerification: "preferred",
      },
    },
  ]);
  const dialog = (await fake.passkeyPending())[0];
  expect(dialog).toMatchObject({ op: "passkeyCreate", origin: EVIL, rpId: "evil.test" });
  await fake.passkeyDecide({ kind: "cancel" });
});

test("breaking the page's own postMessage or addEventListener does not stop the shim", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  await page.evaluate(() => {
    window.postMessage = () => {
      throw new Error("broken");
    };
    window.addEventListener = () => {
      throw new Error("broken");
    };
  });
  await fake.passkeyDecide({ kind: "allow" });
  expect(await rpCreate(page)).toMatchObject({ verify: { ok: true } });
});

test("a page that replaces get after load only reaches its own function", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  await page.evaluate(() => {
    CredentialsContainer.prototype.get = () => Promise.reject(new DOMException("page's own", "NotSupportedError"));
  });
  expect((await rpGet(page, { noVerify: true })).error?.name).toBe("NotSupportedError");
  expect(await fake.passkeyLog()).toEqual([]);
});

test("forged aborts and answers from another tab change nothing, and a forged answer only ends the page's own call", async ({
  page,
  context,
  fake,
}) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  const other: Page = await context.newPage();
  await open(other, evilRp);
  await page.bringToFront();

  await rpStart(page, "create");
  const dialog = await pendingDialog(fake);
  expect(dialog.origin).toBe(BANK);

  // Tab B posts aborts and answers for every id tab A could be using.
  await other.evaluate(() => {
    for (let id = 0; id < 8; id++) {
      window.postMessage({ type: "mosaic-passkey-abort", id }, "*");
      window.postMessage({ type: "mosaic-passkey-res", id, result: { kind: "error", name: "NotAllowedError" } }, "*");
    }
  });
  await other.waitForTimeout(600);
  expect(await fake.passkeyPending()).toMatchObject([{ requestId: dialog.requestId }]);
  expect((await fake.passkeyLog()).filter((e) => e.op === "passkeyCancel")).toEqual([]);

  // Tab A's call was untouched and completes.
  await fake.passkeyDecide({ kind: "allow" });
  expect(await rpResult(page)).toMatchObject({ verify: { ok: true } });

  // Tab B's own call ends with the answer it forged for itself; that is its right.
  await other.bringToFront();
  // The page's call registers id 1 in the shim only once its own steps are done, and a forged answer
  // that arrives earlier is dropped (the call would then wait for the desktop for good). The bridge's
  // ack for id 1 proves the shim has registered it, so forge only after that.
  await other.evaluate(() => {
    (window as unknown as { acked: Promise<void> }).acked = new Promise<void>((resolve) => {
      window.addEventListener("message", (event) => {
        const data = event.data as { type?: string; id?: number } | null;
        if (data?.type === "mosaic-passkey-ack" && data.id === 1) {
          resolve();
        }
      });
    });
  });
  await rpStart(other, "get", { noVerify: true });
  await other.evaluate(() => (window as unknown as { acked: Promise<void> }).acked);
  await other.evaluate(() => {
    window.postMessage({ type: "mosaic-passkey-res", id: 1, result: { kind: "error", name: "NotAllowedError" } }, "*");
  });
  expect((await rpResult(other)).error?.name).toBe("NotAllowedError");
  await other.close();
});

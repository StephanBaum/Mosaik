// @ts-check
/* global atob, btoa, document, history, location, navigator */
/**
 * The security-key page of the sync server (CRYPTO_SPEC §9.3, ADR-0077).
 *
 * The Mosaic app opens `/webauthn/#<handoff id>` in the system browser and shows a 6-digit code.
 * This page reads the ID from the URL fragment only (never from the query), asks the user to type
 * the code, fetches the WebAuthn options with it, runs `navigator.credentials.create` or `get`
 * and posts the answer. Plain ES2022, no framework, no inline script (the CSP allows scripts from
 * this origin only). Everything the server or the user supplies (the device name, the account) is
 * written with `textContent`, never as HTML. Texts come from `strings.json`.
 */

const ID_PATTERN = /^[A-Za-z0-9_-]{22}$/;
const CODE_PATTERN = /^[0-9]{6}$/;
/** Wrong codes the page counts before it gives up (the server ends the handoff at five). */
const MAX_WRONG_CODES = 5;
const PURPOSES = ["login", "register", "confirm"];

/** @typedef {Record<string, string>} Catalog */
/** @typedef {"login" | "register" | "confirm"} Purpose */
/** @typedef {Record<string, unknown>} Json */
/**
 * @typedef {object} Env
 * @property {Document} document
 * @property {string} hash `location.hash`.
 * @property {typeof fetch} fetch
 * @property {CredentialsContainer | undefined} credentials
 * @property {readonly string[]} languages
 * @property {() => void} [clearFragment] Removes the ID from the address bar and the history entry once it is read.
 */
/**
 * @typedef {object} Options
 * @property {Purpose} purpose
 * @property {string} deviceName
 * @property {number} requestedAt
 * @property {Json} publicKey
 */

/** @param {unknown} v @returns {v is Json} */
function isRecord(v) {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** @param {string} s @returns {Uint8Array} */
function fromB64url(s) {
  const plain = s.replaceAll("-", "+").replaceAll("_", "/");
  const bin = atob(plain.padEnd(Math.ceil(plain.length / 4) * 4, "="));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/** @param {ArrayBuffer} buf @returns {string} */
function toB64url(buf) {
  let bin = "";
  for (const b of new Uint8Array(buf)) bin += String.fromCharCode(b);
  return btoa(bin).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

/** The handoff ID of a URL fragment, or null if it is not a 16-byte base64url value. @param {string} hash */
export function handoffId(hash) {
  const id = hash.startsWith("#") ? hash.slice(1) : hash;
  return ID_PATTERN.test(id) ? id : null;
}

/** @param {readonly string[]} languages @returns {"en" | "de"} */
export function pickLocale(languages) {
  for (const tag of languages) {
    const base = tag.toLowerCase().split("-")[0];
    if (base === "de" || base === "en") return base;
  }
  return "en";
}

/** @param {Catalog} catalog @returns {(key: string, values?: Record<string, string>) => string} */
function translator(catalog) {
  return (key, values) =>
    (catalog[key] ?? key).replace(/\{([a-zA-Z]+)\}/g, (_, name) => values?.[name] ?? "");
}

/** @param {Document} doc */
function refs(doc) {
  /** @param {string} id @returns {HTMLElement} */
  const el = (id) => {
    const found = doc.getElementById(id);
    if (found === null) throw new Error(`missing #${id}`);
    return found;
  };
  return {
    heading: el("heading"),
    lead: el("lead"),
    facts: el("facts"),
    factDevice: el("fact-device"),
    factTime: el("fact-time"),
    factAccount: el("fact-account"),
    factAccountRow: el("fact-account-row"),
    factLabels: {
      device: el("fact-device-label"),
      time: el("fact-time-label"),
      account: el("fact-account-label"),
    },
    form: /** @type {HTMLFormElement} */ (el("code-form")),
    codeLabel: el("code-label"),
    code: /** @type {HTMLInputElement} */ (el("code")),
    codeError: el("code-error"),
    submit: /** @type {HTMLButtonElement} */ (el("code-submit")),
    status: el("status"),
    message: el("message"),
    retry: /** @type {HTMLButtonElement} */ (el("retry")),
  };
}

/**
 * @typedef {object} Answer
 * @property {number} status HTTP status, 0 if the server could not be reached.
 * @property {unknown} json
 */

/** @param {Env} env @param {string} path @param {unknown} body @returns {Promise<Answer>} */
async function post(env, path, body) {
  try {
    const r = await env.fetch(path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      credentials: "omit",
      cache: "no-store",
      referrerPolicy: "no-referrer",
    });
    /** @type {unknown} */
    let json = null;
    try {
      json = await r.json();
    } catch {
      json = null;
    }
    return { status: r.status, json };
  } catch {
    return { status: 0, json: null };
  }
}

/** @param {unknown} v @returns {Options | null} */
function parseOptions(v) {
  if (!isRecord(v)) return null;
  const { purpose, deviceName, requestedAt, publicKey } = v;
  if (typeof purpose !== "string" || !PURPOSES.includes(purpose)) return null;
  if (typeof deviceName !== "string" || typeof requestedAt !== "number") return null;
  if (!isRecord(publicKey)) return null;
  return { purpose: /** @type {Purpose} */ (purpose), deviceName, requestedAt, publicKey };
}

/** @param {unknown} list @returns {unknown[] | undefined} */
function descriptors(list) {
  if (!Array.isArray(list)) return undefined;
  return list.map((d) => {
    if (!isRecord(d) || typeof d["id"] !== "string") throw new Error("bad credential descriptor");
    return { ...d, id: fromB64url(d["id"]) };
  });
}

/** @param {Json} pk @returns {Json} */
function decodeOptions(pk) {
  if (typeof pk["challenge"] !== "string") throw new Error("no challenge");
  /** @type {Json} */
  const out = { ...pk, challenge: fromB64url(pk["challenge"]) };
  const user = pk["user"];
  if (isRecord(user) && typeof user["id"] === "string") {
    out["user"] = { ...user, id: fromB64url(user["id"]) };
  }
  for (const key of ["excludeCredentials", "allowCredentials"]) {
    if (key in pk) out[key] = descriptors(pk[key]);
  }
  return out;
}

/**
 * Runs the browser ceremony and returns the credential as the server wants it (base64url
 * members, the shape of `PublicKeyCredential.toJSON()`).
 * @param {CredentialsContainer} credentials @param {Options} o @returns {Promise<Json>}
 */
async function ceremony(credentials, o) {
  const publicKey = decodeOptions(o.publicKey);
  if (o.purpose === "register") {
    const made = await credentials.create({
      publicKey: /** @type {PublicKeyCredentialCreationOptions} */ (/** @type {unknown} */ (publicKey)),
    });
    const cred = /** @type {PublicKeyCredential | null} */ (made);
    if (cred === null) throw new Error("no credential");
    const r = /** @type {AuthenticatorAttestationResponse} */ (cred.response);
    const transports = typeof r.getTransports === "function" ? r.getTransports() : [];
    return {
      id: cred.id,
      rawId: toB64url(cred.rawId),
      type: cred.type,
      response: {
        clientDataJSON: toB64url(r.clientDataJSON),
        attestationObject: toB64url(r.attestationObject),
        ...(transports.length > 0 ? { transports } : {}),
      },
    };
  }
  const got = await credentials.get({
    publicKey: /** @type {PublicKeyCredentialRequestOptions} */ (/** @type {unknown} */ (publicKey)),
  });
  const cred = /** @type {PublicKeyCredential | null} */ (got);
  if (cred === null) throw new Error("no credential");
  const r = /** @type {AuthenticatorAssertionResponse} */ (cred.response);
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      clientDataJSON: toB64url(r.clientDataJSON),
      authenticatorData: toB64url(r.authenticatorData),
      signature: toB64url(r.signature),
      ...(r.userHandle !== null && r.userHandle.byteLength > 0
        ? { userHandle: toB64url(r.userHandle) }
        : {}),
    },
  };
}

/** @param {typeof fetch} fetchFn @returns {Promise<{ en: Catalog, de: Catalog } | null>} */
async function loadStrings(fetchFn) {
  try {
    const r = await fetchFn("/webauthn/strings.json", { cache: "no-store" });
    return /** @type {{ en: Catalog, de: Catalog }} */ (await r.json());
  } catch {
    return null;
  }
}

/**
 * Starts the page: reads the fragment, shows the code form and drives the ceremony.
 * @param {Env} env
 * @returns {Promise<void>}
 */
export async function start(env) {
  const ui = refs(env.document);
  const catalogs = await loadStrings(env.fetch);
  if (catalogs === null) return;
  const locale = pickLocale(env.languages);
  const t = translator(catalogs[locale]);
  env.document.documentElement.lang = locale;
  env.document.title = t("webauthn.documentTitle");
  ui.factLabels.device.textContent = t("webauthn.facts.device");
  ui.factLabels.time.textContent = t("webauthn.facts.time");
  ui.factLabels.account.textContent = t("webauthn.facts.account");
  ui.codeLabel.textContent = t("webauthn.code.label");
  ui.submit.textContent = t("webauthn.code.submit");

  /**
   * One screen: every part is replaced, none is left over from the one before.
   * @param {{ heading: string, lead?: string, status?: string, message?: string, retry?: string, form?: boolean, facts?: boolean }} v
   */
  const render = (v) => {
    ui.heading.textContent = v.heading;
    ui.lead.textContent = v.lead ?? "";
    ui.status.textContent = v.status ?? "";
    ui.message.textContent = v.message ?? "";
    ui.form.hidden = v.form !== true;
    ui.facts.hidden = v.facts !== true;
    ui.retry.hidden = v.retry === undefined;
    ui.retry.textContent = v.retry ?? "";
    // Every screen starts at its heading, so a screen reader announces what changed.
    ui.heading.focus();
  };
  /** A final screen; the heading takes the focus so a screen reader announces it. @param {"expired" | "failed" | "invalid"} kind @param {string} [body] */
  const end = (kind, body) => {
    render({ heading: t(`webauthn.heading.${kind}`), message: body ?? t(`webauthn.${kind}.body`) });
    ui.heading.focus();
  };

  const id = handoffId(env.hash);
  if (id === null) {
    end("invalid");
    return;
  }
  // The ID is read: it does not stay in the address bar or in the history (M9-F72).
  env.clearFragment?.();

  let wrongCodes = 0;
  /** @param {string} text @param {boolean} focus */
  const codeError = (text, focus) => {
    ui.codeError.textContent = text;
    ui.code.setAttribute("aria-invalid", text === "" ? "false" : "true");
    if (focus) ui.code.focus();
  };
  const askForCode = () => {
    render({ heading: t("webauthn.heading.code"), lead: t("webauthn.lead.code"), form: true });
    ui.code.focus();
  };

  /** @param {Options} o */
  const present = (o) => {
    const when = new Intl.DateTimeFormat(locale, { dateStyle: "short", timeStyle: "short" }).format(
      new Date(o.requestedAt),
    );
    ui.factDevice.textContent = o.deviceName;
    ui.factTime.textContent = when;
    const user = o.publicKey["user"];
    const account = isRecord(user) && typeof user["name"] === "string" ? user["name"] : "";
    ui.factAccount.textContent = account;
    ui.factAccountRow.hidden = account === "";
  };

  /** @param {Options} o */
  const run = async (o) => {
    const head = t(`webauthn.heading.${o.purpose}`);
    present(o);
    if (env.credentials === undefined) {
      render({ heading: head, message: t("webauthn.unsupported") });
      return;
    }
    render({ heading: head, lead: t("webauthn.lead.ask"), status: t("webauthn.status.waiting"), facts: true });
    /** @type {Json} */
    let credential;
    try {
      credential = await ceremony(env.credentials, o);
    } catch (e) {
      const name = isRecord(e) || e instanceof Error ? String(Reflect.get(e, "name")) : "";
      const key =
        name === "InvalidStateError"
          ? "webauthn.alreadyRegistered"
          : name === "NotAllowedError" || name === "AbortError"
            ? "webauthn.cancelled"
            : "webauthn.browserError";
      render({ heading: head, message: t(key), retry: t("webauthn.retry"), facts: true });
      again = () => void run(o);
      return;
    }
    await finish(o, head, credential);
  };

  /** @param {Options} o @param {string} head @param {Json} credential */
  const finish = async (o, head, credential) => {
    render({ heading: head, status: t("webauthn.status.sending"), facts: true });
    const r = await post(env, "/v1/webauthn/handoff/finish", { handoffId: id, credential });
    if (r.status === 200) {
      render({ heading: t("webauthn.heading.done"), message: t(`webauthn.done.${o.purpose}`) });
      ui.heading.focus();
    } else if (r.status === 410) {
      end("expired");
    } else if (r.status === 429) {
      end("failed", t("webauthn.rateLimited"));
    } else if (r.status === 0) {
      render({ heading: head, message: t("webauthn.network"), retry: t("webauthn.retry"), facts: true });
      again = () => void finish(o, head, credential);
    } else {
      end("failed");
    }
  };

  /** What the retry button does now. @type {() => void} */
  let again = () => undefined;
  ui.retry.addEventListener("click", () => {
    again();
  });

  ui.form.addEventListener("submit", (event) => {
    event.preventDefault();
    const typed = ui.code.value.trim();
    if (!CODE_PATTERN.test(typed)) {
      codeError(t("webauthn.code.format"), true);
      return;
    }
    codeError("", false);
    ui.submit.disabled = true;
    ui.status.textContent = t("webauthn.status.sending");
    void post(env, "/v1/webauthn/handoff/options", { handoffId: id, code: typed }).then((r) => {
      ui.submit.disabled = false;
      ui.status.textContent = "";
      const options = r.status === 200 ? parseOptions(r.json) : null;
      if (options !== null) {
        ui.code.value = "";
        void run(options);
      } else if (r.status === 401) {
        wrongCodes += 1;
        if (wrongCodes >= MAX_WRONG_CODES) end("failed", t("webauthn.code.tooMany"));
        else codeError(t("webauthn.code.invalid"), true);
      } else if (r.status === 410) {
        end("expired");
      } else if (r.status === 409) {
        end("failed", t("webauthn.unavailable"));
      } else if (r.status === 429) {
        end("failed", t("webauthn.rateLimited"));
      } else if (r.status === 0) {
        codeError(t("webauthn.network"), true);
      } else {
        end("failed");
      }
    });
  });

  askForCode();
}

if (document.getElementById("app") !== null) {
  void start({
    document,
    hash: location.hash,
    fetch: globalThis.fetch.bind(globalThis),
    credentials: navigator.credentials,
    languages: navigator.languages,
    clearFragment: () => {
      history.replaceState(null, "", location.pathname);
    },
  });
}

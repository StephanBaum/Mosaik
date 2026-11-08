/**
 * Response headers of the website (design concept §5 Privacy, website v2 §1.4.5). Zero
 * third-party requests, no inline scripts or styles, and `connect-src 'none'` on every page:
 * no page can send anything anywhere, the self-check included. The CSP is generated per route
 * from the route table: pages that render the capture form get `form-action 'self'` once
 * capture is enabled. The build writes the policy into each page (minus `frame-ancestors`,
 * which a meta tag cannot carry) and into `_headers`, one block per path.
 */
import { NOT_FOUND_PATH, ROUTES, type Route } from "./routes.ts";
import { site } from "./site.ts";

function directives(r: Route | null): string[] {
  const form = r?.capture === true && site.captureEnabled ? "form-action 'self'" : "form-action 'none'";
  return [
    "default-src 'none'",
    "script-src 'self'",
    "style-src 'self'",
    "img-src 'self'",
    "font-src 'self'",
    "connect-src 'none'",
    "manifest-src 'self'",
    "base-uri 'none'",
    form,
    "frame-ancestors 'none'",
  ];
}

/** The full policy of a page (`null`: the 404 page). */
export function cspFor(r: Route | null): string {
  return directives(r).join("; ");
}

/** The policy as a meta tag can carry it. */
export function cspMetaFor(r: Route | null): string {
  return directives(r)
    .filter((d) => !d.startsWith("frame-ancestors"))
    .join("; ");
}

/** The default policy, also used by `vite preview`. */
export const CSP = cspFor(null);

const OTHER_HEADERS: Readonly<Record<string, string>> = {
  "Referrer-Policy": "no-referrer",
  "Permissions-Policy":
    "accelerometer=(), autoplay=(), camera=(), clipboard-read=(), display-capture=(), encrypted-media=(), fullscreen=(), geolocation=(), gyroscope=(), hid=(), magnetometer=(), microphone=(), midi=(), payment=(), picture-in-picture=(), publickey-credentials-get=(), screen-wake-lock=(), serial=(), usb=(), xr-spatial-tracking=()",
  "Cross-Origin-Opener-Policy": "same-origin",
  "X-Content-Type-Options": "nosniff",
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains",
};

export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  "Content-Security-Policy": CSP,
  ...OTHER_HEADERS,
};

/**
 * The headers as a Netlify / Cloudflare Pages `_headers` file. The shared headers apply to
 * every path; each page path then gets exactly one CSP, so policies never stack.
 */
export function headersFile(): string {
  const block = (path: string, headers: Readonly<Record<string, string>>): string[] => [
    path,
    ...Object.entries(headers).map(([name, value]) => `  ${name}: ${value}`),
  ];
  const lines = [...block("/*", OTHER_HEADERS)];
  for (const r of ROUTES) {
    for (const path of [r.en, r.de]) {
      lines.push(...block(path, { "Content-Security-Policy": cspFor(r) }));
    }
  }
  lines.push(...block(NOT_FOUND_PATH, { "Content-Security-Policy": cspFor(null) }));
  return [...lines, ""].join("\n");
}

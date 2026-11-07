/**
 * Response headers of the landing page (design concept §5 Privacy). Zero third-party
 * requests, no inline scripts or styles. The host sets them; the build also writes a
 * `_headers` file and puts the CSP (minus `frame-ancestors`, which a meta tag cannot carry)
 * into every page.
 */
const DIRECTIVES = [
  "default-src 'none'",
  "script-src 'self'",
  "style-src 'self'",
  "img-src 'self'",
  "font-src 'self'",
  "connect-src 'none'",
  "manifest-src 'self'",
  "base-uri 'none'",
  "form-action 'none'",
  "frame-ancestors 'none'",
] as const;

export const CSP = DIRECTIVES.join("; ");

export const CSP_META = DIRECTIVES.filter((d) => !d.startsWith("frame-ancestors")).join("; ");

export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  "Content-Security-Policy": CSP,
  "Referrer-Policy": "no-referrer",
  "Permissions-Policy":
    "accelerometer=(), autoplay=(), camera=(), clipboard-read=(), display-capture=(), encrypted-media=(), fullscreen=(), geolocation=(), gyroscope=(), hid=(), magnetometer=(), microphone=(), midi=(), payment=(), picture-in-picture=(), publickey-credentials-get=(), screen-wake-lock=(), serial=(), usb=(), xr-spatial-tracking=()",
  "Cross-Origin-Opener-Policy": "same-origin",
  "X-Content-Type-Options": "nosniff",
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains",
};

/** The headers as a Netlify / Cloudflare Pages `_headers` file. */
export function headersFile(): string {
  const lines = Object.entries(SECURITY_HEADERS).map(([name, value]) => `  ${name}: ${value}`);
  return ["/*", ...lines, ""].join("\n");
}

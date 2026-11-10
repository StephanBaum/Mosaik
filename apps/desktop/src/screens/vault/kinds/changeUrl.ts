/**
 * Whether Mosaic would use a change-page address (the desktop's rule, `change_url::resolve`): it
 * must be an `https` URL on the same site as one of the login's own `https` addresses. The site is
 * the last two host labels (three under a two-letter country domain with a short second level):
 * a guess that errs towards warning, and only a warning; the desktop decides.
 */
function siteOf(host: string): string | null {
  const labels = host.toLowerCase().split(".");
  if (labels.length < 2 || labels.some((label) => label === "") || /^[\d.]+$/.test(host)) {
    return null;
  }
  const tld = labels.at(-1) ?? "";
  const second = labels.at(-2) ?? "";
  const keep = tld.length === 2 && second.length <= 3 && labels.length >= 3 ? 3 : 2;
  return labels.slice(-keep).join(".");
}

function httpsHost(address: string): string | null {
  try {
    const url = new URL(address.trim());
    return url.protocol === "https:" ? url.hostname : null;
  } catch {
    return null;
  }
}

export function usableChangeUrl(address: string, loginUrls: readonly string[]): boolean {
  const host = httpsHost(address);
  const site = host === null ? null : siteOf(host);
  if (site === null) {
    return false;
  }
  return loginUrls.some((own) => {
    const ownHost = httpsHost(own);
    return ownHost !== null && siteOf(ownHost) === site;
  });
}

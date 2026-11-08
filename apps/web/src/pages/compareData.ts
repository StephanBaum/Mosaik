/**
 * The comparison matrix and its sources (marketing/comparison.md §1 and §5, origin/main,
 * researched 2026-11). A cell is a catalog key plus source numbers, or `null` where the fact
 * could not be verified: that cell renders "not verified", never a guess. Rows whose Mosaic
 * column would need an owner decision (licence) are left out until it is made.
 */
import type { WebKey } from "./i18n.ts";

export const PRODUCTS = ["Mosaic", "1Password", "Bitwarden", "Proton Pass", "KeePassXC", "Enpass"] as const;

export type Product = (typeof PRODUCTS)[number];

export interface Cell {
  k: WebKey;
  refs?: readonly number[];
}

export interface Row {
  id: string;
  label: WebKey;
  /** One cell per product, in `PRODUCTS` order. */
  cells: readonly [Cell, ...(Cell | null)[]];
}

const v = (k: WebKey, ...refs: number[]): Cell => (refs.length === 0 ? { k } : { k, refs });

export const ROWS: readonly Row[] = [
  {
    id: "sync",
    label: "web.cmp.row.sync",
    cells: [
      v("web.cmp.sync.mosaic"),
      v("web.cmp.v.vendorHosted", 1),
      v("web.cmp.sync.bitwarden", 2),
      v("web.cmp.v.vendorHosted", 7),
      v("web.cmp.sync.keepassxc", 4),
      v("web.cmp.sync.enpass", 10),
    ],
  },
  {
    id: "detect",
    label: "web.cmp.row.detect",
    cells: [v("web.cmp.detect.mosaic"), v("web.cmp.v.notDescribed", 1, 11), null, null, null, null],
  },
  {
    id: "login",
    label: "web.cmp.row.login",
    cells: [
      v("web.cmp.login.mosaic"),
      v("web.cmp.login.srp", 1),
      v("web.cmp.login.bitwarden", 12),
      v("web.cmp.login.srp", 13),
      v("web.cmp.v.noAccount"),
      v("web.cmp.v.noAccount"),
    ],
  },
  {
    id: "secret",
    label: "web.cmp.row.secret",
    cells: [v("web.cmp.secret.mosaic"), v("web.cmp.secret.1password", 1), null, null, v("web.cmp.v.keyFile"), null],
  },
  {
    id: "eth",
    label: "web.cmp.row.eth",
    cells: [
      v("web.cmp.eth.mosaic"),
      v("web.cmp.eth.1password", 11),
      v("web.cmp.eth.bitwarden", 11),
      v("web.cmp.v.notStudied", 11),
      v("web.cmp.v.notStudied", 11),
      v("web.cmp.v.notStudied", 11),
    ],
  },
  {
    id: "matching",
    label: "web.cmp.row.matching",
    cells: [v("web.cmp.matching.mosaic"), v("web.cmp.matching.1password", 14), v("web.cmp.matching.bitwarden", 15), null, null, null],
  },
  {
    id: "telemetry",
    label: "web.cmp.row.telemetry",
    cells: [v("web.cmp.telemetry.mosaic"), v("web.cmp.telemetry.1password", 16), null, null, null, null],
  },
  {
    id: "passkeys",
    label: "web.cmp.row.passkeys",
    cells: [
      v("web.cmp.passkeys.mosaic"),
      null,
      v("web.cmp.v.yes", 8),
      v("web.cmp.passkeys.proton", 7),
      v("web.cmp.passkeys.keepassxc", 4),
      null,
    ],
  },
  {
    id: "ssh",
    label: "web.cmp.row.ssh",
    cells: [v("web.cmp.ssh.mosaic"), null, v("web.cmp.ssh.bitwarden", 17), v("web.cmp.ssh.proton", 18), null, null],
  },
  {
    id: "mobile",
    label: "web.cmp.row.mobile",
    cells: [v("web.cmp.mobile.mosaic"), v("web.cmp.v.yes"), v("web.cmp.v.yes"), v("web.cmp.v.yes", 7), v("web.cmp.mobile.keepassxc", 4), v("web.cmp.v.yes")],
  },
  {
    id: "audit",
    label: "web.cmp.row.audit",
    cells: [
      v("web.cmp.audit.mosaic"),
      v("web.cmp.audit.1password", 5),
      v("web.cmp.audit.bitwarden", 2),
      v("web.cmp.audit.proton", 3),
      v("web.cmp.audit.keepassxc", 4),
      null,
    ],
  },
  {
    id: "price",
    label: "web.cmp.row.price",
    cells: [
      v("web.cmp.price.mosaic"),
      v("web.cmp.price.1password", 6),
      v("web.cmp.price.bitwarden", 9),
      v("web.cmp.price.proton", 7),
      v("web.cmp.v.free"),
      null,
    ],
  },
  {
    id: "track",
    label: "web.cmp.row.track",
    cells: [v("web.cmp.track.mosaic"), v("web.cmp.v.long"), v("web.cmp.v.long"), v("web.cmp.track.proton"), v("web.cmp.v.long"), v("web.cmp.v.long")],
  },
];

/** Numbered sources, as listed in comparison.md §5 (document titles are not translated). */
export const SOURCES: readonly (readonly [title: string, url: string])[] = [
  ["1Password Security Design white paper, release 0.5.2", "https://agilebits.github.io/security-design/1Password_Security_Design.pdf"],
  ["Bitwarden: ETH Zurich audits Bitwarden cryptography", "https://bitwarden.com/blog/security-through-transparency-eth-zurich-audits-bitwarden-cryptography/"],
  ["Proton: Pass open source security audit", "https://proton.me/blog/pass-open-source-security-audit"],
  ["KeePassXC project page", "https://github.com/keepassxreboot/keepassxc/"],
  ["1Password security assessments", "https://support.1password.com/security-assessments/"],
  ["MacRumors: 1Password March price increase", "https://www.macrumors.com/2026/02/24/1password-march-price-increase/"],
  ["Proton Pass pricing", "https://proton.me/pass/pricing"],
  ["Blackvoid: Bitwarden biometrics and passkeys", "https://www.blackvoid.club/bitwarden-biometrics-passkeys-new-passwordless-technologies/"],
  ["Business Wire: Bitwarden plans and prices, January 2026", "https://www.businesswire.com/news/home/20260121289564/en/"],
  ["Enpass manual: sync", "https://www.enpass.io/docs/manual-desktop/sync.html"],
  ["ETH Zurich, “Zero Knowledge (About) Encryption”, IACR ePrint 2026/058", "https://eprint.iacr.org/2026/058"],
  ["Bitwarden security white paper", "https://bitwarden.com/help/bitwarden-security-white-paper/"],
  ["Proton: encrypted email authentication (SRP)", "https://proton.me/blog/encrypted-email-authentication"],
  ["1Password: autofill behavior", "https://support.1password.com/autofill-behavior/"],
  ["Bitwarden: URI match detection", "https://bitwarden.com/help/uri-match-detection/"],
  ["1Password: telemetry", "https://support.1password.com/telemetry"],
  ["Bitwarden: SSH agent", "https://bitwarden.com/help/ssh-agent/"],
  ["Proton Pass roadmap, spring and summer 2026", "https://proton.me/blog/pass-roadmap-spring-summer-2026"],
];

export const SOURCE_URLS: readonly string[] = SOURCES.map(([, url]) => url);

/** The sources a set of rows cites, in order of first use. */
export function citedSources(rows: readonly Row[], columns: readonly number[]): number[] {
  const seen = new Set<number>();
  for (const row of rows) {
    for (const c of columns) {
      for (const ref of row.cells[c]?.refs ?? []) {
        seen.add(ref);
      }
    }
  }
  return [...seen].sort((a, b) => a - b);
}

/**
 * Launch gate for claims about features that ship after the site (website launch plan, Task 16).
 * Each gate names the catalog keys (exact, or a prefix ending in ".") whose text promises
 * something that must exist in the release the site goes live with, and what to check. All gates
 * start unverified: the owner flips `verified` after checking the release. A production build
 * (`MOSAIC_DEPLOY=production`) stops while any gate is open, listing the ids.
 */
export interface Claim {
  id: string;
  /** Catalog keys the claim covers: an exact key, or a prefix (a key starting with it). */
  keys: readonly string[];
  /** What to verify in the release before flipping `verified`. */
  check: string;
  verified: boolean;
}

export const CLAIMS: readonly Claim[] = [
  {
    id: "G1",
    keys: ["web.agent.", "web.agent.t."],
    check:
      "`mosaic agent mcp` is released (stateless MCP and agent access phase 1, merged in PRs #97 and #92); references are `mosaic://<vault>/<item>/<field>`; requests to hosts outside the key's list are refused; responses are masked; the client named in `web.agent.t.session` is a tested client",
    verified: false,
  },
  {
    id: "G2",
    keys: [
      "web.tools.import",
      "web.tools.import.",
      "web.switch.bring.text",
      "web.pricing.free.item4",
      "web.switchPage.src.",
      "web.switchPage.passkeys.",
      "web.switchPage.meta.description",
    ],
    check:
      "every source named has a parser and a guide in the released desktop app (importers merged in PR #94, onboarding-import plan); the cleanup step offers to delete the export",
    verified: false,
  },
  {
    id: "G3",
    keys: ["web.tools.intro", "web.tools.programs", "web.features.programs."],
    check: "Windows program fill phase 2 (merged in PR #96) ships enabled in the Windows release",
    verified: false,
  },
  {
    id: "G4",
    keys: ["web.tools.ext", "web.platform.chrome", "web.platform.edge", "web.platform.firefox", "web.downloadPage.browser", "web.faq.a8"],
    check: "the Chrome, Edge and Firefox extensions are published or installable as the download page says, and each FAQ line naming a browser is still true",
    verified: false,
  },
  {
    id: "G5",
    keys: [
      "web.features.phone.",
      "web.tools.mobile",
      "web.tools.phone.",
      "web.cmp.mobile.mosaic",
      "web.cmp.passkeys.mosaic",
      "web.hero.note",
      "web.pricing.free.item1",
      "web.platform.android",
      "web.faq.a8",
      "web.downloadPage.meta.description",
    ],
    check: "the Android app is in the Play Store (`site.stores.android`), fills logins and passkeys inside apps and unlocks with the fingerprint; no copy promises an iPhone or Linux app (not launch targets)",
    verified: false,
  },
  {
    id: "G6",
    keys: ["web.features.androidLink."],
    check: "Android one-tap linking (merged in PR #87) ships with a biometric confirmation",
    verified: false,
  },
  {
    id: "G7",
    keys: [
      "web.features.folders.",
      "web.only.cloud.text",
      "web.pricing.free.item5",
      "web.teamsPage.storage.company",
      "web.how.step2.text",
      "web.teams.company.text",
    ],
    check:
      "each named provider works on each platform the sentence names (desktop: any synced folder including iCloud Drive; Android folder sync, merged in PR #95: Dropbox, OneDrive, Nextcloud)",
    verified: false,
  },
  {
    id: "G8",
    keys: ["web.features.terminal.", "web.tools.cli", "web.platform.cli", "web.downloadPage.cli", "web.pricing.free.item7"],
    check:
      "the CLI demo literals match the released CLI (docs/CLI.md): `mosaic totp REF`, `mosaic run --env NAME=REF -- COMMAND`, `mosaic ssh list`",
    verified: false,
  },
  {
    id: "G9",
    keys: [
      "web.features.passkeys.",
      "web.features.ssh.",
      "web.cmp.ssh.mosaic",
      "web.hero.sub1",
      "web.features.intro",
      "web.features.meta.description",
      "web.pricing.free.item3",
      "web.shot.passkeys",
      "web.check.tip.browser",
    ],
    check: "passkey sync across devices and the SSH agent's per-use prompt are in the release",
    verified: false,
  },
  {
    id: "G10",
    keys: ["web.challenge.", "web.security.challenge.", "web.report.", "web.footer.challenge"],
    check:
      "the repository is public; SECURITY.md, docs/BUG_BOUNTY.md, docs/CHALLENGE.md and docs/HALL_OF_FAME.md are on main; private vulnerability reporting is on; response times match ADR-0087; `site.challengeOpen` matches the test server; the safe-harbor wording matches BUG_BOUNTY.md",
    verified: false,
  },
];

/** Words that make a web string a claim about a feature in flight; a string with one must be gated. */
export const CLAIM_TERMS =
  /agent|MCP|API-Key|API key|LastPass|Dashlane|Keeper|NordPass|Proton Pass|RoboForm|Enpass|Safari|Windows program|Windows-Programm|Android|iPhone|Face ID|Dropbox|OneDrive|Nextcloud|iCloud|passkey|Passkey|SSH|command line|Kommandozeile|challenge|Challenge|test server|Testserver/;

export const unverifiedClaims = (): Claim[] => CLAIMS.filter((claim) => !claim.verified);

/** True when `key` is covered by any gate. */
export const isGated = (key: string): boolean => CLAIMS.some((claim) => claim.keys.some((prefix) => key === prefix || key.startsWith(prefix)));

/** The message of the production-build failure. */
export const claimsFailure = (): string =>
  `prerender: claims about unreleased features are not verified (${unverifiedClaims()
    .map((claim) => claim.id)
    .join(", ")}); flip each gate in src/pages/claims.ts after checking its release.`;

/**
 * Build flags of the website (website v2 §2.2, §5.8, §5.11). Each flag switches one state;
 * nothing else changes when it flips.
 */
import type { Platform } from "../downloads.ts";

/** The public repository, where bug reports and private security reports go. */
export const REPO = "https://github.com/StephanBaum/Mosaik";

/** External links of the report and challenge pages; navigation only, the site sends nothing. */
export const LINKS = {
  issues: `${REPO}/issues/new/choose`,
  advisory: `${REPO}/security/advisories/new`,
  policy: `${REPO}/blob/main/SECURITY.md`,
  rules: `${REPO}/blob/main/docs/BUG_BOUNTY.md`,
  targets: `${REPO}/blob/main/docs/CHALLENGE.md`,
  hallOfFame: `${REPO}/blob/main/docs/HALL_OF_FAME.md`,
} as const;

export interface Site {
  captureEnabled: boolean;
  /** The challenge test server is published (gate G10: `{CONTEST_HOST}` is set in docs/CHALLENGE.md). */
  challengeOpen: boolean;
  legalReady: boolean;
  files: Partial<Record<Platform, string>>;
  stores: Record<"android", boolean>;
  contactUser: string;
}

export const site: Site = {
  /**
   * The switch-kit form posts to `/api/v1/subscribe` (owner and M9 item). Until the endpoint is
   * live the pages show the honest coming-soon card instead of a form that swallows addresses.
   */
  captureEnabled: false,
  challengeOpen: false,
  /** The owner's Impressum and privacy texts exist. The production build refuses to ship without them. */
  legalReady: false,
  /**
   * Release files per platform. A tile without a file links straight to the next-steps page;
   * with one, the link starts the file and the downloads island then opens next steps.
   */
  files: {},
  /** Store listings that exist. The Android tile says "Coming soon" until then. */
  stores: { android: false },
  /** The contact address for team rollouts, on the placeholder domain until the domain is set. */
  contactUser: "hello",
};

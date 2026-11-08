/**
 * Build flags of the website (website v2 §2.2, §5.8, §5.11). Each flag switches one state;
 * nothing else changes when it flips.
 */
import type { Platform } from "../downloads.ts";

export interface Site {
  captureEnabled: boolean;
  legalReady: boolean;
  files: Partial<Record<Platform, string>>;
  stores: Record<"iphone" | "android", boolean>;
  contactUser: string;
}

export const site: Site = {
  /**
   * The switch-kit form posts to `/api/v1/subscribe` (owner and M9 item). Until the endpoint is
   * live the pages show the honest coming-soon card instead of a form that swallows addresses.
   */
  captureEnabled: false,
  /** The owner's Impressum and privacy texts exist. The production build refuses to ship without them. */
  legalReady: false,
  /**
   * Release files per platform. A tile without a file links straight to the next-steps page;
   * with one, the link starts the file and the downloads island then opens next steps.
   */
  files: {},
  /** Store listings that exist. Phone tiles say "Coming soon" until then. */
  stores: { iphone: false, android: false },
  /** The contact address for team rollouts, on the placeholder domain until the domain is set. */
  contactUser: "hello",
};

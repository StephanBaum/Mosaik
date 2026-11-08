/**
 * English strings of the website (apps/web), one module per page or area (website v2 §1.4),
 * merged into one `web` group. ICU MessageFormat, formatted at build time only.
 */
import { selfCheck } from "./check.ts";
import { compare } from "./compare.ts";
import { features } from "./features.ts";
import { home } from "./home.ts";
import { landing } from "./landing.ts";
import { plans } from "./plans.ts";
import { security } from "./security.ts";
import { site } from "./site.ts";
import { switchGuide } from "./switch.ts";

export const web = {
  ...landing,
  ...site,
  ...home,
  ...features,
  ...security,
  ...compare,
  ...plans,
  ...switchGuide,
  ...selfCheck,
} as const satisfies Record<string, string>;

/**
 * Page bodies per route. Each view renders its page's `<main>` content; `jsonLd` adds a
 * structured-data block to the head (FAQPage on the FAQ page).
 */
import type { JSX } from "react";
import type { Ctx } from "../i18n.ts";
import type { RouteId } from "../routes.ts";
import { Compare, VendorPage } from "./compare.tsx";
import { Download, Next, Verify } from "./download.tsx";
import { Faq, faqJsonLd } from "./faq.tsx";
import { Features } from "./features.tsx";
import { SelfCheck } from "./check.tsx";
import { Home } from "./home.tsx";
import { Pricing, Teams } from "./plans.tsx";
import { Security } from "./security.tsx";
import { Legal, Thanks } from "./small.tsx";
import { Switch } from "./switch.tsx";

export interface View {
  Page: (props: { ctx: Ctx }) => JSX.Element;
  jsonLd?: (ctx: Ctx) => unknown;
}

export const VIEWS: Readonly<Record<RouteId, View>> = {
  home: { Page: Home },
  features: { Page: Features },
  security: { Page: Security },
  compare: { Page: Compare },
  vs1password: { Page: ({ ctx }) => VendorPage({ ctx, id: "vs1password" }) },
  vsBitwarden: { Page: ({ ctx }) => VendorPage({ ctx, id: "vsBitwarden" }) },
  pricing: { Page: Pricing },
  teams: { Page: Teams },
  download: { Page: Download },
  next: { Page: Next },
  verify: { Page: Verify },
  switch: { Page: Switch },
  check: { Page: SelfCheck },
  faq: { Page: Faq, jsonLd: faqJsonLd },
  thanks: { Page: ({ ctx }) => Thanks({ ctx }) },
  confirmed: { Page: ({ ctx }) => Thanks({ ctx, confirmed: true }) },
  imprint: { Page: ({ ctx }) => Legal({ ctx, kind: "imprint" }) },
  privacy: { Page: ({ ctx }) => Legal({ ctx, kind: "privacy" }) },
};

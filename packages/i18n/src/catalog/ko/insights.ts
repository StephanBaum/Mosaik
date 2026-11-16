// UNTRANSLATED: copy of English, replace every value
/**
 * Korean UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Security",
  "insights.intro":
    "Checks run on this device. Last checked {checkedAt, date, medium} at {checkedAt, time, short} across {count, plural, other {# items}}.",
  "insights.checkAgain": "Check again",
  "insights.summary": "Summary",
  "insights.fixFirst": "Fix these first",
  "insights.showAll": "Show all {count, plural, other {# findings}}",
  "insights.empty": "No issues found. Mosaic checks again whenever you open this page.",
  "insights.kind.weak": "Weak",
  "insights.kind.weak.description": "Easy to guess",
  "insights.kind.reused": "Reused",
  "insights.kind.reused.description": "{groups, plural, other {# groups}}",
  "insights.kind.old": "Old (over 1 year)",
  "insights.kind.old.description": "Not changed in over a year",
  "insights.kind.missingTotp": "Missing 2FA",
  "insights.kind.missingTotp.description": "The site supports it, but you haven't set it up",
  "insights.kind.insecureUrl": "Unsecured HTTP",
  "insights.kind.insecureUrl.description": "Sends your password unencrypted",
  "insights.kind.breached": "In a breach",
  "insights.kind.breached.description": "Found by the opt-in breach check",
  "insights.finding.breached":
    "This password appears in a known data breach. Change it now, everywhere you used it.",
  "insights.finding.weak": "Easy to guess. Change it to a generated password.",
  "insights.finding.reused":
    "Also used by {count, plural, other {# other items}}. Give each site its own password.",
  "insights.finding.old": "Not changed in over a year. Change it if you still use this account.",
  "insights.finding.missingTotp":
    "{site} offers two-factor codes. Turn them on there and save the code in Mosaic.",
  "insights.finding.insecureUrl":
    "Uses plain HTTP: {url}. Switch to https:// if the site supports it.",
  "insights.severity.high": "High",
  "insights.severity.medium": "Medium",
  "insights.severity.low": "Low",
  "insights.action.changePassword": "Change password",
  "insights.action.setUpTotp": "Set up 2FA",
  "insights.action.reviewUrl": "Review",
  "insights.attribution.twoFactor": "2FA data: 2FA Directory (2fa.directory), MIT license.",
  "insights.counts": "{count, plural, =0 {No issues} other {# issues}}",
  "insights.documentation": "How to turn it on: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;

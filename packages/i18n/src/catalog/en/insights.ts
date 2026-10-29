/** English UI strings: insights. ICU MessageFormat. */
export const insights = {
  "insights.title": "Security",
  "insights.intro":
    "Checks run on this device. Last checked {checkedAt, date, medium} at {checkedAt, time, short} across {count, plural, one {# item} other {# items}}.",
  "insights.checkAgain": "Check again",
  "insights.summary": "Summary",
  "insights.fixFirst": "Fix these first",
  "insights.showAll": "Show all {count, plural, one {# finding} other {# findings}}",
  "insights.empty": "No issues found. Mosaic checks again whenever you open this page.",
  "insights.kind.weak": "Weak",
  "insights.kind.weak.description": "Easy to guess",
  "insights.kind.reused": "Reused",
  "insights.kind.reused.description": "{groups, plural, one {# group} other {# groups}}",
  "insights.kind.old": "Old (over 1 year)",
  "insights.kind.old.description": "Not changed in over a year",
  "insights.kind.missingTotp": "Missing 2FA",
  "insights.kind.missingTotp.description": "The site supports it, but you haven't set it up",
  "insights.kind.insecureUrl": "Unsecured HTTP",
  "insights.kind.insecureUrl.description": "Sends your password unencrypted",
  "insights.finding.weak": "Easy to guess. Change it to a generated password.",
  "insights.finding.reused":
    "Also used by {count, plural, one {# other item} other {# other items}}. Give each site its own password.",
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
  "insights.counts": "{count, plural, =0 {No issues} one {# issue} other {# issues}}",
  "insights.documentation": "How to turn it on: {url}",
  "insights.openItem": "Open {title}",
} as const satisfies Record<string, string>;

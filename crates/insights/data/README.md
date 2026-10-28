# Vendored data for `mosaic-insights`

The file is embedded into the crate with `include_str!`. It is kept
byte-for-byte as downloaded; never edit it by hand. To update it, download it
again, check the license below, update the SHA-256 and the entry counts here
and in `src/directory.rs` (test `data_loads_and_matches_the_recorded_sha256`),
check `EXACT_ONLY` (`src/directory.rs`), and run
`cargo test -p mosaic-insights`.

Checking `EXACT_ONLY`: every directory domain (main and additional) that is
on the Public Suffix List, or under which the list has a rule (ICANN and
private sections, wildcards included), must be in `EXACT_ONLY`. One way:
for each domain `d`, ask a current PSL implementation (for example `tldts`,
`getPublicSuffix("probe." + d, { allowPrivateDomains: true })`) and list `d`
when the answer is `d` or ends in `.d`. Also review new domains of hosting
platforms that give users their own subdomains but are not on the list
(`substack.com`, `tumblr.com`). Record the check date and list version in
the `EXACT_ONLY` documentation. The tests check that every entry is still in
the data (`every_exact_only_domain_is_in_the_data`) and catch two-label
domains such as `co.xx` (`every_public_suffix_like_domain_is_exact_only`).

## `2fa-directory-totp.json`

- What: the 2FA Directory's list of services that support TOTP one-time
  codes, API version 3 (an array of `[name, { domain, additional-domains,
  tfa, documentation, ... }]`). 1294 entries, 1653 distinct domains.
  Mosaic uses the domains, the service name and the documentation link.
- Source: <https://api.2fa.directory/v3/totp.json>, downloaded 2026-10-27.
  The API is generated from the repository
  <https://github.com/2factorauth/twofactorauth> (`entries/`).
- SHA-256: `a4652045def5f9e6c0e81034eb0f3f694909d84118edc7566b3225d293bd2746`
- License: MIT. The repository's `LICENSE.md` (two MIT notices: Copyright
  2021- 2factorauth and Contributors; Copyright 2014-2020 Josh Davis) is
  copied unchanged to [`2fa-directory-LICENSE.md`](2fa-directory-LICENSE.md)
  and must stay next to the data file (the MIT license requires the notice
  to ship with copies). The API documentation
  (<https://2fa.directory/api/>, checked 2026-10-27) states that the data is
  licensed under the MIT license and asks for attribution where the data is
  visible to users. The dashboard therefore shows the i18n string
  `insights.attribution.twoFactor` ("2FA data: 2FA Directory
  (2fa.directory), MIT license.").
- Decision: redistribution is permitted (MIT), so Mosaic ships the data
  (brief §5: "use 2fa.directory data where license permits"). Nothing is
  fetched at run time; updates come with Mosaic releases.
- Caveat from upstream: the data is crowdsourced and may be outdated. A
  missing-2FA finding is a hint, rated `Medium`, never a blocker.

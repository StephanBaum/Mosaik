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

## `change-password-URLs.json`

- What: Apple's Password Manager Resources quirks file that maps a domain to
  the URL of the site's change-password page (M7b, ADR-0070). An object of
  `"domain": "https://..."` pairs, 805 entries. Mosaic reads it in
  `src/change_url.rs`: the key is looked up as the host of an item URL and as
  each of its parent domains (down to two labels); an entry whose key is not a
  DNS name or whose value is not a clean `https` URL is skipped by the loader.
- Source: <https://github.com/apple/password-manager-resources>, file
  `quirks/change-password-URLs.json`, downloaded 2026-11-10 from
  `raw.githubusercontent.com/apple/password-manager-resources/main/`. The
  file's last upstream change is commit
  `e6a7cf40fe91aa8897b1be07b0ab416332c31488` (2026-11-03); `main` was
  `316412ba01dba1d84dc52d51267f503964638e20` that day (it does not touch this
  file). Kept byte-for-byte as downloaded.
- SHA-256: `62838bb54084663e5dad60396c0008b78d13584955e1e9df0c5b93a0237cf2b0`
  (test `data_matches_recorded_sha256`).
- License: MIT, Copyright 2020 - 2026 Apple Inc., the same repository and
  license as `crates/generator/data/password-rules.json`. The license text is
  copied unchanged to
  [`password-manager-resources-LICENSE.md`](password-manager-resources-LICENSE.md)
  and must stay next to the data file (the MIT license requires the notice to
  ship with copies). The generator's copy is
  `crates/generator/data/password-manager-resources-LICENSE.md`.
- Decision: redistribution is permitted (MIT), so Mosaic ships the data (spec
  M7b section 2.4, OD-M7b-7). Nothing is fetched at run time; the URLs are
  never requested by Mosaic either: the user opens one in the browser, or the
  extension does, on a user action. The data is crowdsourced and may be
  outdated; a wrong URL costs the user one click.
- To update: download the file again, check the license, update the SHA-256
  and the entry count here and in the test, and run
  `cargo test -p mosaic-insights change_url`.

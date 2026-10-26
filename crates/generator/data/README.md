# Vendored data for `mosaic-generator`

Both files are embedded into the crate with `include_str!`. They are kept
byte-for-byte as downloaded; never edit them by hand. To update one,
download it again, check the SHA-256 and the license below, update this
file, and run `cargo test -p mosaic-generator` (the tests check the word
count, uniqueness and that every site rule parses and generates).

## `eff_large_wordlist.txt`

- What: the EFF large wordlist for dice-generated passphrases, 7776 words
  (five dice), one `ddddd<TAB>word` entry per line.
- Source: <https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt>,
  downloaded 2026-10-26.
- SHA-256: `addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e`
- License: [Creative Commons Attribution 3.0 United States (CC BY 3.0 US)](https://creativecommons.org/licenses/by/3.0/us/).
- Attribution: "EFF large wordlist" by the **Electronic Frontier
  Foundation**, <https://www.eff.org/dice>. Used unmodified.

## `password-rules.json`

- What: Apple's Password Manager Resources quirks file, mapping domains to
  `passwordrules` strings (436 entries at this revision).
- Source: <https://github.com/apple/password-manager-resources>, file
  `quirks/password-rules.json` at commit
  `0879dd8857d87150a6221d075933a51bef988766` (2026-10-13), identical to
  `main` on 2026-10-26.
- SHA-256: `d3dc3b25cd879b13af37d3c6bb8e5cc028c2c4a5076fb093a7b3bf0137263531`
- License: MIT, Copyright 2020 - 2026 Apple Inc. The full license text is in
  [`password-manager-resources-LICENSE.md`](password-manager-resources-LICENSE.md)
  and must stay next to the data file (the MIT license requires the notice to
  ship with copies of the data).
- Matching semantics (from the upstream README): an entry for `example.com`
  applies to `example.com` and every subdomain, unless the entry sets
  `"exact-domain-match-only": true`.

The parser in `src/rules.rs` follows Apple's reference parser
`tools/PasswordRulesParser/PasswordRulesParser.js` at the same commit (MIT);
its conformance cases are ported into the unit tests.

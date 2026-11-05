# Test-only certificate

`cert.pem` and `key.pem` are a self-signed P-256 certificate for `*.test` (plus the few host
names the hostile pages use). They exist so the hostile-site server can speak HTTPS in the
Playwright tests; the browser trusts them only because the tests start it with
`--ignore-certificate-errors`.

**Do not use this key for anything else.** The private key is committed on purpose, so anyone
can read it. It protects nothing and no real host name: `.test` is reserved (RFC 6761) and the
tests map it to 127.0.0.1 inside the test browser only.

`generate.mjs` rebuilds both files with Node's `crypto` alone (Node can sign but has no
certificate builder, so it writes the X.509 structure in DER itself). The certificate is valid
until 2126, so it does not need renewing.

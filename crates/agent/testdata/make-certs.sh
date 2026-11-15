#!/bin/sh
# Regenerates the fixture CAs and the leaf of the agent test TLS server (crates/agent/src/testing).
# Run once with Git Bash openssl; the DER output is committed. Test-only: the CA keys are thrown
# away after signing, so nothing can sign for them again, and no build trusts these CAs.
# EC P-256, valid 100 years, leaf names api.mosaic.test and evil.mosaic.test.
set -eu
export MSYS_NO_PATHCONV=1
here=$(cd "$(dirname "$0")" && (pwd -W 2>/dev/null || pwd))
tmp=$(mktemp -d)
tmp=$(cygpath -m "$tmp" 2>/dev/null || echo "$tmp")
trap 'rm -rf "$tmp"' EXIT

# One CA configuration for both CAs (the common name is passed with -subj).
cat > "$tmp/ca.cnf" <<'EOF'
[req]
distinguished_name = dn
prompt = no
x509_extensions = v3_ca
[dn]
CN = placeholder
[v3_ca]
basicConstraints = critical,CA:TRUE,pathlen:0
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
EOF

openssl ecparam -name prime256v1 -genkey -noout -out "$tmp/ca.key"
openssl req -x509 -new -key "$tmp/ca.key" -sha256 -days 36500 -config "$tmp/ca.cnf" \
  -subj "/CN=Mosaic agent test CA" -outform DER -out "$here/ca.der"
openssl x509 -inform DER -in "$here/ca.der" -out "$tmp/ca.pem"

openssl ecparam -name prime256v1 -genkey -noout -out "$tmp/leaf.key"
openssl pkcs8 -topk8 -nocrypt -in "$tmp/leaf.key" -outform DER -out "$here/leaf.key.der"
openssl req -new -key "$tmp/leaf.key" -subj "/CN=api.mosaic.test" -out "$tmp/leaf.csr"
cat > "$tmp/leaf.ext" <<'EOF'
subjectAltName = DNS:api.mosaic.test,DNS:evil.mosaic.test
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = serverAuth
EOF
openssl x509 -req -in "$tmp/leaf.csr" -CA "$tmp/ca.pem" -CAkey "$tmp/ca.key" -CAcreateserial \
  -days 36500 -sha256 -extfile "$tmp/leaf.ext" -outform DER -out "$here/leaf.der"

# An unrelated CA: trusting only this one makes the leaf untrusted (tests of the TLS refusal).
openssl ecparam -name prime256v1 -genkey -noout -out "$tmp/other.key"
openssl req -x509 -new -key "$tmp/other.key" -sha256 -days 36500 -config "$tmp/ca.cnf" \
  -subj "/CN=Mosaic agent test CA (other)" -outform DER -out "$here/other-ca.der"

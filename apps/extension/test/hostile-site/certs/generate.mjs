/**
 * Regenerates cert.pem and key.pem: a self-signed P-256 certificate for `*.test`, built with
 * Node's `crypto` only (Node can sign but has no certificate builder, so the X.509 structure is
 * written out here in DER). TEST ONLY: the private key is committed on purpose, see README.md.
 *
 *   node apps/extension/test/hostile-site/certs/generate.mjs
 */
import { Buffer } from "node:buffer";
import { X509Certificate, generateKeyPairSync, randomBytes, sign } from "node:crypto";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/** @param {number} tag @param {...Buffer} parts */
function tlv(tag, ...parts) {
  const body = Buffer.concat(parts);
  let length;
  if (body.length < 128) {
    length = Buffer.from([body.length]);
  } else {
    const bytes = [];
    for (let n = body.length; n > 0; n >>= 8) {
      bytes.unshift(n & 255);
    }
    length = Buffer.from([128 | bytes.length, ...bytes]);
  }
  return Buffer.concat([Buffer.from([tag]), length, body]);
}

const seq = (...parts) => tlv(0x30, ...parts);
const set = (...parts) => tlv(0x31, ...parts);
const utf8 = (text) => tlv(0x0c, Buffer.from(text, "utf8"));
const octets = (buffer) => tlv(0x04, buffer);
const bits = (buffer) => tlv(0x03, Buffer.concat([Buffer.from([0]), buffer]));
const explicit = (n, ...parts) => tlv(0xa0 | n, ...parts);
const bool = (value) => tlv(0x01, Buffer.from([value ? 255 : 0]));

/** @param {string} dotted */
function oid(dotted) {
  const [first, second, ...rest] = dotted.split(".").map(Number);
  const bytes = [first * 40 + second];
  for (const arc of rest) {
    const chunk = [arc & 127];
    for (let n = arc >> 7; n > 0; n >>= 7) {
      chunk.unshift((n & 127) | 128);
    }
    bytes.push(...chunk);
  }
  return tlv(0x06, Buffer.from(bytes));
}

const integer = (buffer) => tlv(0x02, buffer[0] & 128 ? Buffer.concat([Buffer.from([0]), buffer]) : buffer);
const time = (tag, text) => tlv(tag, Buffer.from(text, "ascii"));
const extension = (id, critical, value) => seq(oid(id), ...(critical ? [bool(true)] : []), octets(value));

const { privateKey, publicKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
const name = seq(set(seq(oid("2.5.4.3"), utf8("Mosaic hostile-site test (*.test) - NOT A REAL CERTIFICATE"))));
const ecdsaSha256 = seq(oid("1.2.840.10045.4.3.2"));
// "*.test" for browsers; OpenSSL-style matchers refuse a wildcard directly under a TLD, so the
// hosts the hostile pages use are listed too.
const HOSTS = ["*.test", "bank.test", "evil.test", "bank-login.test", "xn--bnk-6cd.test"];
const subjectAltNames = seq(...HOSTS.map((host) => tlv(0x82, Buffer.from(host))));

const tbs = seq(
  explicit(0, integer(Buffer.from([2]))),
  integer(Buffer.concat([Buffer.from([1]), randomBytes(15)])),
  ecdsaSha256,
  name,
  seq(time(0x17, "260101000000Z"), time(0x18, "21260101000000Z")),
  name,
  publicKey.export({ type: "spki", format: "der" }),
  explicit(
    3,
    seq(
      extension("2.5.29.19", true, seq()),
      extension("2.5.29.15", true, tlv(0x03, Buffer.from([7, 0x80]))),
      extension("2.5.29.37", false, seq(oid("1.3.6.1.5.5.7.3.1"))),
      extension("2.5.29.17", false, subjectAltNames),
    ),
  ),
);

const signature = sign("sha256", tbs, { key: privateKey, dsaEncoding: "der" });
const der = seq(tbs, ecdsaSha256, bits(signature));
const pem = (label, bytes) =>
  `-----BEGIN ${label}-----\n${bytes.toString("base64").match(/.{1,64}/g)?.join("\n") ?? ""}\n-----END ${label}-----\n`;

const certificate = pem("CERTIFICATE", der);
const check = new X509Certificate(certificate);
if (!check.verify(publicKey) || check.checkHost("bank.test") === undefined) {
  throw new Error("generated certificate does not verify");
}
writeFileSync(join(here, "cert.pem"), certificate);
writeFileSync(join(here, "key.pem"), pem("PRIVATE KEY", privateKey.export({ type: "pkcs8", format: "der" })));
process.stdout.write(`wrote cert.pem (${check.subject.trim()}) and key.pem\n`);

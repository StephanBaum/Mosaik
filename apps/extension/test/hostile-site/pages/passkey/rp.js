// The relying party of the passkey suite (see rp.html): `window.rp` runs WebAuthn calls on the page's own origin.
const b64 = (buffer) => {
  let text = "";
  for (const byte of new Uint8Array(buffer)) text += String.fromCharCode(byte);
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};
const unb64 = (value) => Uint8Array.from(atob(value.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
const asError = (error) => ({ error: { name: error?.name ?? "?", message: String(error?.message ?? error) } });

const rp = {
  controller: null,
  pending: null,
  async challenge() {
    return (await (await fetch("/passkey/challenge", { cache: "no-store" })).json()).challenge;
  },
  async verify(body) {
    const response = await fetch("/passkey/verify", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    return { status: response.status, ...(await response.json()) };
  },
  /** `navigator.credentials.create` with a fresh challenge; verified by the server unless `noVerify`. */
  async create(o = {}) {
    const challenge = o.challenge ?? (await rp.challenge());
    const options = {
      publicKey: {
        rp: { name: "Test bank", ...(o.rpId ? { id: o.rpId } : {}) },
        user: { id: unb64("AQIDBAUGBwgJCgsMDQ4PEA"), name: "ada@bank.test", displayName: "Ada" },
        challenge: unb64(challenge),
        pubKeyCredParams: [{ type: "public-key", alg: -7 }, { type: "public-key", alg: -257 }],
        excludeCredentials: (o.exclude ?? []).map((id) => ({ type: "public-key", id: unb64(id) })),
        authenticatorSelection: { residentKey: "required", userVerification: "preferred" },
        extensions: { credProps: true },
        attestation: "none",
      },
      ...(o.signal ? { signal: o.signal } : {}),
    };
    try {
      const credential = await navigator.credentials.create(options);
      const result = {
        id: credential.id,
        type: credential.type,
        attachment: credential.authenticatorAttachment,
        isPublicKeyCredential: credential instanceof PublicKeyCredential,
        extensions: credential.getClientExtensionResults(),
        clientDataJSON: b64(credential.response.clientDataJSON),
        attestationObject: b64(credential.response.attestationObject),
        challenge,
      };
      if (!o.noVerify) {
        result.verify = await rp.verify({ kind: "registration", challenge, clientDataJSON: result.clientDataJSON, attestationObject: result.attestationObject });
      }
      return result;
    } catch (error) {
      return asError(error);
    }
  },
  /** `navigator.credentials.get`; `allow` is a list of credential ids (base64url). */
  async get(o = {}) {
    const challenge = o.challenge ?? (await rp.challenge());
    const options = {
      publicKey: {
        ...(o.rpId ? { rpId: o.rpId } : {}),
        challenge: unb64(challenge),
        allowCredentials: (o.allow ?? []).map((id) => ({ type: "public-key", id: unb64(id) })),
        userVerification: "preferred",
      },
      ...(o.signal ? { signal: o.signal } : {}),
    };
    try {
      const credential = await navigator.credentials.get(options);
      const result = {
        id: credential.id,
        isPublicKeyCredential: credential instanceof PublicKeyCredential,
        clientDataJSON: b64(credential.response.clientDataJSON),
        authenticatorData: b64(credential.response.authenticatorData),
        signature: b64(credential.response.signature),
        userHandle: credential.response.userHandle ? b64(credential.response.userHandle) : null,
        challenge,
      };
      if (!o.noVerify) {
        result.verify = await rp.verify({ kind: "assertion", credentialId: credential.id, challenge, ...result });
      }
      return result;
    } catch (error) {
      return asError(error);
    }
  },
  /** Starts a call and keeps its promise in `rp.pending`; `rp.abort()` aborts it. */
  start(op, o = {}) {
    rp.controller = new AbortController();
    rp.pending = rp[op]({ ...o, signal: rp.controller.signal });
  },
  abort() {
    rp.controller.abort();
  },
  result() {
    return rp.pending;
  },
};
window.rp = rp;

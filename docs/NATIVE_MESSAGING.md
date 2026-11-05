# Native messaging protocol

The normative description of the channel between the Mosaic browser extension, the
native messaging host and the desktop app (ADR-0035, ADR-0036). Code:
`crates/native-messaging` (`mosaic-native-messaging`, host binary
`mosaic-native-host`) and `mosaic_core::browser_channel`. The cryptography of the
host-to-desktop link is CRYPTO_SPEC §15; the threats are THREAT_MODEL §4.4. If this document
and the code disagree, the code is wrong until the document is changed first.

Mosaic is pre-audit. Nothing here has been externally reviewed.

```
[Extension background] --connectNative, stdio JSON--> [mosaic-native-host]
                                                          |  local endpoint:
                                                          |  pre-auth JSON, then AEAD frames
                                                          v
                                         [mosaic-desktop: BrowserService] -- vault session
```

The host is a small relay. It never launches the desktop app, holds no vault data, and
never parses a request body. The desktop decides every request.

Key words MUST, MUST NOT and MAY are used as in RFC 2119.

## 1. Endpoint

```rust
pub const APP_IDENTIFIER: &str = "io.github.stephanbaum.mosaic"; // = tauri.conf.json identifier
pub enum Endpoint { UnixSocket(PathBuf), NamedPipe(String) }
```

| OS | Endpoint |
|---|---|
| Linux | `$XDG_RUNTIME_DIR/mosaic/browser.sock` if the variable is set and absolute; else `${XDG_DATA_HOME:-$HOME/.local/share}/io.github.stephanbaum.mosaic/browser/browser.sock` |
| macOS | `$HOME/Library/Application Support/io.github.stephanbaum.mosaic/browser/browser.sock` |
| Windows | a named pipe `\\.\pipe\mosaic-browser-<32 lowercase hex>` with a fresh `random(16)` name per desktop start, published in `%LOCALAPPDATA%\io.github.stephanbaum.mosaic\browser\endpoint.json` |

**Unix rules.** The path is absolute and at most 103 bytes, else `EndpointInvalid`. The
directory is `0700` and owned by the user, the socket is `0600`. The server replaces a
stale socket only if nothing answers on it; any other file at the path stays and the bind
fails (`EndpointInUse`). These are the rules of `crates/ssh-agent/src/transport.rs`,
copied (M9 extracts one shared crate).

- The server disconnects a peer whose UID is not the socket owner's.
- The client requires: the parent is a real directory (not a symbolic link) with
  `mode & 0o077 == 0`; the path is a socket owned by the directory's owner; after
  `connect`, the peer's UID equals the directory's owner.

**Windows rules.** The desktop creates the pipe as first instance
(`first_pipe_instance(true)`) and rejects remote clients (`reject_remote_clients(true)`).
After the bind it writes the discovery file atomically (a temporary file in the same
directory, then a rename) and deletes it at exit:

```json
{"version":1,"pipe":"\\\\.\\pipe\\mosaic-browser-0123456789abcdef0123456789abcdef"}
```

The host reads at most 4096 bytes, requires exactly the keys `version` (1) and `pipe`, and
validates the name against `^\\\\\.\\pipe\\mosaic-browser-[0-9a-f]{32}$`. A missing file is
`desktopNotRunning`. Other OS accounts cannot read the file, so they cannot squat the
name in advance (named pipe names are machine-global). Windows gives no peer process
identity without unsafe code (THREAT_MODEL R42).

**Debug override.** Debug builds read `MOSAIC_BROWSER_ENDPOINT` (a socket path or a pipe
name of the form above) instead of the table, for integration tests. Release builds ignore
the variable.

The host resolves the endpoint again for every connect, so a restarted desktop (a new pipe
name on Windows) is found.

## 2. Host binary

`mosaic-native-host` is started by the browser, or by a developer:

| Command | Effect |
|---|---|
| `mosaic-native-host install [--browsers chrome,edge,chromium,firefox]` | writes the host manifests (§7); all four by default |
| `mosaic-native-host uninstall` | removes everything `install` wrote |
| anything else | the relay below |

**Caller check.** The browser tells the host who started it. The host takes its full
argument vector and accepts exactly:

- Chromium family: `args[1]` is `chrome-extension://<id>/` and equals one of
  `allowed_chromium_origins()`. (Windows adds `--parent-window=<n>` after it.)
- Firefox: `args[2]` equals the add-on ID (`args[1]` is the manifest path).

Anything else exits with code 2 without connecting.

```rust
pub const HOST_NAME: &str = "io.github.stephanbaum.mosaic";
pub const FIREFOX_EXTENSION_ID: &str = "mosaic@stephanbaum.github.io"; // the owner confirms with AMO
pub const DEV_CHROMIUM_EXTENSION_ID: &str = "lccdgfpfnmegbdgooanfejdbahhokikm";
pub const STORE_CHROMIUM_EXTENSION_IDS: &[&str] = &[]; // the owner: Chrome Web Store, Edge Add-ons
pub enum Caller { Chromium { extension_id: String }, Firefox }
pub fn allowed_chromium_origins() -> Vec<String>; // store IDs; the dev ID only in debug builds
```

The dev ID is derived from the public key in `apps/extension/keys/dev-public-key.b64`
(a base64 SPKI of an RSA-2048 key; the private key was discarded): SHA-256 of the DER,
the first 16 bytes, each nibble `0..15` written as `a..p`. Dev and e2e extension builds
carry that key as the manifest `key`. A Rust test recomputes the ID from the file.

**Relay.** `relay::run_host(caller, stdin, stdout, endpoint)` (`endpoint` is an
`EndpointSource`: a fixed endpoint, or the default one resolved at every connect) runs on a
current-thread
Tokio runtime, without the `macros` feature. One task reads stdin, one writes stdout, one
reads the desktop connection, and one main loop owns the state and the session's sending
half; they talk over channels.

- The host exits with code 0 when stdin reaches EOF, with code 3 on a malformed browser
  frame (§3) or an envelope that is not valid JSON of the request shape, and with code 1
  on another failure.
- It keeps `K_pair` (as a `PairingKey`) and the pairing ID in memory for its lifetime.
- It forwards a request only if `op` is in `FORWARDED_OPS` (§5) and the sealed frame fits
  the cap (`frameTooLarge` otherwise). It re-serializes the envelope and relays the body
  verbatim. Every other op, including every name that is not in §5, is answered with
  `invalidRequest` by the host and never reaches the desktop.
- It relays the desktop's responses verbatim. It validates every desktop plaintext first
  (a response of the right shape for a request in flight, or a `state` or `unpaired`
  event) and ignores responses for ids that are not in flight.
- If the desktop drops (EOF, a write failure, a frame that does not authenticate): it
  pushes the event `disconnected`, answers every request in flight with
  `desktopNotRunning` (`desktopNotAuthentic` if a frame failed to authenticate), and
  connects again
  with a fresh handshake on the next request, pushing a `state` event when that works.
- A request id that is already in flight is answered with `invalidRequest`.
- `hello` and `pair` are answered by the host (§5.2). With a stored pairing, `status` and
  `focus` go through the session; with none (or when the desktop refuses the pairing with
  `notPaired`) they use the pre-auth messages `status` and `focus` (§4.2). Any other op
  without a pairing is answered `notPaired`.
- `unpaired` from the desktop is relayed and then the host forgets the pairing and closes
  the session.

## 3. Framing

| Link | Format | Cap |
|---|---|---|
| browser and host (stdio) | `u32` in native byte order, then UTF-8 JSON | `MAX_FRAME_BYTES = 262144` both ways |
| host and desktop | `u32` big-endian, then the payload: pre-auth JSON before the handshake finishes, AEAD frames after it (§4.1) | the same |

A length of 0, a length above the cap, or a stream that ends inside a frame is an error. A
stream that ends between frames is a clean close. The length is checked before any payload
is read. One message is written with one `write` of prefix and payload together. Read
buffers are zeroizing.

(Chrome limits a host-to-browser message to 1 MiB and a browser-to-host message to 64 MiB;
the 256 KiB cap is stricter on purpose.)

## 4. Handshake and session crypto

### 4.1 Handshake and frames

The formulas are CRYPTO_SPEC §15; this is a summary. A connection starts with pre-auth JSON
frames and, once the handshake succeeded, carries AEAD frames only.

```
pairing_id = random(16), K_pair = random(32)               desktop, at approval
nonce_h = random(32) host, nonce_d = random(32) desktop     per connection
K_auth  = HKDF(K_pair, salt = pairing_id, ctx("browser/auth"), 32)
T       = ctx("browser/handshake") || 0x00 || pairing_id || nonce_h || nonce_d
proof_d = HMAC-SHA-256(K_auth, T || 0x02)                   the desktop proves first
proof_h = HMAC-SHA-256(K_auth, T || 0x01)
okm     = HKDF(K_pair, salt = pairing_id || nonce_h || nonce_d, ctx("browser/session"), 64)
K_h2d = okm[0..32], K_d2h = okm[32..64]
frame   = header || ct,  header = 0x01 || 0x01 || seq:u64 (big-endian)
ct      = XChaCha20-Poly1305(K_dir, nonce = 0^16 || seq:u64, plaintext,
                             AAD = header || aad("browser/frame", dir:u8, pairing_id))
dir: 0x01 host to desktop, 0x02 desktop to host. seq starts at 0 per direction;
the receiver requires seq == the next expected number.
```

```
host                                       desktop
hello {pairingId, nonce}              ->
                                      <-   challenge {nonce, proof}      (or refused)
finish {proof}                        ->
                                      <-   AEAD frame 0: {"event":"state","state":...}
```

- The host verifies `proof_d` before it sends `finish`. A wrong `proof_d` is
  `desktopNotAuthentic` on the host, which hangs up; the desktop sees the connection close.
- The desktop answers an unknown `pairingId` and a wrong `proof` alike with
  `refused notPaired`. After `finish` verifies, the desktop's first AEAD frame is the
  `state` event; the host waits for it.
- Every step has a timeout of 5 s.
- Any failure of a frame (bad tag, wrong sequence number, unknown version or algorithm) is
  one error; the receiver closes the connection.
- A sequence number at `u64::MAX` is not used: the sender stops.

### 4.2 Pre-auth messages

JSON, one per frame, with `"type"` and `"v": 1`. Unknown fields are rejected. Hex is
lowercase with a fixed length: `pairingId` 32 digits, `nonce` and `proof` 64, `key` 64. Key
order is not significant.

| Direction | Message |
|---|---|
| host to desktop | `hello {pairingId, nonce}` |
| | `finish {proof}` |
| | `pairRequest {browser: "chromium" \| "firefox", hint}` |
| | `status {}` |
| | `focus {}` |
| desktop to host | `challenge {nonce, proof}` |
| | `refused {reason: ErrorCode}` |
| | `status {state: "locked" \| "unlocked"}` |
| | `focused {}` |
| | `pairPending {code, expiresInSeconds}` |
| | `pairApproved {pairingId, key}` |

- A message with `v` other than 1 is answered `refused protocolVersion`; a message that
  does not parse is answered `refused invalidRequest`.
- A pair connection closes after `pairApproved`; the host then handshakes on a new
  connection.
- `browser` is set by the host from the verified caller. `hint` comes from the extension
  (`chrome`, `edge`, `brave`, `opera`, `vivaldi`, `chromium`, `firefox`, `other`; any other
  value is read as `other`) and is for display only.
- The pairing code is 30 random bits written as six symbols of Crockford base32
  (`0123456789ABCDEFGHJKMNPQRSTVWXYZ`), shown as `XXX-XXX`. It correlates a request with
  the user's click; it is not a cryptographic short authentication string (R45).
- `pairApproved` carries `K_pair` in the clear over the OS-protected endpoint. That is by
  design (CRYPTO_SPEC §15).

## 5. Extension and desktop messages

### 5.1 Envelope

The same JSON travels between the extension and the host and, inside AEAD frames, between
the host and the desktop. Rules of DESKTOP_IPC §1.4: camelCase, every key present, `null` for
no value, unknown keys rejected. Secrets are JSON strings (`protocol::Secret`: zeroizing,
not `Clone`, `Debug` prints `[REDACTED]`).

```
request  {"v":1,"id":<u32>,"op":<Op>,"body":{...}}
response {"v":1,"id":7,"ok":true,"body":{...}}  |  {"v":1,"id":7,"ok":false,"error":<ErrorCode>}
event    {"v":1,"event":"state","state":"locked"|"unlocked"}
         {"v":1,"event":"unpaired"}
         {"v":1,"event":"disconnected"}
         {"v":1,"event":"pairCode","requestId":<id>,"code":"4F7K2Q","expiresInSeconds":120}
```

`state` and `unpaired` come from the desktop; `disconnected` and `pairCode` are the host's.
A request with `v` other than 1 gets `protocolVersion`.

### 5.2 Ops

`FORWARDED_OPS` is every op except `hello` and `pair`. There is no list or dump API.

| op | request body | response body |
|---|---|---|
| `hello` (host-local) | `{pairing: {pairingId, key} \| null, hint}` | `{desktop: "notRunning" \| "locked" \| "unlocked", paired}` |
| `pair` (host-local, then pre-auth) | `{hint}` | event `pairCode`, then `{pairingId, key}` |
| `status` | `{}` | `{state}` |
| `focus` | `{}` | `{}` (works unauthenticated too) |
| `listForOrigin` | `{origin, topOrigin}` | `{items: LoginMatch[] (at most 50), warnings: OriginWarning[]}` |
| `search` | `{query (1 to 128 chars, trimmed), origin \| null}` | `{items: SearchHit[] (at most 20)}` |
| `fill` | `{vaultId, itemId, origin, topOrigin, frame: "top" \| "sameOriginFrame", want: {login, otp}}` | `{username: string \| null, password: Secret \| null, otp: Secret \| null}` |
| `copy` | `{source: {kind: "item", vaultId, itemId, field: "username" \| "password" \| "totp"} \| {kind: "generated"}}` | `{clearAfterSeconds: number \| null}` |
| `generate` | `{request: <desktop GenerateRequest>}`, where `forSite` gains `rules: string \| null` (at most 4096) | `{value: Secret, entropyBits, strength, rulesSource: "page" \| "siteQuirks" \| null}` |
| `checkSave` | `{origin, topOrigin, username: string \| null, password: Secret}` | `{kind: "new" \| "update" \| "unchanged", itemTitle: string \| null}` |
| `proposeSave` | `{origin, topOrigin, username: string \| null, password: Secret}` | `{outcome: "saved" \| "updated" \| "cancelled", vaultId \| null, itemId \| null}` |
| `unpair` | `{}` | `{}` |

- `hello` starts or restarts the host's session with the stored pairing and reports the
  desktop state. With `pairing: null` it asks for the lock state without a session
  (`paired: false`). `paired` is true when the session with the stored pairing is
  established, or when the desktop could not be reached to find out. `pair` shows the
  desktop's code as an event and answers with the new pairing when the user allows it;
  the host then connects with it.
- `LoginMatch` = `{vaultId, itemId, title \| null, username \| null, host \| null, hasOtp,
  favorite, matchedBy: "exactOrigin" \| "etld1", blocked: "insecure" \| null}`.
- `SearchHit` = `{vaultId, itemId, title, username, host, hasOtp, favorite, fillable}`.
- `OriginWarning` = `{kind: "idn", unicodeHost}` | `{kind: "insecure"}` |
  `{kind: "lookalike", savedHost}`.
- Limits: origin at most 2048 bytes, username 256, password 1024, title 256 characters.

### 5.3 Error codes

`ErrorCode` is a camelCase string.

- Desktop: `invalidRequest`, `notPaired`, `protocolVersion`, `busy`, `locked`,
  `rateLimited`, `originMismatch`, `insecureOrigin`, `crossOriginFrame`, `itemNotFound`,
  `vaultUnavailable`, `noOtp`, `otpNotFillable`, `fieldEmpty`, `clipboardUnavailable`,
  `cancelled`, `pairingDenied`, `pairingExpired`, `pairingConflict`, `pairingBlocked`,
  `pairingLimit`, `internal`.
- Host: `desktopNotRunning`, `desktopNotAuthentic`, `timeout`, `frameTooLarge`.
- Extension-local, never on the wire: `hostMissing`, `hostForbidden`, `hostExited`,
  `unsupportedPage`, `noFields`, `noMatch`, `frameNeedsConfirmation`, `frameOpaque`,
  `originChanged`, `pageHidden`.

A host answer to a drop of the desktop is `desktopNotRunning`. An unknown or forbidden op,
a body that does not parse and a request id already in flight are `invalidRequest`. A
pairing the desktop does not know is `notPaired`; the extension keeps its record, shows
"Connect again", and never deletes the pairing on this alone.

## 6. Limits, timeouts and behaviour

| Constant | Value |
|---|---|
| `CONNECT_TIMEOUT` | 2 s |
| `HANDSHAKE_STEP_TIMEOUT` | 5 s |
| `REQUEST_TIMEOUT` | 15 s (the extension waits 20 s) |
| `PAIR_TIMEOUT` | 120 s |
| `SAVE_PROMPT_TIMEOUT` | 300 s |
| desktop idle close | 10 min |
| extension port idle disconnect | 60 s with nothing in flight |
| `MAX_CONNECTIONS` | 8 (more are accepted and closed) |
| `MAX_IN_FLIGHT` | 4 per connection |
| `MAX_PAIRINGS` | 16 (`pairingLimit`) |

Rate limits are the desktop's, per pairing, as sliding windows: `fill` 10 per 10 s, `copy`
20 per 60 s, `listForOrigin` and `search` together 60 per 60 s, `generate` 30 per 60 s,
`checkSave` 10 per 60 s, `proposeSave` 3 per 60 s; `focus` 1 per 2 s (global);
`pairRequest` 3 per 60 s (global), and 3 denials within 10 min block pair requests for
10 min (`pairingBlocked`).

States. Desktop not running: `hello` says `notRunning`. Locked: the authenticated session
stays up; `status`, `focus` and `unpair` work and every other op returns `locked`. Pairing
while locked returns `locked`. State changes are pushed as `state` events. On lock, pending
pair and save prompts close with reason `locked` and the held password is dropped.

## 7. Packaging (developer builds)

`mosaic-native-host install` writes host manifests with `path` set to the running
executable:

- Chromium family: `{name, description, path, type: "stdio", allowed_origins:
  allowed_chromium_origins()}`.
- Firefox: the same with `allowed_extensions: [FIREFOX_EXTENSION_ID]` in place of
  `allowed_origins`.

| OS | Chromium family | Firefox |
|---|---|---|
| Windows | JSON in `%LOCALAPPDATA%\io.github.stephanbaum.mosaic\NativeMessagingHosts\{chromium,firefox}.json`; the registry default value under `HKCU\Software\Google\Chrome\NativeMessagingHosts\<HOST_NAME>`, `HKCU\Software\Microsoft\Edge\...` and `HKCU\Software\Chromium\...` | `HKCU\Software\Mozilla\NativeMessagingHosts\<HOST_NAME>` |
| macOS | `~/Library/Application Support/{Google/Chrome,Microsoft Edge,Chromium}/NativeMessagingHosts/<HOST_NAME>.json` | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |
| Linux | `~/.config/{google-chrome,microsoft-edge,chromium}/NativeMessagingHosts/` | `~/.mozilla/native-messaging-hosts/` |

The registry is written with `reg.exe add ... /ve /t REG_SZ /d <path> /f` (no unsafe code and
no extra crate). `uninstall` removes every file and key `install` can write, for every
browser. The M9 installer takes over (sidecar `externalBin`, store IDs only, signing).
Flatpak and Snap browsers cannot run native hosts; that is documented as unsupported.

## 8. Fixtures and tests

`crates/native-messaging/fixtures/protocol.json` holds one request and one `ok` response for
every op, every error code as an error response, and every event. The extension's validators
(T3) check each sample in both directions. Rewrite it with
`MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-native-messaging protocol_fixtures`; the test
fails if the file is stale. The Rust tests also cover the frame codecs, the strict envelopes,
the op allowlist, the caller check, the endpoint derivation for every OS, the discovery-file
validator, the handshake over in-memory streams (success, a wrong key, a replay, an unknown
pairing, protocol versions, per-step timeouts), the relay, and the host binary against an
in-test desktop built from `server_handshake` and the session code. The fuzz targets are
`envelope`, `preauth_message` and `channel_open` (`crates/native-messaging/fuzz`).

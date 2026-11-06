// attempts: post well-formed shim requests into the parent and the top window (forge-frames.html), directly, with a
// transferred MessagePort, and by running code in the parent's realm. The bridge must ignore them: their
// `event.source` is this frame, not the window the bridge serves, and a cross-origin realm is out of reach.
(() => {
  const request = (id) => ({
    type: "mosaic-passkey-req",
    id,
    op: "get",
    options: { rpId: null, challenge: "AAAAAAAAAAAAAAAAAAAAAA", allowCredentials: [], userVerification: "preferred" },
  });
  const targets = [window.parent, window.top].filter((target) => target && target !== window);
  let id = 500;
  for (const target of targets) {
    target.postMessage(request(id++), "*");
    const channel = new MessageChannel();
    target.postMessage(request(id++), "*", [channel.port2]);
    try {
      // Cross-origin: this throws (a same-origin frame could do it, and would gain nothing beyond its own origin).
      target.Function(`postMessage(${JSON.stringify(request(id++))}, "*")`)();
    } catch {
      // expected
    }
  }
  document.title = `forged ${String(id - 500)}`;
})();

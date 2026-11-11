// attempts: as another installed extension's content script, reach Mosaic's content script state
// (it lives in Mosaic's isolated world, not this one) and pass on fill-shaped messages from its
// own extension, which `tabs.sendMessage` delivers only to this script, never to Mosaic's.
document.documentElement.dataset.evilExtensionId = chrome.runtime.id;
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message !== null && typeof message === "object" && message.check === true) {
    sendResponse(typeof globalThis.mosaicContent);
  }
  return false;
});

// attempts (e2e/hostile/never-save-spoof.spec.ts, M4-R10): from this extension's content script,
// read or empty Mosaic's never-save list through runtime.sendMessage and a port named "popup"
// to Mosaic's ID. The page (never-save-spoof.html) names the ID and the origins; the outcome is
// written to `data-evil-never-save` for the test.
let neverSaveTried = false;
window.addEventListener("message", (event) => {
  const data = event.data;
  if (event.source !== window || data === null || typeof data !== "object" || typeof data.evilNeverSave !== "string") {
    return;
  }
  if (neverSaveTried) {
    return;
  }
  neverSaveTried = true;
  const mosaicId = data.evilNeverSave;
  const origins = Array.isArray(data.origins) ? data.origins : [];
  const send = (message) =>
    chrome.runtime.sendMessage(mosaicId, message).then(
      (reply) => `reply:${JSON.stringify(reply)}`,
      () => "refused",
    );
  const tries = [send({ op: "neverSaveList" }), send({ id: 1, request: { op: "neverSaveList" } })];
  for (const origin of origins) {
    tries.push(send({ op: "neverSaveRemove", origin }), send({ id: 1, request: { op: "neverSaveRemove", origin } }));
  }
  tries.push(
    new Promise((resolve) => {
      const port = chrome.runtime.connect(mosaicId, { name: "popup" });
      port.onMessage.addListener((reply) => {
        resolve(`reply:${JSON.stringify(reply)}`);
      });
      port.onDisconnect.addListener(() => {
        resolve("disconnected");
      });
      for (const origin of origins) {
        port.postMessage({ id: 2, request: { op: "neverSaveRemove", origin } });
      }
      port.postMessage({ id: 3, request: { op: "neverSaveList" } });
      setTimeout(() => {
        resolve("open");
      }, 2000);
    }),
  );
  void Promise.all(tries).then((results) => {
    document.documentElement.dataset.evilNeverSave = JSON.stringify(results);
  });
});

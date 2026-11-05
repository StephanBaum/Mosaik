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

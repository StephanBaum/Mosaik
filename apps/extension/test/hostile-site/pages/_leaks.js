// Records every value that arrives in an input, as a hostile page would see it.
window.__mosaicLeaks = [];
for (const type of ["input", "change"]) {
  document.addEventListener(
    type,
    (event) => {
      const target = event.target;
      if (target instanceof HTMLInputElement) {
        window.__mosaicLeaks.push({ type, id: target.id, value: target.value, trusted: event.isTrusted });
      }
    },
    true,
  );
}

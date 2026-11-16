// RustCatch Content Script: SPA navigation observer and frame/document candidate reporter
(() => {
  let localEpoch = 1;
  let lastHref = location.href;
  const observer = new MutationObserver(() => {
    if (location.href !== lastHref) {
      lastHref = location.href;
      localEpoch += 1;
      chrome.runtime.sendMessage({ type: "rustcatch.spa_navigation", href: lastHref, localEpoch });
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
})();

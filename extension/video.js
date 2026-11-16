// RustCatch Video Floating Bar (UI-09): binds to active <video> element and canonical watch/shorts URL
(() => {
  function attachFloatingButton(videoEl) {
    if (videoEl.dataset.rustcatchBound === "true") return;
    videoEl.dataset.rustcatchBound = "true";
  }
  document.querySelectorAll("video").forEach(attachFloatingButton);
})();

document.addEventListener("DOMContentLoaded", () => {
  const statusEl = document.getElementById("bridge-status");
  if (statusEl) {
    statusEl.textContent = "Native Host: com.rustcatch.bridge (协议 V1 已就绪)";
  }
});

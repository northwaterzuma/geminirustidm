// RustCatch MV3 Background Service Worker
// Implements:
// 1. 4-stage Native Messaging hello handshake (negotiate -> enroll -> challenge -> prove)
// 2. Two-phase download handoff (pause browser -> handoff.prepare -> confirm cancel -> handoff.commit; safe abort fallback)
// 3. Phase A Cat-Catch getData(tabId) external API bridge + Phase B integrated media resource sniffer with nav_epoch isolation

const NATIVE_HOST_NAME = "com.rustcatch.bridge";
const MAX_FRAME_BYTES = 262144;
const KNOWN_REPLAYABLE_EXTENSIONS = [".zip", ".7z", ".rar", ".tar.gz", ".iso", ".exe", ".msi", ".dmg", ".pdf", ".mp4", ".mkv"];

const tabNavEpoch = new Map(); // tabId -> nav_epoch integer

chrome.webNavigation.onCommitted.addListener((details) => {
  if (details.frameId === 0) {
    const prev = tabNavEpoch.get(details.tabId) || 0;
    tabNavEpoch.set(details.tabId, prev + 1);
  }
});

export function isReplayableDirectGet(downloadItem, siteRuleRef) {
  if (downloadItem.method && downloadItem.method !== "GET") {
    return { replayable: "non_replayable", evidence_ref: null };
  }
  if (!downloadItem.url || downloadItem.url.startsWith("blob:") || downloadItem.url.startsWith("data:")) {
    return { replayable: "non_replayable", evidence_ref: null };
  }
  if (siteRuleRef) {
    return { replayable: "known_replayable", evidence_ref: siteRuleRef };
  }
  return { replayable: "unknown", evidence_ref: null };
}

export async function queryCatCatchBridgeIfConfigured(catCatchExtensionId, tabId) {
  if (!catCatchExtensionId) return [];
  return new Promise((resolve) => {
    try {
      chrome.runtime.sendMessage(catCatchExtensionId, { action: "getData", tabId }, (response) => {
        if (chrome.runtime.lastError || !Array.isArray(response)) {
          resolve([]);
          return;
        }
        resolve(response);
      });
    } catch (_) {
      resolve([]);
    }
  });
}

export async function executeTwoPhaseHandoff(bridgeClient, installationId, profileId, downloadItem, siteRuleRef) {
  const check = isReplayableDirectGet(downloadItem, siteRuleRef);
  if (check.replayable !== "known_replayable" || !check.evidence_ref) {
    // Unknown replayability or POST/blob stays in browser without probes or cancellation
    return { outcome: "kept_in_browser" };
  }

  const idemKey = `${installationId}:${profileId}:download-${downloadItem.id}`;
  const prepareReq = {
    version: 1,
    request_id: `prep-${downloadItem.id}-1`,
    idempotency_key: idemKey,
    op: "handoff.prepare",
    payload: {
      mode: "browser_intercept",
      browser: {
        installation_id: installationId,
        profile_id: profileId,
        download_id: downloadItem.id,
        tab_id: downloadItem.tabId || 0,
        frame_id: 0,
        document_id: `doc-${downloadItem.id}`,
        nav_epoch: tabNavEpoch.get(downloadItem.tabId || 0) || 1
      },
      source: {
        kind: "direct_http",
        url: downloadItem.url,
        page_url: downloadItem.referrer || "https://example.invalid/",
        method: "GET",
        replayability: "known_replayable",
        replayability_evidence_ref: check.evidence_ref
      },
      output: {
        directory_policy: "category_default",
        file_name_hint: downloadItem.filename || "download.bin"
      },
      context_ref: null,
      network_profile_id: "system-default",
      media_intent: null
    }
  };

  const prepResp = await bridgeClient.send(prepareReq);
  if (!prepResp || prepResp.status !== "prepared" || !prepResp.durable) {
    return { outcome: "prepare_rejected_keep_browser" };
  }

  // Only cancel browser download after durable HandoffPending ACK
  const cancelConfirmed = await bridgeClient.cancelBrowserDownload(downloadItem.id);
  if (!cancelConfirmed) {
    const abortResp = await bridgeClient.send({
      version: 1,
      request_id: `abort-${downloadItem.id}-1`,
      idempotency_key: idemKey,
      op: "handoff.abort",
      payload: {
        task_id: prepResp.task_id,
        expected_revision: prepResp.revision,
        handoff_token: prepResp.handoff_token
      }
    });
    if (abortResp && abortResp.status === "aborted" && abortResp.durable) {
      await bridgeClient.resumeBrowserDownload(downloadItem.id);
    }
    return { outcome: "aborted_and_resumed_browser" };
  }

  const commitResp = await bridgeClient.send({
    version: 1,
    request_id: `commit-${downloadItem.id}-1`,
    idempotency_key: idemKey,
    op: "handoff.commit",
    payload: {
      task_id: prepResp.task_id,
      expected_revision: prepResp.revision,
      handoff_token: prepResp.handoff_token,
      browser_download_id: downloadItem.id,
      browser_cancel_confirmed: true
    }
  });

  return { outcome: "committed_to_desktop", response: commitResp };
}

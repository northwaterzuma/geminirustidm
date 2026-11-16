//! Candidate aggregation for MDM native capture and Cat-Catch (`getData(tabId)`) API bridge.
//! Enforces `nav_epoch` isolation, 100-item page limit, and media stream deduplication without merging distinct audio/video tracks.

use mdm_ipc::{BrowserMeta, CaptureCandidate, ErrorCode, ErrorDetail, MAX_CANDIDATES_PER_PAGE};
use mdm_tools::video::is_ephemeral_playback_chunk;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct CaptureRegistry {
    /// Keyed by `(installation_id, profile_id, tab_id)` -> `(active_nav_epoch, Vec<CaptureCandidate>)`
    by_tab: BTreeMap<(String, String, u32), (u64, Vec<CaptureCandidate>)>,
}

impl CaptureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish_batch(
        &mut self,
        browser: &BrowserMeta,
        candidates: Vec<CaptureCandidate>,
    ) -> Result<usize, ErrorDetail> {
        if candidates.len() > MAX_CANDIDATES_PER_PAGE {
            return Err(ErrorDetail {
                code: ErrorCode::TooLarge,
                message: format!("Candidate batch size {} exceeds {}", candidates.len(), MAX_CANDIDATES_PER_PAGE),
                retryable: false,
                retry_after_ms: None,
            });
        }

        let tab_key = (
            browser.installation_id.clone(),
            browser.profile_id.clone(),
            browser.tab_id,
        );
        let entry = self
            .by_tab
            .entry(tab_key)
            .or_insert_with(|| (browser.nav_epoch, Vec::new()));

        if browser.nav_epoch < entry.0 {
            return Err(ErrorDetail {
                code: ErrorCode::StaleNavigation,
                message: format!("Stale nav_epoch {} < active {}", browser.nav_epoch, entry.0),
                retryable: false,
                retry_after_ms: None,
            });
        }
        if browser.nav_epoch > entry.0 {
            // Navigation epoch advanced (e.g. SPA video switch): clear old candidates
            entry.0 = browser.nav_epoch;
            entry.1.clear();
        }

        let mut stored = 0usize;
        for cand in candidates {
            if cand.browser.nav_epoch != browser.nav_epoch {
                continue;
            }
            // Filter out ephemeral YouTube googlevideo range playback chunks
            if is_ephemeral_playback_chunk(&cand.url) {
                continue;
            }
            // Deduplicate only if exact URL and MIME and frame_id match; never merge distinct audio/video tracks by title
            let already_exists = entry.1.iter().any(|existing| {
                existing.url == cand.url && existing.mime == cand.mime && existing.browser.frame_id == cand.browser.frame_id
            });
            if !already_exists {
                entry.1.push(cand);
                stored += 1;
            }
        }
        Ok(stored)
    }

    pub fn list_page(
        &self,
        browser: &BrowserMeta,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<(Vec<CaptureCandidate>, Option<String>), ErrorDetail> {
        let bounded = limit.clamp(1, MAX_CANDIDATES_PER_PAGE);
        let tab_key = (
            browser.installation_id.clone(),
            browser.profile_id.clone(),
            browser.tab_id,
        );
        let Some((active_epoch, list)) = self.by_tab.get(&tab_key) else {
            return Ok((Vec::new(), None));
        };
        if browser.nav_epoch != *active_epoch {
            return Err(ErrorDetail {
                code: ErrorCode::StaleNavigation,
                message: "Requested nav_epoch does not match active tab epoch".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        let start_idx = cursor.and_then(|c| c.parse::<usize>().ok()).unwrap_or(0);
        if start_idx >= list.len() {
            return Ok((Vec::new(), None));
        }
        let end_idx = (start_idx + bounded).min(list.len());
        let mut slice: Vec<CaptureCandidate> = list[start_idx..end_idx].to_vec();
        for item in &mut slice {
            item.url = mdm_ipc::redact_sensitive_url(&item.url);
        }
        let next_cursor = if end_idx < list.len() {
            Some(end_idx.to_string())
        } else {
            None
        };
        Ok((slice, next_cursor))
    }
}

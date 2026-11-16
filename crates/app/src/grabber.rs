//! P5-T02 (REQ-15): IDM 4-Step Site Grabber Wizard, domain/depth/page/byte limits,
//! SSRF guard, and offline relative link rewriter.

use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrabberProject {
    pub project_id: String,
    pub start_url: String,
    pub same_site_only: bool,
    pub max_depth: u8,
    pub max_pages: usize,
    pub max_total_bytes: u64,
    pub allowed_extensions: Vec<String>,
    pub visited_urls: HashSet<String>,
    pub queued_urls: VecDeque<(String, u8)>,
    pub downloaded_bytes: u64,
}

impl GrabberProject {
    pub fn new(project_id: impl Into<String>, start_url: impl Into<String>) -> Result<Self, ErrorDetail> {
        let s = start_url.into();
        validate_grabber_target_url(&s, false)?;
        let mut q = VecDeque::new();
        q.push_back((s.clone(), 0));
        Ok(Self {
            project_id: project_id.into(),
            start_url: s,
            same_site_only: true,
            max_depth: 2,
            max_pages: 500,
            max_total_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            allowed_extensions: vec!["html".into(), "css".into(), "js".into(), "png".into(), "jpg".into(), "pdf".into(), "zip".into()],
            visited_urls: HashSet::new(),
            queued_urls: q,
            downloaded_bytes: 0,
        })
    }

    /// Evaluates discovered links on a page without executing page scripts, enforcing same-site, depth <= 2,
    /// max_pages <= 500, and rejecting local/metadata SSRF addresses.
    pub fn discover_links(&mut self, current_url: &str, current_depth: u8, hrefs: &[String]) -> Vec<String> {
        if current_depth >= self.max_depth || self.visited_urls.len() >= self.max_pages {
            return Vec::new();
        }
        let Ok(base) = Url::parse(current_url) else {
            return Vec::new();
        };
        let Ok(root) = Url::parse(&self.start_url) else {
            return Vec::new();
        };
        let mut accepted = Vec::new();

        for raw_href in hrefs {
            if self.visited_urls.len() + self.queued_urls.len() >= self.max_pages {
                break;
            }
            let Ok(mut resolved) = base.join(raw_href.trim()) else {
                continue;
            };
            resolved.set_fragment(None);
            let url_str = resolved.to_string();
            if validate_grabber_target_url(&url_str, false).is_err() {
                continue;
            }
            if self.same_site_only && resolved.host_str() != root.host_str() {
                continue;
            }
            if self.visited_urls.contains(&url_str) || self.queued_urls.iter().any(|(u, _)| u == &url_str) {
                continue;
            }
            self.queued_urls.push_back((url_str.clone(), current_depth + 1));
            accepted.push(url_str);
        }
        accepted
    }
}

/// Blocks SSRF access to `file://`, loopback `127.0.0.0/8`, link-local metadata `169.254.169.254`, and `localhost`
/// when spidering untrusted web links (unless explicitly added by user for LAN NAS).
pub fn validate_grabber_target_url(raw_url: &str, allow_explicit_lan: bool) -> Result<Url, ErrorDetail> {
    let parsed = Url::parse(raw_url).map_err(|_| ErrorDetail {
        code: ErrorCode::InvalidRequest,
        message: format!("Invalid URL: {}", raw_url),
        retryable: false,
        retry_after_ms: None,
    })?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ErrorDetail {
            code: ErrorCode::PermissionDenied,
            message: format!("Disallowed scheme in grabber: {}", parsed.scheme()),
            retryable: false,
            retry_after_ms: None,
        });
    }
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if host == "169.254.169.254" || host == "metadata.google.internal" || host == "localhost" || host.starts_with("127.") {
        if !allow_explicit_lan {
            return Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: format!("Blocked loopback/metadata address in site grabber: {}", host),
                retryable: false,
                retry_after_ms: None,
            });
        }
    }
    Ok(parsed)
}

/// Rewrites downloaded asset links in HTML to offline relative paths while preserving non-downloaded external URLs.
pub fn rewrite_html_links_for_offline(html: &str, url_to_local_rel_path: &[(String, String)]) -> String {
    let mut out = html.to_string();
    for (remote_url, local_rel) in url_to_local_rel_path {
        out = out.replace(&format!("href=\"{}\"",remote_url), &format!("href=\"{}\"",local_rel));
        out = out.replace(&format!("src=\"{}\"",remote_url), &format!("src=\"{}\"",local_rel));
    }
    out
}

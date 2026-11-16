use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceValidator {
    pub etag: Option<String>,
    pub is_strong_etag: bool,
    pub last_modified: Option<String>,
    pub total_bytes: Option<u64>,
    pub content_encoding: String,
}

impl SourceValidator {
    pub fn from_headers(etag: Option<&str>, last_modified: Option<&str>, total_bytes: Option<u64>, content_encoding: Option<&str>) -> Self {
        let (clean_etag, is_strong) = match etag {
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.starts_with("W/") || trimmed.starts_with("w/") {
                    (Some(trimmed.to_string()), false)
                } else {
                    (Some(trimmed.to_string()), !trimmed.is_empty())
                }
            }
            None => (None, false),
        };
        Self {
            etag: clean_etag,
            is_strong_etag: is_strong,
            last_modified: last_modified.map(|s| s.trim().to_string()),
            total_bytes,
            content_encoding: content_encoding.unwrap_or("identity").trim().to_ascii_lowercase(),
        }
    }

    /// Returns whether this validator provides a reliable strong identity for multi-segment or cross-session resume.
    pub fn can_resume_reliably(&self) -> bool {
        if self.content_encoding != "identity" {
            return false;
        }
        self.total_bytes.is_some() && (self.is_strong_etag || self.last_modified.is_some())
    }

    /// Checks whether a subsequent response validator matches the recorded baseline validator.
    pub fn matches(&self, newer: &SourceValidator) -> bool {
        if self.content_encoding != newer.content_encoding {
            return false;
        }
        if self.total_bytes != newer.total_bytes {
            return false;
        }
        if let (Some(e1), Some(e2)) = (&self.etag, &newer.etag) {
            if !self.is_strong_etag || !newer.is_strong_etag || e1 != e2 {
                return false;
            }
        } else if self.etag.is_some() != newer.etag.is_some() {
            return false;
        }
        if let (Some(lm1), Some(lm2)) = (&self.last_modified, &newer.last_modified) {
            if lm1 != lm2 {
                return false;
            }
        }
        true
    }

    /// Produces an If-Range header value only when a strong ETag is present per RFC 9110.
    pub fn if_range_header(&self) -> Option<&str> {
        if self.is_strong_etag {
            self.etag.as_deref()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRangeInfo {
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub total: Option<u64>,
}

/// Parses RFC 9110 Content-Range header: `bytes start-end/total` or `bytes */total`.
pub fn parse_content_range(header: &str) -> Result<ContentRangeInfo, ErrorDetail> {
    let trimmed = header.trim();
    let Some(rest) = trimmed.strip_prefix("bytes ") else {
        return Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: format!("Invalid Content-Range unit: {}", header),
            retryable: false,
            retry_after_ms: None,
        });
    };
    let Some((range_part, total_part)) = rest.split_once('/') else {
        return Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: format!("Missing '/' in Content-Range: {}", header),
            retryable: false,
            retry_after_ms: None,
        });
    };
    let total = if total_part == "*" {
        None
    } else {
        Some(total_part.parse::<u64>().map_err(|_| ErrorDetail {
            code: ErrorCode::NetworkError,
            message: format!("Invalid total length in Content-Range: {}", header),
            retryable: false,
            retry_after_ms: None,
        })?)
    };

    if range_part == "*" {
        return Ok(ContentRangeInfo {
            start: None,
            end: None,
            total,
        });
    }

    let Some((start_s, end_s)) = range_part.split_once('-') else {
        return Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: format!("Invalid range bounds in Content-Range: {}", header),
            retryable: false,
            retry_after_ms: None,
        });
    };
    let start = start_s.parse::<u64>().map_err(|_| ErrorDetail {
        code: ErrorCode::NetworkError,
        message: "Invalid start offset".to_string(),
        retryable: false,
        retry_after_ms: None,
    })?;
    let end = end_s.parse::<u64>().map_err(|_| ErrorDetail {
        code: ErrorCode::NetworkError,
        message: "Invalid end offset".to_string(),
        retryable: false,
        retry_after_ms: None,
    })?;
    if start > end {
        return Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: "Content-Range start > end".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    if let Some(tot) = total {
        if end >= tot {
            return Err(ErrorDetail {
                code: ErrorCode::NetworkError,
                message: "Content-Range end >= total length".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
    }
    Ok(ContentRangeInfo {
        start: Some(start),
        end: Some(end),
        total,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeResponseDecision {
    /// Valid 206 Partial Content; returns `[write_start, write_end_exclusive)`.
    WritePartial { start: u64, end_exclusive: u64, remaining_start: Option<u64> },
    /// Server ignored Range and returned 200 OK; only safe if task has no committed non-zero segments.
    DowngradeToSingleStream,
    /// HTTP 416 where `bytes */N` matches validator and all intervals `[0, N)` are already complete.
    TransitionToVerify,
}

/// Validates HTTP status (206, 200, 416) against the requested half-open interval `[req_start, req_end_exclusive)`.
pub fn evaluate_range_response(
    status: u16,
    req_start: u64,
    req_end_exclusive: u64,
    content_range_hdr: Option<&str>,
    actual_body_len: u64,
    baseline_validator: &SourceValidator,
    response_validator: &SourceValidator,
    has_committed_nonzero_segments: bool,
    all_intervals_complete: bool,
) -> Result<RangeResponseDecision, ErrorDetail> {
    if !baseline_validator.matches(response_validator) {
        return Err(ErrorDetail {
            code: ErrorCode::ResourceChanged,
            message: "Remote ETag, Last-Modified, or Content-Encoding changed; refusing to splice stale bytes".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    match status {
        206 => {
            let hdr = content_range_hdr.ok_or_else(|| ErrorDetail {
                code: ErrorCode::NetworkError,
                message: "HTTP 206 missing Content-Range header".to_string(),
                retryable: false,
                retry_after_ms: None,
            })?;
            let cr = parse_content_range(hdr)?;
            let (Some(start), Some(end)) = (cr.start, cr.end) else {
                return Err(ErrorDetail {
                    code: ErrorCode::NetworkError,
                    message: "HTTP 206 cannot have wildcard byte range".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                });
            };
            if start != req_start {
                return Err(ErrorDetail {
                    code: ErrorCode::NetworkError,
                    message: format!("Content-Range start {} does not match requested {}", start, req_start),
                    retryable: false,
                    retry_after_ms: None,
                });
            };
            let end_exclusive = end + 1;
            if end_exclusive > req_end_exclusive {
                return Err(ErrorDetail {
                    code: ErrorCode::NetworkError,
                    message: format!("Content-Range end {} exceeds requested {}", end_exclusive, req_end_exclusive),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            if let (Some(expected_tot), Some(resp_tot)) = (baseline_validator.total_bytes, cr.total) {
                if expected_tot != resp_tot {
                    return Err(ErrorDetail {
                        code: ErrorCode::ResourceChanged,
                        message: "Content-Range total length changed".to_string(),
                        retryable: false,
                        retry_after_ms: None,
                    });
                }
            }
            let expected_len = end_exclusive - start;
            if actual_body_len != expected_len {
                return Err(ErrorDetail {
                    code: ErrorCode::NetworkError,
                    message: format!("Actual body length {} != Content-Range length {}", actual_body_len, expected_len),
                    retryable: true,
                    retry_after_ms: Some(1000),
                });
            }
            let remaining_start = if end_exclusive < req_end_exclusive {
                Some(end_exclusive)
            } else {
                None
            };
            Ok(RangeResponseDecision::WritePartial {
                start,
                end_exclusive,
                remaining_start,
            })
        }
        200 => {
            if req_start > 0 || has_committed_nonzero_segments {
                Err(ErrorDetail {
                    code: ErrorCode::ResourceChanged,
                    message: "Server ignored Range and returned 200 OK on non-zero offset; refusing to write full body into partial offset".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                })
            } else {
                Ok(RangeResponseDecision::DowngradeToSingleStream)
            }
        }
        416 => {
            let hdr = content_range_hdr.ok_or_else(|| ErrorDetail {
                code: ErrorCode::ResourceChanged,
                message: "HTTP 416 missing Content-Range bytes */N".to_string(),
                retryable: false,
                retry_after_ms: None,
            })?;
            let cr = parse_content_range(hdr)?;
            if cr.start.is_some() || cr.total.is_none() {
                return Err(ErrorDetail {
                    code: ErrorCode::ResourceChanged,
                    message: "HTTP 416 invalid Content-Range format".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            if cr.total == baseline_validator.total_bytes && all_intervals_complete {
                Ok(RangeResponseDecision::TransitionToVerify)
            } else {
                Err(ErrorDetail {
                    code: ErrorCode::ResourceChanged,
                    message: "HTTP 416 received but intervals are incomplete or total size changed".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                })
            }
        }
        429 | 503 => Err(ErrorDetail {
            code: ErrorCode::RateLimited,
            message: format!("Server returned status {}", status),
            retryable: true,
            retry_after_ms: Some(30_000),
        }),
        401 | 403 => Err(ErrorDetail {
            code: ErrorCode::AuthRequired,
            message: format!("Authentication or fresh signature required (HTTP {})", status),
            retryable: false,
            retry_after_ms: None,
        }),
        _ => Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: format!("Unexpected HTTP status {}", status),
            retryable: status >= 500,
            retry_after_ms: None,
        }),
    }
}

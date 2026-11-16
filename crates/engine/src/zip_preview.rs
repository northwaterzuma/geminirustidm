use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};

pub const MAX_CENTRAL_DIR_BYTES: u64 = 8 * 1024 * 1024; // 8 MiB
pub const MAX_PREVIEW_ENTRIES: usize = 10_000;
pub const MAX_COMPRESSION_RATIO: u64 = 200; // Zip bomb guard

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipEntryPreview {
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub is_encrypted: bool,
    pub compression_method: u16,
}

/// Parses a tail Range buffer of a ZIP/ZIP64 archive to locate the End of Central Directory (EOCD)
/// and Central Directory entries without downloading the full archive.
pub fn parse_zip_tail_preview(tail_bytes: &[u8], total_archive_size: u64) -> Result<Vec<ZipEntryPreview>, ErrorDetail> {
    if tail_bytes.len() < 22 {
        return Err(ErrorDetail {
            code: ErrorCode::InvalidRequest,
            message: "Buffer too small to contain ZIP EOCD record".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    // Search backwards for EOCD signature 0x06054b50 (PK\x05\x06)
    let mut eocd_pos = None;
    for i in (0..=tail_bytes.len() - 22).rev() {
        if &tail_bytes[i..i + 4] == b"PK" {
            eocd_pos = Some(i);
            break;
        }
    }
    let eocd_idx = eocd_pos.ok_or_else(|| ErrorDetail {
        code: ErrorCode::FormatUnavailable,
        message: "ZIP End of Central Directory (EOCD) not found in Range tail".to_string(),
        retryable: false,
        retry_after_ms: None,
    })?;

    let disk_num = u16::from_le_bytes([tail_bytes[eocd_idx + 4], tail_bytes[eocd_idx + 5]]);
    let cd_disk = u16::from_le_bytes([tail_bytes[eocd_idx + 6], tail_bytes[eocd_idx + 7]]);
    if disk_num != 0 || cd_disk != 0 {
        return Err(ErrorDetail {
            code: ErrorCode::FormatUnavailable,
            message: "Multi-volume split ZIP archives are not supported for Range preview".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    let entry_count = u16::from_le_bytes([tail_bytes[eocd_idx + 10], tail_bytes[eocd_idx + 11]]) as usize;
    let cd_size = u32::from_le_bytes([
        tail_bytes[eocd_idx + 12],
        tail_bytes[eocd_idx + 13],
        tail_bytes[eocd_idx + 14],
        tail_bytes[eocd_idx + 15],
    ]) as u64;
    let cd_offset = u32::from_le_bytes([
        tail_bytes[eocd_idx + 16],
        tail_bytes[eocd_idx + 17],
        tail_bytes[eocd_idx + 18],
        tail_bytes[eocd_idx + 19],
    ]) as u64;

    if cd_size > MAX_CENTRAL_DIR_BYTES || entry_count > MAX_PREVIEW_ENTRIES {
        return Err(ErrorDetail {
            code: ErrorCode::TooLarge,
            message: format!("ZIP Central Directory size {} or count {} exceeds safety limit", cd_size, entry_count),
            retryable: false,
            retry_after_ms: None,
        });
    }

    let tail_start_offset = total_archive_size.saturating_sub(tail_bytes.len() as u64);
    if cd_offset < tail_start_offset {
        return Err(ErrorDetail {
            code: ErrorCode::InvalidRequest,
            message: "Central Directory starts before fetched tail buffer; fetch exact CD range".to_string(),
            retryable: true,
            retry_after_ms: None,
        });
    }

    let mut pos = (cd_offset - tail_start_offset) as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        if pos + 46 > tail_bytes.len() || &tail_bytes[pos..pos + 4] != b"PK" {
            return Err(ErrorDetail {
                code: ErrorCode::InvalidRequest,
                message: "Truncated or corrupt Central Directory entry header".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        let flags = u16::from_le_bytes([tail_bytes[pos + 8], tail_bytes[pos + 9]]);
        let method = u16::from_le_bytes([tail_bytes[pos + 10], tail_bytes[pos + 11]]);
        let comp_size = u32::from_le_bytes([
            tail_bytes[pos + 20],
            tail_bytes[pos + 21],
            tail_bytes[pos + 22],
            tail_bytes[pos + 23],
        ]) as u64;
        let uncomp_size = u32::from_le_bytes([
            tail_bytes[pos + 24],
            tail_bytes[pos + 25],
            tail_bytes[pos + 26],
            tail_bytes[pos + 27],
        ]) as u64;
        let name_len = u16::from_le_bytes([tail_bytes[pos + 28], tail_bytes[pos + 29]]) as usize;
        let extra_len = u16::from_le_bytes([tail_bytes[pos + 30], tail_bytes[pos + 31]]) as usize;
        let comment_len = u16::from_le_bytes([tail_bytes[pos + 32], tail_bytes[pos + 33]]) as usize;

        if pos + 46 + name_len + extra_len + comment_len > tail_bytes.len() {
            return Err(ErrorDetail {
                code: ErrorCode::InvalidRequest,
                message: "Central Directory entry extends past buffer".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        let name_str = String::from_utf8_lossy(&tail_bytes[pos + 46..pos + 46 + name_len]).to_string();
        if name_str.contains("..") || name_str.starts_with('/') || name_str.starts_with('\\') || name_str.contains(':') {
            return Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: format!("Unsafe ZIP entry path rejected: {}", name_str),
                retryable: false,
                retry_after_ms: None,
            });
        }
        if comp_size > 0 && uncomp_size / comp_size > MAX_COMPRESSION_RATIO {
            return Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: format!("Suspected zip-bomb compression ratio in entry {}", name_str),
                retryable: false,
                retry_after_ms: None,
            });
        }

        entries.push(ZipEntryPreview {
            name: name_str,
            compressed_size: comp_size,
            uncompressed_size: uncomp_size,
            is_encrypted: (flags & 0x0001) != 0,
            compression_method: method,
        });
        pos += 46 + name_len + extra_len + comment_len;
    }

    Ok(entries)
}

use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FtpTransferMode {
    PassivePasv,
    PassiveEpsv,
    ActivePort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FtpsSecurity {
    PlainFtp,
    ExplicitTlsAuthTls,
    ImplicitTls990,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FtpCapabilities {
    pub supports_rest: bool,
    pub supports_size: bool,
    pub supports_mdtm: bool,
    pub supports_epsv: bool,
    pub supports_auth_tls: bool,
    pub max_sessions_per_host: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FtpResumeCheck {
    pub remote_size: Option<u64>,
    pub remote_mdtm: Option<String>,
    pub can_resume_with_rest: bool,
}

/// Evaluates whether an interrupted FTP/FTPS transfer can safely resume via `REST <offset>`
/// or must restart from byte 0 without splicing mismatched files.
pub fn evaluate_ftp_resume(
    caps: &FtpCapabilities,
    security: FtpsSecurity,
    tls_cert_valid: bool,
    baseline_size: Option<u64>,
    baseline_mdtm: Option<&str>,
    current_size: Option<u64>,
    current_mdtm: Option<&str>,
) -> Result<FtpResumeCheck, ErrorDetail> {
    if security != FtpsSecurity::PlainFtp && !tls_cert_valid {
        return Err(ErrorDetail {
            code: ErrorCode::NetworkError,
            message: "FTPS TLS certificate validation failed; refusing plaintext downgrade".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    if baseline_size.is_some() && baseline_size != current_size {
        return Err(ErrorDetail {
            code: ErrorCode::ResourceChanged,
            message: "FTP remote SIZE changed; refusing to append to stale part file".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    if let (Some(m1), Some(m2)) = (baseline_mdtm, current_mdtm) {
        if m1 != m2 {
            return Err(ErrorDetail {
                code: ErrorCode::ResourceChanged,
                message: "FTP remote MDTM modification timestamp changed; refusing to splice".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
    }
    let can_resume = caps.supports_rest && caps.supports_size && current_size.is_some() && baseline_mdtm == current_mdtm;
    Ok(FtpResumeCheck {
        remote_size: current_size,
        remote_mdtm: current_mdtm.map(|s| s.to_string()),
        can_resume_with_rest: can_resume,
    })
}

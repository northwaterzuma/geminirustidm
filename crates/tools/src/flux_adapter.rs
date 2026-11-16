//! Optional P3-T03 experimental adapter reference.
//! Default disabled (`flux_experimental = false`). No AGPL-3.0 FluxDown code is compiled into GPL-3.0 binaries.

use mdm_ipc::{ErrorCode, ErrorDetail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxAdapterConfig {
    pub enabled: bool,
    pub agpl_combination_approved: bool,
}

impl Default for FluxAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            agpl_combination_approved: false,
        }
    }
}

pub fn check_flux_adapter_gate(cfg: &FluxAdapterConfig) -> Result<(), ErrorDetail> {
    if !cfg.enabled {
        return Err(ErrorDetail {
            code: ErrorCode::ToolUnavailable,
            message: "FluxExperimental adapter is disabled by default per ADR-008".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    if !cfg.agpl_combination_approved {
        return Err(ErrorDetail {
            code: ErrorCode::PermissionDenied,
            message: "AGPL-3.0 license combination gate not approved for GPL-3.0 distribution".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    Ok(())
}

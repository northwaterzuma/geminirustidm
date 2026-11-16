use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthScheme {
    Basic,
    Digest,
    Ntlm,
    Kerberos,
    Negotiate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthScopePolicy {
    pub allowed_origin_host: String,
    pub allowed_port: Option<u16>,
    pub require_https_for_basic: bool,
    pub allow_proxy_sso: bool,
}

/// Ensures Authorization / NTLM / Negotiate tokens and scoped cookies are stripped on cross-origin redirects.
pub fn should_forward_credentials_on_redirect(original_url: &str, redirect_target_url: &str) -> bool {
    let (Ok(orig), Ok(target)) = (Url::parse(original_url), Url::parse(redirect_target_url)) else {
        return false;
    };
    if orig.scheme() == "https" && target.scheme() != "https" {
        return false;
    }
    orig.host_str() == target.host_str() && orig.port_or_known_default() == target.port_or_known_default()
}

/// Evaluates server/proxy `WWW-Authenticate` or `Proxy-Authenticate` challenge.
pub fn select_auth_challenge(challenges: &[AuthScheme], is_windows_sspi_available: bool) -> Result<AuthScheme, ErrorDetail> {
    for &scheme in challenges {
        match scheme {
            AuthScheme::Negotiate | AuthScheme::Kerberos | AuthScheme::Ntlm => {
                if is_windows_sspi_available {
                    return Ok(scheme);
                }
            }
            AuthScheme::Digest | AuthScheme::Basic => return Ok(scheme),
        }
    }
    Err(ErrorDetail {
        code: ErrorCode::AuthRequired,
        message: "No supported authentication mechanism available in current OS environment".to_string(),
        retryable: false,
        retry_after_ms: None,
    })
}

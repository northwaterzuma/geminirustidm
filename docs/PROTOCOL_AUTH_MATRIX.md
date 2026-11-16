# P5-T01 FTP/FTPS 与企业级高级认证矩阵 (PROTOCOL_AUTH_MATRIX)

- **模块位置**：`crates/engine/src/{ftp,auth}.rs`
- **FTP/FTPS (`evaluate_ftp_resume`)**：支持 `PassivePasv / PassiveEpsv / ActivePort` 与 `PlainFtp / ExplicitTlsAuthTls / ImplicitTls990`；显式/隐式 FTPS 强制验证 TLS 证书（拒绝降级明文）；断点续传前核对 `SIZE` 与 `MDTM`，若服务器不支持 `REST` 或文件已变更则拒绝拼接。
- **高级认证 (`select_auth_challenge`)**：隔离支持 `Basic / Digest / Ntlm / Kerberos / Negotiate`，跨站或降级重定向自动剥离认证头（`should_forward_credentials_on_redirect`）。企业 Windows AD 域控实测标记为 `needs_env` (`controlled_domain`)。

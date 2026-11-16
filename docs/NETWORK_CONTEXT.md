# 网络配置、凭据作用域与资源预算契约 (NETWORK_CONTEXT - P2-T03)

- **模块位置**：`crates/engine/src/{auth,rate_limiter}.rs`、`crates/tools/src/relay.rs`、`crates/ipc/src/task_api.rs`
- **代理一致性**：`NetworkProfile` 支持 `system / manual / direct`，`direct_fallback` 固定为 `false`；手动代理不可达时立即报错 `NetworkError`，绝不静默直连泄漏真实出口 IP。
- **凭据隔离与续期/撤销**：
  - `context.put` 上限 `32,768` 字节，仅允许白名单头 `User-Agent / Referer / Origin / Authorization` 且拦截 CR/LF/NUL。
  - 跨域名或 HTTPS->HTTP 重定向自动剥离 `Authorization` 与原域 Cookie。
  - `context.revoke` 或 15 分钟 TTL 到期立即关闭关联 `LocalRelaySession` 并终止对应外部工具进程树。

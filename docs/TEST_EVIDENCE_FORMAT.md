# 测试证据格式与脱敏规范 (TEST_EVIDENCE_FORMAT)

1. 每次测试运行产出一份遵循 `templates/ACCEPTANCE_RESULT.json` 的机器可读证据文件，保存在 `evidence/<phase_or_run>/<TEST-ID>/result.json`。
2. 凡依赖 Windows 10/11 GUI 真机 (`os_vm`)、真实 Edge/Chrome 扩展环境 (`browser_vm`)、授权在线 YouTube 样本 (`authorized_online`) 或企业域控 (`controlled_domain`) 的子用例，在离线沙箱中必须如实标记 `"status": "needs_env"`，并在 `not_run_reason` 写明最小补足条件。
3. 所有离线可执行子用例（Schema 正反例、HTTP Range 矩阵、检查点崩溃恢复、Native Messaging 帧/HMAC/两阶段接管故障、HLS/DASH/DRM 解析、yt-dlp 错误分类、格式选择、虚拟时钟调度、代理/凭据隔离、批量导入导出、MCP 权限、站点抓取改写、周期同步、ZIP64 Range 预览）必须实际运行并校验 SHA-256 与状态不变量。
4. 任何日志与证据输出均经过脱敏过滤器，禁止出现明文 Cookie、Authorization、代理密码、handoff_token 明文或带签名直链参数。

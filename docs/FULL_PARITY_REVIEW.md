# P5-T03 IDM 全量功能对齐评审与历史遗留协议说明 (FULL_PARITY_REVIEW)

- **已实现对齐项 (REQ-01..REQ-18)**：HTTP/HTTPS 动态分段、两阶段浏览器接管、单扩展流媒体嗅探、YouTube 受控管线、多队列/DST 计划、全局限速、代理与凭据隔离、批量/剪贴板/导入导出、MCP 边界、FTP/FTPS、NTLM/Kerberos/Negotiate 适配层、四步站点抓取、周期同步、远程 ZIP Range 预览、队列完成后动作（通知/打开目录/外部结构化 argv 杀毒扫描/可撤销关机倒计时）。
- **明确不随现代系统实现的过时功能登记**：
  - 传统调制解调器电话拨号上网 (Modem Dial-up RAS)：现代宽带/光纤系统不适用，保留历史状态说明。
  - 早期 Windows Media Player 专有 `MMS://` / `RTSP` 私有流协议：已由标准 HLS/DASH 取代，遇到 `mms://` URL 明确返回 `UnsupportedVersion` / 不支持提示。

# P5-T04 Windows / macOS / Linux 跨平台适配矩阵 (PLATFORM_MATRIX)

| 能力维度 | Windows 10/11 (`x86_64-pc-windows-msvc`) | Linux (`x86_64-unknown-linux-gnu`) | macOS (`aarch64-apple-darwin`) |
|---|---|---|---|
| 桌面 GUI (`egui`/`eframe`) | 原生 DirectX/WGPU/Glow | X11 / Wayland | Metal |
| 子进程树管控 | Windows Job Object | Unix Process Group (`killpg`) | Unix Process Group (`killpg`) |
| 凭据安全存储 | Windows DPAPI / Credential Manager | Secret Service / Keyring (无可用时仅内存) | macOS Keychain |
| Native Host 清单路径 | 注册表 `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.rustcatch.bridge` | `~/.config/google-chrome/NativeMessagingHosts/com.rustcatch.bridge.json` | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.rustcatch.bridge.json` |
| GitHub Actions 构建 | `windows-latest` | `ubuntu-latest` | `macos-latest` |

# IDM 风格界面绑定与差异记录 (UI_DIFFERENCES - P1-T03)

- **模块位置**：`crates/app/src/{ui,app}.rs`
- **已绑定核心窗口 (UI-01..UI-12)**：主窗口（菜单栏、32px 工具栏、180px 分类树、24px 紧凑任务表、24px 状态栏）、添加地址、下载信息、分段进度、视频格式选择、完成与属性、选项、多队列与计划、浏览器浮条、批量与四步抓取向导、异常恢复诊断、托盘与悬浮篮。
- **操作语义严格对齐**：
  - 键盘 `Delete` 默认仅调用 `ControlOp::RemoveRecord`（移除列表记录，绝不隐式删除磁盘文件）；仅 `Shift+Delete` 或勾选删除文件时调用 `ControlOp::DeleteFiles`。
  - 关闭主窗口默认最小化到系统托盘（`close_to_tray = true`），不中断后台传输；显式退出执行受控停止与检查点保存。

Desktop Dashboard v0.1.0 是首个公开的 Windows 11 x64 便携版本。

## 主要功能

- Memo、Todo、Countdown、Codex Usage、Codex Reset 以独立窗口运行
- 支持每个组件独立移动、调整大小、缩放、启用和关闭
- 系统托盘统一管理显示、隐藏、设置、布局锁定和退出
- 提供 4 个深色主题、4 个浅色主题以及颜色和透明度设置
- 按显示器拓扑、分辨率、DPI、排列和主屏幕保存独立布局
- 使用 SQLite 持久化，并支持自动备份、立即备份及 JSON 导入导出
- 可配置显示桌面后的自动渐进显示
- Codex Usage 显示 primary/secondary 剩余额度、重置时间和 banked reset credits
- Codex Reset 对非官方公开数据进行容错解析和状态展示

## 安装

下载 `Desktop-Dashboard-0.1.0-windows-x64-portable.zip`，解压后运行 `Desktop Dashboard.exe`。程序不显示任务栏按钮，管理入口位于系统托盘。

ZIP 的 SHA256：

```text
047A1252C6BCE60EE2A5EAA5271BB3445D391AEC20D4CB998C7BEA409AD75A9F
```

## 已知问题

- 开机自启动有待复现和验证的异常报告。
- 显示配置布局需要继续覆盖更多多屏、投屏和远程连接组合。
- UU 远程连接刷新后偶尔可能改变窗口层级，目前难以稳定复现。
- Explorer 在系统卡顿后重启时，透明区域偶尔可能暂时显示为黑色矩形。
- Codex Reset 依赖非官方网络数据源，数据源不可用时仅影响该组件。

## 验证

- 前端测试：19 项通过
- Rust 测试：43 项通过，1 项需要真实服务的测试默认忽略；该项已单独通过
- 前端生产构建、Debug 和 Release 原生启动冒烟检查通过

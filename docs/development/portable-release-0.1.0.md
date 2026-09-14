# Codex Beacon 0.1.0 便携 ZIP 发布记录

日期：2026-09-14

## 实现方式

- `scripts/New-PortableRelease.ps1` 从三个工程清单核对版本一致性，调用正式 Release 构建，然后仅复制嵌入前端资源的 `desktop-dashboard.exe`。
- 打包目录加入 `便携版说明.txt` 和 `VERSION.json`。版本清单记录目标平台、应用标识、构建时间以及 EXE SHA-256。
- PowerShell `Compress-Archive` 生成 ZIP，旁边生成同名 `.sha256` 文件。
- 用户数据库、自动备份、运行日志、Codex 数据、PDB、源码和开发依赖不进入包。
- 便携表示无需安装即可启动；用户数据继续保存到 `%APPDATA%\local.desktopdashboard.widgets`，不会跟随 ZIP 移动。

## 产物

- `release/Codex-Beacon-0.1.0-windows-x64-portable.zip`
- `release/Codex-Beacon-0.1.0-windows-x64-portable.zip.sha256`
- ZIP SHA-256：`7091E06910DC0B26766EDA2B1D46471D6521856DF9CA41B23AE602C32E9D9165`
- 包内程序：`Codex-Beacon.exe`，10,287,616 字节

## 验证结果

- Node 测试：19 项通过，0 项失败。
- Rust 测试：43 项通过，0 项失败；1 项真实网络测试默认忽略。
- TypeScript 检查、Vite 生产构建和 Cargo Release 优化构建通过。
- Release 原生烟测通过：独立组件渲染身份、事件隔离、移动/调整大小、SQLite 建库和干净退出均满足自动断言。
- ZIP 可正常读取，共 3 个文件；版本清单中的 EXE 哈希匹配实际文件，ZIP SHA-256 文件匹配实际 ZIP。
- 禁止内容检查为 0：没有数据库、WAL、日志、实例锁、PDB、`auth.json` 或用户内容。

烟测退出时 WebView2 输出一次 `Failed to unregister class Chrome_WidgetWin_0 (1412)`，进程仍以成功状态退出且应用日志包含干净退出断言。本记录不把该 Chromium 清理信息判为发布阻断项。

## 限制

- 当前产物未进行代码签名，其他电脑首次运行时可能出现 Windows SmartScreen 提示。
- 仅完成当前 Windows 11 x64 主机的自动检查；主题、布局、备份文件对话框等视觉与交互项目仍以对应 Validation 的人工状态为准。
- 本包依赖目标 Windows 上的 WebView2 Runtime，不附带运行时。

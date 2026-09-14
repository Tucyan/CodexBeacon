# Desktop Dashboard

Windows 11 独立桌面组件。每个启用的 Memo、Todo、Countdown、Codex Usage、Codex Reset 使用自己的窗口；统一托盘、设置、SQLite 和 Provider。正式范围见[最终开发计划](最终开发计划.md)。

## 开发与启动

已配置 Rust MSVC、C++ Build Tools/SDK、Node。依赖使用已缓存版本；Cargo 配置 rsproxy，npm 配置 npmmirror。rustfmt/Clippy 可选，尚未安装。

```powershell
& .\scripts\Build-DesktopDashboard.ps1
& .\scripts\Start-DesktopDashboard.ps1
```

构建默认离线；只有缺缓存并需要联网时显式使用 `-Online`。脚本不循环重试。首次 Release 使用 `-Release`，可能需要较长编译时间。开发调试可运行 `npm run tauri -- dev`。

默认只启用 Memo/Todo。通过托盘打开设置，可开启其余组件、锁定布局、调整缩放与主题。关闭组件等同隐藏；隐藏与禁用保留内容。数据库位于应用数据目录 `local.desktopdashboard.widgets/dashboard.db`，不写入 Codex 凭据。

“显示桌面时自动出现”使用已验证的 E 组策略，首次可能短暂消失再出现；可选择“跟随系统”，渐显可独立关闭。此处仍需正式 Tauri 版本的人工集成验收，不能用旧 PoC 通过代替。

## 检查

```powershell
npm test
npm run build
cargo test --offline --manifest-path src-tauri/Cargo.toml
```

## 便携 ZIP

```powershell
& .\scripts\New-PortableRelease.ps1
```

脚本核对 `package.json`、`tauri.conf.json` 与 `Cargo.toml` 的版本号，执行离线 Release 构建，并在 `release/` 生成 Windows x64 便携 ZIP 及其 SHA-256 文件。若 Release EXE 已由同一源码构建，可用 `-SkipBuild` 只重新归档。

便携包不包含用户数据。程序仍使用 `%APPDATA%\local.desktopdashboard.widgets` 保存数据库和自动备份；跨设备迁移应使用设置中的 JSON 导出/导入。

`docs/development/` 保存实现契约和本轮报告，根目录 `可行性验证.md` 是可行性总表。`experiments/` 保存原有 PoC，不是正式源码；构建暂复用其中的 Cargo target 缓存，不覆盖 PoC 可执行文件。

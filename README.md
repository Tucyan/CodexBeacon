# Desktop Dashboard

Desktop Dashboard 是面向 Windows 11 的轻量桌面组件程序。Memo、Todo、Countdown、Codex Usage 和 Codex Reset 均以独立窗口存在，可分别移动、缩放、调整大小和启用或关闭，无需依附统一背景板。

## 下载与运行

当前版本为 **v0.1.0**，支持 Windows 11 x64。

1. 从 [GitHub Releases](https://github.com/Tucyan/DesktopDashboard/releases/latest) 下载 `Desktop-Dashboard-0.1.0-windows-x64-portable.zip`。
2. 解压到任意可写目录。
3. 运行 `Desktop Dashboard.exe`。
4. 程序不会显示任务栏按钮，请通过系统托盘打开设置、显示或隐藏组件、锁定布局和退出程序。

可在 PowerShell 中校验下载文件：

```powershell
Get-FileHash .\Desktop-Dashboard-0.1.0-windows-x64-portable.zip -Algorithm SHA256
```

发行页同时提供对应的 `.sha256` 文件。Windows 需要可用的 Microsoft Edge WebView2 Runtime；Windows 11 通常已预装。

## 功能

- Memo、Todo、Countdown、Codex Usage、Codex Reset 独立组件窗口
- 无边框透明窗口，不显示在任务栏，统一由系统托盘管理
- 组件独立移动、调整大小、缩放、启用和关闭
- 布局编辑与锁定模式，锁定后仍可编辑备忘录和操作待办
- 4 个深色主题和 4 个浅色主题，并支持自定义颜色与背景、内容透明度
- 按显示器拓扑、分辨率、DPI、排列和主屏幕分别保存布局；首次遇到新显示配置时继承当前布局
- SQLite 状态持久化，以及定时或立即 JSON 备份、导出和导入
- 可配置“显示桌面时自动出现”和渐进显示
- 当前 Windows 用户开机自启动选项
- Codex 额度窗口显示剩余额度、重置时间和 banked reset credits

## 数据与隐私

应用数据默认保存在：

```text
%APPDATA%\local.desktopdashboard.widgets
```

Codex Usage 通过本机 Codex app-server 只读获取账户与额度数据。程序不保存 Codex access token，也不会把凭据发送给第三方。

Codex Reset 使用 `codex-resets.com` 的公开非官方数据。该站点汇总社区对可能发生的全局 reset 事件的观察与预测，因此组件只展示来源状态和可确认的时间信息，不将其表述为 OpenAI 官方公告或确定承诺。

## 已知问题与验证边界

- 开机自启动已有异常报告，当前标记为待验证。
- 显示配置布局已按显示器特征隔离保存，仍需在更多多屏、投屏和远程连接组合中人工验证。
- UU 远程连接刷新后窗口可能短暂出现在普通窗口上方；该现象暂未稳定复现。
- Explorer 在系统长时间卡顿后重启时，透明窗口可能短暂出现黑色矩形；重新显示或重启程序可恢复。
- Codex Reset 依赖非官方网络数据源，站点不可用或响应格式变化时会显示 unavailable/error，不影响其他组件。

问题与建议请提交到 [GitHub Issues](https://github.com/Tucyan/DesktopDashboard/issues)。

## 开发

需要 Node.js、Rust MSVC toolchain、Visual Studio C++ Build Tools 和 Windows SDK。

```powershell
npm install
npm run build
cargo test --manifest-path .\src-tauri\Cargo.toml
```

启动开发版：

```powershell
npm run tauri dev
```

构建正式程序：

```powershell
npm run tauri build
```

生成便携 ZIP：

```powershell
& .\scripts\New-PortableRelease.ps1
```

## 文档

- [最终开发计划](./最终开发计划.md)
- [可行性验证总表](./可行性验证.md)
- [实现契约](./docs/development/CONTRACT.md)
- [开发与验证报告](./docs/development/REPORT-2026-09-05.md)
- [已知问题记录](./docs/development/known-issues-2026-09-14.md)
- [技术决策](./docs/decisions/)

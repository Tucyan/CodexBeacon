# VALIDATION-024 开机自动运行

问题：便携 ZIP 能否在 Windows 当前用户登录后自动运行，并由设置页面安全地启用、关闭和检查状态？

假设：使用 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 可以避免管理员权限；将绝对 EXE 路径作为带双引号的单一命令参数，可支持空格和中文路径。

## 实现方式

- 新增 Rust `autostart` 模块，使用已有 `windows-sys` 的 `Win32_System_Registry` 功能，不下载新的 Tauri 或前端插件。
- 注册表值名固定为 `Desktop Dashboard`。启用写入当前 EXE 的带引号绝对路径，关闭删除该值；重复操作保持幂等。
- `get_autostart_status` 和 `set_autostart_enabled` 仅允许 Settings 窗口调用。前端只接收 `enabled`、`pathCurrent`，不接收实际文件路径。
- 状态以 Windows 注册表为准，不加入 AppSettings、SQLite 设置或 JSON 备份，避免从另一台电脑导入备份时静默改变系统启动项。
- 应用启动时仅在启动项已经存在且目标文件已经不存在时改写为当前 EXE。仍然存在的另一个程序副本不会被 Debug/其他副本覆盖；Settings 提供“改为当前程序”。
- `DASHBOARD_SMOKE_TEST` 模式跳过启动路径刷新，自动测试不会修改真实用户启动项。
- 注册表和当前 EXE 错误转换为固定错误码，前端显示统一中文提示，不泄露本地路径。

## 自动验证

- 前端先建立错误提示断言并观察失败，再实现固定提示；单项转绿。
- Rust 先用 `todo!` 观察4项预期失败，再实现命令引用、路径解析、刷新决策和状态计算；主流程 `cargo test --lib` 38项通过。
- 全部 Node 检查19项通过，TypeScript类型检查通过。
- TypeScript/Vite、Release优化构建和Release原生smoke通过。烟测使用独立数据目录并跳过启动项刷新，不修改真实用户注册表。
- 已将当前用户的 `Desktop Dashboard` 启动项登记为新版便携 EXE；读取后的注册表命令与实际启动进程路径一致。ZIP SHA-256 为 `543FC9F5A37EA0711FBF355540B5A487EAB0B3B1AB3391321C3647A63916B1B7`。

## 人工验收步骤

1. 打开托盘“设置”并进入 Behavior。
2. 勾选“开机自动运行”，确认状态显示从当前程序位置启动。
3. 退出程序并重新登录 Windows，确认 Dashboard 自动启动且只有一个实例。
4. 取消勾选，重新登录后确认不再自动启动。
5. 可选：移动便携目录并删除旧目录，从新目录手动运行一次，确认启动项路径得到刷新。

预期结果：所有操作仅影响当前用户，不弹出管理员授权；开关、重新登录和移动路径行为符合上述规则。

实际结果：自动检查和当前用户启动项登记完成；重新登录属于人工系统行为，等待用户确认。

结论：进行中。

是否通过：否，等待人工确认。

后续方案：用户确认后将 VALIDATION-024 标为可行；若系统策略禁止 HKCU Run，保留明确错误提示，不改用管理员服务或计划任务绕过策略。

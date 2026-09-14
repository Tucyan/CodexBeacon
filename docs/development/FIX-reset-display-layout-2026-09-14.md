# Reset 与显示布局修复记录（2026-09-14）

## Reset API兼容

问题根因：`codex-resets.com/api/v1/status` 的`data`新增`scheduled_reset`，旧解析器要求对象键集合完全相等，因此将HTTP 200响应判为`invalid_response`。

实现方式：解析器改为要求所有消费字段存在且类型有效，同时允许服务端添加未知字段。传给前端的数据仍由白名单重新构造，只包含`latestReset`与`activeWatch`，不展示或误解释`scheduled_reset`。测试先加入新增字段失败用例，再完成实现；另使用与正式程序相同的WinHTTP路径读取真实接口。

## 按显示配置保存布局

问题根因：旧实现仅保存一套组件坐标，并将所有可见窗口的Moved/Resized事件当成用户操作。Windows在分辨率或DPI变化时自动调整窗口，因而可能覆盖用户布局。

实现方式：

- 以显示器名称、物理分辨率、虚拟桌面位置、DPI缩放及主屏标志生成稳定拓扑键；任务栏工作区变化不单独生成拓扑。
- 每个拓扑保存五个组件的位置和尺寸。首次遇到新拓扑时，按新旧可用区域比例映射位置，并限制在完整可见范围；已有拓扑直接恢复。
- 新拓扑连续稳定750毫秒后才切换，切换完成后再隔离750毫秒的系统窗口事件。过渡期间不把Moved、Resized或ScaleFactorChanged写回布局。
- SQLite沿用通用settings表保存活动键和布局集合；JSON备份格式升为v2并继续读取v1。导入后立即切换到当前机器的显示拓扑。

## 验证范围

自动测试覆盖新增API字段白名单、真实WinHTTP读取、拓扑键差异、新布局继承、旧布局恢复、稳定期隔离、SQLite往返以及JSON v1/v2兼容。实际切换显示器后的视觉位置仍需用户确认。

最终自动结果：Rust 43项通过、1项联网测试默认忽略；该联网测试单独运行并使用正式程序同一WinHTTP和解析路径通过。前端19项通过，TypeScript与Vite生产构建通过。Debug及Release原生烟测均通过双独立窗口、事件隔离、SQLite和正常退出检查。Release烟测证据为`docs/development/evidence/smoke-20260914-172016-977`。

便携包：`release/Desktop-Dashboard-0.1.0-windows-x64-portable.zip`，SHA256 `047A1252C6BCE60EE2A5EAA5271BB3445D391AEC20D4CB998C7BEA409AD75A9F`。可执行文件SHA256为`46D81E68CF471CC2B6B931B918E197B53FD67AED56FAD6B83E68FA5DE66B7BD1`。

独立代码审查首轮发现恢复布局未完全限制在工作区、显示追踪状态提交过早；补充失败用例并修复后复审未发现Critical或Important问题。

UU远程层级和Explorer重启后偶发黑底未改代码，仅保留在已知问题文档。开机自启动异常同样只记录为待验证。

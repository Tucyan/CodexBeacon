# Widget unavailable 与拖动/缩放闪烁修复

用户现象：正式版本显示 `Widget unavailable / This widget does not exist.`；移动或调整窗口大小时，在错误页与正常组件之间高频切换。

## 根因

Tauri 2.11.5 的 `WebviewWindow` 直接实现全局 `Emitter`，调用 `window.emit` 不会自动限制收件窗口。原 `broadcast` 虽然按窗口裁剪了 Snapshot，却逐份向所有 renderer 广播。前端 `listen` 默认使用 `Any`，因此 Memo/Todo/Settings 都能收到其他窗口的快照。

前端按 revision 接受首个新快照；如果首个是其他组件的数据，当前窗口找不到路由指定的 widget。移动/缩放会反复发布快照，所以产生正常页面和错误页交替闪烁。渐显及 flush 也使用了同样的全局发送/监听模式，存在串窗风险。

本地 Tauri 源码中 `event/listener.rs::match_any_or_filter` 明确允许 Any listener 接收定向事件。因此仅将 Rust 改成 `emit_to` 不足以解决问题。

## 实现方式

- Rust 的 state、appearance prepare/start/cancel、flush 全部改为 `emit_to(label, ...)`。
- 前端对应 listener 都显式使用当前原生窗口 label 作为 target。
- 保持已有窗口权限、Snapshot裁剪、revision与渐显token校验；不删除数据库，不重置布局，不改变显示桌面策略。
- 新增 `tests/window-events.test.ts`，通过真实 `@tauri-apps/api` 注册逻辑与受控IPC桥复现 Any 监听语义，验证双组件与Settings的数据、渐显、flush隔离。修复前断言失败：每窗口均收到其他窗口事件；修复后通过。
- 增强原生烟测：在独立临时目录中连续12次修改原生窗口位置/大小并广播快照；React提交后回报实际组件DOM标识、快照组件ID和revision。要求两个renderer始终匹配自身，且收到最终revision。只记录ID与revision，不记录业务内容或账户信息。诊断仅在 `DASHBOARD_SMOKE_TEST` 模式启用。

## 验证边界

前一版烟测只证明两个窗口创建、ready握手、可见及正常退出，不能证明收到更新后仍呈现正确组件。本次补上了真实renderer与连续更新检查。用户视觉反馈仍是最终闪烁验收依据。

## 实际结果

- 前端12项测试通过，含新增串窗回归；TypeScript strict及Vite打包通过。
- 离线原生构建13.32秒通过，日志`evidence/fix-window-event-isolation-20260905-010810-407.json`。
- 增强原生烟测通过：`evidence/smoke-20260905-010901-267/`中日志包含`smoke_two_independent_renderers_passed`、`smoke_event_isolation_passed`、`application_exited_cleanly`，没有identity mismatch。期间进行了12次原生位置/尺寸更新与快照广播。
- 二进制SHA256：`8375805610F84F8E58F706FDF89D16D9B63B2A2CD8182BF9D5A1520228D78EB0`。
- 用户已通过托盘退出旧程序；修复后正式程序重新启动，PID119832。未重置正式数据库或布局。视觉闪烁是否完全消失等待用户复测。

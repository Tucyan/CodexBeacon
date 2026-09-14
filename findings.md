# 实验发现

## 2026-09-04 最终决定

用户确认020/021可行，独立组件窗口019跳过专门验证。正式实现每实例独立Tauri窗口，无共同背景板；E策略与可选250ms渐显进入v0.1。下方实验阶段的“019/020仍待”是历史状态，最新范围以根`最终开发计划.md`和`可行性验证.md`为准。015仍需在实现阶段完成。原型机制通过不代表正式Tauri多窗口、SQLite及模式切换代码已经完成。

## 2026-09-04 显示桌面分组最新结论

A基线两入口消失、切回应用后出现；B/C两个置顶样式对照均持续可见；D非topmost相对定位与E临时topmost均通过两入口自动出现和Hide/Show原生验证。用户接受点击D导致层级上移，输入只确认强制置顶应用遮住D场景。E首次进入仍短暂消失后出现，再次切换无闪，用户认为E的Show优于D，因此首选E作为Tauri接入候选，D备选。

最终E日志512条sample、15次原生操作，退出码0；隐藏期间无native可见样本，观测到的退出Active路径均撤销置顶。下文早期“禁止topmost/仅非topmost候选/两入口失败”等属于历史路线，最新技术选择以本段、根目录最终需求与分组报告为准。原生成功不等于Tauri已修复；019/020模式切换、持久化、多组件及输入集成仍待验收。报告：`experiments/show-desktop-groups/REPORT.md`。

## 早期调查记录

- 原开发计划包含 VALIDATION-001–012；本轮补充透明区域输入、焦点、缩放、后台恢复、SQLite 和资源采样。
- 官方 Tauri 窗口 API 包含置底、透明、隐藏任务栏、拖动、resize；实际 Fences 兼容性由用户确认。
- Codex CLI 本机版本 0.147.0；官方 app-server 文档包含 account/read、account/rateLimits/read、account/usage/read。握手需 initialize + initialized。真实账户可用性尚未验证。
- codex-resets.com 提供 /api/openapi.json、/api/v1/status、/api/v1/resets。reset_type 区分 regular/banked；announced_at 是公告/首次观测时间，不能直接认定为所有账户的精确重置时刻。

来源：https://learn.chatgpt.com/docs/app-server 、https://v2.tauri.app/reference/javascript/api/namespacewindow/ 、https://codex-resets.com/api/docs 。

## 实验证据复核

- Codex真实读取取得ChatGPT认证、额度窗口300/10080分钟、reset credit和usage汇总；白名单证据在codex-provider/evidence/real-read.json。客户端边界与生命周期仍在复核。
- SQLite/后台计算12项通过，017仅证明Python sqlite3/SQLite引擎和策略，未证明Rust/Tauri接入。
- Reset官方schema要求source为对象、active_watch为Watch|null，分页为cursor；必须纠正fixture后再标可行。

## 系统工具链安装后复核

- MSVC/SDK 已安装。Rustup 管理器与 Rust 编译器工具链须分开判断：管理器能运行，toolchain list 仍为空。cargo/rustc 是符号链接，仅检查文件存在不足以确认可构建。
- winget 在安装器下载阶段已有超时，最终内部安装失败未保存 stderr；不能仅凭外层退出码推断权限原因或断言具体组件下载失败。后续只补工具链并保留完整日志，用户执行。

## 2026-09-04 最终窗口需求变更

- 用户明确组件应分别独立存在，不依附共同背景板。当前单一 Dashboard Window + Canvas 是旧假设；产品语义应改为“每个启用的 Widget 是独立桌面窗口，Settings/Tray 是统一管理面”。
- 在独立小窗口架构下，窗口外区域天然属于桌面；组件窗口自身矩形内的透明像素不穿透鼠标可以接受，因此013的“不穿透”不再是阻碍，但需由新的独立窗口验证覆盖实际命中范围。
- 用户要求设置项控制“显示桌面”：跟随系统时组件随 Win+D/任务栏最右按钮隐藏；“显示桌面时自动出现”模式允许组件短暂消失，随后无焦点恢复。当前PoC实测会被隐藏/最小化，原计划“不处理Win+D”已被本次需求覆盖。
- Microsoft公开API可检测最小化状态/事件：WM_SIZE(SIZE_MINIMIZED)、IsIconic、EVENT_SYSTEM_MINIMIZESTART/END与SetWinEventHook；ShowWindow/ShowWindowAsync可恢复窗口，SW_SHOWNOACTIVATE可无激活显示。公开资料未提供“将普通置底窗口从Show Desktop中排除”的专用标志。
- Microsoft Q&A中同类桌面Widget问题指出WS_EX_TOOLWINDOW仍受Show Desktop影响；Explorer内部消息/进程Hook方案依赖未公开实现且不保证跨版本。WorkerW/Progman桌面嵌入同样不是公开稳定契约，继续保留为最后备选，不直接进入正式架构。
- Tauri 2可按唯一label创建多个`WebviewWindow`，并按窗口label分配capability。最小最终结构为Rust/AppHandle协调层、每个Widget一个透明置底窗口、一个普通Settings窗口；托盘和共享状态由协调层管理。
- 单窗口的纯布局函数与工具链证据仍有效；原生启动、资源基线以及置底、任务栏、锁定、托盘、Fences等行为必须在多窗口结构重新测量。组件矩形内部透明边距仍可能拦截鼠标，因此窗口尺寸需贴合内容。
- 当前注册的最小化WinEvent只能报告最小化/恢复，不能可靠区分Win+D、任务栏Show Desktop和其他外部最小化来源；SetWinEventHook本身也支持注册前台等其他事件。由于Widget无任务栏入口和最小化按钮，PoC可用“期望可见且非应用主动Hide”作为恢复门控；托盘Hide必须优先。
- 用户最新纠正：Win+D也异常，先前“正常”为观察误差。两入口采样均出现桌面移至组件之前、组件非最小化/cloak0；原版只监听最小化且维持绝对置底，无法处理此状态。取消置底会使用HWND_NOTOPMOST，微软文档说明它对已非topmost窗口无效，因此单独取消标记未必改变层级。当时修订草稿选择非topmost相对定位，不修改Explorer窗口；其可行性尚未验证。
- 2026-09-04 后续开源调研：普通 topmost 不保证 Show Desktop 豁免。OpenPets 组合禁止最小化和重设置顶，VPet/Shimeji 的所读窗口源码未提供专门恢复机制。Rainmeter PR #413 专门修复两种入口；固定提交 d1aa92ad252eb592aff934d646e1751003993f20 的 OnDesktop 路径通过桌面宿主识别、前台事件和定时器检测状态，在显示桌面期间使用临时topmost/helper定位，离开后置底。此新候选涉及放宽草稿“始终非topmost”的实现约束，不代表用户已修改产品需求或候选已通过。本轮仅研究，见 experiments/reports/VALIDATION-020-开源方案调研.md。

来源：https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow 、https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos 、https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook 、https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants 、https://learn.microsoft.com/en-ie/answers/questions/2127546/how-to-avoid-minimize-on-show-desktop-or-peek-desk 、https://v2.tauri.app/learn/security/capabilities-for-windows-and-platforms/ 、https://v2.tauri.app/reference/javascript/api/namespacewebviewwindow/ 。

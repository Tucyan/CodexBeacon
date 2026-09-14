# Windows Desktop Dashboard v0.1 最终开发计划

> **供执行 agent 使用：** 后续按本计划分阶段执行，使用 `subagent-driven-development` 或 `executing-plans` 技能管理任务。已约定主agent、subagent与用户并行：主agent统筹和集成，subagent承担独立模块，用户确认视觉交互。复选框用于记录正式开发进度。

**目标：** 在Windows 11上实现可与Fences共存的独立桌面组件应用，提供Memo、Todo、Countdown、Codex Usage和Codex Reset。

**架构：** 每个Widget实例对应一个独立Tauri原生窗口，位置、尺寸、显示状态和缩放分别管理，不存在共同Dashboard宿主窗口或背景板。一个Rust主进程协调窗口、托盘、设置、SQLite和共享Provider；React仅渲染本窗口内容及设置界面。

**技术栈：** Tauri 2、React、TypeScript strict、Rust/Tauri Commands、SQLite；轻量React状态管理，必要时采用Zustand。

**版本与依据：** 2026-09-04最终范围基线。依据用户本轮原计划全文、此前确认需求及实验记录。本轮只敲定计划和记录验收，不创建正式应用。已验证的本机依赖版本是实施起点，正式初始化时锁定依赖；不将版本号解释为长期兼容保证。

---

## 1. 需求与范围

软件长期展示桌面信息，不管理桌面图标，不替代Fences。保留无边框、透明、隐藏任务栏入口、拖动、resize、布局锁定、主题、背景/内容独立透明度、全局/组件缩放、组件启用/禁用和持久化。

**独立组件窗口是必须交付的产品形态：** Memo与Todo等组件可以分别放到桌面任意位置、分别调整大小；移动一个不会平移其他组件。各组件仅绘制自己的卡片背景，组件之间的空白完全属于桌面/Fences，不创建连接它们的透明大窗口。

一个应用实例统一管理所有窗口；WebView2渲染进程和自有Codex app-server子进程属于正常运行结构，不要求操作系统总进程数等于1。

v0.1不做：多显示器适配、混合DPI/跨屏专项适配、Explorer注入或修改窗口树、Fences API或Hook、云同步、第三方Widget安装、Marketplace、移动端、Linux/macOS、ChatGPT Plus统一剩余额度、自动或手动消耗Codex reset信用、复杂通知、复杂Markdown编辑器、复杂项目管理、自动避让布局。允许组件重叠。

`Win+D`和任务栏“显示桌面”的两种设置模式已纳入v0.1，删除原“不处理Win+D”的排除项。不追求系统显示桌面动画期间完全不消失，不扩展为锁屏、虚拟桌面或全屏场景常驻保证。基础坐标转换必须在当前显示缩放正确；这不扩大为多屏DPI专项。

## 2. 原计划逐节对照

原始文本保存在[原计划存档](../../../experiments/environment/original-plan.md)。下表覆盖原37节，冲突处以本最终计划为准。

| 原章节 | 决定 | 最终安排 |
| --- | --- | --- |
| 1 项目背景 | 保留 | Windows 11桌面信息组件，与Fences共存 |
| 2 核心目标 | 修订 | “窗口/内部组件自由布局”统一为独立组件窗口自由布局 |
| 3 明确不做 | 修订 | 仅移除Win+D排除项；其他范围限制保留 |
| 4 技术栈 | 保留 | Tauri 2 + React/TS + Rust + SQLite |
| 5 总体架构 | 替换 | Widget Window Manager管理多个组件窗口，没有Dashboard Canvas |
| 6 架构原则 | 替换/保留 | 删除单宿主限制；保留不注入Explorer；不用固定alwaysOnBottom强制层级 |
| 7 Widget系统 | 保留并扩展 | Registry与实例ID保留；每实例映射独立window label |
| 8 配置 | 保留 | 主题颜色、颜色选择器和HEX规范化 |
| 9 透明度 | 修订 | 删除共同Dashboard背景；卡片背景/内容分层；渐显为独立临时乘子 |
| 10 缩放 | 明确 | 继承时组件倍率=1；自定义倍率仍与globalScale相乘 |
| 11 布局模式 | 修订 | 锁定/编辑作用于各组件窗口，不再存在整板拖动/resize |
| 12 数据模型 | 扩展 | SQLite保存每实例窗口几何、可见意图和业务数据 |
| 13 Provider分层 | 保留 | UI→Service→Provider；所有窗口共享数据服务 |
| 14 Codex Usage | 以证据定字段 | 只读stdio app-server；窗口时长、额度桶及可选usage按实际响应展示 |
| 15 Codex Reset | 明确语义 | Public API优先；公告/首次观测时间不代表个人账户生效时间 |
| 16 验证机制 | 保留并接受豁免 | 未知事实建验证项；用户明确跳过019，不再安排其独立实验 |
| 17 强制可行性 | 已完成适用部分 | 使用既有experiments证据，不重建一套PoC目录 |
| 18 Validation Gate | 更新 | P0已有可行证据；012旧恢复阻塞随020通过解除；015留在实现阶段 |
| 19 工程初始化 | 待执行 | 正式src/src-tauri与experiments隔离 |
| 20 Window Shell | 替换 | 直接构建独立组件窗口壳、普通Settings与全局Tray |
| 21 Widget Runtime | 修订 | Registry→实例→窗口Host，不是Canvas内坐标管理 |
| 22 Persistence | 修订 | 无Dashboard总位置/尺寸；保存各窗口与全局设置 |
| 23 Settings | 扩展 | 四页保留，加入显示桌面模式与渐显开关 |
| 24 基础Widget | 保留 | Memo→Todo→Countdown |
| 25 Codex Widgets | 保留并约束 | 真实用量字段；全局Reset与个人reset分开展示 |
| 26 Provider容错 | 保留 | 六种状态、可重试、缓存陈旧标记、错误隔离 |
| 27 视觉交互 | 保留/扩展 | 极简低干扰；渐显为明确要求的可选动画 |
| 28 集成测试 | 扩展 | 独立窗口联动、两种显示桌面模式、渐显取消与退出清理 |
| 29 并行开发 | 保留 | 共享模型先定，主agent集成，用户负责视觉反馈 |
| 30 新增实验规则 | 保留 | 新的具体不确定性建新编号；不换名重建已豁免019 |
| 31 决策记录 | 修订 | 用“独立Widget窗口”决策替换单Dashboard窗口决策 |
| 32 代码质量 | 保留 | strict TS、显式错误、集中类型、可追踪migration、小模块 |
| 33 Security | 保留 | 不读/上传凭据、不存token、不记录原始账户响应 |
| 34 最终验收 | 重写 | 见第12节，不再验收共同Dashboard拖动/尺寸 |
| 35 优先级 | 保留顺序 | 窗口稳定→Runtime→持久化/设置→业务→Provider→视觉→发布 |
| 36 首项任务 | 阶段迁移 | 可行性阶段已有成果；下一步初始化正式工程与共享模型 |
| 37 最重要原则 | 保留 | 稳定窗口 > 可维护架构 > 可靠数据 > 功能数量 > 特效 |

## 3. 可行性结果与开发准入

当前总表见[可行性验证](../../../可行性验证.md)。

| 项目 | 当前决定 | 对正式开发的影响 |
| --- | --- | --- |
| 001–008、013、014 | 基础窗口/交互已有用户通过反馈 | 保留能力证据，正式独立窗口做正常集成验收 |
| 009–011 | Codex只读、子进程生命周期及公开Reset API可行 | 可移植Provider，不能照搬实验脚本当生产代码 |
| 012 | 先前常规托盘确认 + D/E Hide/Show反馈 + 本次020通过 | 历史托盘/恢复可行性证据合并关闭；多窗口产品的Tray/Settings/Lock/Quit仍在P2/P7验收 |
| 015 | 缩放与坐标进行中，未获用户通过反馈 | 在正式窗口/Runtime阶段完成，交用户验收；不把它写成已通过 |
| 016–018 | 时间算法、SQLite策略及短时资源/清理证据可行 | Rust集成、真实唤醒及Release长期资源仍须验证 |
| 019 | 用户要求跳过独立可行性验证 | 不再排期、不阻塞正式开发；架构要求照常实现；不是实测通过 |
| 020 | 用户本轮明确确认通过 | 采用E组显示桌面策略进入正式实现 |
| 021 | 用户本轮明确确认通过 | 采用可选250ms渐显进入正式实现 |

总表仍使用原定四种状态；019列“未开始/无需人工确认/—”，备注“已跳过”，从待办实验中排除。当前22项中20可行、015进行中、019跳过。

**允许开始正式工程初始化。** 此处P0指原计划的001/002/003/005/006/007/009/012基础可行性门槛，不指正式产品验收。无需重新执行001–021，不再以019为前置门槛；正式应用的验收不等于再次做独立可行性实验。020模式切换、Tauri动画合成、SQLite/Rust通道等尚未实现的产品功能照常交付和测试，不把用户的“可行”确认写成这些代码已完成。

## 4. 正式窗口架构

```text
一个应用实例 / Rust主进程
├─ WidgetWindowManager
│  ├─ widget:<instance-id> → Memo原生窗口 → React WidgetHost
│  ├─ widget:<instance-id> → Todo原生窗口 → React WidgetHost
│  ├─ widget:<instance-id> → Countdown原生窗口 → React WidgetHost
│  ├─ widget:<instance-id> → Codex Reset原生窗口 → React WidgetHost
│  └─ widget:<instance-id> → Codex Usage原生窗口 → React WidgetHost
├─ DesktopVisibilityController（共享检测、层级、渐显与可见意图）
├─ Settings Window（普通设置窗口）
├─ Tray（全部显示/隐藏、Settings、Lock/Unlock、Quit）
├─ Persistence（SQLite与保存队列）
└─ Shared Services
   ├─ CodexService → 自有stdio app-server
   └─ CodexResetService → 无凭据公开HTTP API
```

实例ID持久稳定；实际Tauri label由统一函数从ID生成，使用适配Tauri标签规则的安全字符串，例如`widget-<uuid>`，不直接拼接用户名称或图示中的冒号。每个实例创建且仅创建一个窗口；重复Show使用现有窗口，不产生重复实例。

组件窗口默认无边框、透明、无任务栏入口。Settings是普通管理窗口，不参加组件置底、自动恢复或渐显。托盘统一管理；关闭Settings不退出应用；关闭组件视为主动隐藏，内容与布局保留；只有Quit结束应用。

普通状态下组件默认在普通应用下方；用户主动点击允许其上移，已被用户接受。禁止通过永久置顶覆盖普通应用来模拟桌面常驻；也不启用框架固定`alwaysOnBottom=true`与动态恢复策略互相争夺层级。

组件窗口矩形贴合自身内容。窗口内部透明像素可以不穿透；窗口外无额外拦截区域。原生窗口之外不会设置一个透明Canvas承接拖动。

### 显示桌面行为

| 设置值 | 正式行为 |
| --- | --- |
| `auto`，界面“显示桌面时自动出现”，默认 | Win+D/任务栏按钮进入显示桌面时，恢复期望可见组件；可短暂消失，恢复不抢焦点；退出后撤销临时置顶并恢复默认层级 |
| `follow-system`，界面“跟随系统” | 应用不执行自动置顶/恢复，不清除用户可见意图；显示桌面时由系统隐藏/遮住组件，再次切换时应由系统恢复原可见状态和布局 |

采用E组原理：自有隐藏参照窗 + 只读桌面宿主层级，统一检测Active/Inactive/Unknown；只操作本应用窗口。D组非topmost相对定位保留为有证据的备选，不同时叠加多套纠正循环。禁止Explorer私有消息、注入、修改宿主窗口树、WorkerW/Progman嵌入。

Rust控制器维护每实例的`enabled`、`visibleWanted`及操作版本号。`enabled=false`时不创建新窗，已有窗口保存状态后销毁，业务数据保留；Hide/关闭使`visibleWanted=false`。自动恢复只处理两者均为true的实例。托盘Show显式恢复全部已启用组件，托盘Hide隐藏全部；禁用组件不会被Show启用。用户在Settings重新启用组件是明确显示操作，将`enabled`与`visibleWanted`同时设为true。

`visibleWanted`只由明确用户操作或持久化恢复改变。系统显示桌面造成的临时隐藏、桌面遮挡、原生visible/iconic/cloaked观测和动画alpha=0都不写回该字段；这些是瞬时运行状态。禁用再启用按同一实例ID重建窗口并恢复其布局和数据。

Hide、禁用、销毁、切换到跟随系统、退出应用均使旧任务失效。模式切换应撤销已有临时置顶；异步回调检查版本和窗口是否仍存在。桌面状态Unknown不推断为Active、不反复争抢层级；检测失效时记录有限诊断并安全撤销本应用临时置顶，不操作Explorer。

只建立一个共享检测器，不按组件各自轮询桌面。以已验证250ms检测基线起步，原生变更在适当UI线程调度，自动操作采用不激活标志；只有发现具体问题才新增验证。前台hook优化不是进入正式开发的必要步骤。

### 渐显与透明度

Settings中“启用渐显动画”默认勾选，持久化，全局作用于组件。时长250ms，缓出；不增加动画时长滑块或大量其他动画。

自动恢复，以及显式Hide→Show时触发；重复Show、正常轮询、同一Active阶段重复纠正和退出显示桌面均不重播。取消勾选立即结束渐显，Hide立即隐藏并取消；过期动画不能把窗口再次显示。

正式渲染使用两层独立样式：背景层应用`backgroundOpacity`，文字/图标/进度条等内容层应用`contentOpacity`。临时`appearanceProgress`从0到1作用于外层合成，结束严格为1；实际背景和内容效果分别为原透明度乘以该进度。不得通过改变整个窗口的永久opacity来代替这两个设置。

先准备不可见的首帧，再执行恢复和渐显。原生C# PoC的`Form.Opacity`证明观感方案，不直接假定其可无改动移植WebView2；Tauri/React端可用外层CSS/动画实现，需验证显示前准备、后台渲染及取消顺序。250ms仅为动画时长，不承诺按键至显示的总延迟或零消失。

## 5. Registry、共享模型与状态所有权

Registry保留五类内置组件；Widget自身只声明内容、默认几何、配置及能力，不负责创建窗口、访问网络、启动Codex或写SQLite。

共享类型集中于`src/types/`，Rust对应传输类型集中于`src-tauri/src/model/`。接口变更由主agent统一维护，序列化命名通过契约测试核对。以下是正式模型要求，不表示当前已存在这些文件：

```ts
import type { ComponentType } from 'react';

type WidgetType = 'memo' | 'todo' | 'countdown' | 'codex-reset' | 'codex-usage';
type ScaleMode = 'inherit' | 'custom';
type ShowDesktopMode = 'auto' | 'follow-system';
type ProviderState = 'idle' | 'loading' | 'ready' | 'offline' | 'error' | 'unavailable';

interface WidgetInstance {
  id: string;
  type: WidgetType;
  enabled: boolean;
  visibleWanted: boolean;
  x: number; y: number;
  width: number; height: number;
  scaleMode: ScaleMode;
  scale: number;
  config: Record<string, unknown>;
}

interface ThemeConfig {
  accent: string; background: string; foreground: string;
  backgroundOpacity: number; contentOpacity: number;
}

interface AppSettings {
  layoutLocked: boolean;
  globalScale: number;
  theme: ThemeConfig;
  showDesktopMode: ShowDesktopMode;
  fadeEnabled: boolean;
}

interface WidgetDefinition {
  type: WidgetType;
  name: string;
  component: ComponentType<{ instance: WidgetInstance }>;
  defaultWidth: number; defaultHeight: number;
  minWidth: number; minHeight: number;
  defaultScale: number;
  defaultConfig: Record<string, unknown>;
}
```

持久化单位统一为96DPI基准的DIP浮点数。`x/y`相对当前工作区左上角，`width/height`为未应用用户缩放的基础尺寸。渲染倍率`r = globalScale × (inherit ? 1 : scale)`，当前显示因子`s = dpi / 96`。原生尺寸为`round(width × r × s)`、`round(height × r × s)`，位置为工作区物理原点加`round(x × s)`、`round(y × s)`。原生resize保存时除回`r × s`，位置只除`s`；不反复把取整后的几何写回基础尺寸。框架若已返回DIP，先统一输入单位，不能再乘一次Windows缩放。

启动按当前DPI转换已保存DIP布局，不用旧物理像素直接恢复；落在工作区外按下述规则约束。015核对当前目标显示缩放下的结果；运行中切换显示DPI、跨屏及混合DPI不是本版新增专项承诺。

更改倍率以组件左上角为锚点，扩大后将可拖动区域约束在当前工作区；恢复过界布局时至少保留可操作入口。最小尺寸由Registry定义。015在该实现阶段检查：缩放后拖动/resize无跳变，布局保存重开一致，窗体之外不残留透明命中区域。

Rust持有全局权威状态、可见意图和保存队列；各WebView通过命令修改、通过带revision快照/事件同步。组件窗口只订阅自身状态和所需Provider，Settings订阅全局。防止窗口之间互相回写旧数据；不用多个localStorage作为独立真相来源。

Locked禁止组件拖动和原生resize，隐藏编辑边界及手柄；Memo编辑、Todo勾选、Countdown操作和Settings仍可用。Edit只在指定手柄开始拖动，八方向resize不得覆盖业务输入控件。没有“拖动整个Dashboard”操作。

## 6. 持久化与设置

SQLite至少包含`settings`、`widget_instances`、`memo`、`todos`、`countdowns`；业务表以实例ID关联，组件扩展配置存`config_json`。禁用/隐藏不删除业务数据。无共同Dashboard尺寸与位置字段。

保存主题、锁定、全局倍率、显示桌面模式、渐显开关，以及各实例启用状态、可见意图、位置、尺寸、倍率模式和配置。时间存UTC时间戳，界面按本地时区显示。

采用已验证的显式事务与schema版本迁移；启动识别未来schema时拒绝写入并展示错误，不悄悄降级。Rust存储层单点写入，迁移失败保留原库。初始migration与后续迁移分文件，不在生产库上执行实验夹具。

变更使用约300ms防抖快照，拖动/resize结束及正常退出flush；保存失败保留待保存状态并显示可重试错误。正常Quit等待保存结果，失败时留在应用提示重试，不能一边报告已保存一边退出。异常强杀允许丢失尚未提交的防抖修改，但已提交内容不得半笔写入。

倒计时每次从绝对目标时间重新计算`max(0, ceil(target-now))`，不依赖累加timer tick；恢复可见、系统唤醒或时钟变化后重算。所有时间参数测试与真实睡眠测试分别记录。

Settings四页：

| 页面 | 内容 |
| --- | --- |
| Components | 五类组件的启用/禁用、单组件显示、继承/自定义倍率、单组件Reset Layout |
| Appearance | Accent/Background/Foreground的Color Picker和HEX；背景与内容两个透明度 |
| Layout | Global Scale、Lock/Edit、全部Reset Layout、两种显示桌面模式、启用渐显动画 |
| Data | Codex CLI/账户/Provider状态、Reset Provider状态、最后成功刷新时间、Retry |

HEX接受`FFFFFF`或`#FFFFFF`，统一存`#FFFFFF`；拒绝非法输入，不把半输入值写入设置。透明度范围0–1。Reset Layout沿用PoC语义，只恢复目标组件默认位置和基础尺寸，保留倍率，不删除Memo/Todo/Countdown，不重置主题、模式或禁用状态；全部Reset只批量执行这些布局规则。使用当前倍率后的窗口仍须按工作区约束保持可操作。

## 7. Provider与业务功能

### Codex Usage

采用Dashboard拥有的直接`codex.exe app-server`子进程，stdio JSONL/JSON-RPC连接；不接管已有Codex应用进程、不依赖发现或连接已有server。先验证可执行路径与版本，再初始化；正常退出关stdin并有界等待，只清理自己创建的子进程。

握手为`initialize`→响应→`initialized`。只读请求白名单为`account/read`（`refreshToken:false`）、`account/rateLimits/read`、`account/usage/read`；`account/rateLimits/updated`是通知，不是要调用的RPC。禁止请求`account/rateLimitResetCredit/consume`及会开始任务或修改账户的操作。

本机Codex 0.147.0已实际返回额度和usage；以本地证据为兼容起点。`rateLimitsByLimitId`可用时优先保留各额度桶，旧`rateLimits`作为兼容路径。5h/weekly按实际`windowDurationMins`标注；若时长不同，显示真实时长。`remainingPercent=clamp(100-usedPercent,0,100)`；缺失字段保持不可用，不显示假0或假100%。

必需界面是可用额度窗口的Used%、Remaining%、Reset time；banked reset取实际响应`rateLimitResetCredits.availableCount`，可规范化为DTO的`resetCreditsAvailableCount`，缺失时不可用。余额/credits不是banked次数。usage已验证summary字段为`lifetimeTokens`、`peakDailyTokens`、`longestRunningTurnSec`、`currentStreakDays`、`longestStreakDays`；不承诺日用量图表，不推断当前会话剩余token。没有登录或方法不支持时保留其他可用数据，显示局部unavailable。

JSONL处理分片UTF-8、请求id关联、通知穿插、空响应、超时、进程退出和stderr排空。默认每60秒单次刷新额度，usage每5分钟；共享服务负责去重，不因窗口数量增加请求。通知只含稀疏更新，作为合并后的刷新提示，不用通知中缺失/null元数据清空上次快照，不把通知当作持续完整账户快照。进程崩溃使未完成请求失败并进入error，用户Retry可重新建立自有连接，不无限重启。

### Codex Reset

只读访问`https://codex-resets.com/api/v1/status`；`/api/v1/resets`保留为读取历史的接口，本版默认仅展示最近事件，不增加历史管理页面。结构以已保存OpenAPI及线上回放证据为起点。

展示最新全局Reset的公告/首次观测日期、本地时间和“多久以前”；`announced_at`不代表所有账户准确生效时间。保留regular/banked事件类型，但这与个人额度窗口reset和个人可用reset信用不同。允许`latest_reset:null`，呈现无事件状态。

默认每15分钟一次GET，使用ETag/If-None-Match和已校验缓存；5秒请求超时，无并发重复请求。304只可使用已有缓存；429遵守Retry-After，其他失败进入对应状态，等待下一正常刷新或用户Retry，不做紧密重试。保留最后成功值并明确标为旧数据，错误响应不能覆盖好缓存。无任何Codex凭据随请求发送。

以上刷新间隔为正式实现初始值，不是已做长期负载实验的结论；根据实际服务返回的缓存/限流约束调整，勿为刷新策略增加无必要设置页。

### 内置业务Widget

- Memo：创建、编辑、自动保存普通文本；无复杂Markdown编辑器。
- Todo：添加、修改、完成、删除；截止日期为原计划可选项，本轮不将其提高为首版硬性门槛。
- Countdown：多条名称/目标时间，显示天数或天/小时；到期归零，不引入复杂通知。
- Codex Usage、Codex Reset：只消费共享服务快照，不在组件代码访问网络或启动进程。

所有Provider统一六种状态`idle/loading/ready/offline/error/unavailable`，并携带最后成功刷新时间、可选缓存和安全错误码。一个Provider错误不导致其他组件白屏。业务数据加载与Provider数据独立。

## 8. 安全与质量要求

不读取、复制、上传`auth.json`；不把Access Token、原始账户响应或原始RPC错误文本写日志；不把Codex Token保存到SQLite；不把账户数据发给Reset站点。凭据归Codex所有，本应用只消费必要的脱敏字段。

Tauri Commands校验实例ID、参数范围与调用窗口权限：组件只能改自身业务/布局，全局管理由Settings/Tray执行；前端不暴露任意shell或文件访问。数据库位于应用数据目录，实验数据与正式数据隔离。

TypeScript strict；Rust显式错误；UI、Runtime、Provider解耦；集中类型；migration可追踪；避免巨大单文件和按组件名层层if/else。资源清理覆盖窗口监听、桌面检测、动画、后台计时、HTTP任务、SQLite保存队列及自有Codex子进程。

多窗口会增加WebView资源开销，正式Release必须记录真实进程树、空闲CPU/内存和运行后变化；不拿短时Debug样本宣称已轻量或长期无泄漏。优化从共享Provider、按需建窗、取消无用刷新开始。

## 9. 文件与模块规划

以下为待创建的正式工程路径，均相对工作区根目录；`experiments/`原样保留为证据，不直接批量复制进正式工程。

```text
package.json / package-lock.json / tsconfig.json / vite.config.ts
src/
  app/bootstrap.tsx              # 根据窗口身份挂载组件或Settings
  runtime/WidgetHost.tsx         # 单窗口内容与错误边界
  runtime/layout.ts              # 缩放、几何换算和编辑手柄规则
  runtime/appearance.ts          # 渐显生命周期及临时进度
  settings/SettingsApp.tsx       # 四页设置入口
  settings/{Components,Appearance,Layout,Data}.tsx
  widgets/registry.ts           # 定义与默认值注册
  widgets/{memo,todo,countdown,codex-reset,codex-usage}/
  components/                   # 颜色输入、状态提示、手柄等共享UI
  stores/                       # 后端快照订阅，不承担持久化真相
  services/bridge.ts             # 类型化命令、事件和revision
  types/{widget,settings,provider}.ts
  utils/{color,time}.ts
src-tauri/
  tauri.conf.json / capabilities/ / Cargo.toml / Cargo.lock
  src/model/                    # 与TS对应的DTO和校验
  src/window/{manager,desktop,visibility,geometry}.rs
  src/storage/{mod,migrations,writer}.rs
  src/codex/{process,protocol,provider}.rs
  src/reset/{client,model,provider}.rs
  src/tray/mod.rs
  src/commands/                 # 明确权限边界的命令
  migrations/001_initial.sql
docs/
  architecture.md
  validation/README.md          # 链接总表及experiments，不重造已有证据
  decisions/
    001-independent-widget-windows.md
    002-public-desktop-visibility.md
    003-owned-readonly-codex-provider.md
    004-widget-registry-and-state.md
    005-opacity-and-optional-fade.md
```

决策记录写清问题、选择、原因、替代方案及影响。首次初始化记录现有依赖是否可复用；需要下载时优先可信国内镜像并锁定版本，不把实验目录中偶然可用的node_modules当作正式依赖声明。

## 10. 分阶段实施与检查点

本节是产品级开发安排，执行时按当前模块细化可运行的小任务；不在敲定范围时预写整套未经编译的应用代码。检查点通过后再进入依赖它的阶段。

### P1：工程初始化与共享契约

- [ ] 创建上述正式目录、Tauri/React工程配置、锁文件和strict TS配置；记录构建/运行命令。
- [ ] 定义第5节DTO、窗口label规则、命令/事件及revision语义，Registry注册五类组件定义。
- [ ] 建立最小错误边界和安全命令边界，编写并实际核对TS↔Rust序列化契约。
- [ ] 创建架构与五份技术决策记录，链接已有验证证据。

涉及：根工程配置、`src/types/`、`src/services/bridge.ts`、`src/widgets/registry.ts`、`src-tauri/src/model/`、`docs/`。交付：能启动空壳与Settings，共享协议明确。尚不把完整业务功能标为完成。

### P2：独立Window Shell与Widget Runtime

- [ ] 实现manager按实例创建/复用/隐藏/销毁窗口，绑定实例身份；直接在正式应用使用两个最小内容组件完成开发联调，不另建019 PoC或前置门槛。
- [ ] 实现每窗口拖动、八方向resize、Lock/Edit和工作区约束；业务输入不受锁定影响。
- [ ] 实现全局×组件缩放与正确几何转换；用缩放前后拖动/resize/重开场景完成015。
- [ ] 实现Tray与E策略控制器，接入两种显示桌面模式、Hide优先及取消规则；先交付直接恢复路径，再接入可选渐显。

涉及：`src/runtime/`、`src-tauri/src/window/`、`src-tauri/src/tray/`、`tauri.conf.json`。交付：组件无共同背景板、独立位置/尺寸、全局托盘可管理，用户确认实际交互。技术验证采用可见意图/版本状态断言与原生检查，真实视觉不以窗口flag替代。

### P3：SQLite与Settings

- [ ] 建首版schema及migration runner；用临时真实SQLite数据库检查升级、事务回滚、未来版本拒绝和中文数据往返。
- [ ] 实现单点保存、防抖、结束操作flush和正常Quit保存失败处理，测试提交前后异常退出的差异。
- [ ] 实现四页Settings、HEX规范化、独立透明度、倍率、Reset Layout、显示桌面模式和渐显持久化。
- [ ] 验证多窗口快照同步、隐藏/禁用不丢内容、重启位置/尺寸/设置恢复。

涉及：`src-tauri/src/storage/`、`migrations/001_initial.sql`、`src/settings/`、`src/stores/`、`src/utils/color.ts`。交付：完整窗口设置持久化；不继续使用PoC localStorage保存正式状态。

### P4：Memo、Todo、Countdown

- [ ] 依次实现Memo文本创建/编辑与自动保存、Todo增改完成删除、Countdown多项目标时间。
- [ ] 业务数据关联实例，禁用再启用后数据保持；错误仅影响对应组件。
- [ ] 倒计时用绝对时间重算，测试时间跳变、到期和隐藏后的恢复；实际睡眠/唤醒留真实测试记录。

涉及：三个业务Widget目录、对应Commands与SQLite访问。交付：三个实际组件可独立使用，不依赖Codex或网络。

### P5：两个Provider与Codex Widgets

- [ ] Rust移植Codex JSONL解析/请求关联/只读白名单/生命周期；用本地fixture验证分片、通知、退出、超时、未知字段和缺失指标。
- [ ] 实现Reset HTTP、schema校验、ETag缓存、Retry-After与错误映射；使用本地HTTP fixture注入故障。
- [ ] Codex/Reset共享服务接入对应Widget与Data页面，真实数据缺失时显示不可用，缓存显示最后成功时间。
- [ ] 在本机做一次有界真实兼容读取，分别记录模拟与真实结果；不得消耗reset信用、改动登录或启动Codex任务。

涉及：`src-tauri/src/codex/`、`src-tauri/src/reset/`、两个Codex Widget目录及provider DTO。交付：五类Widget齐备，网络/账户错误不影响其他组件。

### P6：视觉交互完善

- [ ] 完成Hover、Edit边界/手柄、主题、空/加载/错误状态与可选渐显；保持极简低干扰。
- [ ] 用户验证组件背景/内容透明度、输入/中文IME、缩放、Fences与两入口实际观感。
- [ ] 核查自动恢复与设置模式切换不抢焦点，动画取消不反弹，正常应用可覆盖组件。

交付：已实现功能的视觉与交互验收，不扩展无必要特效。

### P7：集成、资源与Release

- [ ] 按第12节在Windows 11/Fences环境跑完整流程，记录版本、步骤、日志和人工结论。
- [ ] 完成正常退出、异常Provider退出、重启、实际唤醒与配置恢复；确认无自有残留子进程。
- [ ] 构建Release并采样五类组件启用时的进程树/空闲资源，记录长时间运行前后差异及已知限制。
- [ ] 产出可安装/运行包、使用与故障排查说明；本机安装/卸载验收需保留用户数据策略明确，发布或上传另按实际请求执行。

本阶段通过后才将v0.1标为完成；可行性通过不能替代Release验收。

### 验证执行约定

正式初始化时定义`npm run typecheck`（`tsc --noEmit`）、`npm test`（明确测试runner）、`npm run build`（类型检查与Vite构建）与Tauri CLI入口。Rust本地逻辑使用`cargo test --manifest-path src-tauri/Cargo.toml --offline`；尚无本地依赖时先显式处理环境，不通过反复执行偷偷下载。

每项验证记录命令、退出码和适用范围。长时间命令/下载/缺权限操作最多尝试一次，超时交用户执行，未经用户明确提出不得重试；使用可信国内镜像加速必要下载。只为具体风险运行必要测试，不在低影响文档/样式修改上堆叠机械测试。仓库当前未初始化Git，不能假称已有提交；建立版本管理后按实际实现提交。

## 11. 并行协作安排

共享DTO、命令边界与Registry约定由主agent先统一，然后分工作流：

| 工作流 | 所有权 | 依赖与交付 |
| --- | --- | --- |
| A 窗口/Windows | window/、tray/ | 使用共享状态接口；交付独立窗口与桌面行为 |
| B Runtime | runtime/、registry.ts | 依赖DTO与窗口命令；交付布局、缩放、渐显渲染 |
| C 存储/设置 | storage/、migrations/、settings/ | 共享模型固定后启动；提供业务读写与设置同步 |
| D 基础组件 | memo/todo/countdown | 存储接口固定后接入；独立于两个外部Provider |
| E Codex | codex/、codex-usage | 协议fixture与真实兼容证据分开 |
| F Reset | reset/、codex-reset | HTTP fixture与公开API读取独立 |

按实际agent槽位分批，不要求六组同时启动；不同agent不同时修改同一契约文件。主agent负责架构决策、根验证表、审查与集成；用户负责视觉/交互成果反馈。不得让各组件各自定义一套状态或Provider生命周期。

## 12. v0.1最终验收清单

以下均为**正式产品待完成项**，不因PoC通过预先勾选。

### 窗口与桌面

- [ ] Windows 11正常启动/退出/重启，与Fences同时运行；不管理Fences图标。
- [ ] 每组件独立窗口、无共同背景板，无边框/任务栏入口、无异常黑底；记录Alt+Tab/任务视图表现（不是首版硬阻断）。
- [ ] 组件可独立移动/resize，互不改变位置尺寸；普通应用可覆盖组件，主动点击上移按已接受口径处理。
- [ ] 组件窗口外点击/右键/框选/Fences正常；内部透明像素不穿透属于已接受限制。
- [ ] Lock禁止布局改变但保留文本/勾选/倒计时输入；Edit手柄不误触业务操作。
- [ ] Tray Show/Hide/Settings/Lock/Unlock/Quit稳定；禁用组件不被自动恢复。

### 显示桌面与渐显

- [ ] 自动出现模式分别通过Win+D和任务栏最右按钮，允许短暂消失；恢复不抢焦点，退出桌面状态撤销临时置顶。
- [ ] 跟随系统模式由系统隐藏/恢复原可见状态和布局，不改写用户可见意图；运行中切换模式取消旧任务，不留下永久置顶窗口。
- [ ] 渐显默认开启、约250ms；开关关闭直接显示、设置重启保留，重复Show/退出桌面不重播。
- [ ] 动画中Hide/禁用/关闭/模式切换无反弹；最终背景和内容仍是各自设置值。

### 布局、主题与业务

- [ ] 单组件及全局缩放、resize、坐标和重启恢复一致，完成015人工确认。
- [ ] 单组件/全局Reset Layout不删除业务内容、不误改主题或启用状态。
- [ ] Accent/Background/Foreground颜色选择器和HEX输入正确；背景与内容透明度独立。
- [ ] Memo、Todo、多个Countdown正常增改和保存；五类Widget可启用/禁用并恢复状态。
- [ ] 截止时间、个人reset时间、全局事件时间按各自语义显示，本地时区与相对时间正确。

### 数据、安全与可靠性

- [ ] Codex正常/未安装/未登录/方法不可用/进程崩溃均有局部状态；只清理自有app-server。
- [ ] Codex窗口时长、额度桶及可选字段按真实值显示，不模拟缺失数据，不调用reset消费接口。
- [ ] Reset正常、离线、超时、429/503、无事件、损坏响应和304缓存均正确处理。
- [ ] SQLite迁移、事务、保存失败、正常退出flush和重启恢复通过；日志无原始账户或凭据。
- [ ] 实际长时间运行和唤醒后行为正常，Release资源有记录，退出无自有残留进程。

## 13. 关键证据索引

- [基础窗口报告](../../../experiments/window-poc/REPORT.md)
- [A–E分组实现与结果](../../../experiments/show-desktop-groups/REPORT.md)
- [可选渐显实现与结果](../../../experiments/show-desktop-fade/REPORT.md)
- [Codex只读数据](../../../experiments/codex-provider/REPORT-009.md)、[生命周期](../../../experiments/codex-provider/REPORT-010.md)
- [Reset API](../../../experiments/reset-provider/REPORT-011.md)
- [SQLite和计时](../../../experiments/storage-runtime/REPORT-016-017.md)
- [环境及资源证据边界](../../../experiments/reports/REPORT-000-018.md)
- [本次需求与验收决定](../../../experiments/reports/2026-09-04-final-plan-decisions.md)

旧Rust相对层级修订草稿未完成正式审查/构建，不能当作已实现的E策略直接投入使用。历史文件中的单窗口、强制alwaysOnBottom或要求019先做PoC的描述保留为历史；执行以本计划和根验证总表为准。

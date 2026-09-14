# 正式工程共享契约（2026-09-05）

根agent拥有src/types/index.ts、src-tauri/src/model.rs及所有顶层配置，修改契约请先联系root。

- 单Rust主进程管理每实例一个原生窗口，label为widget-<id>（内置id memo/todo/countdown/codex-reset/codex-usage）。Settings label=settings。不存在Dashboard Canvas。
- `get_snapshot`命令无参数，返回Snapshot；`dispatch`参数`{action: Action}`，返回新Snapshot。UI先订阅`dashboard-state`和appearance/flush，再get_snapshot，DOM提交后才widget_ready；只采纳revision更高的事件。用户操作以dispatch返回值更新，未知错误只展示安全消息。
- `get_snapshot`可由组件调用；snapshot是无凭据数据，Settings得到全量；root会限制组件权限/数据作用域。Action的settings/widget/reset-layout/retry仅Settings，content/hide允许自身组件，其他组件不可改。
- Action：`{type:'settings',settings:AppSettings}`、`{type:'widget',id,enabled,scaleMode,scale}`（后3字段可选）、`{type:'content',id,content:WidgetContent}`、`{type:'hide',id}`、`{type:'show',id}`、`{type:'reset-layout',id?:string}`、`{type:'retry',provider:'codex'|'reset'}`。
- Window原生位置/尺寸由Rust事件处理，x/y为工作区相对DIP，width/height为未缩放DIP，factor=globalScale×(inherit?1:scale)。拖动/resize由root提供命令`start_drag`和`start_resize`（参数direction字符串Top/Bottom/Left/Right/TopLeft/TopRight/BottomLeft/BottomRight），Rust核对Lock和窗口归属。前端不要直接调用Tauri创建/销毁/设置窗口尺寸API。
- 动画：root调用前端的`dashboard-appearance`事件`{revision:number, animate:boolean}`后用统一CSS进度恢复外观；root后续对显示前准备增加专用握手，frontend需暴露/监听此事件。不修改背景/内容保存值。
- AppSettings：layoutLocked:boolean、globalScale:number、theme:{accent,background,fieldBackground,foreground,mutedForeground:string; backgroundOpacity,contentOpacity:number}、showDesktopMode:'auto'|'follow-system'、fadeEnabled:boolean、backupIntervalDays:number（整数1–365，旧数据默认7）。
- WidgetInstance：id/type:string、enabled/visibleWanted:boolean、x/y/width/height/scale:number、scaleMode:'inherit'|'custom'、config:Record<string,unknown>。
- WidgetContent：memo:string、todos:Array<{id,text:string;completed:boolean}>、countdowns:Array<{id,name:string;target:number}>。target为UTC epoch秒；每实例有独立content记录。
- ProviderSnapshot：state:'idle'|'loading'|'ready'|'offline'|'error'|'unavailable'、lastSuccess:number|null（UTC秒）、error:string|null（安全错误码）、data:JSON|null。Codex data为sanitize快照{account:{authType,planType},rateLimits:{buckets:[{limitId,primary,secondary}],resetCreditsAvailableCount},usage:{summary}}；Reset data为{latestReset:{id,announcedAt,resetType}|null}。不得将原始账户响应/error文本返回UI或日志。
- Snapshot：revision:number、settings、widgets:WidgetInstance[]、content:Record<string,WidgetContent>、providers:{codex:ProviderSnapshot,reset:ProviderSnapshot}。
- Rust模型使用serde camelCase（enum变体按kebab-case），`Snapshot::default()`生成5实例，初始仅memo/todo启用，其余可从Settings开启。
- 存储agent接口：`storage::Store::open(path: &Path)->Result<Store,String>`、`load(&self)->Result<Option<Snapshot>,String>`、`save(&mut self,snapshot:&Snapshot)->Result<(),String>`。只存settings/widgets/content，不存providers；load时providers默认。revision可保存。SQLite真实事务/migration/JSON快照与业务分表，不用JSON文件替代SQLite。
- Provider agent接口：`providers::read_codex(cli:Option<&Path>)->ProviderSnapshot`、`providers::read_reset()->ProviderSnapshot`或提供长期worker对象（需与root约定）；不在UI线程阻塞。先完成可测试解析、通信与受控进程退出，root负责调度/快照广播。无无限重试或reset消费。
- 子agent只能修改分配目录。不得运行Cargo build/fetch、安装依赖或提交，root统一执行；Node纯逻辑测试可运行，长命令最多一次超时交用户。无新PoC/019验证门槛。

2026-09-05 Reset扩展：latestReset增加sourceType（x_post/observed）；data增加activeWatch:null或{level,chancePercent:number|null,forecastWindow,observedAt,expiresAt}。它是AI分类预测，不是投票结果或已发生的重置，前端仅在未过期时显示。Usage默认界面不展示token统计，但Provider仍保留原白名单结构。

## 2026-09-05 主题方案 A

五个基础色由用户编辑；八个主题预设按深色/浅色各四个分组。边框和按钮颜色派生，不新增可编辑配置字段。选预设只替换五个颜色，保留两项透明度；预设匹配依据颜色，不另存主题 ID。旧三色 JSON 缺少的新字段由 Rust 根据原背景和正文派生，新值随正常 SQLite 保存写回，数据库表结构与 schema version 不变。

## 2026-09-05 备份接口

仅 Settings 可调用 `get_backup_status`、`backup_now`、`export_backup`、`import_backup`。备份文档只含 settings/widgets/content，固定 format=`desktop-dashboard-backup`、version=1；禁止加入 providers/revision/凭据。导入先进行32MiB限制、严格反序列化和正式Snapshot校验，再事务保存并广播。自动/立即备份在应用数据 `backups` 下保留最近10份；用户导出文件不参与轮换。

# 配置备份设计与实验记录

日期：2026-09-05。状态：实现和自动检查完成，等待人工操作确认。

## 范围

备份包含 AppSettings、WidgetInstance 列表和 Memo/Todo/Countdown 内容。它包含用户可恢复的主题、行为、布局、可见性和业务内容；不包含运行时 revision、Provider 状态、Codex 账户/额度响应、访问凭据或原始网络响应。

JSON 顶层使用固定格式标识与版本号。导入仅接受支持的版本、完整的固定组件集合和通过现有模型校验的数据；读入设置大小上限。导入验证完成后以 SQLite 事务保存，再替换内存状态和广播，失败时保留当前状态。

## 行为

- 自动备份默认间隔 7 天，可在 1–365 天内调整；启动后及运行期间检查到期时间。
- 自动与“立即备份”写入应用数据目录下 `backups`，保留最近 10 份，文件名不含用户内容。
- 导出通过 Windows JSON 保存对话框选择路径；导入通过 Windows JSON 打开对话框选择文件。
- 用户取消文件对话框是正常结果，不显示为失败。导入前设置页明确确认它会替换现有配置、布局和内容。
- 手动操作先要求所有 Widget 完成待保存内容的 flush，避免遗漏刚编辑的文本。

## 验证计划

覆盖 JSON 往返、敏感 Provider 排除、格式/版本/大小/模型错误拒绝、原状态不变、SQLite 导入保存、周期到期、清理边界和旧 AppSettings 缺少间隔字段的默认迁移。最后执行前端测试与构建、Rust 测试、原生 smoke；视觉与文件对话框交互由用户确认。

## 实现结果

- Rust `backup.rs` 明确定义 BackupDocument/BackupData；最大读入32MiB，严格拒绝未知字段、错误格式和未来版本。导入沿用正式 `normalize_snapshot`，保留当前Provider，仅为导入状态生成新revision。
- 手动操作先触发所有Widget内容flush；导入使用`writes_frozen`阻止并发修改，按既有锁顺序把候选数据写入SQLite，再替换内存状态和广播。前端在调用备份命令前等待设置Action队列清空。
- `lifecycle`在启动后及每小时检查一次；只在普通数据库保存完成、dirty为空时判断到期并写标准备份。标准文件用毫秒时间戳命名，只轮换符合自身命名规则的文件。
- 保存/打开路径使用缓存中已有的`windows-sys 0.61.2` Common Dialog接口；取消返回`cancelled`，不会当作错误。
- 设置新增“备份”标签、1–365天输入、立即备份/导出/导入按钮、最近备份时间与路径。导入按钮先提示将替换设置、布局与三类业务内容。

## 自动检查证据

- 首次Rust检查发现导入闭包错误类型缺少显式标注，修正后34项通过：`evidence/backup-rust-tests-fixed-20260905-151826-838.json`。
- 前端TypeScript、Vite构建及18项Node检查通过：`evidence/backup-frontend-build-20260905-151947-613.json`。
- 离线原生构建通过：`evidence/backup-native-build-20260905-152001-598.json`。
- 原生多窗口smoke通过：`evidence/smoke-20260905-152018-279/result.json`。同一隔离数据目录实际生成1份标准备份；复核为format/version正确、5个Widget与5个content键齐全，且不存在revision/providers字段。
- 正式程序升级启动成功，旧设置自动取得默认间隔7天，并在Roaming应用数据的`backups`目录生成首份2686字节标准备份；只检查结构与字段计数，没有把用户内容输出到报告或日志。进程PID 122428响应正常，二进制SHA256为`FE905153A5C25A081FC998E5D39D5E74D5D5062F16554610ED0E80EC5E2D5E47`。
- 调整最终用户提示后再次执行前端构建与离线原生构建，均通过：`evidence/backup-ui-copy-build-20260905-152258-337.json`、`evidence/backup-final-native-build-20260905-152318-317.json`。最终程序PID 122188响应正常，SHA256为`39E16E46B0378C5FF7427866D4ADB1973FDD7E5067506B7C0782DAEB2975C4A7`。
- 已知既有日志：编译保留未使用Provider代码警告；smoke退出时Chromium记录1412注销窗口类消息，但进程退出码与全部断言通过。

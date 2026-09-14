# 卡片外观与信息精简

用户依据三张截图提出：移除外侧矩形阴影、压缩Codex Usage信息、复核Reset网站语义、修复Memo双滚动条并美化滚动条。

## 实现方式

- 卡片`box-shadow:none`，外层保持透明，不改变窗口坐标或缩放。保留圆角卡片与边缘Resize命中区域。
- Usage只保留账户、banked credits、Primary/Secondary时长/已用/剩余/本地重置时间；不再显示Ready、codex桶标题、进度条和token统计。优先展示codex额度桶，找不到则使用首个桶。所有值来自真实Provider，没有写死截图数值。
- Memo的body使用flex并隐藏自身overflow；textarea设为block、flex:1、min-height:0，仅textarea滚动，避免父子双滚动条。
- WebView滚动条6px、圆角、透明轨道、隐藏箭头、hover增亮；使用标准scrollbar属性作非WebKit回退。
- Reset按公告/观察来源标注，展示类型、当地时间、相对时间；banked明确为赠送可手动使用的次数。仅有未过期activeWatch时显示AI预测，不将其写成已发生Reset。日期换日不再误写Today。

## Reset网站核查

查阅日期：2026-09-05（本机UTC+8）。

- [主页](https://codex-resets.com/)说明追踪Tibo（@thsottiaux）的Reset公告，并保留历史；页面标明非OpenAI所属、数据由机器人分类。
- [公开API定义](https://codex-resets.com/api/openapi.json)将Reset公告和Watch预测分开，明确Watch是AI分类预测，不是官方承诺。当前页面/API没有发现投票数据，不能把它概括为投票网站。
- [当前状态API](https://codex-resets.com/api/v1/status)本次返回latest_reset类型banked、公告时间`2026-09-03T23:12:09Z`、来源x_post，active_watch=null。因此现阶段卡片只显示最新公告，不凭空显示预测概率。
- reset_type=regular表示普通重置公告，banked表示赠送重置次数。announced_at是公告或首次观察时间，不能当作该账户已经收到权益的精确时间。

Provider只新增白名单sourceType与activeWatch字段；预测的原始帖子文本/URL不传入UI。有效预测展示用合成fixture验证，当前线上没有activeWatch，不能称为线上预测实测。

## 验证

- TypeScript/Vite打包通过；前端15项通过，包含公告类型区分、相对时间与预测过期。
- Rust23项通过，包含regular/observed、banked/x_post、null和有效预测的字段白名单。证据：`evidence/reset-display-contract-tests-20260905-013805-257.json`。
- 用户数据、窗口大小及布局不作重置。视觉紧凑程度与滚动体验由用户最终验收。
- 原生离线构建9.10秒通过：`evidence/compact-widgets-build-20260905-013858-309.json`；双窗口事件隔离/原生move-resize/退出烟测通过：`evidence/smoke-20260905-013912-781/`。用户托盘退出后已更新程序并重新启动。

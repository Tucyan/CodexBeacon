# 首批模型行为断言

先建立Rust单元测试，再实现对应模型校验与操作：

- 非settings调用者不能改变全局设置或另一个组件内容。
- settings Hide后可见意图false；Show不启用已禁用组件。
- 系统隐藏只改变原生状态，不调用模型用户Hide操作。
- 非有限数/越界倍率和非法HEX被拒绝；合法无#颜色规范化。
- Reset Layout保留倍率/业务内容/主题/启用状态。
- serde序列化必须得到camelCase字段和kebab-case Action。

编译验证统一由root在所有模块就绪后单次有界执行；测试未执行前不能标GREEN。

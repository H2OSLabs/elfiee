
## User Journey Index（Authorization Enforcement）

| Journey ID   | 角色     | 场景描述                | 结果                    |
| ------------ | ------ | ------------------- | --------------------- |
| UJ-AUTHZ-001 | System | certificator 拦截越权操作 | 操作被拒绝，无非法 Event 产生    |


---

## 详细 User Journey

### UJ-AUTHZ-001｜certificator 拦截越权操作

**场景**
一个 Bot Editor 尝试对某个 Block 执行 `markdown.write`，但该 Bot 并未被授予对该 Block 的写权限。Engine 中的 certificator 需要检测到这一越权行为，拒绝该操作，确保没有非法的 Event 被写入事件日志。

**流程**

- Bot Editor 发出 Command：`{ editor_id: "bot-1", cap_id: "markdown.write", block_id: "block-xyz", payload: {...} }`。
- Engine 从 CapabilityRegistry 加载 `markdown.write` 的 Capability 定义，获取其 certificator 函数。
- certificator 执行授权检查：
    - 检查 `bot-1` 是否为 `block-xyz` 的 Owner → 否。
    - 查询 CapabilitiesGrant 表，查找 `(bot-1, markdown.write, block-xyz)` 或 `(bot-1, markdown.write, *)` → 未找到。
    - 判定为越权，返回拒绝。
- Engine 不调用 handler，不生成任何 Event，不修改任何状态。
- Engine 向前端发送 `command_rejected` 事件，包含拒绝原因。

**预期结果**

- **零非法事件：** certificator 拦截在 handler 执行之前，确保 `_eventstore.db` 中不会出现任何未授权的事件。
- **状态不变：** 被拒绝的操作不会导致任何内存状态或持久化状态的变化。
- **明确拒绝：** 系统返回结构化的拒绝信息，说明"哪个 Editor 对哪个 Block 的哪个操作被拒绝"。
- **可预测行为：** 相同的权限配置下，相同的操作始终得到相同的结果（允许或拒绝）。

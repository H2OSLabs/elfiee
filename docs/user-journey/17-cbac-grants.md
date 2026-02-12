
## User Journey Index（CBAC Grants）

| Journey ID  | 角色     | 场景描述                              | 结果                            |
| ----------- | ------ | --------------------------------- | ----------------------------- |
| UJ-CBAC-001 | Owner  | 授予 Editor 对特定 Block 的 Capability  | 被授权 Editor 可操作指定 Block        |


---

## 详细 User Journey

### UJ-CBAC-001｜授予 Editor 对特定 Block 的 Capability

**场景**
Owner 创建了一个 Markdown Block，并希望允许某个 Bot Editor 对该 Block 执行 `markdown.write` 操作。Owner 需要通过 Capability Grant 明确授权，Bot 才能对该 Block 进行写入。

**流程**

- Owner 选择目标 Block 和目标 Editor（Bot），指定要授予的 Capability（如 `markdown.write`）。
- 系统生成一条 `core.grant` Command，携带三元组 `(bot_editor_id, "markdown.write", target_block_id)`。
- Engine 处理该 Command：certificator 检查当前发出 grant 的 Editor 是否有权执行 `core.grant`（Owner 永远有权）。
- 校验通过后，handler 生成一条 Grant Event，写入 `_eventstore.db`。
- State Projector 将该 Grant 投影到内存中的 CapabilitiesGrant 表。
- 从此刻起，该 Bot Editor 对指定 Block 执行 `markdown.write` 时，certificator 在 CapabilitiesGrant 表中可以找到匹配的授权记录，操作将被允许。

**预期结果**

- **精确授权：** Bot 只能对指定的 Block 执行指定的 Capability，不能越权操作其他 Block 或执行其他 Capability。
- **即时生效：** Grant 事件被投影后，权限立即可用，无需重启或刷新。
- **事件记录：** Grant 操作被完整记录到 `_eventstore.db`，包含授权者（Owner 的 editor_id）、被授权者、Capability 和目标 Block。

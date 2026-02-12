

> **格式约定：** 本文档描述 Elfiee 事件日志（Event Log）的记录、不可变性、归因与审计的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 事件语义 / 审计能力**，不涉及 API 与实现细节。

---

## 1. Event Log 的产品定义

在 Elfiee 中，Event Log 是一切状态变更的不可变记录。每个 Event 遵循 EAVT 结构：Entity（被变更的对象 ID）、Attribute（`{editor_id}/{cap_id}`，谁用什么能力做的）、Value（JSON 载荷，做了什么）、Timestamp（Vector Clock，何时发生的）。Event 一旦写入 `_eventstore.db` 即不可修改、不可删除。

Event Log 是 Elfiee 的根基：
- State Projector 从 Event 流重建当前状态
- Timeline UI 从 Event 流渲染可视化时间线
- Time Travel 从 Event 流回溯到任意历史时刻
- CBAC 权限表从 Grant/Revoke Event 投影而来
- 审计与归因从 Event 的 Attribute 字段提取

---

## 2. Event Log 在整体闭环中的位置

```
用户 / Agent 操作
 ↓
Command（意图）
 ↓
Engine（授权 → 执行 → 冲突检测）
 ↓
Event 写入 _eventstore.db  ← 不可变记录点
 ↓                ↓                ↓
State Projector  Timeline UI    审计查询
（当前状态）     （可视化历史）   （归因追溯）
```

如果没有 Event Log：
- 系统无法重建状态，也无法进行时间旅行
- 无法回答"谁在什么时候做了什么"
- Block 的变更历史将不可追溯

---

## 3. User Journey Index（Event Log）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-EVENT-001 | System | 记录一次 Block 修改的完整事件 | EAVT 四元组写入 EventStore |
| UJ-EVENT-002 | User | 查看某个 Block 的变更归因 | 明确知道谁做了什么 |
| UJ-EVENT-003 | System | 保证事件的不可变性 | 历史记录不可篡改 |
| UJ-EVENT-004 | User | 按条件查询与过滤事件流 | 从海量事件中定位关键变更 |
| UJ-EVENT-005 | PM / Auditor | 审计某个 Editor 的所有操作 | 完整的操作轨迹报告 |

---

## 4. 详细 User Journey

### UJ-EVENT-001｜记录一次 Block 修改的完整事件

**场景** 开发者对某个 Markdown Block 进行了内容编辑。系统需要将这次修改完整记录为一个不可变事件，包含"谁、用什么能力、对哪个对象、做了什么、在什么时间"。

**流程**

- 用户对 Block 执行编辑操作，前端发送 Command 到 Engine。
- Engine 完成授权检查和执行后，生成一个 Event，包含：Entity（`block_id`）、Attribute（`{editor_id}/markdown.write`）、Value（修改内容的 JSON 载荷）、Timestamp（当前 Vector Clock 快照）。
- Event 被原子性写入 `_eventstore.db`。
- State Projector 接收 Event，更新内存中的 Block 状态。

**预期结果**

- **完整归因：** 事件记录包含操作者身份（editor_id）和使用的能力（cap_id），可追溯到具体的人或 Agent。
- **时序明确：** Vector Clock 提供因果序关系，Wall Clock 提供人类可读的时间。
- **原子写入：** 事件要么完整写入，要么不写入，不存在部分写入的中间状态。

---

### UJ-EVENT-002｜查看某个 Block 的变更归因

**场景** PM 想要了解某个关键代码 Block 最近被谁修改过、修改了什么内容，以便追溯一个引入的问题。

**流程**

- 用户打开目标 Block 的详情面板，选择「查看历史」。
- 系统从 `_eventstore.db` 中查询 Entity 等于该 `block_id` 的所有事件。
- 事件按时间顺序排列，每条展示：操作者（从 Attribute 解析 editor_id）、操作类型（从 Attribute 解析 cap_id）、变更摘要（从 Value 提取关键信息）、时间戳。
- 用户可逐条查看具体的变更内容（Value 载荷）。

**预期结果**

- **归因清晰：** 每次修改都有明确的操作者标识，区分人类和 AI Agent 的操作。
- **变更可读：** 用户可以看到每次修改的具体内容，而非仅知道"发生了修改"。
- **因果链完整：** 从 Block 创建到当前状态的所有中间步骤均有记录。

---

### UJ-EVENT-003｜保证事件的不可变性

**场景** 系统需要确保已写入的事件不会被修改或删除，即使是管理员也无法篡改历史记录。这是事件溯源模型的根本保证。

**流程**

- Event 一旦通过 Engine 的冲突检测并写入 `_eventstore.db`，系统不提供任何修改或删除已有事件的接口。
- 如果需要"撤销"某个操作，系统会创建一个新的补偿事件（compensating event），而非删除原事件。
- `_eventstore.db` 采用追加写入模式（append-only），不支持 UPDATE 或 DELETE 已有记录。
- 任何从事件流重建的状态与事件日志严格一致。

**预期结果**

- **历史不可篡改：** 任何人都无法通过系统接口修改已发生的事件。
- **撤销即补偿：** "撤销"操作本身也是一个事件，保留了"先做了 A，后撤销了 A"的完整轨迹。
- **审计可信：** 事件日志可作为可信的审计依据，因为它不存在被事后修改的可能。

---

### UJ-EVENT-004｜按条件查询与过滤事件流

**场景** 用户需要在大量事件中快速定位特定的变更记录。例如，查找"过去一周内所有由 AI Agent 执行的代码修改"。

**流程**

- 用户在事件查询界面设置过滤条件，可组合以下维度：
  - Entity 过滤：指定 `block_id` 或 `editor_id`。
  - Attribute 过滤：指定操作者或能力类型（如 `*/markdown.write` 或 `agent-001/*`）。
  - 时间范围过滤：指定起止时间。
- 系统从 `_eventstore.db` 查询匹配的事件集合。
- 结果按时间顺序展示，支持分页浏览。

**预期结果**

- **精准定位：** 多维过滤帮助用户从海量事件中快速找到目标记录。
- **灵活组合：** 过滤条件可自由组合，适应不同的查询场景。
- **性能可用：** 即使事件量较大，查询仍能在合理时间内返回结果。

---

### UJ-EVENT-005｜审计某个 Editor 的所有操作

**场景** 项目负责人需要审计某个 AI Agent 在 Project 中的所有操作，以评估其行为是否符合预期，是否存在越权或异常操作。

**流程**

- 用户指定目标 Editor（如 `agent-001`），发起审计查询。
- 系统从 `_eventstore.db` 中查询所有 Attribute 包含该 `editor_id` 的事件。
- 系统生成操作轨迹报告：按时间排列，每条事件标注操作对象（Entity）、操作类型（Capability）和变更摘要。
- 报告中高亮标注权限相关事件（`core.grant`、`core.revoke`），以便审查权限变更。

**预期结果**

- **完整轨迹：** 该 Editor 在 Project 中的每一步操作均可追溯。
- **权限审计：** 权限变更事件被特别标注，便于审查授权行为。
- **行为评估：** 审计报告为评估 Agent 行为质量和安全性提供客观依据。

---

## 5. 版本记录

- v2026-02-11：初版，覆盖事件记录、归因查看、不可变性保证、条件查询、Editor 审计。

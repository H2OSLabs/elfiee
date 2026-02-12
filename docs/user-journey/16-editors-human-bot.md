
## User Journey Index（Editor）

| Journey ID    | 角色            | 场景描述             | 结果                            |
| ------------- | ------------- | ---------------- | ----------------------------- |
| UJ-EDITOR-001 | User          | 注册为 Human Editor | 人类用户获得唯一 editor_id，可参与操作      |
| UJ-EDITOR-002 | User / System | 注册 Bot Editor    | AI Agent 获得唯一 editor_id，可参与操作 |
| UJ-EDITOR-003 | User          | 切换 Active Editor | 后续操作归因到新的 Active Editor       |
| UJ-EDITOR-004 | User          | 查看操作归因           | 在事件日志和 Timeline 中确认每个操作的执行者   |
| UJ-EDITOR-005 | User / Bot    | 多 Editor 并发操作    | 各 Editor 的操作独立记录，冲突被正确检测      |

---

## 详细 User Journey

### UJ-EDITOR-001｜注册为 Human Editor

**场景**
用户首次使用 Elfiee 打开一个 Project，或一个新的协作者加入已有 Project。系统需要为该用户创建一个 Human Editor 身份，使其成为可追踪的操作主体。

**流程**

- 用户打开 Project 时，系统检查是否已存在与该用户关联的 Editor 记录。
- 若不存在，系统创建一个新的 Human Editor，分配唯一的 `editor_id`，类型标记为 `Human`。
- Editor 的创建本身作为一条事件记录到 `_eventstore.db`，确保 Editor 注册行为可追溯。
- 新注册的 Editor 默认成为当前 Active Editor。

**预期结果**

- **身份唯一性：** 每个 Human Editor 拥有全局唯一的 `editor_id`，不会与其他 Editor（Human 或 Bot）冲突。
- **可追溯注册：** Editor 的创建被记录为事件，系统可以追溯"这个 Editor 是何时加入的"。
- **即刻可用：** 注册完成后，用户可以立即开始操作，无需额外的激活步骤。

---

### UJ-EDITOR-002｜注册 Bot Editor

**场景**
用户在 Project 中启用一个 AI Agent（如通过创建 Agent Block 或配置 MCP 连接）。系统需要为该 Agent 创建一个 Bot Editor 身份，使其操作在事件日志中具备独立的归因。

**流程**

- 用户在 Project 中创建 Agent Block 或通过配置启用 AI Agent。
- 系统为该 Agent 创建一个新的 Bot Editor，分配唯一的 `editor_id`，类型标记为 `Bot`。
- Bot Editor 的创建同样作为事件记录到 `_eventstore.db`。
- Bot Editor 的初始权限由创建它的 Human Editor（通常是 Owner）通过 CBAC Grant 授予。

**预期结果**

- **独立身份：** Bot Editor 与 Human Editor 一样拥有独立的 `editor_id`，其操作在事件日志中不会与人类操作混淆。
- **权限可控：** Bot Editor 不自动拥有所有权限，必须通过显式的 Capability Grant 获得操作授权（详见 17-cbac-grants.md）。
- **类型可区分：** 系统和 UI 可以根据 Editor 类型（Human/Bot）提供不同的展示方式（如在 Timeline 中用不同图标标记）。

---

### UJ-EDITOR-003｜切换 Active Editor

**场景**
在同一个工作会话中，用户可能需要在自己的 Human Editor 身份和某个 Bot Editor 身份之间切换。例如，用户想以 Bot 的身份手动触发一次操作来测试 Agent 行为，或在调试时模拟 Bot 的操作。更常见的情况是：当 AI Agent 接管执行时，系统自动将 Active Editor 切换到对应的 Bot Editor。

**流程**

- 用户在 Editor 选择器中查看当前 Project 中已注册的所有 Editor（Human 和 Bot）。
- 用户选择目标 Editor 作为新的 Active Editor。
- 系统更新当前会话的 Active Editor 状态。
- 从此刻起，所有新发出的 Command 中的 `editor_id` 字段将使用新的 Active Editor 的 ID。
- UI 中展示当前 Active Editor 的标识，确保用户清楚"现在以谁的身份在操作"。

**预期结果**

- **归因正确切换：** 切换后的所有操作在事件日志中归因到新的 Active Editor，不会错误地归因到旧的 Editor。
- **即时生效：** 切换操作立即生效，无需重新加载或重启。
- **用户感知清晰：** UI 始终明确展示当前 Active Editor 的身份，避免用户在不知情的情况下以错误身份操作。
- **权限随身份切换：** 切换到 Bot Editor 后，操作受该 Bot 的 CBAC 权限约束，可能某些操作会被 certificator 拒绝。

---

### UJ-EDITOR-004｜查看操作归因

**场景**
用户在回顾 Project 历史时，需要确认每个操作是由哪个 Editor 执行的。这在协作场景中尤其重要 — 当 AI Agent 修改了某个 Block 时，用户需要知道"这段修改是 AI 做的还是人做的"。

**流程**

- 用户在 Timeline UI 中浏览事件列表。
- 每条事件的 Attribute 字段格式为 `{editor_id}/{cap_id}`，系统从中解析出执行该操作的 Editor 身份。
- Timeline UI 以可视化方式展示归因信息：Editor 名称、类型（Human/Bot 图标）、执行的 Capability。
- 用户可以按 Editor 筛选 Timeline，只查看某个 Editor 的操作记录。
- 对于单个 Block，用户可以查看"谁在什么时间对这个 Block 做了什么修改"的完整操作历史。

**预期结果**

- **归因可见：** 每个操作的执行者在 Timeline 中一目了然。
- **Human 与 Bot 可区分：** 用户清楚地知道哪些修改来自人类决策，哪些来自 AI Agent 的自动处理。
- **审计能力：** 归因信息结合时间戳，构成完整的操作审计链，支持复盘和问责。
- **信任基础：** 明确的归因是 Human-AI 协作信任的基础 — 用户知道 AI 做了什么，才能信任 AI 的参与。

---

### UJ-EDITOR-005｜多 Editor 并发操作

**场景**
多个 Editor 同时对 Project 进行操作。例如，Human 在编辑一个 Markdown Block 的同时，Bot 在另一个 Code Block 中执行代码生成。或者两个 Editor 同时修改同一个 Block。系统需要正确处理这些并发操作。

**流程**

- 多个 Editor 各自发出 Command，每个 Command 携带各自的 `editor_id`。
- Engine Actor 按序处理 Command（Actor 模型保证串行化）：
    - 对于不同 Block 的操作：各自独立处理，无冲突。
    - 对于同一 Block 的操作：通过 Vector Clock 检测并发冲突。
- 若检测到并发冲突，后到达的 Command 被拒绝，对应 Editor 收到 `command_rejected` 通知，需要基于最新状态 re-base 后重试。
- 每个成功处理的 Event 独立记录其 Editor 归因，不同 Editor 的事件在 `_eventstore.db` 中有各自的 Vector Clock 分量。

**预期结果**

- **独立归因：** 每个 Editor 的操作在事件日志中保持独立的归因，不会因并发而混淆。
- **冲突安全：** 对同一 Block 的并发修改不会导致静默覆盖，Vector Clock 确保冲突被检测到。
- **不同 Block 互不干扰：** 多个 Editor 同时操作不同 Block 时，各自独立完成，不产生不必要的冲突。
- **Vector Clock 正确递增：** 每个 Editor 的事务计数在其 Vector Clock 分量中正确递增，准确反映因果关系。

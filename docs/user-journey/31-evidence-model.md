
## User Journey Index（Evidence Model）

| Journey ID | 角色        | 场景描述                  | 结果                         |
| ---------- | --------- | --------------------- | -------------------------- |
| UJ-EVID-01 | System    | 测试结果作为 Evidence 写入    | 测试通过/失败被结构化记录              |
| UJ-EVID-02 | System    | Evidence 与 Task 关联    | Evidence 成为 Task 的验证依据     |


---

## 详细 User Journey

### UJ-EVID-01｜测试结果作为 Evidence 写入

**场景** 代码执行完毕后，测试框架输出的通过/失败结果需要被结构化地记录为 Evidence，而非仅仅是一段输出文本。

**场景流程**

- 用户或 AI Agent 在 Terminal 中执行测试命令（如 `cargo test`、`pytest`），Terminal Bridge 捕获完整输出。

- 系统解析测试输出，提取结构化信息：测试总数、通过数、失败数、失败用例名称与错误消息。

- 系统将解析后的结构化数据写入 Evidence Block，类型标记为 `test_result`。

- Evidence Block 通过 Event Sourcing 记录创建事件，包含：Evidence 类型、原始输出、结构化摘要、关联的 Code Block ID、时间戳。

**预期结果**

- **结构化而非文本化：** 测试结果不再是需要人眼扫描的文本流，而是机器可读的结构化数据。AI Agent 可以直接判断"3 个测试失败"并针对性修复。

- 即使同一测试被反复执行，每次结果都作为独立 Evidence 保留，形成验证的时间线。

---

### UJ-EVID-02｜Evidence 与 Task 关联

**场景** 零散的 Evidence 需要被关联到对应的 Task，才能形成"Task 是否完成"的完整证据链。

**场景流程**

- 系统检测到产生 Evidence 的 Code Block 通过 `implement` 关系链隶属于某个 Task。

- 系统自动建立 Evidence Block 与 Task 之间的关联关系。

- 如果关联路径不明确（例如在独立 Terminal 中执行的命令），系统提示用户手动指定关联的 Task。

- Task 的详情视图中展示所有关联的 Evidence 列表，按时间倒序排列。

**预期结果**

- **自动编织证据网：** 大多数情况下，Evidence 自动沿着 Block 关系链关联到 Task，用户无需手动操作。

- 用户在查看 Task 时，可以一目了然地看到所有验证证据，而非在多个代码块之间来回切换。


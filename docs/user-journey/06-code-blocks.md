
## User Journey Index（Code Block）

| Journey ID | 角色             | 场景描述                | 结果                     |
| ---------- | -------------- | ------------------- | ---------------------- |
| UJ-CODE-01 | Developer      | 创建 Code Block 并编写代码 | Block 具备语言标注与语法高亮      |
| UJ-CODE-02 | Developer      | 触发代码执行并查看输出         | 执行结果回写至 Block，形成 Event |
| UJ-CODE-03 | Developer / AI | 执行结果作为证据关联到 Task    | Task 获得客观验证依据          |
| UJ-CODE-04 | Developer      | 查看 Code Block 的版本历史 | 代码演进过程完整可追溯            |
| UJ-CODE-05 | AI Agent       | Agent 自动生成并执行代码     | Agent 产出的代码与执行记录均被归因   |

---

## 详细 User Journey

### UJ-CODE-01｜创建 Code Block 并编写代码

**场景**
开发者需要在 Project 中编写一段可执行代码。代码可能是功能实现、测试脚本或数据处理脚本，需要明确标注编程语言以获得编辑器支持。

**流程**

- 开发者在 Block 编辑区创建一个新的 Code Block，并指定目标语言（如 Python、Rust、TypeScript）。
- 编辑器根据语言声明自动启用语法高亮、缩进辅助等编辑增强功能。
- 开发者编写代码内容，每次保存均产生 Event 记录到 `_eventstore.db`。
- Code Block 的 `contents` 中同时保存代码文本与语言元信息。

**预期结果**

- **语言感知**：Code Block 根据声明的语言提供语法高亮与格式化。
- **独立追踪**：Code Block 作为独立 Block 拥有完整的变更历史。
- **类型明确**：Block 的 `block_type` 标识其为 Code Block，与 Markdown Block 等类型清晰区分。

---

### UJ-CODE-02｜触发代码执行并查看输出

**场景**
开发者编写或修改代码后，希望在 Elfiee 内直接执行代码并查看输出结果，而不需要切换到外部终端。执行通过 Terminal 桥接完成，结果回写到 Block。

**流程**

- 开发者在 Code Block 上触发"执行"操作。
- 系统通过 Terminal Bridge 将代码发送到对应语言的运行时环境。
- Terminal 捕获执行过程中的 stdout、stderr 及 exit code。
- 执行结果作为 Event 回写到该 Code Block，包含完整输出与执行状态（成功 / 失败 / 超时）。
- 编辑器在 Code Block 下方或侧边显示执行输出。

**预期结果**

- **即时反馈**：开发者无需离开 Elfiee 即可验证代码行为。
- **结果持久化**：执行输出不是临时信息，而是作为 Event 永久记录在 `_eventstore.db` 中。
- **错误可归因**：执行失败时，stderr 与 exit code 被完整保留，可追溯到具体的 Code Block 版本。

---

### UJ-CODE-03｜执行结果作为证据关联到 Task

**场景**
开发者围绕某个 Task 编写代码并执行验证。执行结果（通过测试、编译成功、脚本输出符合预期）需要作为该 Task 的验证证据，证明实现满足 Task 定义的验收标准。

**流程**

- 开发者将 Code Block 通过 `implement` 关系关联到目标 Task。
- 开发者触发代码执行，系统记录执行结果 Event。
- 执行结果自动继承 Code Block 与 Task 之间的关联关系，成为 Task 的证据链条的一部分。
- 在 Task 视图中，PM 或开发者可以看到与该 Task 关联的所有执行结果及其状态。

**预期结果**

- **因果闭环**：Task 的"意图 → 实现 → 验证"形成完整链条，每个环节均有 Event 记录。
- **客观验证**：Task 的完成状态不仅基于人工判断，还有实际执行结果作为客观依据。
- **可审计性**：任何人可以追溯"这个 Task 是基于哪次代码执行被标记为完成的"。

---

### UJ-CODE-04｜查看 Code Block 的版本历史

**场景**
开发者需要理解一段代码的演进过程——它经历了哪些修改、每次修改由谁做出、修改的动机是什么。这在调试、代码审查或归档时尤为重要。

**流程**

- 开发者在 Code Block 上打开版本历史视图。
- 系统从 `_eventstore.db` 中回放该 Block 的所有变更 Event，按时间线呈现。
- 每个版本标注 Editor（Human 或 AI Agent）、时间戳与变更摘要。
- 开发者可以选择某个历史版本进行查看或对比。

**预期结果**

- **完整追溯**：代码的每一次变更均可追溯到具体的 Editor 与时间点。
- **人机归因**：清晰区分哪些修改由人类完成、哪些由 AI Agent 完成。
- **回溯安全**：查看历史版本是只读操作，不影响当前 Block 状态。

---

### UJ-CODE-05｜Agent 自动生成并执行代码

**场景**
AI Agent 在执行 Task 过程中，自动生成代码并触发执行以验证结果。Agent 的行为需要与人类开发者遵循相同的 Event 记录与权限约束。

**流程**

- AI Agent 基于 Task 描述，在 Project 中创建或修改 Code Block。
- Agent 的所有写操作均需通过 CBAC 权限校验（`code.write` Capability）。
- Agent 触发代码执行，执行结果回写到 Block，Event 的 `editor_id` 标注为 Agent。
- Agent 可根据执行结果决定是否继续迭代代码（多轮修改-执行循环）。

**预期结果**

- **权限一致**：Agent 与人类开发者遵循相同的 Capability 授权模型。
- **行为透明**：Agent 的每次代码生成与执行均记录为 Event，可被人类审查。
- **迭代可追踪**：Agent 的多轮修改-执行循环形成清晰的时间线，每轮尝试均可独立回溯。

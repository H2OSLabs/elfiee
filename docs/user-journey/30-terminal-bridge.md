
## User Journey Index（Terminal Bridge）

| Journey ID | 角色        | 场景描述                | 结果                     |
| ---------- | --------- | ------------------- | ---------------------- |
| UJ-TERM-01 | User / AI | 从 Code Block 发起代码执行 | Terminal Session 启动并执行 |
| UJ-TERM-02 | System    | 捕获 stdout/stderr 输出 | 输出流被完整记录               |
| UJ-TERM-03 | System    | 输出回写到 Block         | 执行结果成为 Block 内容        |


---

## 4. 详细 User Journey

### UJ-TERM-01｜从 Code Block 发起代码执行

**场景** 用户或 AI Agent 编写完代码后，需要在真实环境中执行并观察结果。

**场景流程**

- 用户在 Code Block 中完成代码编写，点击"执行"按钮；或 AI Agent 通过 Capability 调用发起执行请求。

- 系统检查当前 Editor 是否拥有该 Code Block 的执行权限（CBAC 校验）。

- 系统为本次执行分配或复用一个 Terminal Session，建立 Code Block 与 Terminal Session 的关联。

- 代码被发送到 Terminal 执行环境，开始运行。

**预期结果**

- **执行可追溯：** 每一次执行都有明确的发起者（Editor）、源代码块（Code Block）、执行环境（Terminal Session）三者绑定，事后可完整回溯。

- 用户无需离开 Elfiee 切换到外部终端，编辑与执行在同一界面内完成。

---

### UJ-TERM-02｜捕获 stdout/stderr 输出

**场景** 代码在 Terminal 中运行期间，系统需要完整捕获所有输出流，作为后续回写和归因的原材料。

**场景流程**

- Terminal 执行环境启动代码运行，系统开始实时捕获 stdout 和 stderr 两路输出流。

- 输出以流式方式传输，用户可以在 UI 中实时看到执行进度（渐进显示）。

- 当代码执行结束时（正常退出或异常退出），系统记录退出码（exit code）并关闭输出捕获。

- 系统将完整输出暂存，等待回写到 Block。

**预期结果**

- **完整性保障：** stdout 和 stderr 被分离捕获，不会丢失或混淆。

- **实时反馈：** 用户在执行过程中即可看到输出，而非等待执行完成后才看到结果。

---

### UJ-TERM-03｜输出回写到 Block

**场景** 执行完成后，Terminal 的输出不应停留在临时缓冲区，而应成为 `.elf` 容器中可追溯的持久化内容。

**场景流程**

- 系统将捕获的 stdout/stderr 结构化为输出内容，写入与源 Code Block 关联的 Output Block。

- 回写操作通过 Event Sourcing 记录为事件，包含：源 Code Block ID、Terminal Session ID、输出内容、退出码、执行时间戳。

- 如果该 Code Block 已有历史执行输出，新输出作为新版本追加，历史输出通过 Event Log 可回溯。

- UI 在源 Code Block 下方渲染输出内容，stdout 和 stderr 使用不同的视觉样式区分。

**预期结果**

- **输出即资产：** 执行输出不再是转瞬即逝的终端文本，而是 `.elf` 容器中的持久化内容，可以被引用、搜索和回溯。

- 每次执行的完整输出都被保留在 Event Log 中，支持 Time Travel 回看任意一次执行的结果。

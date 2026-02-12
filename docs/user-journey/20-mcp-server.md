## User Journey Index（MCP Server）

| Journey ID | 角色              | 场景描述                 | 结果                     |
| ---------- | --------------- | -------------------- | ---------------------- |
| UJ-MCP-01  | User / System   | 启动 MCP Server        | Server 就绪，监听协议请求       |
| UJ-MCP-02  | Agent           | 通过 tools/list 发现可用能力 | Agent 获得 Capability 清单 |


---

## 详细 User Journey

### UJ-MCP-01｜启动 MCP Server（协议网关就绪）

**场景** 用户或系统需要为某个 .elf 文件开放 AI Agent 的操作通道。

**场景流程**

- 用户通过命令行执行 `elfiee mcp-server --elf {path}`，或系统在 Agent 启用时自动启动 MCP Server。

- 系统加载目标 .elf 文件，初始化对应的 Engine Actor。

- MCP Server 完成协议握手准备，进入就绪状态，等待 Agent 连接。

- 系统将 MCP Server 的启动事件记录到 EventStore。

**预期结果**

- **协议桥接建立：** .elf 文件的内部能力被安全地暴露为标准化的 MCP 工具接口。

- Agent 可以通过标准协议发现并操作该文件，无需了解 .elf 的内部结构。

---

### UJ-MCP-02｜Agent 发现可用能力（tools/list）

**场景** Agent 连接到 MCP Server 后，需要了解当前 .elf 文件中有哪些操作可以执行。

**场景流程**

- Agent 通过 MCP 协议发送 `tools/list` 请求。

- MCP Server 查询当前 Engine 中已注册的所有 Capability。

- 系统根据当前 Agent 的身份与权限，过滤出该 Agent 被授权使用的 Capability 子集。

- MCP Server 将可用工具列表以 MCP 协议格式返回给 Agent，包含每个工具的名称、描述与参数 Schema。

**预期结果**

- **能力可见性：** Agent 获得一份精确的、与其权限匹配的能力清单。

- Agent 无需硬编码任何特定的 .elf 操作，而是动态发现当前上下文中可执行的操作。

- 不同 Agent 看到的工具列表可能因权限不同而不同，体现 CBAC 的前置过滤。


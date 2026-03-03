
## User Journey Index（Baths 路由层）

| Journey ID  | 角色            | 场景描述              | 结果                   |
| ----------- | ------------- | ----------------- | -------------------- |
| UJ-ROUTE-01 | User / System | Agent 注册到 Project | 绑定关系建立               |
| UJ-ROUTE-02 | System        | 指令路由到正确目标         | 指令进入对应 Agent/Project |


---

## 详细 User Journey

### UJ-ROUTE-01｜Agent 注册到 Project（绑定关系建立）

**场景** 用户需要将某个 Agent 指定为特定 Project 的协作者，使其后续行为限定在该 Project 范围内。

**场景流程**

- 用户在 Project 设置中选择"添加 Agent"，或系统在创建 Agent Block 时自动触发注册流程。

- Agent Register 创建一条绑定记录，关联 Agent ID 与 Project ID。

- 系统校验该 Agent 是否已在其他 Project 中注册（一个 Agent 可以注册到多个 Project）。

- 注册成功后，Agent Register 更新路由表，使 Agent Router 可以定位该 Agent。

- 注册事件被记录到对应 Project 的 EventStore 中。

**预期结果**

- **绑定显式化：** Agent 与 Project 的关系不是隐式推断的，而是通过显式注册建立的一等关系。

- 用户可以在任意时刻查看"哪些 Agent 在哪些 Project 中工作"。

---

### UJ-ROUTE-02｜指令路由到正确目标（单 Project 场景）

**场景** 外部指令到达 Baths，需要被分发到正确的 Agent 和 Project。

**场景流程**

- 外部指令到达 Baths，携带目标 Agent 的标识或目标 Project 的标识。

- Agent Router 查询 Agent Register，查找指令目标对应的绑定记录。

- Agent Router 确认目标 Agent 在目标 Project 中处于 Enabled 状态。

- Agent Router 将指令转发到目标 Project 的 MCP Server 入口。

- MCP Server 接收指令并转化为 Command，进入标准的 Engine 处理流程。

**预期结果**

- **路由透明：** 指令的发送方无需知道目标 Project 的具体位置或 MCP Server 的连接细节。

- 路由过程对用户不可见，系统自动完成从"指令意图"到"目标定位"的映射。

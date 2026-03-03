

> **格式约定：** 本文档描述 Agent Block 的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 产品语义**，不涉及 API 与实现细节。

---

## 1. Agent Block 的产品定义

在 Elfiee 中，Agent 是具有权限边界的主体

Agent Block 定义了 Agent 每一次行动的身份、边界、历史。

---

## 2. Agent 在整体系统中的位置

```
Human Editor
     │
     │（切换 Active Editor）
     ▼
Agent Block  ←── 执行主体
     │
     ▼
Blocks / Tasks / Files
     │
     ▼
Events / Evidence / Commit / Archive
```

---

## 3. User Journey Index（Agent Block）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-AGENT-01 | User | 创建 Agent | Agent = Disabled |
| UJ-AGENT-02 | User | 启用 Agent | Agent = Enabled |
| UJ-AGENT-03 | System | Agent 参与执行 | 行为被归因 |
| UJ-AGENT-04 | User | 切换 Active Editor | 执行权转移 |
| UJ-AGENT-05 | User | 禁用 Agent | 执行权收回 |

---

## 4. 详细 User Journey

### UJ-AGENT-01｜创建 Agent（执行者注册）

**场景** 用户在项目中引入新的 AI 协作力量，准备为其分配任务。

**场景流程**

- 用户在项目中选择“创建 Agent”或在引导流程中启用 AI 协作。
    
- 系统创建一个类型为 `block_type = agent` 的新实体。
    
- 系统将 Agent 初始状态设定为 `Disabled`。
    

**预期结果**

- Agent 被成功注册为**潜在执行者**。
    
- Agent 尚未拥有任何执行能力或权限。
    
- 在管理界面中可见该 Agent 占位，但处于未激活状态。
    

---

### UJ-AGENT-02｜启用 Agent（授予执行资格）

**场景** 用户确认 Agent 已准备就绪，正式授予其在项目中的行动权。

**场景流程**

- 用户执行显式的“启用”操作（如切换开关或确认授权）。
    
- 系统将 Agent 状态从 `Disabled` 更新为 `Enabled`。
    
- 系统激活 Agent 的变更发起权限（Block 变更）与任务参与权限（Task 执行）。
    

**预期结果**

- 用户明确感知到“AI 已获得行动许可”。
    
- Agent 具备发起系统行为的能力，且所有行为开始进入完整审计流。
    
- **核心定义：** 启用不等于“打开对话框”，而是“授予行动资格”。
    

---

### UJ-AGENT-03｜Agent 作为执行者参与系统

**场景** 在启用状态下，Agent 实际介入工作流并产生输出。

**场景流程**

- Agent 根据上下文或指令触发操作。
    
- 系统捕捉该行为，将其记录为 Event。
    
- 系统在 Event 记录中明确标注执行者（Actor）为该特定 Agent。
    

**预期结果**

- 用户能够清晰识别“谁做了什么”以及“为什么做”（溯源）。
    
- 所有 Agent 行为均可被回溯、审计和复盘。
    
- AI 行为从透明的“黑箱生成”转化为**可追责的执行记录**。
    

---

### UJ-AGENT-04｜切换 Active Editor（执行权转移）

**场景** 在人机协作过程中，明确当前“谁拥有控制权/修改权”，避免操作冲突。

**场景流程**

- 用户或系统逻辑触发 Active Editor 的切换请求（Human ↔ Agent）。
    
- 系统更新当前环境的 Active Editor 指针。
    
- 系统将后续产生的所有行为归属于当前激活的执行者。
    

**预期结果**

- 用户始终清楚当前是谁在“动手”以及谁在为当前结果负责。
    
- **执行权是显式的**，有效防止了隐性混合导致的逻辑混乱。
    

---

### UJ-AGENT-05｜禁用 Agent（收回执行资格）

**场景** 任务完成或需要人工介入接管时，撤销 AI 的行动权限。

**场景流程**

- 用户手动禁用 Agent 或系统根据项目阶段结束自动触发。
    
- 系统将 Agent 状态变更为 `Disabled`。
    
- 系统截断 Agent 触发任何新执行行为的路径。
    

**预期结果**

- 用户确信 AI 不会再产生非预期的后续动作。
    
- Agent 的历史执行记录被完整保留，仅丧失未来行动权。
    

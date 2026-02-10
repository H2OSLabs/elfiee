# 19-agent-block.md

> **格式约定：** 本文档描述 Agent Block 的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 产品语义**，不涉及 API 与实现细节。

---

## 1. Agent Block 的产品定义

在 Elfiee 中，**Agent 不是一个聊天机器人**，而是：

- 一个被明确标识的 **执行者（Executor）**
- 一个具有权限边界的 **行动主体**
- 一个能被记录、审计、回溯的 **因果参与者**

> 人与 AI 的根本差异，不在“智能程度”，  
> 而在于：  
> **是否被允许、被授权、被追责。**

Agent Block 的存在，是为了让 AI 的每一次行动：
- 有身份
- 有边界
- 有历史

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

没有 Agent Block：
- AI 只能是“隐形工具”
- 决策链条必然断裂

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

---

### UJ-AGENT-01｜创建 Agent（执行者注册）

**触发方式**

- 用户在项目中选择“创建 Agent”
- 或在引导流程中启用 AI 协作

**用户视角**

- 用户不是在“添加一个功能”
- 而是在说：  
  > *“我要允许一个非人类参与到这个项目中。”*

**系统行为**

- 创建一个 `block_type = agent`
- Agent 初始状态为 `Disabled`
- Agent 尚未拥有任何执行能力

**关键结果**

> Agent 被注册为**潜在执行者**，但尚未被授权。

---

### UJ-AGENT-02｜启用 Agent（授予执行资格）

**触发方式**

- 用户显式启用 Agent

**用户视角**

- 用户清楚地知道：  
  > *“从这一刻起，AI 可以开始动手了。”*

**系统行为**

- Agent 状态更新为 `Enabled`
- Agent 被允许：
  - 发起 Block 变更
  - 参与 Task 执行
- Agent 的所有行为将被完整记录

**关键结果**

> **启用 Agent = 授予行动资格**，  
> 而不是“打开一个助手”。

---

### UJ-AGENT-03｜Agent 作为执行者参与系统

**触发方式**

- Agent 在启用状态下执行操作

**用户视角**

- 用户看到的是：
  - “Agent 做了什么”
  - “为什么这样做”

**系统行为**

- 所有由 Agent 触发的行为：
  - 都被记录为 Event
  - 明确标注执行者为该 Agent
- 行为可被回溯、审计、复盘

**关键结果**

> AI 的行为从“黑箱生成”  
> 变为 **可追责的执行记录**。

---

### UJ-AGENT-04｜切换 Active Editor（执行权转移）

**触发方式**

- 用户在 Human / Agent 之间切换 Active Editor

**用户视角**

- 用户始终清楚：
  - 当前是谁在“动手”
  - 谁在为结果负责

**系统行为**

- 系统更新当前 Active Editor
- 后续所有行为都归属于新的执行者

**关键结果**

> **执行权是显式的，而不是隐式混合的。**

---

### UJ-AGENT-05｜禁用 Agent（收回执行资格）

**触发方式**

- 用户主动禁用 Agent
- 或在项目阶段结束时禁用

**用户视角**

- 用户明确表达：  
  > *“现在，AI 不应再继续行动。”*

**系统行为**

- Agent 状态更新为 `Disabled`
- Agent 无法再触发任何执行行为
- 其历史记录被完整保留

**关键结果**

> 禁用 Agent ≠ 删除历史  
> 而是 **终止其未来影响力**。

---

## 5. Agent Block 的产品边界（刻意不做的事）

- Agent **不等同于对话窗口**
- Agent **不拥有默认全局权限**
- Agent **不被假设为永远可信**

> Agent 的一切能力，  
> 都来自于**显式授权，而非智能假设**。

---

## 6. Agent 对决策资产化的意义

通过 Agent Block：

- AI 的每一次行动都能被：
  - 定位到具体任务
  - 追溯到具体意图
  - 对应到具体结果
- 人类与 AI 在系统中：
  - 地位对称
  - 责任清晰
  - 记录统一

这使得：
> **AI 不再是“帮忙的人”，  
> 而是“被纳入制度的参与者”。**

---

## 7. 小结

- Agent 是 Elfiee 的**执行者抽象**
- 没有 Agent，就没有 AI 原生协作
- Agent Block 让 AI 的参与：
  - 可控
  - 可解释
  - 可学习

> **Elfiee 不追求更聪明的 AI，  
> 而是追求“被制度化的智能”。**


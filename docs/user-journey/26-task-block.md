# 26-task-block.md

> **格式约定：** 本文档描述 Task Block 的完整 User Journey，覆盖从“意图被显式捕获”到“进入可提交、可归档闭环”的全过程。
> 不描述 API，不讨论实现细节，仅描述 **用户视角 + 系统行为 + 状态变化**。

---

## 1. Task 的产品定义

在 Elfiee 中，**Task 不是 Todo**，而是：

- 一次**明确意图（Intent）**的容器  
- 一条 **AI 推理与执行的因果锚点**
- 后续 Commit / Archive / Skill Evolution 的唯一入口

> Task 的存在，是为了回答一个问题：  
> **“这一系列行为，是为了解决什么问题？”**

---

## 2. Task 生命周期总览

Task 具有一个**强约束的状态机**：

```
Pending → InProgress → Committed → Archived
```

- **Pending**：意图已存在，但尚未进入执行
- **InProgress**：已有实现行为与之产生因果关系
- **Committed**：意图对应的实现已被外部世界确认
- **Archived**：该意图已转化为可复用、可学习的经验对象

---

## 3. User Journey Index（Task Block）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-TASK-01 | User / AI | 创建 Task（意图捕获） | Task = Pending |
| UJ-TASK-02 | User / AI | 写入 / 修改 Task 内容 | 意图被结构化 |
| UJ-TASK-03 | System | Task 与执行行为建立关系 | Task → InProgress |
| UJ-TASK-04 | User / AI | Task 被引用为实现上游 | 因果图形成 |
| UJ-TASK-05 | System | Task 进入可提交状态 | Task = Committable |

---

## 4. 详细 User Journey

---

### UJ-TASK-01｜创建 Task（Intent Capture）

**触发方式**

- 用户在 GUI 中创建 Task  
- 或 AI 在外部工具中被要求“创建一个新任务”

**用户视角**

- 用户显式表达：*“我要解决一个问题”*
- 问题此时可以是模糊的、不完整的

**系统行为**

- 创建一个 `block_type = task`
- 初始化状态为 `Pending`
- Task 成为一个**可被引用的因果节点**

**关键结果**

> 意图第一次被“对象化”，而不是停留在自然语言里。

---

### UJ-TASK-02｜写入与演化 Task 内容

**触发方式**

- 用户或 AI 对 Task 进行补充、修改

**用户视角**

- Task 从一句话，逐渐变成：
  - 背景
  - 约束
  - 目标
  - 已知风险

**系统行为**

- 每一次修改都被记录为 Event
- Task 的历史可被完整回溯

**关键结果**

> Task 不追求“一次写对”，而是允许**意图逐步清晰**。

---

### UJ-TASK-03｜Task 与执行行为建立关系

**触发方式**

- AI / 用户创建代码、文档、配置等 Block
- 并声明：**这是为了完成某个 Task**

**用户视角**

- 用户不需要手动维护清单
- 只需认可“这一步是在解决哪个问题”

**系统行为**

- 建立 Task → Block 的因果关系（如 implement）
- Task 状态自动变为 `InProgress`

**关键结果**

> Task 不再是描述文本，而是**执行网络的上游节点**。

---

### UJ-TASK-04｜Task 作为因果锚点被持续引用

**触发方式**

- 后续的修改、测试、讨论持续引用同一 Task

**用户视角**

- Task 成为协作的“共识中心”
- 新加入的 AI / 人类可快速理解上下文

**系统行为**

- 所有关联行为被聚合在 Task 的因果图中
- 支持“从 Task 反查一切相关行为”

**关键结果**

> Task 成为 AI 推理时的 **Context Anchor**，而不是噪音源。

---

### UJ-TASK-05｜Task 进入可提交状态

**触发方式**

- Task 的下游实现已完整
- 用户 / AI 明确表达：*“这个任务可以提交了”*

**用户视角**

- Task 不再只是“在做”
- 而是**准备接受外部世界验证**

**系统行为**

- 校验 Task 的实现关系是否闭合
- 标记 Task 为 `Committable`（逻辑状态）

**关键结果**

> Task 从“执行中的意图”，转为“可被确认的因果假设”。

---

## 5. Task Block 的产品边界（刻意不做的事）

- Task **不等同于 Issue / Ticket**
- Task **不要求完整、规范、一次写好**
- Task **不负责调度、排期、资源分配**

> Task 只做一件事：  
> **成为人和 AI 之间，共同认可的“为什么”。**

---

## 6. Task 在整体闭环中的位置

```
Task
 ↓
Execution (AI / Human)
 ↓
Evidence (Terminal / Tests / Logs)
 ↓
Commit
 ↓
Archive
 ↓
Skill Evolution
```

如果 Task 不成立，后续所有环节都只是**无根之木**。

---

## 7. 小结

- Task 是 Elfiee 中**最重要的认知结构**
- 它不是管理工具，而是**推理起点**
- 写 Task 的成本越低，系统的学习上限越高


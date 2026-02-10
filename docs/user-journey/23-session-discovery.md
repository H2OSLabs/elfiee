# 23-session-discovery.md

> **格式约定：** 本文档描述系统如何发现、识别并定位外部 AI 工具产生的 Session（会话记录），
> 作为后续同步、解析与归因的前置 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 产品语义**，不涉及 API 与实现细节。

---

## 1. Session Discovery 的产品定义

在 Elfiee 中，Session 包括：

- AI 实际发生过的推理与行动历史
- 外部工具（如 Claude Code）与 Elfiee 内部工具产生的 artifacts 的关联关系（*待确认*）

---

## 2. Session Discovery 在整体闭环中的位置

```
External AI Tool
   (Claude / Cursor)
        │
        │  Session Files
        ▼
Session Discovery  ←── 定位与识别
        │
        ▼
Session Ingestion / Linking
        │
        ▼
Task / Evidence / Archive / Skill
```


---

## 3. User Journey Index（Session Discovery）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-SESS-01 | System | 识别外部 AI 工具环境 | 建立工具上下文 |
| UJ-SESS-02 | System | 定位 Session 根目录 | 找到事实发生地 |
| UJ-SESS-03 | System | 发现可用 Session | Session 可被监听 |
| UJ-SESS-04 | User | 切换 / 新增项目 | Discovery 更新 |

---

## 4. 详细 User Journey

---

### UJ-SESS-01｜识别外部 AI 工具环境

**触发方式**

- 用户打开或切换 Project
- Agent 被启用

**用户视角**

- 用户不需要配置路径
- 不需要告诉系统“AI 在哪里工作”

**系统行为**

- 系统识别当前 Project 所关联的外部工作目录
- 判断该目录是否存在已初始化的 AI 工具环境
- 记录该 Project 对应的外部上下文

**关键结果**

> Session Discovery **不要求用户迁移工具或流程**。

---

### UJ-SESS-02｜定位 Session 根目录

**触发方式**

- 外部工具环境被确认存在

**用户视角**

- 用户通常并不知道 Session 存储在哪里
- 也不需要知道

**系统行为**

- 系统根据已知规则，推导 Session 存储位置
- 建立 Project ↔ Session Root 的映射关系
- 确认该位置是：
  - 稳定的
  - 可增量读取的

**关键结果**

> Elfiee 主动适配工具，  
> 而不是要求工具适配 Elfiee。

---

### UJ-SESS-03｜发现可用 Session（进入可监听状态）

**触发方式**

- Session 根目录被成功定位

**用户视角**

- 用户不会立即看到任何变化
- 但系统已进入“可感知 AI 行为”的状态

**系统行为**

- 系统扫描 Session 目录结构
- 识别：
  - 已存在的 Session
  - 正在增长的 Session
- 标记哪些 Session 是新的、可追踪的

**关键结果**

> AI 的“思考现场”第一次  
> **被系统正式感知到**。

---

### UJ-SESS-04｜项目变化触发 Discovery 更新

**触发方式**

- 用户切换 Project
- 新增 / 更换外部工作目录
- 启用或禁用 Agent

**用户视角**

- 用户继续使用原有工具
- 不需要手动刷新或重置

**系统行为**

- 系统重新执行 Session Discovery
- 更新 Project ↔ Session Root 映射
- 停止对旧上下文的发现

**关键结果**

> Session Discovery 始终  
> **与当前 Project 语境保持一致**。


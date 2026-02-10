# 24-session-ingestion.md

> **格式约定：** 本文档描述外部 AI 工具产生的 Session 内容，如何被持续、可控地写入 Elfiee，
> 成为可追溯 Event 与 Evidence 的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 产品语义**，不涉及 API 与实现细节。

---

## 1. Session Ingestion 的产品定义

在 Elfiee 中，**Ingestion 不是“导入聊天记录”**，而是一次：

- 对外部 AI 行为的 **事实筛选**
- 对原始推理轨迹的 **时间序写入**
- 为后续因果分析提供 **可信原始材料**

> 如果说 Session Discovery 解决的是  
> **“事实在哪里发生”**，  
> 那么 Session Ingestion 回答的是：  
> **“哪些事实，值得进入系统历史。”**

---

## 2. Session Ingestion 在整体闭环中的位置

```
External AI Tool
   (Claude / Cursor)
        │
        │  Session Files
        ▼
Session Discovery
        │
        ▼
Session Ingestion  ←── 事实写入闸门
        │
        ▼
Events / Evidence / Linking
        │
        ▼
Task / Commit / Archive / Skill
```

Ingestion 的质量，直接决定：
- AI 行为是否“看得见”
- 失败路径是否会被保留
- 学习是否建立在完整历史之上

---

## 3. User Journey Index（Session Ingestion）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-INGEST-01 | System | 启动 Session 监听 | 进入持续感知 |
| UJ-INGEST-02 | System | 增量读取 Session 内容 | 新事实被捕获 |
| UJ-INGEST-03 | System | 写入 Session 事实记录 | 原始历史形成 |
| UJ-INGEST-04 | System | 处理中断与恢复 | 历史连续 |
| UJ-INGEST-05 | User | 停止或切换 Ingestion | 写入受控 |

---

## 4. 详细 User Journey

---

### UJ-INGEST-01｜启动 Session 监听（进入感知状态）

**触发方式**

- Project 被打开
- Session Discovery 成功完成
- Agent 处于启用状态

**用户视角**

- 用户无需任何操作
- 不会看到“开始录制”的提示

**系统行为**

- 系统开始监听已发现的 Session 来源
- 建立“可增量读取”的监听状态
- 为每个 Session 准备独立的写入上下文

**关键结果**

> Ingestion 是**默认发生、但不打扰用户的**。

---

### UJ-INGEST-02｜增量读取 Session 内容

**触发方式**

- 外部 Session 文件发生变化
- 新的推理或工具调用被写入

**用户视角**

- 用户继续在外部工具中与 AI 交互
- 不感知任何同步过程

**系统行为**

- 系统仅读取新增内容
- 保留原始时间顺序
- 不修改、不重排、不总结

**关键结果**

> Elfiee 记录的是 **“发生过什么”**，  
> 而不是“看起来重要的部分”。

---

### UJ-INGEST-03｜写入 Session 事实记录

**触发方式**

- 新的 Session 内容被成功读取

**用户视角**

- 用户暂时不会直接看到这些内容
- 但它们已进入系统历史

**系统行为**

- 将 Session 内容写入为原始事实记录
- 明确标注：
  - 来源工具
  - 时间顺序
  - 产生上下文
- 不与 Task 自动绑定

**关键结果**

> Session Ingestion 生成的是  
> **未经解释的事实层**。

---

### UJ-INGEST-04｜处理中断与恢复

**触发方式**

- 应用关闭
- 系统重启
- 外部工具异常退出

**用户视角**

- 用户不需要手动“重新导入”
- 不会因为中断而丢失历史

**系统行为**

- 系统记录已读取进度
- 在恢复时继续从中断位置读取
- 避免重复或遗漏

**关键结果**

> Session 历史具备  
> **时间上的连续性与可靠性**。

---

### UJ-INGEST-05｜停止或切换 Ingestion

**触发方式**

- 用户切换 Project
- Agent 被禁用
- 外部工作目录发生变化

**用户视角**

- 用户只是在切换工作上下文
- 不需要额外管理同步状态

**系统行为**

- 系统停止当前 Session 的 Ingestion
- 保存进度状态
- 为新上下文重新准备监听

**关键结果**

> Session Ingestion 始终  
> **服从于当前 Project 语境**。

---

## 5. Session Ingestion 的产品边界（刻意不做的事）

- Ingestion **不做摘要**
- Ingestion **不判断对错**
- Ingestion **不自动推断意图**
- Ingestion **不删除失败或无效尝试**

> Ingestion 只做一件事：  
> **忠实记录发生过的事实。**

---

## 6. Session Ingestion 对决策资产化的意义

通过 Session Ingestion：

- AI 的犹豫、试探、回滚被完整保留
- 成功与失败拥有同等记录权
- 后续系统可以基于：
  - 完整历史
  - 明确时间序
  进行因果分析

这使得：
> **学习不再建立在“被美化的过程”之上，  
> 而是建立在真实发生过的一切之上。**

---

## 7. 小结

- Session Ingestion 是事实进入系统的**唯一闸门**
- 它的克制，决定了系统未来的学习上限
- 少做判断，多留历史

> **Elfiee 相信：  
> 真实，比聪明更重要。**


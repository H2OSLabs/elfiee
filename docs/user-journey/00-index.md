# Elfiee User Journey Index

> 本文档是 **Elfiee 的 User Journey 索引（Index）**，  
> 用于枚举在当前「完整产品架构」下，Elfiee 覆盖的所有核心用户旅程。  
>  
> 本文件 **只定义有哪些 User Journey**，不展开具体步骤与细节，  
> 作为后续 UX / 产品 / 技术文档的索引入口。

---

## 0. 元原则（适用于所有 Journey）

- 所有 Journey 都围绕 **AgentChannel → Agent → AgentContext** 展开  
- Elfiee 是 **工作面 / 决策记忆层 / 编排器**，而非执行者  
- 每个 Journey 都至少涉及以下之一：
  - Context 选择 / 构建
  - Agent 触发 / 路由
  - Artifact（文件 / 代码 /配置）
  - Evidence（Logs / Usage / Session）
- Journey 之间 **可组合、可嵌套、可中断、可回放**

---

## 1. 项目与工作面生命周期 Journeys

### J1. 创建 / 打开 Elfiee Project
- 新建 `.elf` 项目
- 打开已有 `.elf` 项目
- Project Context 初始化

### J2. 多 Project 并行切换
- 在多个 `.elf` 项目之间切换
- 不同 Project 对应不同 AgentContext

### J3. Project 导入 / 导出
- 导入外部目录或已有工程
- 导出 Project 资产用于分享或归档

---

## 2. Context 构建与管理 Journeys

### J4. Machine Context 绑定
- 选择 / 绑定 PC / Mac / ECS
- 本地 / 远端 Machine 切换

### J5. Credential 与 Auth 管理
- Claude / OpenAI 等 Credential 接入
- OneAuth / OneSystem 对接

### J6. FileSystem Context 管理
- configs / code / files 组织
- 本地文件与 AgentContext 同步

### J7. Context Sync 与恢复
- 手动 / 自动 Sync
- Context 断点恢复

---

## 3. Agent 生命周期 Journeys

### J8. Agent 注册（Register）
- 将 Agent 注册到 AgentChannel
- MCP + Bot 能力声明

### J9. Agent 启用 / 禁用
- 启用 Agent 参与当前 Project
- 暂停或移除 Agent

### J10. 多 Agent 并存与隔离
- 多 Agent 同时存在
- Context / 能力边界隔离

---

## 4. Agent 调用与执行 Journeys

### J11. 手动触发 Agent 执行
- 用户主动调用 Agent
- 指定目标与上下文

### J12. 路由触发（Router-driven）
- 由 AgentRouter 决定目标 Agent
- 多 Agent 协同路径

### J13. Slash / Endpoint 触发
- Slash Command
- Endpoint / MessageType 驱动执行

---

## 5. Task 与决策执行 Journeys

### J14. Task 创建与拆解
- 定义 Task / 子 Task
- Task 与上下文绑定

### J15. Task → Agent 执行
- Task 作为输入驱动 Agent
- 生成中间与最终产出

### J16. Task 迭代与再执行
- 修改上下文后重跑
- 对比多次执行结果

---

## 6. Session 与执行过程 Journeys

### J17. Bash / Shell Session 生成
- Agent 启动执行 Session
- Session 生命周期管理

### J18. Session 过程记录
- 命令 / 输出记录
- Session 与 Task / Agent 关联

### J19. Session 回放与调试
- 历史 Session 回看
- 用于 Debug / 复盘

---

## 7. Artifact（产物）管理 Journeys

### J20. Code / File 生成与更新
- Agent 写入代码或文件
- 本地与 Context 同步

### J21. 配置生成与演进
- configs 由 Agent 生成
- 配置版本演进

### J22. 多版本 Artifact 对比
- 不同执行结果对比
- 人工决策介入

---

## 8. Evidence 与 Traceability Journeys

### J23. Logs 收集
- usage logs
- system logs

### J24. Evidence 关联
- Logs / Session / Artifact 关联到 Task
- 形成完整 Trace

### J25. 执行可解释性回溯
- 为什么得到这个结果
- 哪个 Context / Agent / Session 导致

---

## 9. Skill 生成与复用 Journeys

### J26. 从执行中提炼 Skill
- 从重复 Task / Session 中抽象 Skill

### J27. Skill 本地管理
- Skill 版本化
- Skill 作用域管理

### J28. Skill 暴露给 AgentChannel
- MCP + Skill 注册
- 跨 Project 复用

---

## 10. 跨产品协作 Journeys（Elfiee × 其他产品）

### J29. Elfiee ↔ Chatroom（Ezagent）
- 执行结果讨论
- 决策共识形成

### J30. Elfiee ↔ Synnovator
- 结果发布
- Proof-of-Work 展示

### J31. 外部 3PID / IM 集成
- 外部系统触发 Agent
- 状态回传

---

## 11. 回放、审计与学习 Journeys

### J32. 决策历史回放
- 时间轴浏览
- Context 状态重建

### J33. 决策审计
- 人类 / Agent 行为审计
- 合规与责任追踪

### J34. 学习与反馈闭环
- 从 Logs / Usage 中学习
- 反哺 Skill 与 Agent 配置

---

## 12. 异常与边界 Journeys

### J35. 执行失败处理
- Context 不完整
- Credential 失效

### J36. 冲突与中断恢复
- 并发冲突
- Session 中断

### J37. 手动接管与降级
- 人类接管执行
- Agent 降级运行

---

## 13. Journey 组合模式（索引）

- **初始化型**：J1 → J4 → J5 → J8  
- **执行型**：J14 → J11 → J17 → J20 → J23  
- **迭代型**：J16 → J19 → J21  
- **资产化型**：J26 → J27 → J28  
- **发布型**：J15 → J30  
- **学习型**：J24 → J34  

---

## 14. 后续文档规划（占位）

- 每个 Journey 对应一份：
  - User Story
  - 状态机
  - 关键 UI / API
  - 成功与失败路径
- 本 Index 作为 **Elfiee User Journey 的唯一入口**

---

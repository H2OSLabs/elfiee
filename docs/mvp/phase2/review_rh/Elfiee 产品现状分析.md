---
created_date: "20260225"
---


## 1. Elfiee 是什么

Elfiee 的核心命题是 "让决策可学习"，定位是 AI 开发工具（Claude Code、Cursor 等）的决策记忆层——不替代 AI，而是在后台记录意图、实现、验证的完整因果链，让历史决策能被 AI 在下次同类任务中复用。


## 2. 当前实现了什么

Phase 1（已完成）：Record 基础设施
- 核心架构全部就位：Event Sourcing、Block-based、CBAC 权限、Actor 模型
- 功能完整：Dashboard、目录管理、Markdown/Code 编辑、协作者权限、事件 Timeline
- 产品实验已完成：验证了 "Summary + Traceability 是方向，纯 Log 无效"

Phase 2（进行中）：AI 接入
- MCP Server 已工作：2026-02-03 的实测记录显示核心工作流跑通
- Task → Code → Relation → Git Commit 链路可用
- Agent block、Terminal block、Skills 模板均已实现
- 但这是基础设施完成，不等于价值主张验证


## 3. 核心问题：价值闭环尚未合拢

Phase 2 的根本假设是：

"第二次"做同类型任务时，因为有 Elfiee 记录的第一次决策，效率或准确度会系统性提升。

截至目前，这个假设还没有被验证。问题在于：

### 3.1. 实验只完成了 First-use，没有做 Second-use

实验计划要求 "First-use / Second-use Task Pair"。2.03 的记录是一次成功的 First-use（加法改乘法），但配对的 Second-use（同类型任务第二次）没有记录。没有对比，就无法判断学习是否发生。

### 3.2. 关键障碍尚未排除

从 2.03 的实测中暴露出几个严重摩擦点：
- Terminal 输出未被捕获：跑测试的结果无法在 Elfiee 内看到，FPY 指标无法测量
- Directory Block 管理混乱：手动创建的 code block 和 directory.import 创建的 block 是两个不同 block，AI 重复写入了同一份代码两次
- directory.export 权限报错：Agent 没有这个权限，需要手动 grant，这在正常使用流程里不应该出现
- Test 环境未准备：pytest 没有安装，验证闭环断开

这些问题单独看都是 Bug，合在一起则说明工作流的流畅度还不足以支撑实验。

### 3.3. Summary 功能缺失（P2）

Phase 2 的验证指标之一是 "Summary 采纳率 > 80%"，但 AI 生成 Summary + 人工确认这个功能还没有实现。这意味着 "Record 质量提升" 这条路现在走不通。


## 4. 接下来产品应该做什么

按优先级排列：

### 4.1. P0：完成一个完整的 Task Pair 实验

这是 Phase 2 能否有结论的前提。
- 选择一个 "有犯错空间、结构相似" 的真实开发任务（e.g. 新增一个 Capability）
- 第一次执行，记录完整数据
- 修复 Terminal 输出捕获 和 Agent 权限这两个阻塞问题
- 第二次执行同类型任务，对比 TFC、Clarification Count、Constraint Miss

如果 Second-use 没有改善 → 说明当前记录形态不足以支撑学习，需要调整记录结构。
如果 Second-use 有改善 → Phase 2 假设成立，可以往 Phase 3 推进。

### 4.2. P1：降低 Dogfooding 摩擦

- 修 Directory Block 管理的 UX（手动创建的 block 应该能挂到 directory）
- Agent 全局权限应该包含 directory.export（现在这是一个反直觉的障碍）
- Terminal 输出捕获（这是验证闭环的唯一手段）

### 4.3. P2：思考一个战略性问题

当前 Elfiee 的核心价值依赖于：AI 在开发过程中主动调用 Elfiee MCP 来写 Block、建 Relation。但 Claude Code 自身的 /memory 和 CLAUDE.md 也在做类似的事情（记录项目决策）。

Elfiee 的差异化护城河在哪里？

目前最有价值的部分可能是：Event + Block 的结构化因果链（Task → Code 的 implement 关系 + Git Commit 绑定），这在 Claude Code 原生能力里没有。但这个价值需要通过实验数据来证明，而不是设计文档。

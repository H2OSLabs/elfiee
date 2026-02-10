# 29-task-archive.md

> **格式约定：** 本文档描述 Task 在完成 Commit 后进入 Archive 阶段的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 资产化意义**，不涉及 API 与实现细节。

---

## 1. Task Archive 的产品定义

在 Elfiee 中，Archive 是对已验证 Task 的资产化处理

---

## 2. Task Archive 在整体闭环中的位置

```
Task (Intent)
 ↓
Execution (AI / Human)
 ↓
Evidence (Terminal / Tests / Logs)
 ↓
Commit  ← 因果确认点
 ↓
Archive ← 经验固化点
 ↓
Skill Evolution / Reference
```

如果没有 Archive：
- Commit 只是一次历史事实
- AI 无法区分“偶然成功”和“可复用路径”

---

## 3. User Journey Index（Task Archive）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-ARCH-01 | User / AI | 发起 Task Archive | 进入归档判断 |
| UJ-ARCH-02 | System | 汇聚 Task 的完整因果链 | 形成候选经验 |
| UJ-ARCH-03 | System | 区分成功 / 失败归档类型 | 经验被定性 |
| UJ-ARCH-04 | System | 生成归档资产对象 | Archive Block 创建 |
| UJ-ARCH-05 | System | 更新 Task 状态 | Task = Archived |

---

## 4. 详细 User Journey

---

### UJ-ARCH-01｜发起 Task Archive（经验评估请求）

**触发方式**

- 用户明确表达：*“归档这个任务”*
- 或系统在 Commit 后提示可归档

**用户视角**

- 用户并非在说“收起来”
- 而是在说：  
  > *“这次尝试，是否值得被未来参考？”*

**系统行为**

- 系统进入 Task Archive 流程
- Task 仍处于 `Committed` 状态，等待评估

**关键结果**

> Archive 被视为一次 **价值判断请求**。

---

### UJ-ARCH-02｜汇聚 Task 的完整因果链

**触发方式**

- Archive 流程启动

**用户视角**

- 用户无需手动整理过程
- 不需要“复盘式写总结”

**系统行为**

- 系统自动汇聚与 Task 相关的：
  - 意图描述
  - 实现路径
  - 关键决策点
  - 成功与失败的尝试
  - 外部 Commit 证据
- 形成一条完整的因果时间线

**关键结果**

> Archive 的原材料不是“结果”，  
> 而是 **为什么这样做会成功 / 失败**。

---

### UJ-ARCH-03｜区分成功归档与失败归档

**触发方式**

- 系统基于 Commit 结果与 Evidence 进行判定

**用户视角**

- 用户无需手动打标签
- 但可以理解归档类型的差异

**系统行为**

- 将归档分为两类：
  - **成功归档**：  
    - Commit 成立  
    - 关键验证通过  
    - 可作为正向经验
  - **失败归档**：  
    - Commit 被回滚 / 否定  
    - 验证失败  
    - 明确标记为“不可复用路径”

**关键结果**

> **失败经验不是噪音，而是负样本资产。**

---

### UJ-ARCH-04｜生成归档资产对象

**触发方式**

- 归档类型确认完成

**用户视角**

- 用户看到一个新的“归档对象”
- 而不是散落的历史记录

**系统行为**

- 系统生成一个独立的 Archive Block：
  - 压缩 Task 的核心信息
  - 保留可追溯引用
  - 明确其经验属性（成功 / 失败）
- Archive Block 与原 Task 建立关联

**关键结果**

> 执行过程被压缩为  
> **“可被再次调用的经验单元”**。

---

### UJ-ARCH-05｜更新 Task 状态并退出执行循环

**触发方式**

- Archive Block 创建完成

**用户视角**

- Task 明确进入“已结束”状态
- 不再参与执行型操作

**系统行为**

- Task 状态更新为 `Archived`
- Task 退出执行闭环
- 其经验进入学习与引用阶段

**关键结果**

> Task 从“执行对象”  
> 变为 **“历史样本”**。


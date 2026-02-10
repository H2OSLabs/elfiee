# 28-task-commit-git.md

> **格式约定：** 本文档描述 Task → Commit → Git 的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 因果意义**，不涉及 API 与实现细节。

---

## 1. Task Commit 的产品定义

在 Elfiee 中，Commit 是决定该 Task 进入学习与复用阶段的行为。

---

## 2. Task Commit 在整体闭环中的位置

```
Task (Intent)
 ↓
Execution (AI / Human)
 ↓
Evidence (Terminal / Tests / Logs)
 ↓
Commit  ←──【因果确认点】
 ↓
Archive
 ↓
Skill Evolution
```

没有 Commit：
- Task 只是“内部自洽的故事”
- AI 无法判断哪些路径**真的可行**

---

## 3. User Journey Index（Task Commit）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-GIT-01 | User / AI | 发起 Task Commit | 进入确认流程 |
| UJ-GIT-02 | System | 收敛 Task 的实现范围 | 确认因果边界 |
| UJ-GIT-03 | System | 导出实现结果至外部 Repo | 物理世界同步 |
| UJ-GIT-04 | External | Git 接受并记录 Commit | 外部事实成立 |
| UJ-GIT-05 | System | 回写 Commit 结果 | Task = Committed |

---

## 4. 详细 User Journey

---

### UJ-GIT-01｜发起 Task Commit（意图确认）

**触发方式**

- 用户明确表达：*“提交这个任务”*
- 或 AI 判断：当前 Task 已满足提交条件

**用户视角**

- 用户并非在说“保存代码”
- 而是在说：  
  > *“我认为这个问题，已经被解决到可以被外部世界检验的程度”*

**系统行为**

- 系统进入 Task Commit 流程
- Task 仍保持 `InProgress` 状态，等待确认结果

**关键结果**

> Commit 被视为一个**判断请求**，而非立即成功的动作。

---

### UJ-GIT-02｜收敛 Task 的实现边界

**触发方式**

- Commit 流程启动后，系统分析 Task 的因果关系

**用户视角**

- 用户无需手动选择文件
- 也不需要维护提交清单

**系统行为**

- 系统根据 Task 的因果关系图：
  - 识别哪些 Block 是“为该 Task 而存在的”
  - 排除无关修改
- 明确本次 Commit 的**因果边界**

**关键结果**

> Commit 不再是“目录差异”，  
> 而是 **意图 → 实现** 的一次映射。

---

### UJ-GIT-03｜同步实现结果到外部世界

**触发方式**

- Task 的实现边界被确认

**用户视角**

- 用户看到的是一个“即将提交”的状态
- 而不是零散的文件操作

**系统行为**

- 将 Task 关联的实现结果同步到外部 Repo
- 同步过程保持：
  - 内容一致
  - 边界清晰
  - 可被独立验证

**关键结果**

> 外部世界第一次“看见”这个 Task 的结果。

---

### UJ-GIT-04｜Git 记录一次外部事实

**触发方式**

- 外部版本控制系统接受 Commit

**用户视角**

- 对用户而言，这是一次普通的 Git Commit
- 但其含义已发生变化

**系统行为**

- Git 生成一个不可变的 Commit 记录
- Commit 成为：
  - 时间锚点
  - 外部可验证证据
  - 团队共识的一部分

**关键结果**

> Task 从“内部叙事”变为“外部事实”。

---

### UJ-GIT-05｜回写 Commit 结果并更新 Task 状态

**触发方式**

- 外部 Commit 成功完成

**用户视角**

- 用户看到 Task 状态发生变化
- 无需额外操作

**系统行为**

- 将 Commit 的关键信息回写至 Task
- Task 状态更新为 `Committed`
- Task 被标记为：
  - 已完成一次现实验证
  - 可进入归档与学习阶段

**关键结果**

> Task 被正式认定为：  
> **“一次已被世界接受的决策”**。

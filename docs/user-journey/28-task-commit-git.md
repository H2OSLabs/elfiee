

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

### UJ-GIT-01｜发起 Task Commit（意图确认）

**场景** 当任务达到阶段性终点时，用户或 AI 助理发起请求，准备将成果交付给外部工程环境。

**场景流程**

- 用户显式发出指令（如点击“提交任务”），或 AI 逻辑判断当前 Task 已满足预设的 `Committable` 条件。
    
- 系统接收请求，启动 Task Commit 事务流程。
    
- 系统维持 Task 的 `InProgress` 状态，直至外部确认结果返回。
    

**预期结果**

- **判断请求化：** Commit 被视为一种“判断请求”，而非简单的保存动作。
    
- 用户表达的是一种确定性：_“我认为这个问题，已经被解决到可以被外部世界检验的程度。”_
    

---

### UJ-GIT-02｜收敛 Task 的实现边界

**场景** 系统自动理清“哪些改动属于这个任务”，确保提交内容的纯净与精准。

**场景流程**

- Commit 流程启动，系统检索该 Task 的因果关系图谱（Causal Graph）。
    
- 系统自动识别所有被标记为“为该 Task 而存在”的 Block 及其变更记录。
    
- 系统过滤掉与当前 Task 无关的其他并发修改或临时调试代码。
    

**预期结果**

- **因果边界清晰：** Commit 不再是“目录差异（Diff）”，而是 **意图 → 实现** 的精准映射。
    
- 用户摆脱了“从 50 个修改文件中勾选 5 个”的低效操作。
    

---

### UJ-GIT-03｜同步实现结果到外部世界

**场景** 系统将确定的实现边界转化为外部版本控制系统可理解的物理操作。

**场景流程**

- 系统将 Task 关联的 Block 变更提取为文件补丁（Patches）。
    
- 系统调用外部 Git 接口，将这些变更同步至外部 Repository 的暂存区。
    
- 系统确保同步过程的原子性，确保边界清晰且可被独立编译或验证。
    

**预期结果**

- **外部可见性：** 外部工程环境第一次“看见”并准备接收这个 Task 的逻辑产出。
    
- 用户感知到的是一个整体成果的迁移，而非零散的文件操作。
    

---

### UJ-GIT-04｜Git 记录一次外部事实

**场景** 外部版本控制系统正式接受该变更，产生不可篡改的凭证。

**场景流程**

- 外部 Git 系统生成一个唯一的 Commit SHA。
    
- 系统将 Task 的描述、参与的 Agent 以及决策链路（Reasoning Path）的部分摘要作为 Commit Message 写入。
    
- 该记录正式存入 Git 历史，成为团队协作的共识部分。
    

**预期结果**

- **叙事转事实：** Task 从系统的“内部叙事”转变为真实世界的“外部事实”。
    
- 产生了一个永久的时间锚点和可供审计的证据。
    

---

### UJ-GIT-05｜回写 Commit 结果并更新 Task 状态

**场景** 系统根据外部 Git 的成功反馈，完成内部状态的闭环。

**场景流程**

- 系统捕捉到外部 Commit 成功的信号，获取生成的 Commit ID。
    
- 系统将该 ID 回写至 Task Block，建立双向引用。
    
- 系统将 Task 状态正式更新为 `Committed`。
    

**预期结果**

- **决策认定：** Task 被正式认定为：**“一次已被世界接受的决策”**。
    
- 任务进入归档与学习阶段，为后续的 AI 推理提供高价值的历史样本。

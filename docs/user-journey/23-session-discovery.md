

> **格式约定：** 本文档描述系统如何发现、识别并定位外部 AI 工具产生的 Session（会话记录），
> 作为后续同步、解析与归因的前置 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 产品语义**，不涉及 API 与实现细节。

---

## 1. Session Discovery 的产品定义

在 Elfiee 中，Session 包括：

- AI 实际发生过的推理与行动历史
- 外部工具（如 Claude Code）与 Elfiee 内部工具产生的 artifacts 的关联关系 #待确认 

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

### UJ-SESS-01｜识别外部 AI 工具环境

**场景** 用户在 **effiee** 中进入特定项目，系统需要静默确定协作边界。

**场景流程**

- 用户打开/切换 Project，或在项目中启用 Agent。
    
- 系统自动检索当前 Project 绑定的外部工作目录（Working Directory）。
    
- 系统嗅探该目录下是否存在已知的外部 AI 工具配置或环境标识。
    
- 系统记录该 Project 与外部环境的上下文关联。
    

**预期结果**

- **零配置体验：** 用户无需手动输入路径或告诉系统“AI 在哪里工作”。
    
- 系统成功建立项目与外部物理路径的逻辑连接。
    

---

### UJ-SESS-02｜定位 Session 根目录

**场景** 环境确认后，系统需要精准找到 AI 存储其对话或思考记录的具体位置。

**场景流程**

- 在确认外部环境后，系统根据预设的工具规则库推导 Session 的存储位置。
    
- 系统建立 Project ↔ Session Root 的映射关系。
    
- 系统验证该位置的访问稳定性与数据的增量读取可行性。
    

**预期结果**

- **工具适配：** **effiee** 主动适配外部工具的存储习惯，而非强迫工具迁移数据。
    
- 确定一个稳定的、可供后续持续监听的数据源头。
    

---

### UJ-SESS-03｜发现可用 Session（进入可监听状态）

**场景** 系统已锁定了数据位置，开始对具体的会话记录进行扫描和索引。

**场景流程**

- 系统扫描 Session 根目录的文件结构。
    
- 自动识别已完成的历史 Session 和当前正在写入的 Active Session。
    
- 系统内部标记出哪些 Session 属于“待追踪”或“新发现”状态。
    

**预期结果**

- **感知激活：** AI 的“思考现场”第一次被系统正式捕捉。
    
- 虽然 UI 层面可能尚未展示，但底层已进入“可感知 AI 行为”的就绪状态。
    

---

### UJ-SESS-04｜项目变化触发 Discovery 更新

**场景** 用户切换工作上下文或调整配置时，系统自动保持同步。

**场景流程**

- 用户切换 Project、更改工作目录或启停 Agent。
    
- 系统感知触发信号，立即重新执行 Session Discovery 逻辑。
    
- 系统更新 Project ↔ Session Root 映射，并切断对失效上下文的监听。
    

**预期结果**

- **语境一致性：** Session 发现结果始终随 Project 的切换而无缝漂移。
    
- 用户无需手动点击“刷新”或“重置”即可获得最新的 AI 协作视图。
    

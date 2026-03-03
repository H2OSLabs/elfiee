

> **格式约定：** 本文档描述项目库（Dashboard）管理的完整 User Journey，覆盖项目列表浏览、创建/打开/删除、状态概览与最近打开等场景。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 交互意义**，不涉及 API 与实现细节。

---

## 1. Dashboard 的产品定义

在 Elfiee 中，Dashboard 是用户打开应用后的首屏界面，承担 Project 库（Library）的角色。它是用户与所有 `.elf` Project 之间的唯一入口，负责展示、组织和管理用户的全部工作上下文。

Dashboard 不仅是文件列表，更是用户工作状态的全景视图：
- 哪些 Project 正在活跃开发中
- 哪些 Project 已完成归档
- 哪些 Project 最近被访问过
- 每个 Project 的当前健康状态（Task 进度、Block 数量等）

Dashboard 的设计目标是让用户在 **3 秒内** 找到需要继续工作的 Project，或在 **10 秒内** 创建一个新 Project 并开始工作。

---

## 2. Dashboard 在整体架构中的位置

```
用户启动 Elfiee
 ↓
Dashboard（首屏）  ← 全局 Project 入口
 ├─ 浏览 Project 列表
 ├─ 新建 Project（→ 创建 .elf 文件）
 ├─ 打开 Project（→ Engine Actor 启动）
 └─ 删除 / 归档 Project
 ↓
进入 Project 编辑器
 ↓
Block 编辑 / Task 执行 / Timeline 查看 ...
```

如果没有 Dashboard：
- 用户需要依赖操作系统文件管理器定位 `.elf` 文件，缺乏上下文信息
- 无法快速判断哪些 Project 需要关注，哪些已经完成
- 多 Project 场景下工作切换成本极高

---

## 3. User Journey Index（Dashboard）

| Journey ID   | 角色   | 场景描述                    | 结果                       |
| ------------ | ---- | ----------------------- | ------------------------ |
| UJ-DASH-001 | User | 浏览 Project 列表并按条件排序/筛选 | 快速定位目标 Project           |
| UJ-DASH-002 | User | 新建 Project 并初始化工作上下文    | `.elf` 文件创建，Project 进入就绪状态 |
| UJ-DASH-003 | User | 打开已有 Project 继续工作       | Engine 启动，状态从事件日志恢复      |
| UJ-DASH-004 | User | 删除不再需要的 Project         | Project 安全移除，不影响其他数据     |
| UJ-DASH-005 | User | 通过最近打开快速回到上次工作          | 零搜索成本恢复工作上下文             |

---

## 4. 详细 User Journey

### UJ-DASH-001｜浏览 Project 列表并按条件排序/筛选

**场景**
用户拥有多个 `.elf` Project（如不同的实验、产品线或阶段性工作），需要在 Dashboard 中快速浏览并定位到目标 Project。项目数量可能从几个到几十个不等，因此需要有效的排序与筛选机制。

**流程**

- 用户打开 Elfiee，进入 Dashboard 首屏，系统展示所有已知的 `.elf` Project 列表。
- 每个 Project 卡片展示关键摘要信息：项目名称、最后修改时间、当前状态（活跃 / 已归档 / 空项目）、Block 数量、活跃 Task 数量。
- 用户可按名称、修改时间、状态等维度排序，或通过关键词搜索快速筛选。

**预期结果**

- **全景可见：** 用户一目了然地掌握所有 Project 的当前状态，无需逐个打开。
- **状态可区分：** 活跃、已归档、空项目通过视觉标识清晰区分，避免混淆。
- **高效定位：** 排序与搜索功能确保用户在项目数量增长时仍能快速找到目标。

---

### UJ-DASH-002｜新建 Project 并初始化工作上下文

**场景**
用户准备启动一项新的工作（新实验、新产品线、新研究课题），需要在 Elfiee 中创建一个全新的 Project 作为工作容器。

**流程**

- 用户在 Dashboard 中点击"新建 Project"，输入项目名称并选择存储位置。
- 系统创建一个新的 `.elf` ZIP 归档文件，初始化空的 `_eventstore.db`、`_snapshot` 及相关派生缓存。
- 新 Project 立即出现在 Dashboard 列表中，状态标记为空项目。
- 用户可选择立即打开进入编辑，或稍后再开始。

**预期结果**

- **即时可用：** 新 Project 创建后即具备完整的事件溯源基础设施，可以开始记录任何操作。
- **Dashboard 同步：** 新项目在 Dashboard 中立即可见，无需手动刷新。
- **最小化启动成本：** 从点击"新建"到 Project 就绪，用户操作不超过 3 步。

---

### UJ-DASH-003｜打开已有 Project 继续工作

**场景**
用户需要继续之前中断的工作，或查看某个 Project 的当前状态。Dashboard 需要支持快速打开 Project，并确保 Engine 从 `_eventstore.db` 恢复到最新状态。

**流程**

- 用户在 Dashboard 中点击目标 Project 卡片，或双击打开。
- 系统为该 `.elf` 文件启动一个独立的 Engine Actor，从 `_eventstore.db` 重放事件日志恢复内存状态。
- Project 编辑器界面加载完成，展示所有 Block、Task、Timeline 等内容。
- Dashboard 更新该 Project 的"最近打开时间"。

**预期结果**

- **状态完整恢复：** 打开后的 Project 状态与上次关闭时完全一致，包括 Block 内容、Task 进度、Editor 权限。
- **独立 Engine：** 每个打开的 Project 拥有独立的 Engine Actor，多 Project 之间互不影响。
- **无感切换：** 用户可以在 Dashboard 与已打开的 Project 之间自由切换，不丢失工作状态。

---

### UJ-DASH-004｜删除不再需要的 Project

**场景**
某些 Project 已经过时、实验失败或重复创建，用户希望从 Dashboard 中移除它们以保持工作区整洁。

**流程**

- 用户在 Dashboard 中选中目标 Project，触发删除操作。
- 系统弹出确认对话框，明确告知删除将移除 `.elf` 文件及其全部历史数据，且此操作不可撤销。
- 用户确认后，系统将 `.elf` 文件移至系统回收站（或直接删除，取决于配置）。
- Dashboard 列表立即更新，移除该 Project。

**预期结果**

- **安全确认：** 删除操作需要显式确认，防止误删重要 Project。
- **完整移除：** 删除后 Dashboard 中不再显示该 Project，不留残余元数据。
- **不影响其他 Project：** 每个 `.elf` 文件是自包含的，删除一个不影响其他任何 Project 的数据。

---

### UJ-DASH-005｜通过最近打开快速回到上次工作

**场景**
用户频繁在多个 Project 之间切换，或每天启动 Elfiee 后需要立即回到昨天的工作。Dashboard 需要提供"最近打开"功能，将最常用或最近访问的 Project 置于最醒目的位置。

**流程**

- 用户打开 Elfiee，Dashboard 首屏顶部展示"最近打开"区域，按最后访问时间倒序排列。
- 最近打开的 Project 展示更丰富的上下文信息：上次停留的 Block、未完成的 Task 数量、上次操作时间。
- 用户点击即可一键恢复上次的工作状态。

**预期结果**

- **零搜索成本：** 用户无需在列表中翻找，最近工作的 Project 自动出现在最显眼的位置。
- **上下文连续：** 不仅快速打开 Project，还能快速回忆上次的工作进度与上下文。
- **工作惯性保持：** 降低每次启动应用时的"启动摩擦"，让用户以最少的认知成本进入工作状态。


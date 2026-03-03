

> **格式约定：** 本文档描述 Block 基本操作的完整 User Journey，覆盖 Block 创建、类型系统、元数据查看、快照生成、以及 Block 与 Capability 的关系等场景。
> 仅描述 **用户视角 / 系统行为 / 状态变化 / 模型意义**，不涉及 API 与实现细节。

---

## 1. Block 的产品定义

在 Elfiee 中，Block 是最基本的内容单元。每一段文本、每一份代码、每一个 Task 都是一个 Block。Block 具有以下核心属性：

- **`block_id`**（UUID）— 全局唯一标识，创建后不可变
- **`block_type`** — 类型标识（如 `markdown`、`code`、`task`），决定了该 Block 可使用哪些 Capability
- **`contents`**（JSON）— Block 的实际内容，结构由 `block_type` 对应的 Payload Schema 定义
- **`children`** — 关系图，描述 Block 与其他 Block 的关联（如 Task 与其实现代码的 `implement` 关系）

Block 是 Elfiee 事件溯源架构的核心实体。对 Block 的每一次操作（创建、编辑、关联、删除）都会生成不可变的 Event，记录到 `_eventstore.db` 中。这意味着 Block 的完整生命周期 — 从诞生到当前状态的每一个中间版本 — 都是可追溯的。

---

## 2. Block 在整体架构中的位置

```
用户意图（创建 / 编辑 / 关联）
 ↓
Command（携带 cap_id + payload）
 ↓
Engine Actor
 ├─ Certificator 授权校验（CBAC）
 └─ Handler 执行逻辑
 ↓
Event 生成（EAVT）
 ├─ Entity   = block_id
 ├─ Attribute = "{editor_id}/{cap_id}"
 ├─ Value    = JSON payload（内容变更）
 └─ Timestamp = Vector Clock
 ↓
_eventstore.db  ← 持久化
 ↓
State Projection  → Block 内存状态
 ↓
_snapshot 更新  ← 渲染缓存
```

如果没有 Block 抽象：
- 内容只是文件中的文本片段，缺乏身份、类型与关系
- 无法追踪"谁在什么时候修改了什么"
- 无法建立 Task → 代码实现 → 测试结果的因果链

---

## 3. User Journey Index（Block Fundamentals）

| Journey ID    | 角色          | 场景描述                           | 结果                        |
| ------------- | ----------- | ------------------------------ | ------------------------- |
| UJ-BLOCK-001 | User        | 创建不同类型的 Block                  | Block 成为独立可追溯对象           |
| UJ-BLOCK-002 | User        | 编辑 Block 内容并观察事件记录             | 内容更新，Timeline 可见变更历史      |
| UJ-BLOCK-003 | User        | 查看 Block 元数据                   | 了解 Block 的身份、类型、创建者与权限信息 |
| UJ-BLOCK-004 | System      | 生成 Block 快照（`_snapshot`）       | 可预览的渲染缓存被更新               |
| UJ-BLOCK-005 | User / System | 建立 Block 之间的关联关系（`children`）  | Block 间形成有向图结构            |

---

## 4. 详细 User Journey

### UJ-BLOCK-001｜创建不同类型的 Block

**场景**
用户在 Project 中需要承载不同性质的内容：文档说明需要 Markdown Block，算法实现需要 Code Block，工作规划需要 Task Block。每种 `block_type` 决定了该 Block 可使用的 Capability 集合，以及 `contents` 的 JSON Schema。

**流程**

- 用户选择要创建的 Block 类型（如 `markdown`、`code`、`task`），系统通过 `core.create` capability 生成一个新 Block。
- 系统为新 Block 分配全局唯一的 `block_id`（UUID），并根据 `block_type` 初始化空的 `contents` 结构。
- 创建操作生成一条 `create` 类型的 Event，记录到 `_eventstore.db`，包含创建者的 `editor_id`。
- 新 Block 在编辑器中可见，创建者自动成为该 Block 的 owner，拥有全部 Capability 权限。

**预期结果**

- **独立可追溯：** 每个 Block 拥有唯一 `block_id`，从创建瞬间起即可被引用、关联与追溯。
- **类型即能力：** `block_type` 决定了可用的 Capability（如 `markdown` 类型的 Block 可使用 `markdown.write`，`code` 类型的 Block 可使用代码相关 Capability）。
- **Owner 权限：** 创建者自动获得该 Block 的全部操作权限，无需额外授权。

---

### UJ-BLOCK-002｜编辑 Block 内容并观察事件记录

**场景**
用户创建 Block 后开始填写或修改内容。每次编辑操作通过对应的 Capability Handler 执行（如 `markdown.write`），并生成不可变的 Event。用户可以在 Timeline 中看到完整的修改历史。

**流程**

- 用户在编辑器中修改 Block 的 `contents`（如在 Markdown Block 中输入文本）。
- 系统通过对应 Capability 的 Handler 处理变更，生成携带 JSON payload 的 Event。
- Event 以 EAVT 格式写入 `_eventstore.db`，Attribute 字段记录 `{editor_id}/{cap_id}`，标识"谁执行了什么操作"。
- State Projection 更新内存中的 Block 状态，编辑器实时反映变更。

**预期结果**

- **实时反馈：** 编辑内容后，编辑器立即展示更新后的 Block 状态。
- **完整历史：** 每次修改都生成独立的 Event，用户可在 Timeline 中看到从创建到当前的全部变更序列。
- **归因可追溯：** 每条 Event 的 Attribute 明确记录了操作者（`editor_id`）与操作类型（`cap_id`），支持"谁在什么时候做了什么"的完整审计。

---

### UJ-BLOCK-003｜查看 Block 元数据

**场景**
用户需要了解某个 Block 的身份信息、类型属性、创建者、权限状态等元数据，以便决定是否可以编辑、是否需要申请权限、或理解该 Block 在 Project 中的角色。

**流程**

- 用户选中目标 Block，打开元数据面板或信息弹窗。
- 系统展示 Block 的核心元数据：`block_id`、`block_type`、创建时间（首条 Event 的时间戳）、最后修改时间、owner（创建者 `editor_id`）。
- 系统展示该 Block 的权限状态：当前用户拥有哪些 Capability Grant，是否可读、可写、可关联。
- 系统展示 `children` 关系列表：该 Block 关联了哪些其他 Block，以什么关系类型关联。

**预期结果**

- **身份透明：** 用户清楚地知道"这个 Block 是什么、谁创建的、什么时候创建的"。
- **权限可见：** 用户在操作前即可了解自己是否有权编辑该 Block，避免操作后被授权拒绝。
- **关系图可见：** 通过 `children` 列表，用户理解该 Block 在 Project 知识图谱中的位置与连接。

---

### UJ-BLOCK-004｜生成 Block 快照（`_snapshot`）

**场景**
当 Block 内容发生变更时，系统需要更新 `_snapshot` 缓存，生成该 Block 的可预览渲染结果（如 Markdown 渲染为 HTML）。快照是派生数据，用于加速预览和搜索，不影响事件日志的完整性。

**流程**

- Block 内容变更后，Engine 触发快照更新流程。
- 系统调用该 `block_type` 对应 Extension 的渲染方法 `(block: Block) => string`，生成可预览的文本表示。
- 渲染结果写入 `.elf` 归档中的 `_snapshot` 缓存。
- 快照用于 Dashboard 预览、搜索索引和快速浏览。

**预期结果**

- **预览加速：** 快照作为缓存，用户浏览 Block 列表时无需实时渲染，降低延迟。
- **派生数据语义：** `_snapshot` 是从 `_eventstore.db` 派生的缓存，丢失后可通过事件重放重建，不构成数据风险。
- **类型感知渲染：** 不同 `block_type` 的 Block 使用各自 Extension 定义的渲染逻辑，确保预览结果与编辑器中的渲染一致。

---

### UJ-BLOCK-005｜建立 Block 之间的关联关系（`children`）

**场景**
用户需要在 Block 之间建立语义关联，例如将 Code Block 关联为某个 Task Block 的实现（`implement` 关系），或将多个 Block 组织为文档的章节层级。Block 的 `children` 字段存储了这些有向关系。

**流程**

- 用户选中源 Block，通过拖拽或操作菜单将其关联到目标 Block，并选择关系类型（如 `implement`、`reference`）。
- 系统通过 `core.link` capability 处理关联请求，生成一条 `link` 类型的 Event。
- 源 Block 的 `children` 关系图更新，新增到目标 Block 的有向关系。
- `_blocks_relation` 缓存同步更新，反映最新的关系图。

**预期结果**

- **因果关系显式化：** Block 之间的关联从隐式的"同在一个文件中"变为显式的、有类型的有向关系。
- **知识图谱构建：** 随着关联的积累，Project 中的 Block 逐渐形成一个可导航的知识图谱，支持"谁定义了我""我实现了什么"等查询。
- **事件可追溯：** 关联操作同样以 Event 形式记录，用户可以在 Timeline 中看到"什么时候、谁建立了这个关联"。

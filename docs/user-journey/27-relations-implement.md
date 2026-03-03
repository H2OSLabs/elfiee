
## User Journey Index（implement 关系）

| Journey ID | 角色 | 场景描述 | 结果 |
| --- | --- | --- | --- |
| UJ-REL-01 | User / AI | 建立 implement 关系 | 因果链条形成 |
| UJ-REL-02 | System | DAG 环检测与拒绝 | 环被阻止，图保持 DAG |
| UJ-REL-03 | User / AI | 反向索引查询（谁实现了我） | 下游实现可追溯 |
| UJ-REL-04 | User / AI | implement 链条追溯 | 完整因果链可视化 |
| UJ-REL-05 | User / AI | 移除 implement 关系 | 因果链条断开，历史保留 |

---

## 详细 User Journey

### UJ-REL-01｜建立 implement 关系（因果链条形成）

**场景** 开发者或 Agent 创建了一个新的 Block（如代码、测试），需要将其与上游的意图 Block（如 Task）建立明确的因果关系。

**场景流程**

- 用户在 GUI 中将 Code Block 拖拽到 Task Block 下方并选择"实现"关系，或 Agent 在执行 Task 时通过 `core.link` Capability 显式声明 implement 关系。

- 系统在目标 Block（Task）的 Block.children 中添加一条 `implement` 类型的关系记录，指向来源 Block（Code）。

- 系统执行 DAG 环检测（见 UJ-REL-02），确认该关系不会引入环。

- 关系建立成功后，系统生成一条 Event，记录关系的建立者、时间与涉及的双方 Block。

- Task 状态可能因首次建立 implement 关系而自动转为 `InProgress`。

**预期结果**

- **因果显式化：** 代码与任务之间不再是松散的、需要人脑记忆的关联，而是系统中可查询、可追溯的一等关系。

- 从 Task 出发，可以直接看到"有哪些 Block 正在实现这个意图"。

---

### UJ-REL-02｜DAG 环检测与拒绝（图完整性保护）

**场景** 在建立新的 implement 关系时，系统需要确保关系图不会出现环（A implements B implements C implements A），以保证因果链的方向性和可解释性。

**场景流程**

- 系统在每次执行 `core.link`（关系类型为 implement）之前，对关系图执行环检测。

- 系统从目标 Block 出发，沿 implement 关系向下游遍历，检查是否能回到来源 Block。

- 若检测到环，系统拒绝该关系的建立，并返回明确的错误信息，指出环路径。

- 若未检测到环，关系建立正常继续。

**预期结果**

- **DAG 不变量：** 系统保证 implement 关系图在任何时刻都是有向无环图，这是因果追溯的数学基础。

- 环检测是前置校验，不是事后修复。错误的关系永远不会被写入 EventStore。

- 用户收到的错误信息足够清晰，可以理解"为什么这条关系不被允许"。

---

### UJ-REL-03｜反向索引查询（谁实现了我）

**场景** 用户或 Agent 需要从某个 Block 出发，查看"有哪些下游 Block 实现了它"。

**场景流程**

- 用户在 GUI 中点击某个 Task Block 的"查看实现"，或 Agent 通过能力查询某个 Block 的反向 implement 关系。

- 系统查询反向索引，返回所有通过 implement 关系指向该 Block 的下游 Block 列表。

- 返回结果包含每个下游 Block 的基本信息（Block ID、Block Type、创建者、创建时间）。

- 若该 Block 没有任何实现者，系统返回空列表，表示"尚无实现"。

**预期结果**

- **下游可见性：** 任何 Block 都可以回答"谁在实现我"这一问题，无需人工维护清单。

- 反向索引查询是 O(1) 级别的查找（通过 `_blocks_relation` 缓存），不需要全图遍历。

- 对于 Task Block，这个查询直接回答了"这个任务的实现进度如何"。

---

### UJ-REL-04｜implement 链条追溯（完整因果链）

**场景** 用户或 Agent 需要从某个末端 Block（如 Test）出发，追溯整条 implement 链条直到最上游的意图 Block（如 Task）。

**场景流程**

- 用户在 GUI 中选择"追溯因果链"，或 Agent 沿 implement 关系进行递归向上查询。

- 系统从当前 Block 出发，沿 implement 关系反向遍历，逐层收集上游 Block。

- 系统构建完整的因果链路：Task -> Code -> Test（或更深层级的链条）。

- 系统以可视化形式展示因果链，或以结构化数据返回给 Agent。

**预期结果**

- **端到端追溯：** 从任何 Block 都可以回答"这个东西最终是为了什么目的而存在的"。

- 因果链条是 Commit 阶段自动收敛实现边界的基础：系统沿 implement 链条向下收集所有相关 Block，即为该 Task 的提交范围。

- 在复盘与 Archive 阶段，因果链条提供了"从意图到结果"的完整叙事。

---

### UJ-REL-05｜移除 implement 关系（因果链条断开）

**场景** 用户发现某个 implement 关系是错误建立的，或因重构需要调整 Block 之间的因果归属。

**场景流程**

- 用户在 GUI 中选择"解除实现关系"，或 Agent 通过 `core.unlink` Capability 移除 implement 关系。

- 系统从目标 Block 的 Block.children 中删除对应的 implement 记录。

- 系统生成一条 Event，记录关系的移除者、时间与涉及的双方 Block。

- 被断开的下游 Block 不会被删除，仅失去与上游 Block 的因果关联。

- 移除关系不需要执行环检测（删除边不可能引入环）。

**预期结果**

- **可修正性：** 错误建立的关系可以被安全移除，不会留下不一致的状态。

- 历史上该关系存在过的事实被完整保留在 EventStore 中（先有 link Event，后有 unlink Event），可被审计。

- 被断开的 Block 变为"孤立实现"，可能在 GC 策略中被标记为候选清理对象（参见 UJ-GC 系列）。


## User Journey Index（Ordering & Clocks）

| Journey ID   | 角色            | 场景描述                         | 结果                              |
| ------------ | ------------- | ---------------------------- | ------------------------------- |
| UJ-CLOCK-001 | User          | 跨 Editor 事件在 Timeline 中的统一排序 | 多个 Editor 的事件在同一 Timeline 中有序展示 |

---

## 详细 User Journey

### UJ-CLOCK-001｜跨 Editor 事件在 Timeline 中的统一排序

**场景**
Project 中有多个 Editor（如 Human alice、Bot agent-1、Human bob）的操作事件交织在一起。用户在 Timeline UI 中需要看到所有 Editor 的事件按统一顺序排列，形成一条连贯的 Project 演变时间线。

**流程**

- Timeline UI 从 `_eventstore.db` 加载所有事件，不区分 Editor 来源。
- 系统以 Vector Clock 的因果顺序作为主排序依据，将所有 Editor 的事件合并为一条统一的时间线。
- 对于并发事件（Vector Clock 不可比较），以 Wall Clock 作为辅助排序。
- Timeline 中每条事件标注其 Editor 身份，用户可以通过颜色或图标区分不同 Editor 的操作。
- 用户可以按 Editor 筛选 Timeline，只查看某个 Editor 的操作历史。

**预期结果**

- **全局视图：** 所有 Editor 的操作在同一条 Timeline 中有序展示，用户可以看到完整的 Project 演变过程。
- **归因清晰：** 每条事件的 Editor 身份一目了然，用户清楚"谁在什么时候做了什么"。
- **可筛选性：** 用户可以按 Editor 过滤，聚焦于某个协作者的操作轨迹。
- **排序一致性：** 无论何时查看 Timeline，相同的事件始终以相同的顺序展示。


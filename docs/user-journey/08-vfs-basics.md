

> **格式约定：** 本文档描述 **VFS（虚拟文件系统）的基础概念：路径到 BlockID 的映射与硬链接语义** 的完整 User Journey。
> 仅描述 **用户视角 / 系统行为 / 状态变化**，不涉及 API 与实现细节。

---

## VFS 

Elfiee 的虚拟文件系统（VFS）为用户提供了一个熟悉的"目录 + 文件"视图来组织和浏览 Block。

## User Journey Index（VFS Basics）

| Journey ID | 角色   | 场景描述                         | 结果                     |
| ---------- | ---- | ---------------------------- | ---------------------- |
| UJ-VFS-01  | User | 理解路径与 Block 的映射关系            | 认知建立：路径是别名，BlockID 是身份 |
| UJ-VFS-02  | User | 为同一 Block 创建多个路径引用           | 同一 Block 出现在多个目录下      |
| UJ-VFS-03  | User | 通过不同路径修改同一 Block             | 所有路径下内容同步更新            |
| UJ-VFS-04  | User | 在 Directory 视图中浏览 Project 结构 | 以文件树形式组织和导航 Block      |


---

## 详细 User Journey

### UJ-VFS-01｜理解路径与 Block 的映射关系

**场景**
新用户第一次使用 Elfiee 的目录视图，需要理解"文件列表中看到的条目"与"实际 Block 对象"之间的关系。这个认知模型是使用 VFS 的前提。

**流程**

- 用户在 Directory 视图中看到类似文件树的结构，包含目录和"文件"条目。
- 用户点击某个条目，系统打开对应的 Block 进行编辑。
- 用户注意到条目的属性信息中显示了 `block_id`——这是 Block 的真实身份标识，而路径只是一个人类可读的别名。
- 系统通过 UI 提示帮助用户理解：路径是 dentry，Block 是实体，两者是多对一关系。

**预期结果**

- **认知建立**：用户理解 Elfiee 中"路径"与"内容"的分离模型。
- **身份认知**：用户知道 `block_id` 才是 Block 的唯一标识，路径只是访问入口。
- **预期校准**：用户对后续的多路径引用、dentry 操作不会感到意外。

---

### UJ-VFS-02｜为同一 Block 创建多个路径引用

**场景**
用户有一份 API 文档 Block，希望它同时出现在 `/docs/api/` 和 `/tasks/sprint-3/reference/` 两个目录下，且保持内容一致。传统做法是复制文件，但在 Elfiee 中只需创建第二个 dentry。

**流程**

- 用户在目录视图中找到已有的 Block（如 `/docs/api/auth.md`）。
- 用户在目标目录（如 `/tasks/sprint-3/reference/`）中执行"创建引用"操作，指向同一个 Block。
- 系统在目标目录下创建新的 dentry，映射到相同的 `block_id`。
- 两个路径下显示相同的 Block 内容，且 Block 的属性中列出所有指向它的 dentry。

**预期结果**

- **零复制**：不创建 Block 副本，只新增一个路径映射。
- **内容一致**：两个路径下看到的是同一个 Block，内容始终同步。
- **引用透明**：用户可以查看一个 Block 的所有路径引用列表。

---

### UJ-VFS-03｜通过不同路径修改同一 Block

**场景**
用户从 `/tasks/sprint-3/reference/auth.md` 路径进入一个 Block 并做了修改。由于该 Block 也被 `/docs/api/auth.md` 引用，修改需要在两个路径下同步可见。

**流程**

- 用户通过路径 A 打开 Block 并编辑内容。
- 修改产生 Event，记录到 `_eventstore.db`，Entity 为 `block_id`（而非路径）。
- 通过路径 B 访问同一 Block 时，显示的是修改后的最新状态。
- 无论从哪个路径进入编辑，产生的 Event 在 Timeline 中指向同一个 Block。

**预期结果**

- **自动同步**：无需手动同步——因为只有一个 Block，不存在"不同步"的可能。
- **路径无关**：Event 记录的 Entity 是 `block_id`，不受访问路径影响。
- **Timeline 统一**：该 Block 的所有变更历史在同一条 Timeline 上，无论从哪个路径触发。

---

### UJ-VFS-04｜在 Directory 视图中浏览 Project 结构

**场景**
用户需要以文件树形式浏览 Project 中的所有内容，快速定位目标 Block。Directory 视图提供了一个熟悉的文件管理器体验，但底层是 Block 的组织视图。

**流程**

- 用户打开 Project 的 Directory 面板，看到树状目录结构。
- 目录节点可展开/收起，每个叶子节点是一个 dentry（指向某个 Block）。
- 用户可以通过路径名搜索或层级浏览定位目标。
- 双击 dentry 打开对应 Block 的编辑视图。
- 目录结构本身也是 Block 关系的一种投影——目录节点的 `children` 关系定义了树状层级。

**预期结果**

- **熟悉体验**：目录视图提供类似文件管理器的交互模式，降低学习成本。
- **组织灵活**：用户可以自由创建目录层级来组织 Block，不受 Block 自身结构约束。
- **视图而非实体**：目录结构是 Block 关系的一种视图投影，不是独立的数据实体。


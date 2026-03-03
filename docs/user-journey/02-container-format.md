

## User Journey Index

| Journey ID | 角色     | 场景描述                      | 结果                   |
| ---------- | ------ | ------------------------- | -------------------- |
| UJ-ELF-001 | User   | 保存 Project 并验证持久化完整性      | 容器原子写入，重启后状态完整恢复     |
| UJ-ELF-002 | User   | 导出 `.elf` 文件作为可迁移资产       | 自包含归档，跨设备/跨用户完整可用    |


---

## 详细 User Journey

### UJ-ELF-001｜保存 Project 并验证持久化完整性

**场景**
用户完成阶段性工作后保存 Project。保存操作需要将内存中的 Engine 状态安全地持久化到 `.elf` ZIP 归档中，确保 `_eventstore.db` 中的事件链完整无损，且所有 Block 资产目录均被正确打包。

**流程**

- 用户触发保存操作，系统将当前 Engine Actor 的状态序列化为 `.elf` ZIP 归档。
- `_eventstore.db` 作为核心被原子写入，同时更新 `_snapshot`、`_blocks_hash`、`_blocks_relation` 等派生缓存。
- 用户关闭应用后重新打开该 `.elf` 文件，Engine 通过重放 `_eventstore.db` 中的事件日志恢复内存状态。

**预期结果**

- **原子性保障：** 保存操作要么完整成功，要么不改变原文件，不会出现半写入的损坏状态。
- **状态完整恢复：** 重新打开后，所有 Block、Editor、Grant 关系与 Timeline 均与保存前一致。
- **事件连续性：** `_eventstore.db` 中的事件链无断裂，Vector Clock 时间戳保持单调递增。

---

### UJ-ELF-002｜导出 `.elf` 文件作为可迁移资产

**场景**
用户需要将 Project 交给协作者，或将工作从一台设备迁移到另一台设备。`.elf` 文件作为自包含的 ZIP 归档，承载了 Project 的全部状态与历史，是 Elfiee 中 Project 可移植性的基础。

**流程**

- 用户将 `.elf` 文件通过文件系统复制、云盘同步或直接传输交给目标方。
- 接收方在新环境中使用 Elfiee 打开该 `.elf` 文件。
- Engine 在新环境中从 `_eventstore.db` 重放事件，重建完整的 Project 状态。
- `_block_dir` 等运行时路径在新环境中被自动重新注入，不依赖原始路径。

**预期结果**

- **自包含迁移：** `.elf` 文件包含 Project 的全部信息，无需额外配置文件或外部依赖。
- **完整可用：** 所有 Block 内容、Task 状态、Archive 记录、Editor 权限与 Timeline 在新环境中完整还原。
- **路径无关性：** `_block_dir` 在运行时动态注入，不持久化到事件中，确保 `.elf` 文件在任意文件系统路径下均可正常工作。

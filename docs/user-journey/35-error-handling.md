
## User Journey Index（Error Handling）

| Journey ID | 角色   | 场景描述         | 结果               |
| ---------- | ---- | ------------ | ---------------- |
| UJ-ERR-01  | User | 授权错误的友好提示    | 用户理解权限不足并知道如何获取  |


---

## 详细 User Journey

### UJ-ERR-01｜授权错误的友好提示

**场景** 用户尝试执行一个操作（如写入某个 Block），但 CBAC 校验发现其缺少对应的 Capability Grant。

**场景流程**

- 用户在 UI 中触发操作（如编辑一个 Markdown Block）。

- 后端 Engine 的 certificator 检查 GrantsTable，发现当前 Editor 不具备该 Block 上的 `markdown.write` Capability。

- 后端返回授权错误，包含结构化信息：缺少的 Capability、目标 Block ID、当前 Editor ID。

- 前端将错误映射为用户友好的提示，例如：
    - "你没有编辑这个内容块的权限。"
    - "需要 Block 所有者授予你编辑权限才能继续。"

- 提示中不暴露 `capability_id`、`block_id` 等技术标识符，而是使用 Block 的显示名称。

**预期结果**

- **无术语泄漏：** 用户看到的是"你没有编辑权限"，而非 `CapabilityDenied: editor_abc lacks markdown.write on block_xyz`。

- 用户清楚地知道问题出在"权限"而非"系统故障"，心理预期正确，不会产生恐慌。

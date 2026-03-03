
## User Journey Index（MyST Markdown）

| Journey ID | 角色   | 场景描述                  | 结果                |
| ---------- | ---- | --------------------- | ----------------- |
| UJ-MD-001  | User | 编辑基础 Markdown 内容并实时预览 | 所见即所得的文档编辑体验      |




---

## 详细 User Journey

### UJ-MD-001｜编辑基础 Markdown 内容并实时预览

**场景**
用户在 Markdown Block 中编写文档内容，包括标题、段落、列表、链接、图片引用等标准 Markdown 元素。编辑器需要在用户输入的同时提供实时预览反馈，让用户在编辑过程中即可看到渲染效果。

**流程**

- 用户在 Markdown Block 的编辑区域中输入标准 Markdown 语法（如 `# 标题`、`- 列表项`、`**粗体**`）。
- 编辑器实时解析 Markdown 语法，在预览区域渲染对应的格式化输出。
- 用户完成编辑后，系统通过 `markdown.write` capability 将变更提交到 Engine。
- Engine 生成 Event 并更新 `_snapshot` 缓存。

**预期结果**

- **实时反馈：** 用户输入 Markdown 语法后，预览区域即时呈现渲染效果，无需手动刷新。
- **语法完整支持：** 标题、段落、列表（有序/无序）、链接、图片、行内代码、代码块、引用块、分割线等标准 Markdown 元素均正确渲染。
- **Block 结构不受影响：** Markdown 内容的编辑仅影响 `Block.contents`，不改变 Block 的 `block_id`、`block_type`、`children` 等结构属性。



> **格式约定：** 这是「User Journey 的索引」，仅包含 **模块分组 / 文件清单 / 用例前缀**，不包含任何具体 user journey 内容。


---

  

## 1   目录

  

### 1.1   项目与容器（Project & .elf Container）

  

| 文件                                                 | 模块                            | 用例前缀    |
| -------------------------------------------------- | ----------------------------- | ------- |
| [01-project-lifecycle](01-project-lifecycle.md)    | 创建 / 打开 / 重命名 / 完成任务/ 归档      | UJ-PROJ |
| [02-container-format.md](02-container-format.md)   | `.elf` 、EventStore、资产目录 | UJ-ELF  |
| [03-dashboard-library.md](03-dashboard-library.md) | 项目库（Dashboard）管理              | UJ-DASH |

  

### 1.2   Block 与编辑器（Blocks & Editor）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [04-block-fundamentals.md](04-block-fundamentals.md) | Block 创建 / 类型 / 元数据 / 快照 | UJ-BLOCK |
| [05-markdown-myst.md](05-markdown-myst.md) | MyST Markdown 编辑与渲染 | UJ-MD |
| [06-code-blocks.md](06-code-blocks.md) | 代码块编辑、语法高亮、执行桥接（含 mocked→Terminal） | UJ-CODE |
| [07-assets-attachments.md](07-assets-attachments.md) | 资源文件（图片/附件）导入与引用 | UJ-ASSET |

  

### 1.3   目录与索引

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [08-vfs-basics.md](08-vfs-basics.md) | Directory 视图：路径→BlockID（硬链接语义） | UJ-VFS |
| [09-vfs-ops.md](09-vfs-ops.md) | 新建/重命名/移动/删除（dentry 语义） | UJ-VFSOP |
| [10-import-export.md](10-import-export.md) | 外部 repo 导入 / 导出 / 同步边界 | UJ-IO |
| [11-garbage-collection.md](11-garbage-collection.md) | 不可达块、根集合、清理策略（可选） | UJ-GC |

  

### 1.4   D. 事件、时间线与回溯（Event Sourcing / Timeline）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [12-event-log.md](12-event-log.md) | 事件记录、不可变审计轨迹、归因（who did what） | UJ-EVENT |
| [13-timeline-ui.md](13-timeline-ui.md) | Timeline 浏览、过滤、跳转 | UJ-TL |
| [14-time-travel.md](14-time-travel.md) | `get_state_at_event`、历史快照、restore 语义 | UJ-TT |
| [15-ordering-clocks.md](15-ordering-clocks.md) | Vector Clock 与 Wall Clock（RFC3339）对齐 | UJ-CLOCK |

  

### 1.5   E. 协作者与权限（Editor Identities & CBAC）

  

| 文件                                                 | 模块                                                | 用例前缀      |
| -------------------------------------------------- | ------------------------------------------------- | --------- |
| [16-editors-human-bot.md](16-editors-human-bot.md) | Human/Bot editor、Active Editor 切换                 | UJ-EDITOR |
| [17-cbac-grants.md](17-cbac-grants.md)             | Capability Grant（triplet：Editor×Capability×Block） | UJ-CBAC   |
| [18-authz-enforcement.md](18-authz-enforcement.md) | 后端 certificator 授权校验、前端不可信边界                      | UJ-AUTHZ  |

  

### 1.6   F. Agent 接入与路由（Agent / MCP / Routing）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [19-agent-block.md](19-agent-block.md) | Agent Block：创建 / 启用 / 禁用 / 绑定项目 | UJ-AGENT |
| [20-mcp-server.md](20-mcp-server.md) | `elfiee mcp-server --elf {path}`、tools/list、tools/call | UJ-MCP |
| [21-skills-templates.md](21-skills-templates.md) | `.elf/Agents/elfiee-client/`、SKILL.md / mcp.json 模板与占位符 | UJ-SKILL |
| [22-routing-layer-baths.md](22-routing-layer-baths.md) | Baths（Synopath）：Agent Register / Agent Router（不含冲突处理） | UJ-ROUTE |

  

### 1.7   G. 会话同步（Session Sync）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [23-session-discovery.md](23-session-discovery.md) | 识别 `~/.claude/projects/{path-hash}`、多项目定位 | UJ-SESS |
| [24-session-ingestion.md](24-session-ingestion.md) | JSONL 增量解析、偏移量、落盘到 `.elf/Agents/session/` | UJ-INGEST |
| [25-session-linking.md](25-session-linking.md) | 会话↔Task/Block 的关联策略（基础版） | UJ-SESSLINK |

  

### 1.8   H. 任务闭环与 Git 映射（Task / Commit / Evidence）

  

| 文件                                                     | 模块                                                    | 用例前缀    |
| ------------------------------------------------------ | ----------------------------------------------------- | ------- |
| [26-task-block.md](26-task-block.md)                   | Task：创建/读写/状态机（Pending→InProgress→Committed→Archived） | UJ-TASK |
| [27-relations-implement.md](27-relations-implement.md) | `implement` 关系、DAG 环检测、反向索引（谁定义了我）                    | UJ-REL  |
| [28-task-commit-git.md](28-task-commit-git.md)         | `task.commit`：导出关联文件→git add/commit→回写证据              | UJ-GIT  |
| [29-task-archive.md](29-task-archive.md)               | `task.archive`：生成归档 Markdown、记录 commit hash 与时间线      | UJ-ARCH |

  

### 1.9   I. 终端与验证（Terminal / Tests as Evidence）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [30-terminal-bridge.md](30-terminal-bridge.md) | 代码执行→终端输出回写（含 error_report 归因） | UJ-TERM |
| [31-evidence-model.md](31-evidence-model.md) | 测试结果/日志作为证据：写入、关联、可追溯检索 | UJ-EVID |

  

### 1.10   J. 从事件进化为 Skill（Learn Loop）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [32-auto-skill-generator.md](32-auto-skill-generator.md) | 监听 `task.commit` → 生成 Skill 草稿 / 更新 SKILL.md | UJ-EVOLVE |
| [33-pattern-matcher.md](33-pattern-matcher.md) | 新 Task 触发相似路径检索→注入参考上下文（可选） | UJ-MATCH |
| [34-human-in-the-loop.md](34-human-in-the-loop.md) | Skill 草稿审核 / 发布 / 回滚（可选） | UJ-HITL |

  

### 1.11   K. 横切关注点（Cross-Cutting）

  

| 文件 | 模块 | 用例前缀 |
|------|------|----------|
| [35-error-handling.md](35-error-handling.md) | 统一错误模型、Result<T,E>、用户可理解的提示 | UJ-ERR |
| [36-performance-limits.md](36-performance-limits.md) | 大项目导入、事件回放性能、增量投影策略（可选） | UJ-PERF |
| [37-backup-recovery.md](37-backup-recovery.md) | 备份、恢复、损坏容器处理（可选） | UJ-RECOV |
| [38-security-credentials.md](38-security-credentials.md) | 凭据/本地配置/权限边界（只列索引，不给实现） | UJ-SEC |

  

---

  

## 2   版本说明

  
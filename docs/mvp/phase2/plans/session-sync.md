# 3.4 Session 同步模块 — 开发需求文档

> 日期: 2026-02-04（v2，基于代码实际验证更新）
> 预估总工时: 14 人时（路径计算 2h + 文件监听 4h + 增量解析 4h + Block 写入 4h）
> 前置条件: .elf/ Dir Block 初始化已完成（I10-01 ✅，`elf_meta.rs` 中已预置 `session/` 目录）
> 并行条件: 不依赖 3.1 Agent / 3.2 MCP，可独立开发；3.3 Skills 不阻塞本模块
> 输出目录: `src-tauri/src/sync/` (新建模块)

---

## 一、模块目标

将 Claude Code 在外部项目中产生的 Session 数据（JSONL 格式）自动同步到 Elfiee 的 `.elf/` 内部存储，**解析为可读性强的 Markdown 文档**并入库。

### 三大核心功能

| # | 功能 | 说明 |
|---|------|------|
| **F1** | **根据 Agent 配置找到 Session 目录** | 从 Agent Block 的 `config_dir` 字段推算 `~/.claude/projects/{encoded-path}/` |
| **F2** | **监听聊天内容变更** | 使用 `notify` crate 监听 Session 目录下 `.jsonl` 文件的新增/修改 |
| **F3** | **读取聊天内容，parse 成可读文档，Block 入库** | 增量解析 JSONL，转换为 Markdown 格式的会话记录，存储为 Markdown Block |

### 设计目标

1. **AI 会话可追溯**：每次 Claude Code 交互记录持久化到 .elf 文件
2. **可读性优先**：原始 JSONL 转换为 Markdown 文档，方便人类阅读和 AI 引用
3. **增量同步**：只解析新增 JSONL 行，避免重复处理
4. **自动触发**：监听文件系统变化，无需用户手动操作

### 用户操作流程

用户**不需要手动操作** Session 同步。唯一的触发点是 Agent 的启用/禁用，同步完全在后台自动进行。

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. 用户创建 Agent                                                │
│    在 Elfiee 中执行 agent.create(config_dir: ".claude/")         │
│    → 创建 Agent Block，绑定到外部项目的 AI 工具配置目录             │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. 用户启用 Agent                                                │
│    执行 agent.enable(agent_block_id)                             │
│    → 创建 symlink + 注入 MCP 配置（已有逻辑）                      │
│    → 【自动启动】Session 同步（本模块新增）                         │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. 后台自动运行（用户无感知）                                       │
│                                                                 │
│    a. 从 Agent config_dir 推算 session 目录                       │
│       "D:\workspace\elfiee\.claude"                              │
│       → parent: "D:\workspace\elfiee"                            │
│       → 编码: "d--workspace-elfiee"                               │
│       → 目录: ~/.claude/projects/d--workspace-elfiee/             │
│                                                                 │
│    b. 监听该目录下所有 .jsonl 文件的新增和修改                       │
│                                                                 │
│    c. 检测到变更 → 增量读取新行 → 解析 JSONL → 转换为 Markdown       │
│       - user 消息    → "## User (时间)\n\n内容"                    │
│       - assistant 回复 → "## Assistant (时间)\n\n内容"              │
│       - tool_use      → "> **Tool Use**: 工具名 — 描述"           │
│       - thinking/progress → 跳过（内部数据，不展示）                 │
│                                                                 │
│    d. 写入 .elf/session/{project}/session_{id}.md (Markdown Block) │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. 用户在 Elfiee 中查看 Session 记录                               │
│    .elf/ → session/ → elfiee/ → session_xxxx.md                  │
│    内容为可读的 Markdown 格式对话记录，前端直接渲染                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. 用户禁用 Agent（可选）                                         │
│    执行 agent.disable(agent_block_id)                            │
│    → 清理 symlink + 移除 MCP 配置（已有逻辑）                      │
│    → 【自动停止】Session 同步                                     │
│    → 已同步的 Session Block 保留在 .elf/ 中，不删除                 │
└─────────────────────────────────────────────────────────────────┘
```

**自动恢复**：用户重新打开 .elf 文件时，系统扫描所有已启用的 Agent Block，自动恢复 Session 同步（从上次偏移量继续）。

---

## 二、现状确认

### 2.1 已完成的基础设施

| 组件 | 状态 | 位置 |
|------|------|------|
| .elf/ Dir Block 初始化 | ✅ | `extensions/directory/elf_meta.rs` |
| `session/` 虚拟目录 | ✅ | `elf_meta.rs:93` (`EXTRA_DIRS` 包含 `"session/"`) |
| Agent Block (含 `config_dir`) | ✅ | `extensions/agent/mod.rs:51-79` |
| code.write Capability | ✅ | `extensions/code/code_write.rs` |
| markdown.write Capability | ✅ | `extensions/markdown/markdown_write.rs` |
| core.create Capability | ✅ | `capabilities/builtins/create.rs` |
| directory.write Capability | ✅ | `extensions/directory/directory_write.rs` |
| EngineHandle.process_command() | ✅ | `engine/actor.rs` |
| AppState（多文件管理） | ✅ | `state.rs` |
| BlockMetadata.custom 扩展字段 | ✅ | `models/metadata.rs` |

### 2.2 需要新增

| 模块 | 说明 |
|------|------|
| `sync/mod.rs` | 模块入口，导出公开接口 + SessionSyncManager |
| `sync/session_path.rs` | Session 目录路径计算器（从 Agent config_dir 推算） |
| `sync/watcher.rs` | JSONL 文件监听器 |
| `sync/parser.rs` | JSONL 增量解析器 + Markdown 转换器 |
| `sync/writer.rs` | Session Block 写入器（Markdown Block） |
| `sync/tests.rs` | 单元测试 + 集成测试 |

### 2.3 需要新增的依赖

```toml
# Cargo.toml [dependencies]
notify = "7"          # 跨平台文件系统监听（inotify/FSEvents/ReadDirectoryChanges）
```

---

## 三、Claude Code Session 文件格式（实际验证）

> **重要**：以下内容基于在 Windows 开发机上实际检查 `~/.claude/projects/` 目录得出，纠正了 v1 文档中的多处推测。

### 3.1 目录结构

```
~/.claude/
└── projects/
    └── {encoded-path}/              # 路径编码（见下文）
        ├── {session-uuid}.jsonl     # 完整会话记录
        ├── {session-uuid}/          # 部分 session 有同名目录
        └── ...
```

**实际目录示例**（本机 `C:/Users/Lenovo/.claude/projects/`）：

```
d--workspace-zhidaoyuan-elfiee/          # 本项目
D--workspace-zhidaoyuan-elfiee-peoject-elfiee/
D--workspace-zhidaoyuan-frontend-component-library/
C--Users-Lenovo-AppData-Local-Temp--tmp6mKgVe/
```

### 3.2 路径编码规则（已验证）

Windows 路径 `D:\workspace\zhidaoyuan\elfiee` 的编码结果：

```
D:\workspace\zhidaoyuan\elfiee
↓ Step 1: 将 `\` 替换为 `-`
D:-workspace-zhidaoyuan-elfiee
↓ Step 2: 将 `:` 替换为 `-`
D--workspace-zhidaoyuan-elfiee
↓ Step 3: 驱动器号可能小写
d--workspace-zhidaoyuan-elfiee
```

**关键发现**：
- Windows 下 `:` 替换为 `-`，与 `\` 的 `-` 连续产生 `--`（`D:\` → `D-` + `-workspace` = `D--workspace`）
- 驱动器号**可能小写**（观察到 `d--` 和 `D--` 共存，需兼容两种）
- Unix 下 `/` 替换为 `-`，前缀保留 `-`（如 `-home-yaosh-projects-elfiee`）

**兼容策略**：查找 Session 目录时，同时检查大小写变体。

### 3.3 JSONL 行格式（实际验证）

每行一个 JSON 对象。**实际格式比 v1 文档丰富得多**，主要类型：

#### 类型一览

| `type` 字段 | 说明 | 出现频率 |
|-------------|------|----------|
| `user` | 用户消息 | 高频 |
| `assistant` | AI 回复（可含多个 content block：text/thinking/tool_use） | 高频 |
| `progress` | Agent 子任务进展（嵌套的 agent 消息） | 高频 |
| `file-history-snapshot` | 文件快照记录 | 每条会话开头 |
| `summary` | 会话摘要 | 低频 |
| `system` | 系统消息 | 低频 |
| `result` | 工具执行结果 | 中频 |

#### 实际 JSONL 样例

**用户消息**：
```json
{
  "type": "user",
  "message": {"role": "user", "content": "请帮我检查下现在的代码..."},
  "timestamp": "2026-02-03T06:46:50.519Z",
  "sessionId": "26b41bcf-ed62-4f38-a04f-cfb4e6fd38b9",
  "cwd": "D:\\workspace\\zhidaoyuan\\elfiee",
  "version": "2.1.29",
  "gitBranch": "dev",
  "uuid": "9585ee50-...",
  "parentUuid": null
}
```

**AI 回复（含 thinking + tool_use）**：
```json
{
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-5-20251101",
    "role": "assistant",
    "content": [
      {"type": "thinking", "thinking": "The user is asking me to analyze..."},
      {"type": "text", "text": "Let me analyze the code..."},
      {"type": "tool_use", "id": "toolu_019Be1...", "name": "Task", "input": {"description": "Explore import flow code", "prompt": "..."}}
    ],
    "usage": {"input_tokens": 3, "output_tokens": 10}
  },
  "timestamp": "2026-02-03T06:46:55.797Z",
  "uuid": "c434653a-...",
  "parentUuid": "9585ee50-..."
}
```

**Agent 进展消息**：
```json
{
  "type": "progress",
  "data": {
    "type": "agent_progress",
    "prompt": "I need to understand the full import flow...",
    "agentId": "ab75dbd",
    "message": {"type": "assistant", "message": {"role": "assistant", "content": [...]}}
  },
  "toolUseID": "agent_msg_014a3D37...",
  "parentToolUseID": "toolu_019Be1wY...",
  "timestamp": "2026-02-03T06:47:09.133Z"
}
```

### 3.4 关键字段说明

| 字段 | 说明 | 用于 Markdown 生成 |
|------|------|--------------------|
| `type` | 消息类型 | 决定 Markdown 标题和分区 |
| `message.role` | `user` / `assistant` | 对话角色标识 |
| `message.content` | 字符串或结构化数组 | **核心内容来源** |
| `message.content[].type` | `text` / `thinking` / `tool_use` / `tool_result` | 内容块分类 |
| `message.model` | AI 模型标识 | Markdown 元信息 |
| `timestamp` | ISO 8601 UTC | 时间排序和展示 |
| `sessionId` | Session UUID | 分组和文件关联 |
| `uuid` / `parentUuid` | 消息树结构 | 保持对话层级 |
| `cwd` | 工作目录 | Session 元信息 |
| `gitBranch` | Git 分支 | Session 元信息 |
| `version` | Claude Code 版本 | Session 元信息 |

### 3.5 Session 文件特征

- 文件名即 Session ID：`{uuid}.jsonl`
- 追加写入：新消息追加到文件末尾
- 同时存在多个 session 文件（本项目有 ~190 个）
- 文件在 Claude Code 运行期间持续增长
- 大文件可达 1900+ 行（单个长会话）
- 部分 session 有同名目录（`{uuid}/`），功能未知，**只处理 `.jsonl` 文件**

---

## 四、架构设计

### 4.1 总体流程

```
                  ┌──────────────┐
                  │ Claude Code  │
                  │  (外部进程)   │
                  └──────┬───────┘
                         │ 追加 JSONL
                         ▼
    ~/.claude/projects/{path}/{uuid}.jsonl
                         │
        ┌────────────────┤
        │                │
    F1: 路径计算     F2: notify 监听
    (从 Agent        (文件变更事件)
     config_dir)         │
        │                ▼
        │     ┌──────────────────┐
        └────→│  SessionWatcher  │  ← sync/watcher.rs
              │  (tokio task)    │
              └──────────┬───────┘
                         │ FileChangeEvent
                         ▼
              ┌──────────────────┐
              │ SessionParser    │  ← sync/parser.rs
              │ (增量解析 JSONL  │
              │  → Markdown 转换)│
              └──────────┬───────┘
                         │ 结构化 Markdown
                         ▼
              ┌──────────────────┐
              │ SessionWriter    │  ← sync/writer.rs
              │ (写入 Markdown   │
              │  Block 到 .elf/) │
              └──────────┬───────┘
                         │ process_command
                         ▼
              ┌──────────────────┐
              │ ElfileEngine     │  ← engine/actor.rs
              │ (事件溯源)       │
              └──────────────────┘
```

### 4.2 F1：从 Agent 配置找到 Session 目录

**数据来源**：Agent Block 的 `config_dir` 字段。

```
AgentContents.config_dir = "D:\\workspace\\zhidaoyuan\\elfiee\\.claude"
                                    ↓ 取 parent 得到项目路径
                 project_path = "D:\\workspace\\zhidaoyuan\\elfiee"
                                    ↓ 路径编码
                   encoded = "d--workspace-zhidaoyuan-elfiee"
                                    ↓ 组合
         session_dir = ~/.claude/projects/d--workspace-zhidaoyuan-elfiee/
```

**为什么不用 Directory Block 的 `external_root_path`**：
- Agent 是 Session 同步的逻辑主体（一个 Agent 对应一个外部项目的 AI 集成）
- `config_dir` 已包含足够信息推算项目路径
- Agent enable/disable 直接控制 Session 同步的启停

### 4.3 F2：文件变更监听

- 使用 `notify` crate 监听 `.jsonl` 文件的 Create/Modify 事件
- 支持同时监听多个 Agent 关联的不同项目
- debounce 100ms 合并连续写入事件
- 首次启动时全量扫描已有文件

### 4.4 F3：解析为可读文档并入库

**核心变更**：v1 存储原始 JSONL 为 Code Block → v2 转换为 Markdown → v3 简化 JSON → **v4 保留原始字段的 JSON**。

**JSON 输出格式**：

保留原始 JSONL 的关键字段：`type`、`message`、`timestamp`、`version`、`gitBranch`、`data`。

```json
{
  "session_id": "26b41bcf-ed62-4f38-a04f-cfb4e6fd38b9",
  "project": "elfiee",
  "messages": [
    {
      "type": "user",
      "message": {"role": "user", "content": "请帮我检查下现在的代码..."},
      "timestamp": "2026-02-03T06:46:50.519Z",
      "version": "2.1.29",
      "gitBranch": "dev"
    },
    {
      "type": "assistant",
      "message": {
        "model": "claude-opus-4-5-20251101",
        "role": "assistant",
        "content": [
          {"type": "thinking", "thinking": "Let me analyze..."},
          {"type": "text", "text": "Let me先查看相关的 store 实现..."},
          {"type": "tool_use", "id": "toolu_xxx", "name": "Task", "input": {...}}
        ],
        "usage": {"input_tokens": 3, "output_tokens": 10}
      },
      "timestamp": "2026-02-03T06:46:55.797Z"
    },
    {
      "type": "progress",
      "data": {"type": "agent_progress", "agentId": "abc123", "message": {...}},
      "timestamp": "2026-02-03T06:47:09.133Z"
    }
  ]
}
```

**保留的字段**：

| 字段 | 说明 |
|------|------|
| `type` | 消息类型：`user`、`assistant`、`progress` 等 |
| `message` | 完整的 message 对象（包含 content、model、usage 等） |
| `timestamp` | ISO 8601 时间戳 |
| `version` | Claude Code 版本号 |
| `gitBranch` | Git 分支名 |
| `data` | progress 消息的数据（包含 agent 子任务信息） |

**跳过的消息类型**：
- `file-history-snapshot` — 文件快照（每个 session 开头）
- `summary` — 会话摘要
- `system` — 系统消息
- `result` — 执行结果

### 4.5 存储设计

Session 数据存储为 **Markdown Block**（`block_type: "markdown"`），内容为 **JSON 字符串**，原因：
- 结构化数据，便于程序解析和 AI 引用
- JSON 格式支持增量追加 messages 数组
- 复用现有 `markdown.write` Capability（content 字段存储 JSON 文本）
- 快照机制自动生成 `block-{uuid}/body.md` 物理文件（实际内容为 JSON）
- 前端可解析 JSON 并渲染为对话列表

**Block 命名规范**：
- Block name: `session_{session_id}` （如 `session_26b41bcf-ed62-4f38-a04f-cfb4e6fd38b9`）
- Block metadata.description: `"Claude Code session for {project-name} ({branch})"`
- Block metadata.custom:
  ```json
  {
    "session_id": "26b41bcf-...",
    "project_name": "elfiee",
    "config_dir": "D:\\workspace\\zhidaoyuan\\elfiee\\.claude",
    "source_file": "C:/Users/Lenovo/.claude/projects/d--workspace-zhidaoyuan-elfiee/26b41bcf-....jsonl",
    "sync_offset": 12345,
    "model": "claude-opus-4-5-20251101",
    "git_branch": "dev",
    "message_count": 42,
    "last_synced_at": "2026-02-03T06:50:00Z"
  }
  ```

### 4.6 生命周期管理

```
SessionSyncManager (顶层管理器)
├── start_sync(file_id, agent_block_id) → 从 Agent 配置启动监听
├── stop_sync(agent_block_id)           → 停止监听
└── get_sync_status(agent_block_id)     → 查询状态

SessionWatcher (每个 Agent 一个监听)
├── watch()   → 开始监听 session 目录
├── unwatch() → 停止监听
└── status()  → 当前状态 (watching/stopped/error)
```

**触发时机**：
- `agent.enable` 时：根据 Agent 的 `config_dir` 启动 Session 同步
- `agent.disable` 时：停止该 Agent 关联的 Session 同步
- `open_file` 时：扫描所有已启用的 Agent Block，恢复 Session 同步
- `close_file` 时：停止该文件关联的所有同步

---

## 五、详细任务分解

### 5.1 F10-01: Session 目录计算器（2 人时）

**文件**: `src-tauri/src/sync/session_path.rs`

**功能描述**：
根据 Agent Block 的 `config_dir` 推算对应的 Claude Code session 目录。

**输入/输出**：
```rust
/// 从 Agent 的 config_dir 推算 Session 目录
///
/// # 推算流程
/// 1. 从 config_dir 取 parent 得到 project_path
///    例: "D:\\workspace\\zhidaoyuan\\elfiee\\.claude" → "D:\\workspace\\zhidaoyuan\\elfiee"
/// 2. 对 project_path 应用路径编码
/// 3. 组合为 `~/.claude/projects/{encoded}/`
///
/// # 路径编码规则（已验证）
/// - Unix: `/home/yaosh/projects/elfiee` → `-home-yaosh-projects-elfiee`
/// - Windows: `D:\workspace\zhidaoyuan\elfiee` → `d--workspace-zhidaoyuan-elfiee`
///   - `\` 替换为 `-`
///   - `:` 移除（产生连续 `--`）
///   - 驱动器号**可能小写**
///
/// # Returns
/// - `Ok(PathBuf)`: session 目录路径
/// - `Err(String)`: config_dir 无 parent 或 home 不可用
pub fn compute_session_dir_from_config(config_dir: &str) -> Result<PathBuf, String>

/// 直接从外部项目路径计算 Session 目录（兼容接口）
pub fn compute_session_dir(external_path: &Path) -> Result<PathBuf, String>
```

**实现要点**：

1. **从 config_dir 提取项目路径**：`Path::new(config_dir).parent()`
2. **路径编码**：
   - 取绝对路径字符串
   - 将 `\` 替换为 `-`
   - 将 `/` 替换为 `-`
   - 将 `:` 替换为 `-`（与 `\` 的 `-` 连续产生 `D:` → `D-` + `-` = `D--`）
   - **不添加** `-` 前缀（Windows 编码结果已自然以盘符开头）
   - Unix 路径因以 `/` 开头，编码后自然以 `-` 开头
3. **大小写兼容**：查找目录时，先尝试原始编码，再尝试小写变体
4. **组合路径**：`{home}/.claude/projects/{encoded-path}/`

**辅助函数**：
```rust
/// 对路径应用 Claude Code 的编码规则
///
/// 实际验证结果：
/// - "D:\workspace\zhidaoyuan\elfiee" → "D--workspace-zhidaoyuan-elfiee"
/// - "/home/yaosh/projects/elfiee" → "-home-yaosh-projects-elfiee"
pub fn encode_project_path(path: &Path) -> String

/// 从编码路径反解出项目名称（取最后一段 `-` 分隔的名称）
/// 例如: "d--workspace-zhidaoyuan-elfiee" → "elfiee"
pub fn extract_project_name(encoded_path: &str) -> String

/// 检查 session 目录是否存在（兼容大小写）
/// 返回实际存在的路径或 None
pub fn find_session_dir(external_path: &Path) -> Option<PathBuf>
```

**测试用例**：
```rust
#[test] fn test_unix_path_encoding()
    // "/home/yaosh/projects/elfiee" → "-home-yaosh-projects-elfiee"

#[test] fn test_windows_path_encoding()
    // "D:\workspace\zhidaoyuan\elfiee" → "D--workspace-zhidaoyuan-elfiee"

#[test] fn test_windows_colon_removed_not_replaced()
    // 验证 `:` 移除而非替换为 `-`

#[test] fn test_config_dir_to_session_dir()
    // "D:\workspace\zhidaoyuan\elfiee\.claude" → session dir 正确

#[test] fn test_extract_project_name()
    // "d--workspace-zhidaoyuan-elfiee" → "elfiee"

#[test] fn test_extract_project_name_unix()
    // "-home-yaosh-projects-elfiee" → "elfiee"
```

---

### 5.2 F11-01: JSONL 文件监听器（4 人时）

**文件**: `src-tauri/src/sync/watcher.rs`

**功能描述**：
使用 `notify` crate 监听 Claude Code session 目录下所有 `.jsonl` 文件的变更，支持多 Agent 同时监听。

**核心结构**：
```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event as NotifyEvent};
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::collections::HashMap;

/// Session 同步状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum SyncStatus {
    /// 正在监听
    Watching,
    /// 已停止（目录不存在或手动停止）
    Stopped,
    /// 错误状态
    Error(String),
}

/// 文件变更事件（发送给 Parser）
#[derive(Debug)]
pub struct FileChangeEvent {
    /// 变更的 JSONL 文件路径
    pub path: PathBuf,
    /// 关联的 Agent Block ID
    pub agent_block_id: String,
    /// 关联的 .elf file_id
    pub file_id: String,
    /// Agent 的 config_dir（用于推算 project_name）
    pub config_dir: String,
}

/// Session 文件监听器
pub struct SessionWatcher {
    /// notify watcher 实例
    watcher: RecommendedWatcher,
    /// 变更事件发送端
    event_tx: mpsc::UnboundedSender<FileChangeEvent>,
    /// 已监听的目录映射：session_dir → (file_id, agent_block_id, config_dir)
    watched_dirs: HashMap<PathBuf, (String, String, String)>,
    /// 各 Agent 的同步状态
    status: HashMap<String, SyncStatus>,
}
```

**公开接口**：
```rust
impl SessionWatcher {
    /// 创建新的监听器，返回 (watcher, event_receiver)
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<FileChangeEvent>), String>

    /// 根据 Agent 配置开始监听 session 目录
    ///
    /// 1. 从 config_dir 计算 session 目录（compute_session_dir_from_config）
    /// 2. 检查目录是否存在（兼容大小写）
    /// 3. 使用 notify 注册目录监听（非递归，只监听 .jsonl 文件）
    /// 4. 扫描目录中现有的 .jsonl 文件，发送初始 FileChangeEvent
    pub fn watch_agent(
        &mut self,
        file_id: &str,
        agent_block_id: &str,
        config_dir: &str,
    ) -> Result<(), String>

    /// 停止某个 Agent 关联的 session 监听
    pub fn unwatch_agent(&mut self, agent_block_id: &str) -> Result<(), String>

    /// 获取某个 Agent 的同步状态
    pub fn get_status(&self, agent_block_id: &str) -> SyncStatus

    /// 获取所有 Agent 的同步状态
    pub fn get_all_status(&self) -> HashMap<String, SyncStatus>
}
```

**事件过滤规则**：
- 只关注 `EventKind::Modify` 和 `EventKind::Create` 事件
- 只处理 `.jsonl` 后缀文件
- 忽略同名目录（`{uuid}/`）、临时文件（`.tmp`, `.swp`）
- debounce：100ms 内同一文件的多次变更合并为一次

**实现要点**：

1. **notify 配置**：使用 `RecommendedWatcher`（Windows 下使用 ReadDirectoryChangesW）
2. **事件转发**：std::sync::mpsc → tokio::sync::mpsc 桥接线程
3. **首次扫描**：watch 时遍历目录中已有的 `.jsonl` 文件，全部发送 FileChangeEvent
4. **目录不存在**：状态设为 `Stopped`，不阻塞其他功能

**测试用例**：
```rust
#[test] fn test_watch_existing_directory()
#[test] fn test_watch_nonexistent_directory()
#[test] fn test_file_change_event_filtering()
#[test] fn test_unwatch_stops_monitoring()
#[tokio::test] async fn test_concurrent_watches()
```

---

### 5.3 F12-01: JSONL 增量解析器 + JSON 转换（4 人时）

**文件**: `src-tauri/src/sync/parser.rs`

**功能描述**：
增量解析 Claude Code 的 JSONL 文件，**保留原始字段转换为 JSON 格式**。

**核心结构**：
```rust
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// 单条消息，保留原始 JSONL 字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    #[serde(rename = "type")]
    pub msg_type: String,                    // "user" | "assistant" | "progress" 等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,  // 完整的 message 对象
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,           // ISO 8601 时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,             // Claude Code 版本
    #[serde(rename = "gitBranch", skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,          // Git 分支
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,     // progress 消息的 data
}

/// 完整的 Session 数据结构（用于 JSON 序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub project: String,
    pub messages: Vec<SessionMessage>,
}

/// 解析后的 Session 文档（增量数据）
#[derive(Debug, Clone)]
pub struct SessionDocument {
    /// Session ID
    pub session_id: String,
    /// 新增的消息列表
    pub new_messages: Vec<SessionMessage>,
    /// Session 元信息（从第一条消息提取）
    pub metadata: SessionMetadata,
    /// 本次新增的消息数量
    pub new_message_count: usize,
}

/// Session 元信息
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    pub model: Option<String>,
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub started_at: Option<String>,
}

/// 偏移量跟踪器
pub struct OffsetTracker {
    offsets: HashMap<PathBuf, u64>,
}

/// JSONL 增量解析器
pub struct SessionParser {
    tracker: OffsetTracker,
}
```

**公开接口**：
```rust
impl SessionParser {
    pub fn new() -> Self

    /// 增量解析 JSONL 文件，返回新增消息列表
    ///
    /// # 流程
    /// 1. 打开文件，seek 到上次偏移量
    /// 2. 逐行读取 + JSON 解析
    /// 3. 按 type 分类，转换为 SessionMessage
    /// 4. 组装为 SessionDocument
    /// 5. 更新偏移量
    ///
    /// # 转换规则
    /// - user → SessionMessage { role: "user", timestamp, content }
    /// - assistant (text only) → SessionMessage { role: "assistant", timestamp, content }
    /// - assistant (thinking/tool_use) → 跳过
    /// - progress/file-history-snapshot/summary → 跳过
    pub fn parse_incremental(
        &mut self,
        file_path: &Path,
    ) -> Result<Option<SessionDocument>, String>

    /// 获取/设置偏移量
    pub fn get_offset(&self, file_path: &Path) -> u64
    pub fn set_offset(&mut self, file_path: &Path, offset: u64)
    pub fn reset_offset(&mut self, file_path: &Path)
}

impl SessionData {
    pub fn new(session_id: String, project: String) -> Self
    pub fn to_json(&self) -> String      // 序列化为 JSON 字符串
    pub fn from_json(json: &str) -> Option<Self>  // 从 JSON 字符串解析
}

/// 创建初始 SessionData（用于首次写入）
pub fn create_session_data(
    metadata: &SessionMetadata,
    session_id: &str,
    project_name: &str,
) -> SessionData
```

**JSON 转换逻辑**：

```rust
/// 从 JSONL 对象提取关键字段
fn jsonl_to_message(json: &serde_json::Value) -> Option<SessionMessage> {
    // type 字段必需
    let msg_type = json.get("type")?.as_str()?.to_string();

    // 跳过无用的内部类型
    if matches!(msg_type.as_str(), "file-history-snapshot" | "summary" | "system" | "result") {
        return None;
    }

    Some(SessionMessage {
        msg_type,
        message: json.get("message").cloned(),
        timestamp: json.get("timestamp").and_then(|v| v.as_str()).map(String::from),
        version: json.get("version").and_then(|v| v.as_str()).map(String::from),
        git_branch: json.get("gitBranch").and_then(|v| v.as_str()).map(String::from),
        data: json.get("data").cloned(),
    })
}
```

**保留的消息类型**：
- `user` — 用户消息（包含完整 message 对象）
- `assistant` — AI 回复（包含 content 数组：text/thinking/tool_use）
- `progress` — Agent 子任务进展（包含 data 对象）

**跳过的消息类型**：
- `file-history-snapshot` — 文件快照
- `summary` — 会话摘要
- `system` — 系统消息
- `result` — 执行结果

**关键设计决策**：

| 决策 | 选择 | 原因 |
|------|------|------|
| 存储格式 | **JSON**（保留原始字段） | 保留完整信息，便于前端灵活展示 |
| message 字段 | **完整保留** | 包含 model、content、usage 等原始数据 |
| thinking/tool_use | **保留在 message.content 中** | 前端可选择性展示 |
| progress 消息 | **保留** | 包含 Agent 子任务信息 |
| 偏移量恢复 | **内存 + Block metadata** | 重启后从 metadata.custom.sync_offset 恢复 |

**测试用例**：
```rust
#[test] fn test_parse_user_message()
#[test] fn test_parse_assistant_message()
#[test] fn test_assistant_with_tool_use_preserved()
#[test] fn test_progress_message_preserved()
#[test] fn test_file_history_snapshot_skipped()
#[test] fn test_summary_skipped()
#[test] fn test_session_data_serialization()
#[test] fn test_missing_type_returns_none()
#[test] fn test_optional_fields_omitted()
```

---

### 5.4 F13-01: Session Block 写入器（4 人时）

**文件**: `src-tauri/src/sync/writer.rs`

**功能描述**：
将解析后的 JSON 文档写入 Elfiee 的 **Markdown Block**（内容为 JSON 字符串），通过 EngineHandle 走完整的事件溯源流程。

**核心结构**：
```rust
use crate::engine::actor::EngineHandle;
use crate::models::Command;
use crate::sync::parser::{create_session_data, SessionData, SessionDocument, SessionMetadata};
use std::collections::HashMap;

/// Session Block 写入器 — 写入 JSON 内容
pub struct SessionWriter {
    /// session_id → block_id 映射缓存
    session_blocks: HashMap<String, String>,
}
```

**公开接口**：
```rust
impl SessionWriter {
    pub fn new() -> Self

    /// 将 SessionDocument 写入对应的 Markdown Block（JSON 格式）
    ///
    /// # 流程
    /// 1. 查找或创建 session 对应的 Markdown Block
    /// 2. 读取现有 JSON 内容（如有）→ 解析为 SessionData
    /// 3. 追加新消息到 SessionData.messages
    /// 4. 序列化 SessionData → JSON 字符串
    /// 5. 用 markdown.write 写入
    /// 6. 更新 metadata（sync_offset, message_count 等）
    /// 7. 更新 .elf/ session 目录 entries
    pub async fn write_session(
        &mut self,
        handle: &EngineHandle,
        editor_id: &str,
        elf_block_id: &str,
        project_name: &str,
        config_dir: &str,
        doc: SessionDocument,
        source_file: &str,
        source_offset: u64,
    ) -> Result<(), String>

    /// 查找或创建 session block
    async fn create_session_block(
        &self,
        handle: &EngineHandle,
        editor_id: &str,
        elf_block_id: &str,
        session_id: &str,
        project_name: &str,
        config_dir: &str,
        source_file: &str,
        metadata: &SessionMetadata,
    ) -> Result<String, String>

    /// 恢复已有的 session block 映射和偏移量
    pub async fn load_existing_sessions(
        &mut self,
        handle: &EngineHandle,
        elf_block_id: &str,
    ) -> Result<HashMap<String, u64>, String>
}
```

**写入流程详解**：

#### Step 1: 创建 Session Block（首次同步）

```rust
// 1. core.create — 创建 Markdown Block
let create_cmd = Command::new(
    editor_id.to_string(),
    "core.create".to_string(),
    "".to_string(),
    json!({
        "name": format!("session_{}", session_id),
        "block_type": "markdown",
        "source": "outline",
        "metadata": {
            "description": format!("Claude Code session for {} ({})",
                project_name, metadata.git_branch.as_deref().unwrap_or("unknown")),
            "session_id": session_id,
            "project_name": project_name,
            "config_dir": config_dir,
            "source_file": source_file,
            "sync_offset": 0,
            "model": metadata.model,
            "git_branch": metadata.git_branch,
            "message_count": 0,
            "last_synced_at": now_utc()
        }
    }),
);
let events = handle.process_command(create_cmd).await?;
let block_id = events[0].entity.clone();

// 2. 创建初始 SessionData 并序列化为 JSON
let mut session_data = create_session_data(&metadata, session_id, project_name);
session_data.messages.extend(doc.new_messages);
let initial_content = session_data.to_json();

let write_cmd = Command::new(
    editor_id.to_string(),
    "markdown.write".to_string(),
    block_id.clone(),
    json!({ "content": initial_content }),
);
handle.process_command(write_cmd).await?;

// 3. 更新 .elf/ 的 entries
// directory.write 添加 session/{project_name}/session_{id}.json 条目
```

#### Step 2: 追加内容（增量写入）

```rust
// 1. 获取现有 JSON 内容
let block = handle.get_block(&block_id).await
    .ok_or("Session block not found")?;
let existing_content = block.contents["markdown"].as_str().unwrap_or("");

// 2. 解析为 SessionData，追加新消息
let mut session_data = SessionData::from_json(existing_content)
    .unwrap_or_else(|| create_session_data(&metadata, session_id, project_name));
session_data.messages.extend(doc.new_messages);

// 3. 序列化并写入
let new_content = session_data.to_json();
let write_cmd = Command::new(
    editor_id.to_string(),
    "markdown.write".to_string(),
    block_id.clone(),
    json!({ "content": new_content }),
);
handle.process_command(write_cmd).await?;

// 4. 更新 metadata
let meta_cmd = Command::new(
    editor_id.to_string(),
    "core.update_metadata".to_string(),
    block_id.clone(),
    json!({
        "metadata": {
            "sync_offset": source_offset,
            "message_count": existing_msg_count + doc.new_message_count,
            "last_synced_at": now_utc()
        }
    }),
);
handle.process_command(meta_cmd).await?;
```

**关键设计决策**：

| 决策 | 选择 | 原因 |
|------|------|------|
| Block 类型 | **markdown**（内容为 JSON） | 复用现有 markdown.write 能力 |
| 写入方式 | **全量 markdown.write** | Event Sourcing 要求每次写入完整内容 |
| entries 路径 | `session/{project}/session_{id}.json` | `.json` 后缀，体现 JSON 格式 |
| 偏移量持久化 | **metadata.custom.sync_offset** | 重启后可恢复 |

**Phase 2 限制**：随着 session 增长，单个 Block 内容会越来越大。Phase 3+ 考虑按会话分片。

**测试用例**：
```rust
#[tokio::test] async fn test_create_new_session_block()
#[tokio::test] async fn test_append_to_existing_session()
#[tokio::test] async fn test_multiple_sessions_separate_blocks()
#[tokio::test] async fn test_offset_persisted_in_metadata()
#[tokio::test] async fn test_load_existing_sessions_from_entries()
#[tokio::test] async fn test_frontmatter_in_initial_content()
```

---

## 六、模块入口与管理器

### 6.1 sync/mod.rs

**文件**: `src-tauri/src/sync/mod.rs`

```rust
pub mod session_path;
pub mod watcher;
pub mod parser;
pub mod writer;

#[cfg(test)]
mod tests;

use crate::engine::actor::EngineHandle;
use crate::state::AppState;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Session 同步管理器
///
/// 协调 Watcher → Parser → Writer 的完整流程。
/// 通过 Agent Block 的 config_dir 驱动同步。
pub struct SessionSyncManager {
    watcher: Arc<Mutex<watcher::SessionWatcher>>,
    parser: Arc<Mutex<parser::SessionParser>>,
    writer: Arc<Mutex<writer::SessionWriter>>,
    sync_task: Option<tokio::task::JoinHandle<()>>,
}

impl SessionSyncManager {
    /// 创建新的同步管理器并启动事件处理循环
    pub fn new(app_state: AppState) -> Result<Self, String>

    /// 为某个 Agent 启动 Session 同步
    ///
    /// # 参数
    /// - `file_id`: .elf 文件 ID
    /// - `agent_block_id`: Agent Block ID
    /// - `config_dir`: Agent 的 config_dir（如 "/home/user/repo/.claude"）
    pub async fn start_sync(
        &self,
        file_id: &str,
        agent_block_id: &str,
        config_dir: &str,
    ) -> Result<(), String>

    /// 停止某个 Agent 的 Session 同步
    pub async fn stop_sync(&self, agent_block_id: &str) -> Result<(), String>

    /// 停止所有同步并清理资源
    pub async fn shutdown(&mut self)

    /// 获取同步状态
    pub fn get_status(&self, agent_block_id: &str) -> watcher::SyncStatus
}
```

### 6.2 事件处理循环

```rust
/// 内部事件处理循环
async fn sync_event_loop(
    event_rx: mpsc::UnboundedReceiver<FileChangeEvent>,
    parser: Arc<Mutex<SessionParser>>,
    writer: Arc<Mutex<SessionWriter>>,
    app_state: AppState,
) {
    while let Some(event) = event_rx.recv().await {
        // 1. 增量解析 → Markdown
        let doc = {
            let mut parser = parser.lock().await;
            match parser.parse_incremental(&event.path) {
                Ok(Some(doc)) => doc,
                Ok(None) => continue,  // 无新内容或全部跳过
                Err(e) => {
                    log::error!("Parse error for {:?}: {}", event.path, e);
                    continue;
                }
            }
        };

        // 2. 获取 Engine 和 Editor
        let handle = match app_state.engine_manager.get_engine(&event.file_id) {
            Some(h) => h,
            None => { log::warn!("Engine not found: {}", event.file_id); continue; }
        };
        let editor_id = match app_state.get_active_editor(&event.file_id) {
            Some(id) => id,
            None => { log::warn!("No editor: {}", event.file_id); continue; }
        };

        // 3. 推算 project_name 和 elf_block_id
        let project_name = session_path::extract_project_name_from_config(&event.config_dir);
        let elf_block_id = find_elf_block_id(&handle).await;

        let source_offset = {
            let p = parser.lock().await;
            p.get_offset(&event.path)
        };

        // 4. 写入 Markdown Block
        let mut writer = writer.lock().await;
        if let Err(e) = writer.write_session(
            &handle, &editor_id, &elf_block_id,
            &project_name, doc,
            &event.path.to_string_lossy(), source_offset,
        ).await {
            log::error!("Write error: {}", e);
        }
    }
}
```

---

## 七、集成点

### 7.1 AppState 扩展

在 `state.rs` 中为 AppState 添加 SessionSyncManager：

```rust
pub struct AppState {
    // ... 现有字段 ...
    /// Session 同步管理器（按需初始化）
    pub session_sync: Arc<tokio::sync::Mutex<Option<SessionSyncManager>>>,
}
```

### 7.2 Agent Enable 时启动同步

在 `commands/agent.rs` 的 `perform_enable_io` 后（或在 Tauri command 层）：

```rust
// agent.enable 成功后，启动该 Agent 的 Session 同步
if let Some(sync_manager) = session_sync.lock().await.as_ref() {
    sync_manager.start_sync(
        &file_id,
        &agent_block_id,
        &agent_contents.config_dir,
    ).await?;
}
```

### 7.3 Agent Disable 时停止同步

```rust
// agent.disable 时，停止该 Agent 的 Session 同步
if let Some(sync_manager) = session_sync.lock().await.as_ref() {
    sync_manager.stop_sync(&agent_block_id).await?;
}
```

### 7.4 文件打开时恢复同步

在 `commands/file.rs` 的 `open_file` 末尾：

```rust
// 扫描所有已启用的 Agent Block，恢复 Session 同步
for (block_id, block) in all_blocks {
    if block.block_type == "agent" {
        let contents: AgentContents = serde_json::from_value(block.contents.clone())?;
        if contents.status == AgentStatus::Enabled {
            sync_manager.start_sync(&file_id, &block_id, &contents.config_dir).await?;
        }
    }
}
```

### 7.5 文件关闭时停止同步

```rust
// close_file 时停止所有同步
if let Some(mut sync_manager) = session_sync.lock().await.take() {
    sync_manager.shutdown().await;
}
```

### 7.6 Tauri Command 暴露（可选）

```rust
// commands/sync.rs (Phase 2 可先不暴露，通过 agent enable/disable 间接控制)
#[tauri::command]
#[specta::specta]
pub async fn get_session_sync_status(
    file_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, String>
```

---

## 八、.elf/ 目录结构更新

Session 同步后，`.elf/` 的 entries 结构变为：

```json
{
  "entries": {
    "agents/": { "id": "dir-xxx", "type": "directory" },
    "agents/elfiee-client/": { "id": "dir-xxx", "type": "directory" },
    "session/": { "id": "dir-xxx", "type": "directory" },
    "session/elfiee/": { "id": "dir-xxx", "type": "directory" },
    "session/elfiee/session_26b41bcf-ed62-4f38-a04f-cfb4e6fd38b9.json": {
      "id": "block-uuid-of-markdown-block",
      "type": "file",
      "source": "outline",
      "updated_at": "2026-02-04T..."
    },
    "git/": { "id": "dir-xxx", "type": "directory" },
    "git/hooks/": { "id": "dir-xxx", "type": "directory" }
  }
}
```

**注意**：路径使用 `.json` 后缀，反映 JSON 存储格式。Block 内容为结构化 JSON 字符串。

---

## 九、错误处理与降级策略

| 场景 | 处理方式 |
|------|----------|
| Session 目录不存在 | 静默跳过，状态设为 Stopped，不阻塞启动 |
| 大小写不匹配 | 自动尝试大小写变体查找 |
| JSONL 行格式错误 | 跳过坏行，log::warn 记录，继续处理后续行 |
| Engine 不可用 | log::error 记录，等待下次文件变更时重试 |
| notify watcher 失败 | 降级为定时轮询（每 5s 检查文件修改时间） |
| markdown.write 失败 | log::error 记录，不更新偏移量（下次重试） |
| 文件被截断 | 重置偏移量，从头重新解析 |
| Block 内容过大 | Phase 2 可接受；Phase 3+ 考虑分片 |

---

## 十、开发顺序与验收标准

### 10.1 开发顺序

```
F10-01 Session 路径计算器 (2h)
    ↓ 无依赖，可最先开始
F12-01 JSONL 解析器 + Markdown 转换 (4h)
    ↓ 依赖路径计算，但逻辑独立
F11-01 JSONL 文件监听器 (4h)
    ↓ 依赖路径计算
F13-01 Session Block 写入器 (4h)
    ↓ 依赖解析器和 Engine
模块集成 + 集成测试
```

**可并行**：F11-01 和 F12-01 可以并行开发。

### 10.2 验收标准

| 编号 | 验收条件 | 验证方式 |
|------|----------|----------|
| AC-01 | 路径编码正确（Unix + Windows，含大小写兼容） | 单元测试 |
| AC-02 | 从 Agent config_dir 正确推算 session 目录 | 单元测试 |
| AC-03 | 监听到 .jsonl 文件变化并触发事件 | 集成测试 |
| AC-04 | 增量解析只处理新增行 | 单元测试 |
| AC-05 | user 消息正确转换为 Markdown | 单元测试 |
| AC-06 | assistant 消息（text/tool_use）正确转换 | 单元测试 |
| AC-07 | thinking/progress/snapshot 正确跳过 | 单元测试 |
| AC-08 | Session Markdown Block 正确创建（含 frontmatter） | 集成测试 |
| AC-09 | 增量 Markdown 正确追加到已有 Block | 集成测试 |
| AC-10 | 偏移量在 metadata 中持久化 | 集成测试 |
| AC-11 | .elf/ entries 正确更新 | 集成测试 |
| AC-12 | 多 Agent 同时同步不冲突 | 集成测试 |
| AC-13 | 停止同步后不再写入 | 集成测试 |

### 10.3 文件清单

| 文件 | 类型 | 说明 |
|------|------|------|
| `src-tauri/src/sync/mod.rs` | 新建 | 模块入口 + SessionSyncManager |
| `src-tauri/src/sync/session_path.rs` | 新建 | 路径计算（从 Agent config_dir） |
| `src-tauri/src/sync/watcher.rs` | 新建 | 文件监听 |
| `src-tauri/src/sync/parser.rs` | 新建 | JSONL 解析 + Markdown 转换 |
| `src-tauri/src/sync/writer.rs` | 新建 | Markdown Block 写入 |
| `src-tauri/src/sync/tests.rs` | 新建 | 测试 |
| `src-tauri/src/lib.rs` | 修改 | 添加 `pub mod sync;` |
| `src-tauri/src/state.rs` | 修改 | AppState 添加 session_sync 字段 |
| `src-tauri/Cargo.toml` | 修改 | 添加 `notify = "7"` 依赖 |
| `src-tauri/src/commands/agent.rs` | 修改 | enable/disable 时启停同步 |
| `src-tauri/src/commands/file.rs` | 修改 | open/close 时恢复/停止同步 |

---

## 十一、跨平台注意事项

| 平台 | notify 后端 | 路径编码 | 注意事项 |
|------|------------|----------|----------|
| Linux | inotify | `-home-yaosh-projects-elfiee` | 默认 watch 限制 8192 |
| macOS | FSEvents | `-Users-yaosh-projects-elfiee` | 延迟 ~500ms |
| Windows | ReadDirectoryChangesW | `d--workspace-zhidaoyuan-elfiee` | **`:` 替换为 `-`，驱动器号可能小写** |

**Windows 路径编码（已验证）**：
- `D:\workspace\zhidaoyuan\elfiee` → `d--workspace-zhidaoyuan-elfiee`（实际目录名）
- `:` 移除（不是替换为 `-`），产生连续 `--`
- 驱动器号可能大写或小写，需兼容两种
- 同一机器上观察到 `d--` 和 `D--` 共存

---

## 十二、与 v1 文档的主要变更

| 变更点 | v1 | v2 | v3 | 原因 |
|--------|----|----|-----|------|
| 路径来源 | Directory Block | Agent Block 的 `config_dir` | (同 v2) | Agent 是 Session 同步的逻辑主体 |
| 路径编码 | 假设 `:` 替换为 `-` | 验证 `:` 替换为 `-` | (同 v2) | 实际验证结果一致 |
| 存储格式 | 原始 JSONL → Code Block | Markdown → Markdown Block | **JSON → Markdown Block** | 结构化数据便于程序解析 |
| JSONL 格式 | 简单 user/assistant | **丰富的 type 体系** | (同 v2) | 实际验证结果 |
| 触发时机 | open_file | **agent.enable / disable** | (同 v2) | 与 Agent 生命周期绑定 |
| 文件命名 | `session_{id}.jsonl` | `session_{id}.md` | **`session_{id}.json`** | 反映 JSON 格式 |

---

## 十三、Phase 3+ 演进方向

| 方向 | 说明 |
|------|------|
| **分片存储** | session 超过阈值后自动分片为多个 Block |
| **内容索引** | 对 session 内容建立全文索引，支持搜索 |
| **摘要生成** | 用 AI 生成 session 摘要，关联到 Session Block |
| **关联分析** | Session → Task Block 自动关联（基于时间窗口和内容匹配） |
| **跨模型支持** | 扩展到 Cursor/Copilot 等其他 AI 工具的 session 格式 |
| **thinking 可选展示** | 用户可配置是否保留 AI 思考过程 |
| **progress 摘要** | 对 agent 子任务生成摘要而非完全跳过 |
| **实时同步** | 行级监听，近实时更新 |

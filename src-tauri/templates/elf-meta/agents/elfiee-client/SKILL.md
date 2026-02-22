---
name: elfiee-client
description: "Guide for using Elfiee MCP tools to interact with .elf files. Use when Claude needs to read, write, or manage blocks inside .elf projects via MCP tools (elfiee_file_list, elfiee_block_*, elfiee_markdown_*, elfiee_code_*, elfiee_directory_*, elfiee_terminal_*, elfiee_task_*, elfiee_editor_*, elfiee_exec) or MCP resources (elfiee://files, elfiee://{project}/blocks, elfiee://{project}/block/{id}, elfiee://{project}/grants, elfiee://{project}/events). Includes causal linking protocol (implement relations), DAG constraints, graph-first context navigation, and TDD best practices. Triggers: working with .elf files, managing blocks, reading/writing markdown or code in blocks, directory operations inside .elf, terminal sessions, task management, creating causal links between blocks, navigating block relationships."
---

# Elfiee MCP Tools

Elfiee exposes MCP tools and resources for interacting with `.elf` files. Three connection modes:

| Mode | Transport | When to use |
|------|-----------|-------------|
| **Per-agent mode** | SSE on port 47201–47299 | Each enabled agent gets a dedicated port (configured automatically) |
| **Management mode** | SSE on port 47200 | Fallback/legacy mode using GUI active editor |
| **Standalone mode** | stdio (JSON-RPC) | No GUI needed; Claude Code launches `elfiee mcp-server --elf <path>` |

## Prohibited Actions

**When Elfiee MCP is connected, ALL content managed by .elf blocks MUST be read and written through Elfiee MCP tools.**

### NEVER do these:

| Prohibited | Use instead |
|-----------|-------------|
| `Read` / `cat` / `head` to read block content | `elfiee_markdown_read` / `elfiee_code_read` / `elfiee_block_get` |
| `Write` / `Edit` to modify block content | `elfiee_markdown_write` / `elfiee_code_write` |
| `Bash` with `ls` / `rm` / `mv` on .elf internals | `elfiee_block_list` / `elfiee_block_delete` / `elfiee_block_rename` |
| `Glob` / `Grep` to search inside .elf | `elfiee_block_list` + `elfiee_*_read` |
| Directly editing files that correspond to .elf blocks | Always go through `elfiee_*_write` tools |
| Creating files in the project to store content | `elfiee_block_create` + `elfiee_*_write` |

### Why this matters:

- .elf uses **event sourcing** — direct filesystem edits bypass the event log and will be **lost or overwritten**
- Permissions are enforced through **CBAC** (Capability-Based Access Control) — only MCP tools check authorization
- Block snapshots (physical files) are **derived data** regenerated from events — editing them directly has no lasting effect

## MCP Connection Failure Protocol

If any `elfiee_*` MCP tool returns a connection error, timeout, or "server unavailable":

1. **STOP** all Elfiee-related operations immediately
2. **DO NOT** fall back to filesystem tools (Read, Write, Edit, Bash) to modify block content
3. **DO NOT** try to read or modify files in `.claude/`, `.elf/`, or any path that corresponds to .elf block directories
4. **REPORT** the connection failure to the human user
5. **WAIT** for human confirmation before taking any further action

### Why this matters:

- When Elfiee GUI is running, it holds the event store lock
- Direct filesystem modifications bypass event sourcing and WILL be overwritten
- The human user can check if Elfiee needs to be restarted or the agent re-enabled

### The only exception:

- Files that have been **exported to the filesystem** (via `elfiee_task_commit` or `elfiee_directory_export`) ARE normal filesystem files. You can read/edit them with standard tools after export. But the **source of truth** remains in the .elf event store — re-export will overwrite external changes.

## Standalone Mode

Run `elfiee mcp-server --elf /path/to/project.elf` as a subprocess. Configure in `.claude/mcp.json`:

```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/path/to/project.elf"]
    }
  }
}
```

Standalone mode auto-creates an `mcp-agent` editor with full permissions. Uses SQLite WAL mode for concurrent access.

## Quick Start

1. Call `elfiee_file_list` to get open projects and their paths
2. Use the `project` path (e.g., `"./my.elf"`) in all subsequent calls
3. Call `elfiee_block_list` to discover blocks
4. Use type-specific tools to read/write content

## Common Parameter: `project`

Every tool (except `elfiee_file_list`) requires `project` -- the `.elf` file path as returned by `elfiee_file_list`.

## Block Types

`markdown` | `code` | `directory` | `terminal` | `task`

## Tool Reference

### File Discovery

| Tool | Purpose | Params |
|------|---------|--------|
| `elfiee_file_list` | List open .elf files | (none) |

### Block CRUD

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_block_list` | List all blocks | `project` |
| `elfiee_block_get` | Get block details | `project`, `block_id` |
| `elfiee_block_create` | Create block | `project`, `name`, `block_type`, `parent_id?` |
| `elfiee_block_delete` | Delete block | `project`, `block_id` |
| `elfiee_block_rename` | Rename block | `project`, `block_id`, `name` |
| `elfiee_block_change_type` | Change type | `project`, `block_id`, `new_type` |
| `elfiee_block_update_metadata` | Update metadata | `project`, `block_id`, `metadata` (JSON object) |

### Block Relations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_block_link` | Link parent->child | `project`, `parent_id`, `child_id`, `relation` |
| `elfiee_block_unlink` | Remove relation | `project`, `parent_id`, `child_id`, `relation` |

Relation type: `implement` (the only allowed relation type). Semantic: `A → B` means "A's change caused B's change".

### Content Read/Write

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_markdown_read` | Read markdown | `project`, `block_id` |
| `elfiee_markdown_write` | Write markdown | `project`, `block_id`, `content` |
| `elfiee_code_read` | Read code | `project`, `block_id` |
| `elfiee_code_write` | Write code | `project`, `block_id`, `content` |

### Directory Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_directory_create` | Create file/dir entry | `project`, `block_id`, `path`, `type` (`file`/`directory`), `source` (`outline`/`linked`), `content?`, `block_type?` |
| `elfiee_directory_delete` | Delete entry | `project`, `block_id`, `path` |
| `elfiee_directory_rename` | Move/rename entry | `project`, `block_id`, `old_path`, `new_path` |
| `elfiee_directory_write` | Batch update entries | `project`, `block_id`, `entries` (JSON), `source?` |
| `elfiee_directory_import` | Import from filesystem | `project`, `block_id`, `source_path`, `target_path?` |
| `elfiee_directory_export` | Export to filesystem | `project`, `block_id`, `target_path`, `source_path?` |

> **Prefer `elfiee_task_commit` over `elfiee_directory_export`** when exporting code to a git repo. `elfiee_task_commit` handles the full workflow: export linked blocks → create feature branch → git add/commit. Use `elfiee_directory_export` only for non-git scenarios (e.g., exporting assets to a local directory).

### Terminal Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_terminal_init` | Start terminal session | `project`, `block_id`, `shell?` |
| `elfiee_terminal_execute` | Run command | `project`, `block_id`, `command` |
| `elfiee_terminal_save` | Save session content | `project`, `block_id`, `content` |
| `elfiee_terminal_close` | Close session | `project`, `block_id` |

### Task Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_task_create` | Create a new task | `project`, `name`, `description?` |
| `elfiee_task_write` | Write task content | `project`, `block_id`, `content` |
| `elfiee_task_commit` | Commit task to git | `project`, `block_id` |
| `elfiee_task_link` | Link task to implementation | `project`, `task_id`, `block_id` |

### Permission (CBAC)

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_grant` | Grant capability | `project`, `block_id`, `editor_id`, `cap_id` |
| `elfiee_revoke` | Revoke capability | `project`, `block_id`, `editor_id`, `cap_id` |

Capability IDs: `core.create`, `core.read`, `core.link`, `core.unlink`, `core.delete`, `core.update_metadata`, `core.rename`, `core.change_type`, `markdown.write`, `markdown.read`, `code.write`, `code.read`, `directory.create`, `directory.delete`, `directory.rename`, `directory.write`, `directory.import`, `directory.export`, `terminal.init`, `terminal.execute`, `terminal.save`, `terminal.close`, `task.write`, `task.read`, `task.commit`.

> **Agent permission note**: Agents do NOT have `core.grant` / `core.revoke` capabilities. Permission management is reserved for human owners via the Elfiee GUI. Do not attempt to call `elfiee_grant` / `elfiee_revoke` — they will fail with authorization errors.
>
> **Agent capabilities** (`agent.create/enable/disable`) are GUI-only — no MCP tool exists yet.

### Editor Management

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_editor_create` | Create editor | `project`, `editor_id`, `name?` |
| `elfiee_editor_delete` | Delete editor | `project`, `editor_id` |

### Generic Execution

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_exec` | Execute any capability | `project`, `capability`, `block_id?`, `payload?` |

`elfiee_exec` is a low-level passthrough that only runs the engine command (event generation + state update). It does **NOT** perform I/O side effects. Most capabilities already have dedicated MCP tools — prefer those.

> **When to use `elfiee_exec`**: Only for capabilities that lack a dedicated MCP tool and have no I/O side effects. In practice this is rare — almost all capabilities have dedicated tools listed above.
>
> **Never use `elfiee_exec` for these** (events will be recorded but I/O will be missing, causing inconsistent state):
>
> | Capability | Missing I/O | Use instead |
> |-----------|------------|-------------|
> | `task.commit` | file export, git branch/add/commit | `elfiee_task_commit` |
> | `terminal.*` | PTY session (process spawn, stdin/stdout) | `elfiee_terminal_*` |
> | `agent.*` | MCP server, symlink, .mcp.json | GUI only (no MCP tool yet) |

## Workflow Examples

### Read all markdown blocks

```
1. elfiee_file_list -> get project path
2. elfiee_block_list(project) -> find blocks where block_type == "markdown"
3. elfiee_markdown_read(project, block_id) -> for each markdown block
```

### Create a code file in a directory block

```
1. elfiee_file_list -> get project path
2. elfiee_block_list(project) -> find directory block
3. elfiee_directory_create(project, block_id, path="src/main.rs",
     type="file", source="outline", content="fn main() {}",
     block_type="code")
```

### Execute a terminal command

```
1. elfiee_file_list -> get project path
2. elfiee_block_list(project) -> find terminal block
3. elfiee_terminal_init(project, block_id)
4. elfiee_terminal_execute(project, block_id, command="cargo build")
5. elfiee_terminal_close(project, block_id)
```

### Link blocks with implement relation

```
1. elfiee_block_link(project, parent_id=task_block_id, child_id=code_block_id, relation="implement")
```

### /new-task workflow

When the user says `/new-task` or asks you to create a task:

**Step 1: Create task**
```
elfiee_task_create(project, name="Task name", description="What needs to be done")
-> Store returned task_block_id as ACTIVE_TASK
```

**Step 2: Work on implementation**
For EVERY code/markdown block you create or modify while working on this task:
```
elfiee_code_write(project, block_id, content)       # or markdown_write
elfiee_task_link(project, task_id=ACTIVE_TASK, block_id=block_id)  # auto-link
```
The link is idempotent — calling it multiple times for the same pair is safe.

**Step 3: Commit**
```
elfiee_task_commit(project, block_id=ACTIVE_TASK)
-> Exports implement-linked blocks to their git repos
-> Creates branch: feat/{task_name}
-> Git commit with task description
-> Returns: { commit_hash, branch_name, exported_files }
```

**Step 4: Test (optional)**
```
elfiee_terminal_execute(project, terminal_block_id, command="cd /repo && cargo test")
-> If tests fail: fix code -> commit again -> test again
-> If tests pass: task complete
```

## Causal Linking Protocol

**This is the core of Elfiee editing.** Every modification must be traceable to its cause through `implement` links.

### Rule: Link Before You Modify

Before modifying Block B because of Block A, create the causal link first:

```
elfiee_block_link(project, parent_id=A, child_id=B, relation="implement")
elfiee_code_write(project, block_id=B, content=...)  # then modify
```

### When to Create Links

| Scenario | Link | Example |
|----------|------|---------|
| Task describes requirement, you write Code | Task → Code | `task_block → src/auth.rs` |
| PRD defines tasks, you create Task | PRD → Task | `prd_block → task_block` |
| Code written, you write Test for it | Code → Test | `src/auth.rs → tests/auth_test.rs` |
| Bug report, you fix Code | Bug → Code | `bug_block → src/handler.rs` |
| Design drives UI component | Design → Code | `design_block → src/component.tsx` |

### When NOT to Link

- You only **read** a block for reference (no causal dependency)
- The two blocks are unrelated
- The link already exists (linking is idempotent, but check first to avoid noise)

### DAG Constraint and Cycle Rejection

The relationship graph is a strict **Directed Acyclic Graph (DAG)**. Elfiee automatically detects cycles and rejects link creation if a cycle would form.

**Link direction rule**: Arrow points from **cause** to **effect** — from the upstream block that drove the change to the downstream block that was changed.

```
cause → effect
Task  → Code       ✓ (task drives code writing)
Code  → Test       ✓ (code drives test writing)
Test  → Code       ✗ CYCLE if Code → Test already exists!
```

**If `elfiee_block_link` returns a cycle error**:

1. The link direction is wrong — re-examine the causal relationship
2. The driving force is probably an upstream block (e.g., Task), not the sibling
3. Do NOT create reverse links to work around the constraint

### TDD Best Practice

In TDD workflows, tests and code may be modified alternately. The correct link structure:

```
Task → Code → Test
```

- `Task → Code`: Task requirement drives code implementation
- `Code → Test`: Code implementation drives test writing

When test failure reveals a code bug:
- The root cause is still the **Task** (not the test)
- Do NOT create `Test → Code` — this would form a cycle with `Code → Test`
- The existing `Task → Code` link already captures the causal chain
- Fix the code, update the test — no new links needed

### Multi-block Modification Workflow

When a single task requires modifying multiple blocks:

```
1. elfiee_task_create(project, name="Add auth")  → task_id
2. For each block to modify:
   a. elfiee_block_link(project, parent_id=task_id, child_id=block_id, relation="implement")
   b. elfiee_code_write(project, block_id, content=...)
3. elfiee_task_commit(project, block_id=task_id)  → exports all linked code to git
```

## Graph-First Context Navigation

**Rule: When you need context, traverse the relationship graph BEFORE searching unrelated blocks.**

### Algorithm

1. **Read** the target block
2. **List** all blocks (`elfiee_block_list`) and examine their `children` to build a parent-child map
3. **Traverse up** (parents → grandparents → root) to understand **why** this block exists
4. **Traverse down** (children → grandchildren) to understand **what** this block produced
5. **Read siblings** (other children of the same parent) to understand **related work**
6. **Only then** search unrelated blocks if the graph doesn't provide enough context

### Example: Understanding a Code Block

```
1. elfiee_code_read(project, block_id="src/auth.rs")         # read the code
2. elfiee_block_list(project)                                  # get all blocks
3. Find: Task "Add auth" has children: [src/auth.rs, src/middleware.rs, tests/auth_test.rs]
4. elfiee_task_read(project, block_id=task_block)              # understand the requirement (parent)
5. elfiee_code_read(project, block_id="src/middleware.rs")     # read sibling implementation
6. elfiee_code_read(project, block_id="tests/auth_test.rs")   # read downstream test
```

This gives you the full causal context: **why** (task), **what** (code), **verification** (test) — without searching unrelated blocks.

## MCP Resources

Read-only data accessible via `ReadMcpResourceTool` (server: `elfiee`).

### Static Resources

| URI | Description |
|-----|-------------|
| `elfiee://files` | List of currently open .elf project files |

### Dynamic Resources (per project)

| URI Pattern | Description |
|-------------|-------------|
| `elfiee://{project}/blocks` | All blocks in project (summary) |
| `elfiee://{project}/block/{block_id}` | Full content of a specific block |
| `elfiee://{project}/grants` | Permission grants table |
| `elfiee://{project}/events` | Event sourcing log |

Replace `{project}` with the project path (e.g., `./my.elf`) and `{block_id}` with the block ID.

## Error Handling

| Error | Cause | Fix |
|-------|-------|-----|
| `Project not open` | .elf file not loaded | Open file in Elfiee GUI first, or use standalone mode |
| `Block not found` | Invalid block_id | Use `elfiee_block_list` to get valid IDs |
| `No active editor` | No editor session | GUI must have an active editor session |
| `Engine not found` | Engine not started | Reopen file in GUI |
| `Invalid payload` | Wrong parameters | Check the tool's parameter schema |

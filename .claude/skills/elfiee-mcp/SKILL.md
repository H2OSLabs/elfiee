---
name: elfiee-mcp
description: "Guide for using Elfiee MCP tools to interact with .elf projects. Use when Claude needs to read, write, or manage blocks via MCP tools (elfiee_auth, elfiee_open, elfiee_close, elfiee_file_list, elfiee_block_*, elfiee_document_*, elfiee_session_*, elfiee_task_*, elfiee_grant/revoke, elfiee_editor_*, elfiee_block_history, elfiee_state_at_event, elfiee_exec) or MCP resources (elfiee://files, elfiee://{project}/blocks, elfiee://{project}/block/{id}, elfiee://{project}/grants, elfiee://{project}/events). Triggers: working with .elf files, managing blocks, reading/writing document content, session logging, task management, permission management."
---

# Elfiee MCP Tools

Elfiee is an **EventWeaver** — an event-sourced, capability-controlled block management system.
It exposes MCP tools and resources via **SSE transport** on port 47200.

## Connection Protocol

1. **Authenticate**: `elfiee_auth` — bind your editor_id to this connection
2. **Open project**: `elfiee_open` — open (or create) an .elf project
3. **Operate**: Use block/document/task/session tools

## Prohibited Actions

**ALL content managed by .elf blocks MUST be read and written through Elfiee MCP tools.**

| Prohibited | Use instead |
|-----------|-------------|
| `Read` / `cat` / `head` to read block content | `elfiee_document_read` / `elfiee_block_get` |
| `Write` / `Edit` to modify block content | `elfiee_document_write` |
| `Bash` with `ls` / `rm` / `mv` on .elf internals | `elfiee_block_list` / `elfiee_block_delete` / `elfiee_block_rename` |
| Directly editing files in `.elf/` | Always use MCP tools |

**Why**: .elf uses event sourcing — direct edits bypass the event log and are lost.

## Block Types

| Type | Purpose | Key Fields |
|------|---------|------------|
| `document` | Rich content (markdown, code) | `content`, `format` |
| `task` | Work items with status tracking | `description`, `status`, `assigned_to` |
| `session` | Append-only conversation log | `entries[]` (command/message/decision) |

## Tool Reference

### Connection Management

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_auth` | Authenticate connection | `editor_id`, `project?`, `role?` |
| `elfiee_open` | Open/create project | `project` |
| `elfiee_close` | Close project | `project` |
| `elfiee_file_list` | List open projects | (none) |

### Block CRUD

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_block_list` | List all blocks (CBAC filtered) | `project` |
| `elfiee_block_get` | Get block details (CBAC) | `project`, `block_id` |
| `elfiee_block_create` | Create block | `project`, `name`, `block_type`, `parent_id?` |
| `elfiee_block_delete` | Delete block | `project`, `block_id` |
| `elfiee_block_rename` | Rename block | `project`, `block_id`, `name` |

### Block Relations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_block_link` | Add relation (parent -> child) | `project`, `parent_id`, `child_id`, `relation` |
| `elfiee_block_unlink` | Remove relation | `project`, `parent_id`, `child_id`, `relation` |

Only `implement` relation type is allowed. `A -> B` means "A defines/causes B".

### Content Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_document_read` | Read document content (CBAC) | `project`, `block_id` |
| `elfiee_document_write` | Write document content | `project`, `block_id`, `content` |
| `elfiee_task_read` | Read task details (CBAC) | `project`, `block_id` |
| `elfiee_task_create` | Create task block | `project`, `name`, `description?` |
| `elfiee_task_write` | Update task fields | `project`, `block_id`, `description?`, `status?`, `assigned_to?` |
| `elfiee_task_commit` | Commit task (mark done) | `project`, `block_id` |
| `elfiee_task_link` | Link implementation to task | `project`, `task_id`, `block_id` |
| `elfiee_session_read` | Read session entries (CBAC) | `project`, `block_id` |
| `elfiee_session_append` | Append session entry | `project`, `block_id`, `entry_type`, `data` |

### Event History & Time Travel

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_block_history` | Get block's event history (CBAC) | `project`, `block_id` |
| `elfiee_state_at_event` | Time travel: block state at event (CBAC) | `project`, `block_id`, `event_id` |

### Permission (CBAC)

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_grant` | Grant capability | `project`, `block_id`, `editor_id`, `cap_id` |
| `elfiee_revoke` | Revoke capability | `project`, `block_id`, `editor_id`, `cap_id` |

Capability IDs: `core.create`, `core.link`, `core.unlink`, `core.delete`, `core.grant`, `core.revoke`, `document.read`, `document.write`, `task.read`, `task.write`, `task.commit`, `session.append`, `session.read`, `editor.create`, `editor.delete`.

### Editor Management

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_editor_create` | Create editor | `project`, `editor_id`, `name?` |
| `elfiee_editor_delete` | Delete editor | `project`, `editor_id` |

### Generic Execution

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_exec` | Execute any capability | `project`, `capability`, `block_id?`, `payload?` |

## Causal Linking Protocol

Every time you modify block B because of block A, create a link:
```
elfiee_block_link(project, parent_id=A, child_id=B, relation="implement")
```

| Scenario | Link |
|----------|------|
| Task describes requirement, you create document | Task -> Document |
| PRD defines tasks | PRD -> Task |
| Bug report leads to fix | Bug -> Code |

## Graph-First Context Navigation

When you need context, traverse the relation graph first:

1. `elfiee_block_get(target)` — read the block
2. `elfiee_block_list` — get all blocks with children
3. Build parent map, traverse upstream (the "why")
4. Traverse downstream children (the "how")
5. Read siblings (share same parent)
6. Only then search unlinked blocks

## MCP Resources

| URI | Description |
|-----|-------------|
| `elfiee://files` | Open projects |
| `elfiee://{project}/blocks` | All blocks |
| `elfiee://{project}/block/{id}` | Single block |
| `elfiee://{project}/grants` | Permission table |
| `elfiee://{project}/events` | Event log |
| `elfiee://{project}/editors` | Editor list |
| `elfiee://{project}/my-tasks` | Tasks assigned to current editor |
| `elfiee://{project}/my-grants` | Permissions of current editor |

## CBAC (Capability-Based Access Control)

All read operations are CBAC-filtered — you only see blocks you have permission to access:

- **Block list/get**: Returns only blocks you own or have grants for
- **Document/Task/Session read**: Requires `{type}.read` (e.g., `document.read`, `task.read`, `session.read`)
- **Event history**: Block events filtered by `{block_type}.read`; editor events always visible
- **Time travel**: Requires `{block_type}.read` on the target block
- **Owner**: Always has full access to their own blocks
- **Wildcard grants**: `block_id = "*"` grants access to all blocks

`block_type` is **immutable** — determined at creation time (init/scan), cannot be changed afterwards.

## Error Handling

| Error | Fix |
|-------|-----|
| `Not authenticated` | Call `elfiee_auth` first |
| `Project not open` | Call `elfiee_open` first |
| `Block not found` | Use `elfiee_block_list` to find valid IDs |
| `Not authorized` | Check grants with `elfiee_grant` |
| `Invalid payload` | Check the tool's parameter schema |

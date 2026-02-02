---
name: elfiee-client
description: "Guide for using Elfiee MCP tools to interact with .elf files. Use when Claude needs to read, write, or manage blocks inside .elf projects via MCP tools (elfiee_file_list, elfiee_block_*, elfiee_markdown_*, elfiee_code_*, elfiee_directory_*, elfiee_terminal_*, elfiee_grant/revoke, elfiee_editor_*, elfiee_exec) or MCP resources (elfiee://files, elfiee://{project}/blocks, elfiee://{project}/block/{id}, elfiee://{project}/grants, elfiee://{project}/events). Triggers: working with .elf files, managing blocks, reading/writing markdown or code in blocks, directory operations inside .elf, terminal sessions, permission management."
---

# Elfiee MCP Tools

Elfiee exposes MCP tools and resources for interacting with `.elf` files. Two connection modes:

| Mode | Transport | When to use |
|------|-----------|-------------|
| **GUI mode** | SSE on port 47200 | Elfiee GUI is running with files open |
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

### The only exception:

- `elfiee_directory_export` explicitly exports block content to the filesystem for external use (e.g., git commit). Files created by export ARE normal filesystem files and can be read/edited normally after export.

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

`markdown` | `code` | `directory` | `terminal`

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

Relation types: `contains`, `references`, `implement`, or custom strings.

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

### Terminal Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_terminal_init` | Start terminal session | `project`, `block_id`, `shell?` |
| `elfiee_terminal_execute` | Run command | `project`, `block_id`, `command` |
| `elfiee_terminal_save` | Save session content | `project`, `block_id`, `content` |
| `elfiee_terminal_close` | Close session | `project`, `block_id` |

### Permission (CBAC)

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_grant` | Grant capability | `project`, `block_id`, `editor_id`, `cap_id` |
| `elfiee_revoke` | Revoke capability | `project`, `block_id`, `editor_id`, `cap_id` |

Capability IDs: `core.create`, `core.read`, `core.link`, `core.unlink`, `core.delete`, `core.grant`, `core.revoke`, `core.update_metadata`, `core.rename`, `core.change_type`, `markdown.write`, `markdown.read`, `code.write`, `code.read`, `directory.create`, `directory.delete`, `directory.rename`, `directory.write`, `directory.import`, `directory.export`, `terminal.init`, `terminal.execute`, `terminal.save`, `terminal.close`, `agent.create`, `agent.enable`, `agent.disable`.

### Editor Management

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_editor_create` | Create editor | `project`, `editor_id`, `name?` |
| `elfiee_editor_delete` | Delete editor | `project`, `editor_id` |

### Generic Execution

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_exec` | Execute any capability | `project`, `capability`, `block_id?`, `payload?` |

Use `elfiee_exec` for capabilities not covered by dedicated tools.

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

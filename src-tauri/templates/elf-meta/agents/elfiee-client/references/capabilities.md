# Elfiee Capabilities Reference

All state changes in Elfiee are performed through **Capabilities** — typed operations that generate immutable events. Each capability has a handler (execution logic) and a certificator (authorization check).

## Authorization Model (CBAC)

- **Owner Always Authorized**: Block owners can perform any capability on their blocks
- **Grant-Based**: Non-owners need explicit grants via `core.grant`
- **Wildcard Grants**: `block_id = "*"` grants permission on all blocks

## Core Capabilities

### core.create
- **Purpose**: Create a new block
- **Target**: `core/*` (no existing block required)
- **Params**: `name` (string), `block_type` (string), `source?` (string), `metadata?` (object)
- **Returns**: Event with full initial block state

### core.read
- **Purpose**: Read block metadata (permission gate only, no events generated)
- **Target**: Any block type
- **Params**: (none)

### core.link
- **Purpose**: Add a relation between blocks
- **Target**: Any block type
- **Params**: `relation` (string, e.g. "implement"), `target_id` (string)
- **Returns**: Event with updated children map

### core.unlink
- **Purpose**: Remove a relation between blocks
- **Target**: Any block type
- **Params**: `relation` (string), `target_id` (string)
- **Returns**: Event with updated children map

### core.delete
- **Purpose**: Soft-delete a block
- **Target**: Any block type
- **Params**: (none)
- **Returns**: Deletion event

### core.grant
- **Purpose**: Grant a capability to an editor on a block
- **Target**: Any block type
- **Params**: `target_editor` (string), `capability` (string), `target_block` (string)
- **Returns**: Grant event

### core.revoke
- **Purpose**: Revoke a capability from an editor
- **Target**: Any block type
- **Params**: `target_editor` (string), `capability` (string), `target_block` (string)
- **Returns**: Revoke event

### core.update_metadata
- **Purpose**: Update block metadata fields
- **Target**: Any block type
- **Params**: `metadata` (object to merge)
- **Returns**: Event with updated metadata

### core.rename
- **Purpose**: Rename a block
- **Target**: Any block type
- **Params**: `name` (string)
- **Returns**: Event with new name

### core.change_type
- **Purpose**: Change a block's type
- **Target**: Any block type
- **Params**: `new_type` (string)
- **Returns**: Event with new type

## Content Capabilities

### markdown.write
- **Purpose**: Write markdown content to a block
- **Target**: `markdown` blocks
- **Params**: `content` (string)
- **Returns**: Event with new content

### markdown.read
- **Purpose**: Read markdown content from a block
- **Target**: `markdown` blocks
- **Params**: (none)
- **Returns**: Event recording the read (entity = editor_id)

### code.write
- **Purpose**: Write code content to a block
- **Target**: `code` blocks
- **Params**: `content` (string)
- **Returns**: Event with new content

### code.read
- **Purpose**: Read code content from a block
- **Target**: `code` blocks
- **Params**: (none)
- **Returns**: Event recording the read (entity = editor_id)

## Directory Capabilities

### directory.create
- **Purpose**: Create a file or directory entry within a directory block
- **Target**: `directory` blocks
- **Params**: `path` (string), `type` ("file" | "directory"), `source` ("outline" | "linked"), `content?` (string), `block_type?` (string)

### directory.delete
- **Purpose**: Delete an entry from a directory block
- **Target**: `directory` blocks
- **Params**: `path` (string)

### directory.rename
- **Purpose**: Move/rename an entry within a directory block
- **Target**: `directory` blocks
- **Params**: `old_path` (string), `new_path` (string)

### directory.write
- **Purpose**: Batch update directory entries
- **Target**: `directory` blocks
- **Params**: `entries` (JSON object), `source?` (string)

### directory.import
- **Purpose**: Import files from the filesystem into a directory block
- **Target**: `directory` blocks
- **Params**: `source_path` (string), `target_path?` (string)

### directory.export
- **Purpose**: Export directory block contents to the filesystem
- **Target**: `directory` blocks
- **Params**: `target_path` (string), `source_path?` (string)

## Terminal Capabilities

### terminal.init
- **Purpose**: Initialize a terminal session for a terminal block
- **Target**: `terminal` blocks
- **Params**: `shell?` (string, e.g. "bash", "powershell")

### terminal.execute
- **Purpose**: Execute a command in a terminal session
- **Target**: `terminal` blocks
- **Params**: `command` (string)

### terminal.save
- **Purpose**: Save terminal session content
- **Target**: `terminal` blocks
- **Params**: `content` (string)

### terminal.close
- **Purpose**: Close a terminal session
- **Target**: `terminal` blocks
- **Params**: (none)

## Task Capabilities

### task.write
- **Purpose**: Write markdown content to a task block
- **Target**: `task` blocks
- **Params**: `content` (string)
- **Returns**: Event with new content

### task.read
- **Purpose**: Read content from a task block
- **Target**: `task` blocks
- **Params**: (none)
- **Returns**: Event recording the read (entity = editor_id)

### task.commit
- **Purpose**: Commit a task — export linked blocks to git repos, create branch, commit
- **Target**: `task` blocks
- **Params**: (none)
- **Returns**: Event with downstream_block_ids and commit metadata

## Agent Capabilities

### agent.create
- **Purpose**: Create an Agent Block bound to a `.claude/` directory
- **Target**: `core/*` (no existing block required)
- **Params**: `claude_dir` (string — path to `.claude/` directory), `name?` (string, default "elfiee"), `editor_id?` (string — auto-created if omitted)
- **Returns**: Event with Agent Block initial state (auto-enabled, per-agent MCP server started)

### agent.enable
- **Purpose**: Enable an agent (create symlink + inject MCP config)
- **Target**: `agent` blocks
- **Params**: (none, uses block_id from command)
- **Returns**: Event with status = "enabled"

### agent.disable
- **Purpose**: Disable an agent (clean symlink + remove MCP config)
- **Target**: `agent` blocks
- **Params**: (none, uses block_id from command)
- **Returns**: Event with status = "disabled"

## Editor Capabilities

### editor.create
- **Purpose**: Create a new editor identity
- **Params**: `editor_id` (string), `name?` (string)

### editor.delete
- **Purpose**: Remove an editor identity
- **Params**: `editor_id` (string)

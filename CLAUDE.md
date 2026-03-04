# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## CRITICAL: Required Skills

**Before working with this project, you MUST read these skill documents:**

### System-level: `docs/skills/elfiee-dev/SKILL.md`
How AI interacts with `.elf` files:
- Forbidden filesystem commands (cat, ls, rm, etc.)
- Required elf APIs (Read, Write, File, Terminal)
- NEVER use shell commands on .elf contents

### Project-level: `docs/skills/elfiee-workflow/SKILL.md`
Development rules for frontend and backend:
- Forbidden actions (edit bindings.ts, use invoke(), etc.)
- Frontend/Backend development patterns
- Capability registration and payload reference
- Pre-commit checklist

Violating these rules breaks type safety, event sourcing, or CBAC security.

## Project Overview

**Elfiee** is a **passive EventWeaver** — an event-sourced, capability-controlled block management system. The four core pillars:

1. **Event Sourcing**: All changes captured as immutable event log with complete history
2. **CBAC (Capability-Based Access Control)**: Fine-grained permission model per editor/block/capability
3. **Block DAG**: Typed blocks (document, task, session) linked by `implement` relations
4. **Agent Template**: Declarative role/permission/workflow definitions (Socialware)

Elfiee is a **pure passive MCP Server (EventWeaver)** — it does NOT orchestrate agents. Agents call Elfiee through MCP tools; Elfiee never spawns agents or sends commands to them. Human and Agent editors are equal (Socialware principle).

## Technology Stack

**Dual deployment mode**: Tauri desktop app + `elf serve` headless MCP server.

- **Backend**: Rust — shared engine + services code for both modes
- **Frontend**: React + TypeScript with Vite (Tauri desktop only)
- **Communication**:
  - **MCP SSE**: Primary external protocol (`elf serve` on port 47200), all agents connect here
  - **Tauri IPC**: GUI efficiency optimization via tauri-specta type-safe commands
  - **CLI**: `elf` binary for terminal operations (init, register, scan, grant, revoke, etc.)

## Architecture Overview

### Three-Layer Architecture

All three transport layers route through a unified **services** layer:

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Tauri IPC  │  │  MCP Server │  │    CLI       │
│ (commands/) │  │  (mcp/)     │  │  (cli/)      │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                 ┌──────▼──────┐
                 │  Services   │  ← CBAC filtering + business logic
                 │ (services/) │
                 └──────┬──────┘
                        │
                 ┌──────▼──────┐
                 │   Engine    │  ← Actor model, event sourcing
                 │ (engine/)   │
                 └─────────────┘
```

### Services Layer (`src/services/`)

Encapsulates CBAC filtering and business logic. All read operations filter by editor permissions:

| Module | Purpose |
|--------|---------|
| `project` | Open/close/list projects, seed bootstrap events |
| `block` | List/get/rename blocks (CBAC filtered) |
| `document` | Read/write document content (type-checked + CBAC) |
| `task` | Read/write/commit task blocks (type-checked + CBAC) |
| `session` | Read/append session entries (type-checked + CBAC) |
| `editor` | List/get editor info |
| `event` | List events, block history, state-at-event (CBAC) |
| `grant` | List/grant/revoke permissions |

### Core Data Models

The system uses strongly-typed entities:

- **Block**: Fundamental content unit with `block_id`, `block_type` (document/task/session), `contents` (JSON), and `children` (relation graph)
- **Editor**: User or agent with `editor_id` — human and bot types are equal
- **Capability**: Defines actions with `certificator` (authorization) and `handler` (execution)
- **CapabilitiesGrant**: CBAC table mapping `editor_id` + `cap_id` + `block_id`

### Event Structure (EAVT)

All state changes stored as Events in `eventstore.db`:

- **Entity**: ID of changed entity (block_id/editor_id)
- **Attribute**: Change descriptor `"{editor_id}/{cap_id}"`
- **Value**: JSON payload with **event modes**: `full`, `delta`, `ref`, `append`
- **Timestamp**: Vector clock `Record<editor_id, transaction_count>` for conflict resolution

### .elf Project Format

`.elf/` is a directory (like `.git/`), **not a ZIP archive**:

```
project/
├── .elf/                    # Elfiee project directory
│   ├── eventstore.db        # Canonical event log (SQLite) — source of truth
│   ├── cache.db             # CacheStore: block snapshots for fast replay
│   ├── config.toml          # Project config (Git mode, etc.)
│   └── templates/
│       └── skills/
│           └── default.md   # Default agent skill document
├── src/                     # Project source files
└── ...
```

`save` is a no-op — events are written directly to SQLite. Git handles versioning.

### Engine (Actor Model)

Each `.elf` project has a dedicated **EngineActor** processing commands serially via tokio channels:

1. Receive `Command` from mailbox (mpsc channel)
2. Load `Capability`, `Block`, `Editor` from state projection
3. **Authorize**: Call `Capability.certificator()` → reject if fail
4. **Execute**: Call `Capability.handler()` → produce events
5. **Conflict Check**: Compare vector clocks → reject if stale
6. **Commit**: Atomic append to `eventstore.db`
7. **Project**: Apply events to in-memory state
8. **Notify**: Broadcast via tokio broadcast channel

**EngineHandle** wraps the mpsc channel with async API methods (`process_command`, `get_block`, `get_all_blocks`, `get_all_editors`, `get_all_grants`, `get_editor_grants`, etc.).

### Capabilities

**Built-in (core)**:
- `core.create` — Create new blocks
- `core.write` — Update block metadata (name, description)
- `core.link` / `core.unlink` — DAG relations
- `core.delete` — Soft-delete blocks
- `core.grant` / `core.revoke` — Permission management
- `editor.create` / `editor.delete` — Editor lifecycle

**Extension capabilities** (per block type):
- `document.write` / `document.read` — Document content
- `task.write` / `task.read` / `task.commit` — Task management
- `session.append` / `session.read` — Session logging

**Authorization model**: Owner always authorized → Grant-based → Wildcard grants (`block_id = "*"`)

### MCP Server

Per-connection identity model:
- Each MCP connection gets an independent `ElfieeMcpServer` instance
- `elfiee_auth` tool authenticates with `editor_id`
- `elfiee_open` / `elfiee_close` manage project lifecycle
- All operations go through services layer with CBAC

## File Organization

```
elfiee/
├── CLAUDE.md                # This file
├── package.json             # Frontend dependencies
├── src/                     # React frontend
│   ├── bindings.ts          # AUTO-GENERATED by tauri-specta (NEVER edit)
│   ├── components/          # React components
│   └── ...
├── src-tauri/               # Rust backend
│   ├── Cargo.toml
│   ├── .elfignore           # Default ignore patterns for scan
│   ├── .elftypes            # Extension → block type mapping
│   ├── templates/           # Workflow templates (TOML)
│   └── src/
│       ├── main.rs          # Tauri app entry
│       ├── lib.rs           # Library root (Tauri command registration)
│       ├── bin/
│       │   ├── elf.rs       # CLI binary entry point
│       │   └── serve.rs     # Headless MCP server binary
│       ├── engine/          # Core engine
│       │   ├── actor.rs     # EngineActor (command processing)
│       │   ├── manager.rs   # EngineManager (multi-project)
│       │   ├── state.rs     # StateProjector (event replay)
│       │   ├── event_store.rs  # SQLite event persistence
│       │   └── cache_store.rs  # Block snapshot cache
│       ├── models/          # Data models
│       │   ├── block.rs     # Block entity
│       │   ├── editor.rs    # Editor entity
│       │   ├── event.rs     # Event (with EventMode)
│       │   ├── grant.rs     # Grant entry
│       │   └── payloads.rs  # Core payload types
│       ├── capabilities/    # CBAC system
│       │   ├── registry.rs  # CapabilityRegistry
│       │   ├── grants.rs    # GrantsTable
│       │   ├── core.rs      # Core capability definitions
│       │   └── builtins/    # Built-in capability handlers
│       ├── extensions/      # Block-type extensions
│       │   ├── document/    # document.write, document.read
│       │   ├── task/        # task.write, task.read, task.commit
│       │   └── session/     # session.append, session.read
│       ├── services/        # Business logic + CBAC filtering
│       │   ├── project.rs   # Project open/close/list
│       │   ├── block.rs     # Block CRUD (CBAC filtered)
│       │   ├── document.rs  # Document read/write
│       │   ├── task.rs      # Task read/write/commit
│       │   ├── session.rs   # Session read/append
│       │   ├── editor.rs    # Editor info
│       │   ├── event.rs     # Event history
│       │   └── grant.rs     # Permission management
│       ├── commands/        # Tauri IPC commands (thin wrappers)
│       │   ├── file.rs      # File open/close/list
│       │   ├── block.rs     # Block operations
│       │   ├── editor.rs    # Editor operations
│       │   └── event.rs     # Event queries
│       ├── mcp/             # MCP SSE server
│       │   ├── server.rs    # ElfieeMcpServer (per-connection)
│       │   ├── transport.rs # SSE transport layer
│       │   └── mod.rs       # Server startup
│       ├── cli/             # CLI subcommands
│       │   ├── init.rs      # elf init
│       │   ├── register.rs  # elf register
│       │   ├── scan.rs      # elf scan
│       │   ├── run.rs       # elf run <template>
│       │   ├── status.rs    # elf status
│       │   ├── block.rs     # elf block list
│       │   ├── grant.rs     # elf grant
│       │   ├── revoke.rs    # elf revoke
│       │   └── resolve.rs   # Block name/id resolution
│       ├── elf_project/     # .elf/ directory management
│       │   ├── mod.rs       # ElfProject (init/open/skill)
│       │   └── config.rs    # ProjectConfig (TOML)
│       ├── config.rs        # System config (editor ID)
│       ├── state.rs         # AppState (shared state)
│       ├── events.rs        # Tauri event definitions
│       └── utils/           # Utilities
│           ├── block_type_inference.rs  # .elftypes parser
│           ├── path_validator.rs
│           └── time.rs
├── docs/                    # Documentation
│   ├── mvp/frame/concepts/  # Architecture docs (Layer 0-6)
│   ├── mvp/frame/changelogs/ # Refactoring changelogs
│   └── guides/              # Development guides
└── tests/                   # Integration tests
```

## Key Crates

- `sqlx` — SQLite interface for event store
- `serde`, `serde_json` — JSON serialization
- `uuid` — ID generation
- `rmcp` — MCP server implementation (SSE transport)
- `tokio` — Async runtime for actor model
- `dashmap` — Concurrent hashmap for engine manager
- `tauri`, `tauri-specta` — Desktop app + type-safe IPC
- `clap` — CLI argument parsing
- `ignore` — .gitignore/.elfignore-aware file scanning

## Development Commands

```bash
# Run Tauri dev server (hot reload)
pnpm tauri dev

# Run Rust tests (320+ tests)
cd src-tauri && cargo test

# Run clippy
cd src-tauri && cargo clippy

# Run headless MCP server
cd src-tauri && cargo run --bin elf -- serve --port 47200

# Initialize a project
cd src-tauri && cargo run --bin elf -- init /path/to/project

# Register an agent
cd src-tauri && cargo run --bin elf -- register openclaw --project /path/to/project

# Build production app
pnpm tauri build
```

## TypeScript Bindings Generation (CRITICAL)

**HARD RULE**: `src/bindings.ts` is AUTO-GENERATED by tauri-specta. NEVER modify it manually.

1. Add/modify Tauri commands in Rust (`src-tauri/src/commands/*.rs`)
2. Register commands in `src-tauri/src/lib.rs` (both debug and release handlers)
3. Run `pnpm tauri dev` to regenerate `src/bindings.ts`

## Capability Payload Types (CRITICAL)

**HARD RULE**: For every capability that accepts input, define a typed Rust payload struct with `#[derive(Serialize, Deserialize, Type)]`. NEVER use manual JSON parsing.

**Payload Location Rules**:
- **Extension-specific** payloads: Define in `src/extensions/{extension_name}/mod.rs`
- **Core** payloads: Define in `src/models/payloads.rs`

## Documentation

- **Architecture concepts**: `docs/mvp/frame/concepts/` (Layer 0-6 design docs)
- **Changelogs**: `docs/mvp/frame/changelogs/` (refactoring history)
- **Extension guide**: `docs/guides/EXTENSION_DEVELOPMENT.md`
- **Frontend guide**: `docs/guides/FRONTEND_DEVELOPMENT.md`

**For contributors**: Do not make modifications directly on main/dev branches. Always create a new branch and open a PR to merge to dev.

# API: Commands & Events

> Status: **Implemented (v0.1 MVP)** for the Tauri IPC surface. The REST/WebSocket
> section at the end is **Planned**.

The frontend talks to the Rust core over Tauri IPC. JS argument names are
**camelCase**; Tauri maps them to the snake_case Rust parameters. The typed
bridge is `src/lib/ipc.ts` (`api.*`); DTOs are in `src/lib/types.ts` (mirrored
from `src-tauri/src/model.rs`). Commands are registered in
`src-tauri/src/main.rs` and defined in `src-tauri/src/commands.rs`.

All commands return `Result<T, AppError>`; errors serialize to a plain string
(`no vault is open`, `invalid path`, `a file or folder already exists…`, IO/DB
messages). Most commands require an open vault.

## Command surface

### Vault lifecycle

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `open_vault` | `path: string` | `VaultInfo` | Opens a folder as a vault; kicks off the full index on a background thread (progress via events). |
| `close_vault` | — | `void` | Drops the open vault (and its watcher). |
| `startup_vault` | — | `string \| null` | Returns the `NEXUS_VAULT` env var if set — lets the app/CI launch straight into a vault. |
| `reindex` | — | `void` | Re-runs `reconcile_full` on a background thread. |
| `get_vault_tree` | — | `TreeNode` | The folder/note tree (folders first, then case-insensitive name order). |

### File CRUD

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `read_note` | `path: string` | `RawNote` | Splits frontmatter from body; returns body, frontmatter, hash, mtime. |
| `write_note` | `path: string`, `markdown: string`, `frontmatter: object` | `NoteMeta` | Composes Markdown, atomic write + arm guard, synchronous index upsert. Emits no event. |
| `create_note` | `parent: string`, `name: string` | `NoteMeta` | Creates `parent/name.md` with an `# H1` stub. Errors if it exists. |
| `create_folder` | `parent: string`, `name: string` | `void` | Creates a directory. |
| `rename_path` | `from: string`, `to: string` | `void` | Renames a file or folder (re-indexes the subtree). |
| `move_path` | `from: string`, `toParent: string` | `string` (new path) | Moves into another folder. |
| `delete_path` | `path: string` | `void` | Deletes a file or recursively a folder. |

Names are validated by `paths::is_safe_name` (no `/`, `\`, `.`, `..`) to guard
against path traversal.

### Search & index queries

| Command | Args | Returns |
|---------|------|---------|
| `search` | `query: string`, `limit?: number` (default 50) | `SearchHit[]` |
| `search_suggest` | `prefix: string` | `Suggestion[]` (notes + tags) |
| `get_backlinks` | `path: string` | `Backlink[]` |
| `get_outline` | `path: string` | `Heading[]` |
| `list_tags` | — | `TagCount[]` |
| `notes_with_tag` | `tag: string`, `includeSubtags: boolean` | `SearchHit[]` |

### Graph

| Command | Args | Returns |
|---------|------|---------|
| `graph_global` | — | `GraphData` |
| `graph_local` | `path: string`, `depth: number` | `GraphData` (BFS neighborhood) |

### Databases

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `list_databases` | — | `DbInfo[]` | All registered `.nexusdb.json` folders + record counts. |
| `get_database` | `path: string` | `DbSchema` | Columns + views for one database. |
| `query_database` | `path: string` | `DbResult` | Schema + every record row (frontmatter as `fields`). |
| `upsert_record` | `dbPath: string`, `path: string \| null`, `fields: object` | `NoteMeta` | Creates (when `path` is null) or updates a record by merging `fields` into its frontmatter; refreshes DB membership. |

### Example (`src/lib/ipc.ts`)

```ts
const info = await api.openVault("/Users/you/Notes");
const tree = await api.getVaultTree();
const note = await api.readNote("Projects/Roadmap.md");
await api.writeNote(note.path, note.markdown, { ...note.frontmatter, status: "done" });
const hits = await api.search("kanban OR graph", 20);
const back = await api.getBacklinks("Concepts/Knowledge Graph.md");
```

> Outside Tauri (`pnpm dev` in a plain browser), `ipc.ts` serves every call from
> an **in-memory mock vault**, so the full UI renders and is screenshot-verifiable
> without the backend.

## Events (Rust → UI)

Defined in `src-tauri/src/events.rs` (names under `events::names`); the UI
subscribes via `onEvent` in `src/App.tsx`.

| Event | Payload | Emitted when |
|-------|---------|--------------|
| `fs:created` | `FsEvent { path, kind, meta? }` | A note/folder appears externally. UI refreshes the tree. |
| `fs:changed` | `FsEvent { path, kind, meta? }` | A known note changed externally. Forwarded to the editor (reloads if unsaved-free). |
| `fs:deleted` | `FsEvent { path, kind, meta? }` | A note/folder removed externally. UI refreshes the tree. |
| `index:progress` | `IndexProgress { done, total, phase }` | Periodically during a full reindex (~every 50 notes). |
| `index:ready` | `IndexReady { noteCount }` | A full reindex finished. |
| `sync:status` | `SyncStatus { state }` | State change: `"idle" \| "watching" \| "reconciling"`. |

`FsEvent.kind` is `"note" \| "folder"`. App-initiated writes emit **no** event —
see [04-filesystem-sync.md](./04-filesystem-sync.md).

---

## Planned: local REST / WebSocket API

> Status: **Planned (roadmap)** — not built. Sketch only; names/shapes will firm
> up during implementation.

A future opt-in local server (bound to `127.0.0.1`, off by default, token-gated)
would expose the same capabilities to external tools, scripts, and integrations:

- **REST** (read/write notes, search, query databases), e.g.

  ```
  GET    /api/v1/notes/{path}
  PUT    /api/v1/notes/{path}        # { markdown, frontmatter }
  POST   /api/v1/search              # { query, limit }
  GET    /api/v1/databases/{path}
  POST   /api/v1/databases/{path}/records
  ```

- **WebSocket** at `/api/v1/events` streaming the same `fs:*` / `index:*` /
  `sync:*` events for live external clients.

Auth via a per-vault bearer token; scopes mirror the future plugin capability
model in [07-plugin-architecture.md](./07-plugin-architecture.md).

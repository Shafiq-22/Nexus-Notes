# Architecture

> Status: **Implemented (v0.1 MVP)** unless a line is explicitly marked _Planned_.

Nexus Notes is a **local-first** desktop knowledge app — an Obsidian + Notion
hybrid. It is built as a [Tauri v2](https://tauri.app) application: a Rust core
that owns the filesystem and a SQLite index, paired with a React/TypeScript UI
rendered in the system WebView.

## Stack

| Layer        | Technology |
|--------------|-----------|
| Shell        | Tauri v2 (Rust) |
| UI           | React 18 + TypeScript + Vite |
| Styling      | Tailwind CSS (CSS-variable theme tokens) |
| State        | Zustand (`src/store.ts`) |
| Editor       | Tiptap (ProseMirror) + `tiptap-markdown` |
| Index / FTS  | `rusqlite` (bundled SQLite, FTS5) |
| Watcher      | `notify` + `notify-debouncer-full` |
| Hashing      | `blake3` |
| Walk         | `walkdir` |
| Parsing      | `nexus-vault` crate (`pulldown-cmark`, `regex`, `serde_yaml`) |

## Core principle: disk is truth, index is derived

The only authoritative state is what lives on disk:

- **Notes** are plain Markdown files (`*.md`) with optional YAML frontmatter.
- **Databases** are folders containing a `.nexusdb.json` marker.
- All **derived** state — the metadata index, full-text search, link graph,
  tags — lives in a hidden `.nexus/index.sqlite`.

The index is **fully rebuildable**: deleting `.nexus/index.sqlite` and reopening
the vault (or calling `reindex`) re-walks the folder and rebuilds everything via
`vault::reconcile_full` (`src-tauri/src/vault.rs`). Nothing of value is lost if
the index is corrupted or deleted. This is the contract every backend module is
written to uphold (see the doc comment atop `src-tauri/src/index.rs`).

## Rust module map (`src-tauri/src/`)

| Module | Responsibility |
|--------|----------------|
| `main.rs` | Tauri builder; registers the 24 `invoke_handler` commands and the dialog plugin. |
| `commands.rs` | Every `#[tauri::command]`. Validates input, performs atomic writes through the `WriteGuard`, and updates the index synchronously. |
| `state.rs` | `AppState { vault: Mutex<Option<Vault>> }` — the single managed Tauri state. |
| `vault.rs` | `Vault` struct (root, name, SQLite conn, guard, live watcher) + `reconcile_full` / `reconcile_subtree`. |
| `index.rs` | SQLite schema (`SCHEMA` const) and all queries: upsert/remove notes, links, tags, properties, tree, backlinks, search, graph, databases. |
| `fs_watch.rs` | Debounced filesystem watcher → echo-check → reconcile → emit `fs:*` events. |
| `write_guard.rs` | `WriteGuard` echo-suppression registry (blake3 hash + time-window). |
| `events.rs` | Event names (`events::names`) and payload structs. |
| `model.rs` | Serializable DTOs returned to / received from the frontend (camelCase). |
| `paths.rs` | Vault-relative path helpers, `NEXUS_DIR`, `DB_MARKER`, safety checks. |
| `error.rs` | `AppError` / `AppResult`; serializes to a plain string for the UI. |

### The `nexus-vault` crate (`src-tauri/crates/nexus-vault/`)

A **pure, GUI-free** library crate with no dependency on Tauri or the
filesystem, so all the tricky parsing is unit-testable headlessly
(`cargo test -p nexus-vault`, 13 tests). It provides:

- `split_frontmatter` / `compose_note` — YAML frontmatter ⇄ JSON, byte-stable round-trip.
- `extract_wikilinks` — `[[Target#heading|alias]]` and `![[embed]]`, ignoring code.
- `extract_tags` — inline `#nested/tags` + frontmatter `tags:`.
- `extract_headings`, `to_plain_text`, `title_of`, `basename_no_ext`.
- `hash_hex` — blake3 content hash.
- `index_note` — parses a note into the `IndexedNote` the backend stores.

The backend (`index.rs`) has 8 more unit tests; `write_guard.rs` covers
echo-suppression.

## Data flow

Two directions, both narrow:

1. **UI → Rust (`invoke`)** — the typed bridge in `src/lib/ipc.ts` (`api.*`)
   calls Tauri commands. Each returns a `Promise` of a DTO from `types.ts`.
2. **Rust → UI (`emit`)** — the watcher and indexer emit events
   (`src/App.tsx` subscribes via `onEvent`): tree refreshes on `fs:created` /
   `fs:deleted`, the open editor reacts to `fs:changed`, and the status bar
   tracks `index:progress` / `index:ready` / `sync:status`.

A crucial asymmetry: **app-initiated writes emit no event**. The command already
returns the result and updates the index synchronously, so the UI refetches the
tree itself. Only **external** changes flow back as `fs:*` events. (See
[04-filesystem-sync.md](./04-filesystem-sync.md).)

```
                         ┌───────────────────────────────────────────┐
                         │            WebView (React + TS)            │
                         │  Zustand store · Tiptap editor · panels    │
                         └───────────────▲───────────────┬───────────┘
                            emit (events) │               │ invoke (commands)
                                          │               ▼
                         ┌────────────────┴───────────────────────────┐
                         │                Tauri core (Rust)            │
                         │  commands.rs  ·  AppState{ Vault }          │
                         │     │                  ▲                    │
                         │     │ atomic write      │ synchronous       │
                         │     │ + arm guard       │ index update      │
                         │     ▼                  │                    │
                         │  WriteGuard ──suppress─ fs_watch (300ms)    │
                         │     │                  ▲                    │
                         │     ▼                  │ external change    │
                         │  index.rs (rusqlite + FTS5)                 │
                         └────────────────┬────────────────────────────┘
                                          │ read / write
                                          ▼
                  ┌───────────────────────────────────────────────────┐
                  │  Vault folder on disk  (the source of truth)       │
                  │   *.md  ·  .nexusdb.json  ·  .nexus/index.sqlite   │
                  └───────────────────────────────────────────────────┘
```

## Concurrency model

- `AppState.vault` is a `Mutex<Option<Vault>>`, so a vault can be swapped.
- The SQLite `Connection` lives behind `Arc<Mutex<Connection>>`; commands clone
  the `Arc`s they need and release the `AppState` lock before doing IO.
- Heavy work (the initial full index on `open_vault`, and `reindex`) runs on a
  spawned `std::thread`; progress arrives via events so the UI never blocks.
- SQLite runs in WAL mode (`PRAGMA journal_mode = WAL`).

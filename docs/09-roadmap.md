# Roadmap

This roadmap separates what is **shipped** from what is **planned**. Phase 1 is
the current v0.1 MVP and is done; Phases 2–5 are not yet built. Sequencing and
grouping may change.

## Phase 1 — Local-first core (✅ Done, v0.1 MVP)

The foundation: your notes are plain files, and the app is a fast, faithful
window onto them.

- Open any folder as a vault; `NEXUS_VAULT` env startup.
- Real **two-way filesystem sync** with a live `notify` watcher (300 ms
  debounce), atomic writes, and `WriteGuard` echo suppression.
- Derived, fully-rebuildable **SQLite index** (WAL) with `reconcile_full` /
  `reconcile_subtree`.
- **Tiptap** block editor: `/` slash menu, decorated/clickable wikilinks, tasks,
  tables, code, Markdown in/out.
- File tree with context menu (new / rename / delete / move), favorites, recent.
- **Backlinks**, **outline**, and **properties (frontmatter)** panels.
- **Full-text search** (SQLite **FTS5**, `unicode61`, prefix + boolean + field
  filters, LIKE fallback) and a **cmdk** command palette (⌘K).
- Hierarchical **tags** panel.
- **Knowledge graph** (global, force-directed via `react-force-graph-2d`; a
  `graph_local` backend already exists).
- Notion-style **databases** (Table + Kanban views over frontmatter records).
- Dark / light themes, status bar (sync + index state).

## Phase 2 — Power editing & knowledge surfaces (Planned)

Round out the daily-driver editing experience and the knowledge views.

- **Infinite canvas** — free-form spatial board of notes/cards/links.
- More **database views**: calendar, timeline, gallery, list.
- New **property types**: formula, relation, rollup.
- Editor **conflict-diff UI** for simultaneous local + external edits.
- **Link auto-update on rename** (rewrite inbound `[[…]]` when a note is renamed).
- Local-graph view surfaced in the UI; richer graph filters.

## Phase 3 — Search & performance at scale (Planned)

Make search smarter and keep the app fast on very large vaults.

- **Tantivy** upgrade — replace/augment FTS5 with a Tantivy index for richer
  ranking, faster queries, and more flexible analysis.
- **Semantic / vector search** — embeddings-backed similarity and "related
  notes", layered over the existing index.
- **100k-note performance** work: a **virtualized file tree**, incremental
  indexing tuning, and graph rendering optimizations for large vaults.

## Phase 4 — AI & automation (Planned)

Bring local and hosted models to the knowledge base, opt-in and provider-agnostic.

- **AI integration** with pluggable providers — local (**Ollama**, **LM Studio**)
  and hosted (**OpenAI**, **Anthropic**) — for chat-over-vault, summarization,
  and writing assistance.
- **Local REST / WebSocket API** (see [06-api.md](./06-api.md)) so external tools
  and scripts can read/write notes and stream events.

## Phase 5 — Extensibility, sync & safety (Planned)

Open the platform up and harden data safety.

- **Sandboxed plugin system + marketplace** — see
  [07-plugin-architecture.md](./07-plugin-architecture.md).
- **Version history / git** integration and **automatic backups / snapshots**.
- **Multiple-vault switching** UI and saved **workspace layouts**.
- **Vault encryption** (at-rest).

## Explicitly not built yet

To keep documentation honest, the following are **planned only** and absent from
v0.1: AI integration, infinite canvas, plugin system + marketplace, version
history / git, automatic backups / snapshots, local REST / WebSocket API,
semantic search + Tantivy, additional DB views (calendar / timeline / gallery /
list), formula / relation / rollup properties, multiple-vault switching UI,
workspace layouts, virtualized file tree, editor conflict-diff UI, link
auto-update on rename, and vault encryption.

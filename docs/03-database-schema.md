# SQLite Index Schema

> Status: **Implemented (v0.1 MVP)**.

The index lives at `.nexus/index.sqlite` and is created/queried via `rusqlite`
(bundled SQLite, FTS5 enabled). It is **entirely derived** from the Markdown on
disk — see [01-architecture.md](./01-architecture.md). The schema is the
`SCHEMA` constant in `src-tauri/src/index.rs` and is applied by `init_schema`.

## DDL

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);

CREATE TABLE IF NOT EXISTS folders (
    path TEXT PRIMARY KEY,
    parent_path TEXT,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    parent_path TEXT,
    title TEXT NOT NULL,
    basename TEXT NOT NULL,
    mtime_ms INTEGER NOT NULL,
    size INTEGER NOT NULL,
    hash TEXT NOT NULL,
    is_db_record INTEGER NOT NULL DEFAULT 0,
    db_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_notes_basename ON notes(basename);
CREATE INDEX IF NOT EXISTS idx_notes_parent ON notes(parent_path);
CREATE INDEX IF NOT EXISTS idx_notes_db ON notes(db_path);

CREATE TABLE IF NOT EXISTS properties (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value_text TEXT,
    value_kind TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_props_note ON properties(note_id);
CREATE INDEX IF NOT EXISTS idx_props_key ON properties(key);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY,
    src_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    target_base TEXT NOT NULL,
    heading TEXT,
    block TEXT,
    alias TEXT,
    is_embed INTEGER NOT NULL DEFAULT 0,
    dest_id INTEGER,
    resolved INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_id);
CREATE INDEX IF NOT EXISTS idx_links_dest ON links(dest_id);
CREATE INDEX IF NOT EXISTS idx_links_base ON links(target_base);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    full TEXT NOT NULL UNIQUE,
    parent TEXT
);
CREATE TABLE IF NOT EXISTS note_tags (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);

CREATE TABLE IF NOT EXISTS databases (
    path TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    schema_json TEXT NOT NULL,
    views_json TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    path UNINDEXED, title, body, tags, tokenize = 'unicode61'
);
```

`init_schema` also seeds `meta('schema_version','1')`.

## Tables

| Table | Purpose |
|-------|---------|
| `meta` | Key/value store; currently holds `schema_version`. |
| `folders` | One row per directory (vault-relative `path`, `parent_path`, `name`). Drives the file tree. |
| `notes` | One row per `.md` file. `path` is the unique vault-relative key. Caches `title`, `basename`, `mtime_ms`, `size`, and the blake3 `hash`. `is_db_record`/`db_path` flag records that belong to a database folder. |
| `properties` | EAV store of frontmatter. One row per `(note_id, key)` with `value_text` + `value_kind` (`null`/`bool`/`number`/`string`/`list`/`object`). See [08-data-model.md](./08-data-model.md). |
| `links` | One row per wikilink occurrence. `target_base` is the lookup key (text before `#`); `heading`/`block`/`alias`/`is_embed` capture the rest. `dest_id` + `resolved` are filled by link resolution. |
| `tags` | Hierarchical tags. `full` (e.g. `project/active`) is unique; `parent` points one level up. Ancestors are auto-created. |
| `note_tags` | Many-to-many between `notes` and `tags`. |
| `databases` | One row per registered `.nexusdb.json`. Stores raw `schema_json` (columns) and `views_json`. |
| `notes_fts` | FTS5 virtual table over `title`, `body`, `tags` (path `UNINDEXED`). Powers full-text search. |

## How the index stays in sync

Every mutation routes through `index.rs`:

- **`upsert_note(conn, rel, mtime, size, content)`** — the single entry point for
  a note. It calls `nexus_vault::index_note`, upserts the `notes` row
  (`ON CONFLICT(path) DO UPDATE`), then **fully replaces** that note's
  `properties`, `links`, `note_tags`, and `notes_fts` rows, re-inserts hierarchical
  tags (`upsert_tag`), and re-resolves any links touching the affected basenames.
- **`remove_note`** / **`remove_path_recursive`** — delete the note(s) (and for a
  subtree, matching `folders` and `databases`), then re-resolve links so
  backlinks to a now-missing target become unresolved. `ON DELETE CASCADE`
  cleans up `properties`, `links`, and `note_tags`.
- **`upsert_folder`** — insert/replace a `folders` row.
- **`register_database`** — parse a `.nexusdb.json` and upsert the `databases` row.
- **`refresh_db_membership`** — recompute `is_db_record`/`db_path` for every note
  whose parent folder is a registered database.
- **`clear_index`** — wipe all derived tables before a full rebuild
  (`reconcile_full`).

### Link resolution

Links are stored unresolved and resolved by matching `links.target_base` against
`notes.basename`. On ambiguity (two notes share a basename), the shortest path
wins:

```sql
UPDATE links SET dest_id =
  (SELECT id FROM notes WHERE basename = ?1 ORDER BY length(path), path LIMIT 1)
WHERE target_base = ?1;
UPDATE links SET resolved = (dest_id IS NOT NULL) WHERE target_base = ?1;
```

### Full-text search (FTS5)

`notes_fts` uses the `unicode61` tokenizer. The body indexed is the **plain-text
flattening** of the Markdown (`nexus_vault::to_plain_text`), not the raw source,
so markup characters don't pollute results. Query handling
(`index::fts_query` / `index::search`):

- Plain words are turned into **prefix** queries (`token*`).
- Queries already containing operators (`"`, `*`, `:`, or `AND`/`OR`/`NOT`) are
  passed through to FTS5 verbatim (supports field filters and booleans).
- Snippets use `snippet(notes_fts, 2, '<mark>', '</mark>', '…', 12)`.
- On any FTS5 syntax error or empty token set, it **falls back** to a `LIKE`
  search over titles/basenames so the UI never errors.

> _Planned:_ a Tantivy + semantic/vector search upgrade — see
> [09-roadmap.md](./09-roadmap.md).

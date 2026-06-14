# Data Model

> Status: **Implemented (v0.1 MVP)**.

This document describes the types exchanged across the IPC boundary, the EAV
properties model, the links/tags model, and how on-disk Markdown maps to the
derived SQLite index.

## DTOs (TypeScript ⇄ Rust)

`src/lib/types.ts` is a hand-maintained mirror of `src-tauri/src/model.rs`. Rust
structs use `#[serde(rename_all = "camelCase")]`, so a Rust `mtime_ms` becomes
`mtimeMs` in TS. The set:

| TypeScript (`types.ts`) | Rust (`model.rs`) | Shape |
|--------------------------|-------------------|-------|
| `VaultInfo` | `VaultInfo` | `{ root, name, noteCount }` |
| `NoteMeta` | `NoteMeta` | `{ path, title, mtimeMs, hash, size }` |
| `TreeNode` | `TreeNode` | `{ path, name, isDir, children[] }` |
| `RawNote` | `RawNote` | `{ path, markdown, frontmatter, hash, mtimeMs }` |
| `SearchHit` | `SearchHit` | `{ path, title, snippet }` |
| `Suggestion` | `Suggestion` | `{ kind: "note"\|"tag", label, path\|null }` |
| `Backlink` | `Backlink` | `{ path, title, alias\|null }` |
| `Heading` | `Heading` | `{ level, text, line }` |
| `TagCount` | `TagCount` | `{ full, count }` |
| `GraphNode` | `GraphNode` | `{ id, label, val, group }` |
| `GraphEdge` | `GraphEdge` | `{ source, target }` |
| `GraphData` | `GraphData` | `{ nodes[], links[] }` |
| `DbColumn` | `DbColumn` | `{ key, type, options[] }` |
| `DbView` | `DbView` | `{ type, name, groupBy? }` |
| `DbSchema` | `DbSchema` | `{ path, name, columns[], views[] }` |
| `DbInfo` | `DbInfo` | `{ path, name, count }` |
| `DbRow` | `DbRow` | `{ path, title, fields }` |
| `DbResult` | `DbResult` | `{ schema, rows[] }` |
| `FsEvent` | `FsEvent` | `{ path, kind, meta\|null }` |
| `IndexProgress` | `IndexProgress` | `{ done, total, phase }` |
| `IndexReady` | `IndexReady` | `{ noteCount }` |
| `SyncStatus` | `SyncStatus` | `{ state }` |

Note that `RawNote.markdown` is the **body only** (frontmatter is split out into
`frontmatter`); the editor owns the body and the properties panel owns the
frontmatter. `DbColumn.type` and `DbView.type` use Rust field `kind` renamed to
`type` over the wire.

## On-disk ⇄ index mapping

The parsing that bridges the two lives in `nexus_vault::index_note`
(`crates/nexus-vault/src/lib.rs`); the storage lives in
`index::upsert_note` (`src-tauri/src/index.rs`).

| On disk (`*.md`) | Parsed by | Stored as |
|------------------|-----------|-----------|
| File path | — | `notes.path` (unique), `notes.parent_path`, `notes.basename` |
| Title (frontmatter `title` → first H1 → filename) | `title_of` | `notes.title` |
| File mtime / size | filesystem | `notes.mtime_ms`, `notes.size` |
| Full byte content | `hash_hex` (blake3) | `notes.hash` (change detection + echo suppression) |
| YAML frontmatter | `split_frontmatter` | `properties` (EAV) |
| `[[wikilinks]]` / `![[embeds]]` | `extract_wikilinks` | `links` |
| `#tags` + frontmatter `tags:` | `extract_tags` | `tags` + `note_tags` |
| Body, flattened to plain text | `to_plain_text` | `notes_fts(body)` |
| Headings | `extract_headings` | returned live by `get_outline` (not persisted) |

Writing goes the other way: `compose_note(frontmatter, body)` re-emits
deterministic, byte-stable Markdown (sorted YAML keys; empty frontmatter ⇒ no
fence), so round-tripping a note through the app doesn't churn the file.

## Properties: an EAV model

Frontmatter is stored **Entity-Attribute-Value**, one row per key, so arbitrary
schemas need no migrations:

```
properties(note_id, key, value_text, value_kind)
```

`value_kind` preserves the JSON type so values round-trip exactly
(`index::json_to_prop` / `prop_to_json`):

| JSON value | `value_kind` | `value_text` |
|------------|--------------|--------------|
| `null` | `null` | `NULL` |
| `true`/`false` | `bool` | `"true"`/`"false"` |
| number | `number` | stringified number |
| string | `string` | the string |
| array | `list` | JSON-serialized |
| object | `object` | JSON-serialized |

`query_database` reconstructs each record's `fields` object by reading its
`properties` rows back through `prop_to_json`.

## Links model

Each wikilink occurrence is one `links` row. A `WikiLink`
(`nexus_vault::WikiLink`) decomposes `[[Target#Heading|alias]]` /
`[[Target#^block]]` / `![[Embed]]` into:

- `target_base` — text before `#` (the resolution key),
- `heading` (after `#`) **or** `block` (after `#^`),
- `alias` (after `|`),
- `is_embed` (`![[…]]`).

Resolution matches `target_base` against `notes.basename`; on ambiguity the
shortest path wins. `dest_id` is the resolved note id and `resolved` flips to 1.
Backlinks (`get_backlinks`) are simply the `src` notes of resolved links whose
`dest_id` is the current note. Links inside code spans/fences are ignored
(`strip_code`).

## Tags model

Tags are **hierarchical**. Inserting `#project/active` auto-creates ancestor rows
(`project`, then `project/active`) via `upsert_tag`, each row recording its
`parent`. `note_tags` joins notes to tags many-to-many. `list_tags` returns each
tag's full path and note count; `notes_with_tag(tag, includeSubtags)` can include
descendants via a `LIKE 'tag/%'` match. Sources of tags: inline `#…` syntax and
the frontmatter `tags:` field (string or list); pure-numeric `#123` (heading
anchors) are ignored.

## Database model

A `.nexusdb.json` (`DbSchema`) registers `columns` (`DbColumn { key, type,
options }`) and `views` (`DbView { type, name, groupBy? }`). Implemented column
types: `text | number | date | select | multiselect | checkbox | url`.
Implemented view types: `table | kanban`. A record is a `.md` file in the
database folder whose **frontmatter holds the column values**; `notes.db_path` /
`is_db_record` mark membership (computed by `refresh_db_membership`).

> _Planned:_ `formula` / `relation` / `rollup` property types and
> `calendar` / `timeline` / `gallery` / `list` views — see
> [09-roadmap.md](./09-roadmap.md).

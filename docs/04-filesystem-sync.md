# Filesystem Sync

> Status: **Implemented (v0.1 MVP)**.

Nexus Notes keeps a live, **two-way** sync between the vault folder on disk and
its in-memory/SQLite view. You can edit a note in the app or in any external
editor (VS Code, vim, Finder) and both stay consistent — without the app
fighting an echo of its own writes.

Key files: `src-tauri/src/commands.rs` (app writes), `src-tauri/src/fs_watch.rs`
(watcher), `src-tauri/src/write_guard.rs` (echo suppression),
`src-tauri/src/vault.rs` (reconciliation).

## The echo problem

If the app writes `Note.md`, the OS file watcher fires for `Note.md` too. Naively
re-indexing and re-emitting on that echo would cause redundant work and could
clobber the editor. The `WriteGuard` exists to drop those echoes while still
catching genuine external edits.

## App-initiated write flow

Every mutating command (`write_note`, `create_note`, `create_folder`,
`rename_path`, `move_path`, `delete_path`, `upsert_record`) follows the same
recipe:

1. **Arm the guard** with the blake3 hash of the bytes about to be written
   (`WriteGuard::arm(path, Some(hash))`); deletes/folders/renames arm with
   `None`.
2. **Write atomically** — `atomic_write` writes to a sibling temp file
   (`*.nexustmp`, a non-`.md` name the watcher ignores) then `rename`s it into
   place, so a save is a single atomic event and never a partial read.
3. **Update the index synchronously** in the same command (`index::upsert_note`,
   `remove_path_recursive`, etc.).
4. **Emit nothing.** The command returns the result (e.g. a `NoteMeta`) to the
   caller, and the frontend refetches the tree if needed.

```
write_note(path, md, fm)
  └─ compose_note → bytes → hash
     ├─ guard.arm(abs, Some(hash))
     ├─ atomic_write(abs, bytes)        # temp file + rename
     ├─ index::upsert_note(...)         # synchronous
     └─ return NoteMeta                 # NO event emitted
            │
            ▼ (watcher later fires for abs)
     guard.should_suppress(abs, Some(hash)) == true  → dropped
```

## External-change flow

The watcher is a `notify` recommended watcher wrapped by
`notify-debouncer-full` with a **300 ms debounce**, watching the vault root
recursively. For each debounced event (`fs_watch::handle_event`):

1. Convert to a vault-relative path; **skip** anything under `.nexus/` and any
   hidden path (except the `.nexusdb.json` marker).
2. If the path **exists**:
   - **Directory** → `should_suppress(path, None)`; otherwise `upsert_folder` and
     emit `fs:created` (kind `folder`).
   - **`.nexusdb.json`** → re-`register_database` + `refresh_db_membership`
     (no `fs:*` event).
   - **`.md` file** → read it, compute its blake3 hash, call
     `should_suppress(path, Some(hash))`. If it's our echo, drop it. Otherwise
     `upsert_note` and emit `fs:created` (new path) or `fs:changed` (known path),
     carrying the fresh `NoteMeta`.
3. If the path **no longer exists** → `should_suppress(path, None)`; otherwise, if
   it was known to the index, `remove_path_recursive` and emit `fs:deleted`.

The frontend (`src/App.tsx`) refreshes the tree on `fs:created`/`fs:deleted`, and
forwards `fs:changed` to the open editor.

## WriteGuard echo-suppression algorithm

The guard is a `Mutex<HashMap<PathBuf, Entry>>` where `Entry { hash, expires }`.
`IGNORE_WINDOW = 2500 ms`. The strategy, from
`write_guard.rs::should_suppress` (expired entries are pruned first):

| Recorded hash | Observed hash | Decision |
|---------------|---------------|----------|
| `Some(h)` | `Some(o)`, `h == o` | **Suppress** (our echo) — and **consume** the entry. |
| `Some(_)` | `Some(_)`, differ | **Process** — bytes on disk differ, a genuine external edit. |
| `None` (delete/folder/rename) | any | **Suppress** within the time window, then consume. |
| (no entry for path) | any | **Process** — not one of our writes. |

Two properties make this robust:

- **Content hash is primary.** If the bytes on disk equal what we wrote, it's an
  echo regardless of timing — correct even when inotify coalesces or delays
  events. A differing hash is unambiguously an external edit, even within the
  2.5 s window.
- **Consume on match.** A hash match removes the entry, so if you later make a
  *genuine* external edit that happens to produce byte-identical content, it is
  still processed (the guard won't keep swallowing it).

The time window is only a **safety net** for operations that have no
content hash — deletes, directory creation, and renames.

## Editor reconciliation (conflict handling)

The open editor (`src/features/editor/Editor.tsx`) tracks the last-saved
`baseHash` and a `saved` flag. On an `fs:changed` event for the current note it
reloads **only when it has no unsaved edits** and the incoming hash differs from
`baseHash`:

```ts
if (saved && det.meta && det.meta.hash !== baseHash.current) {
  // safe to reload from disk
}
```

If you have unsaved local edits when the file changes underneath you, the app
keeps your in-progress edits (it does not silently overwrite the buffer).

> _Planned:_ a proper editor **conflict-diff UI** for the case where both sides
> changed, plus link **auto-update on rename**. See [09-roadmap.md](./09-roadmap.md).

## Reconciliation (rebuild & subtree)

- **`reconcile_full`** (`vault.rs`) — the full disk → index rebuild used on
  `open_vault` and `reindex`. Emits `sync:status = reconciling`, calls
  `clear_index`, walks the vault (skipping dot-directories), registers folders,
  databases, and notes, emits `index:progress` every ~50 notes, then
  `index:ready` and `sync:status = watching`. Runs on a background thread.
- **`reconcile_subtree`** — re-indexes a single file or directory subtree after
  an app-initiated `rename`/`move` (the moved tree is removed then re-walked, and
  `refresh_db_membership` re-runs).

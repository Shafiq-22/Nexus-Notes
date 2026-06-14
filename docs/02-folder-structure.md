# Folder Structure

Two distinct layouts matter: the **repository** (the app's own source) and the
**on-disk vault** (a user's data, which the app treats as the source of truth).

## Repository layout

```
Nexus-Notes/
├─ index.html                  # Vite entry
├─ package.json                # scripts + JS deps (pnpm)
├─ vite.config.ts              # fixed port 5173; ignores src-tauri
├─ tailwind.config.ts          # maps Tailwind colors → CSS variables
├─ tsconfig.json
├─ scripts/generate-icon.mjs   # icon generation helper
├─ examples/demo-vault/        # sample vault used in dev / screenshots
│
├─ src/                        # frontend (React + TypeScript)
│  ├─ main.tsx                 # React root
│  ├─ App.tsx                  # vault bootstrap + event subscriptions
│  ├─ store.ts                 # Zustand stores: useVault + useUi
│  ├─ lib/
│  │  ├─ types.ts              # TS mirror of Rust model.rs (camelCase)
│  │  └─ ipc.ts                # api.* bridge + in-browser mock vault
│  ├─ components/              # AppShell, TopBar, BottomBar,
│  │                          #   Left/RightSidebar, CommandPalette, icons
│  ├─ features/
│  │  ├─ editor/               # Editor.tsx, slash.ts, wikilink.ts
│  │  ├─ FileTree.tsx
│  │  ├─ GraphView.tsx
│  │  ├─ DatabaseView.tsx
│  │  └─ Welcome.tsx
│  └─ styles/index.css         # theme tokens + Tiptap/prose styles
│
└─ src-tauri/                  # backend (Rust + Tauri)
   ├─ Cargo.toml               # workspace root; members = [crates/nexus-vault]
   ├─ tauri.conf.json          # window, bundle, identifier com.nexusnotes.app
   ├─ build.rs
   ├─ capabilities/            # Tauri permission set (core:default, dialog:default)
   ├─ icons/
   ├─ src/
   │  ├─ main.rs               # Tauri builder + command registration
   │  ├─ commands.rs           # #[tauri::command] surface
   │  ├─ state.rs · vault.rs · index.rs
   │  ├─ fs_watch.rs · write_guard.rs
   │  ├─ events.rs · model.rs · paths.rs · error.rs
   └─ crates/nexus-vault/      # pure parsing lib (no Tauri/FS deps)
      └─ src/lib.rs
```

`node_modules/`, `dist/`, `src-tauri/target/`, and `src-tauri/gen/` are
build artifacts and git-ignored.

## On-disk vault layout

A vault is just **a plain folder**. There is no proprietary container — open any
directory and Nexus Notes indexes the Markdown inside it. Example
(`examples/demo-vault/`):

```
my-vault/
├─ Welcome.md                  # note: Markdown + optional YAML frontmatter
├─ Concepts/
│  ├─ Knowledge Graph.md
│  └─ Backlinks.md
├─ Projects/
│  ├─ Project Plan.md
│  └─ Roadmap.md
├─ Tasks/                      # a Notion-style DATABASE (has .nexusdb.json)
│  ├─ .nexusdb.json            # database marker: schema + views
│  ├─ Build backend.md         # each record is a .md file; props in frontmatter
│  ├─ Design UI.md
│  └─ Write docs.md
└─ .nexus/                     # hidden, derived, rebuildable
   ├─ index.sqlite
   ├─ index.sqlite-wal
   └─ index.sqlite-shm
```

### Conventions (defined in `src-tauri/src/paths.rs`)

- **`.nexus/`** (`NEXUS_DIR`) — hidden folder holding `index.sqlite`. Entirely
  derived from disk; safe to delete (it rebuilds on next open). Any path whose
  first segment is `.nexus` is skipped by the watcher and indexer.
- **`.nexusdb.json`** (`DB_MARKER`) — placing this file in a folder turns that
  folder into a database. It is the **one** dotfile the indexer reads (every
  other hidden path is ignored via `is_hidden`).
- **Hidden paths** — any path with a segment starting with `.` is excluded from
  indexing, except the DB marker which is detected explicitly.

### Note format

A note is Markdown with optional leading YAML frontmatter:

```markdown
---
title: Build backend
status: done
priority: 2
due: 2026-06-12
tags: [project/active]
---
Filesystem sync + index. Links to [[Roadmap]] and #project.
```

- **Wikilinks**: `[[Target]]`, `[[Target#Heading]]`, `[[Target#^block]]`,
  `[[Target|alias]]`. Embeds: `![[Target]]`.
- **Tags**: inline `#nested/tags` and/or a frontmatter `tags:` list.
- **Title** resolves as: frontmatter `title` → first H1 → filename.

### Database marker (`.nexusdb.json`)

```json
{
  "name": "Tasks",
  "columns": [
    { "key": "status", "type": "select", "options": ["todo", "doing", "done"] },
    { "key": "priority", "type": "number" },
    { "key": "due", "type": "date" }
  ],
  "views": [
    { "type": "table", "name": "All Tasks" },
    { "type": "kanban", "name": "Board", "groupBy": "status" }
  ]
}
```

Each record in the database is an ordinary `.md` file in that folder; its
**frontmatter holds the column values**. This means database records are also
normal, linkable, searchable notes — nothing is hidden in a binary store.

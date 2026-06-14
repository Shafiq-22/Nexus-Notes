# UI / UX

> Status: **Implemented (v0.1 MVP)**.

The frontend is a React 18 + TypeScript app styled with Tailwind. UI state lives
in two Zustand stores in `src/store.ts`: `useVault` (open vault, file tree,
active note/view, recent, favorites) and `useUi` (theme, panels, command
palette, status bar).

When no vault is open, `App.tsx` renders `Welcome`; otherwise `AppShell`.

## Three-pane layout (`src/components/AppShell.tsx`)

```
┌──────────────────────────────────────────────────────────────┐
│ TopBar                                                         │
├──────────────┬──────────────────────────────┬─────────────────┤
│ Left sidebar │            Center             │ Right sidebar   │
│  (260px)     │         minmax(0,1fr)         │   (300px)       │
│              │                               │                 │
│  Files / Tags│  Editor | Graph | Database    │  Backlinks /    │
│              │                               │  Outline /      │
│              │                               │  Properties     │
├──────────────┴──────────────────────────────┴─────────────────┤
│ BottomBar (status)                                             │
└──────────────────────────────────────────────────────────────┘
```

The shell is a CSS grid (`grid-rows-[auto_1fr_auto]`); columns collapse to `0px`
when a sidebar is toggled off. The right sidebar is shown **only** in the editor
view (`showRight = rightOpen && centerView === "editor"`).

### Left sidebar (`LeftSidebar.tsx`)
Two tabs (`useUi.leftTab`):
- **Files** — the virtual-free file tree (`features/FileTree.tsx`) with a context
  menu for **New note / New folder / Rename / Delete**. Also surfaces
  **Favorites** and **Recent** (persisted in `localStorage`).
- **Tags** — the hierarchical tags panel (from `list_tags`); selecting a tag
  lists notes via `notes_with_tag`.

### Center
Switches on `useVault.centerView`:
- **`editor`** — the Tiptap block editor for `activePath`.
- **`graph`** — the global knowledge graph.
- **`database`** — the active database's views.

### Right sidebar (`RightSidebar.tsx`)
Three tabs (`useUi.rightTab`), all scoped to the active note:
- **Backlinks** — inbound links (`get_backlinks`); click to navigate.
- **Outline** — headings (`get_outline`); click to jump (dispatches
  `nexus:goto-heading`).
- **Properties** — a frontmatter editor. Each row maps to a frontmatter key;
  booleans render as checkboxes, arrays as comma-joined text. Saving re-reads the
  note first so it never clobbers the editor's body, then calls `write_note`.

### Bottom bar (`BottomBar.tsx`)
Status strip: a colored dot + label for sync state (`watching` → green "Synced",
`reconciling` → amber "Indexing…", else "Idle"), the live index progress
(`Indexing done/total`) or note count, the vault root path, and the version
(`Nexus Notes 0.1.0`).

## Editor (`src/features/editor/`)

A Tiptap (ProseMirror) WYSIWYG block editor configured with `StarterKit`,
`tiptap-markdown` (Markdown in/out, no HTML), task lists, tables, link, and
placeholder, plus two custom extensions:

- **Slash menu (`slash.ts`)** — typing `/` opens a DOM-rendered command popup:
  Heading 1–3, Bullet / Numbered / To-do list, Quote, Code block, Divider,
  Table. Arrow keys + Enter select; Escape closes.
- **Wikilink decoration (`wikilink.ts`)** — `[[…]]` is decorated (the `.wikilink`
  CSS class) and clickable; clicking resolves the target via `search_suggest` and
  opens the best match.

Editing is **debounced ~600 ms**, then persisted via `write_note(path, markdown,
frontmatter)`. A header breadcrumb shows the path, a star toggles favorite, and a
`Saved` / `Saving…` indicator reflects state. The editor reloads on external
changes only when there are no unsaved edits (see
[04-filesystem-sync.md](./04-filesystem-sync.md)).

## Graph view (`features/GraphView.tsx`)

A force-directed knowledge graph rendered with `react-force-graph-2d`, fed by
`graph_global` (nodes = notes, edges = resolved links, node size scales with
degree, `group` = top-level folder). The backend also exposes `graph_local(path,
depth)` for a focused neighborhood.

## Database view (`features/DatabaseView.tsx`)

Renders a registered database (`get_database` + `query_database`) as **Table** or
**Kanban** views over the records' frontmatter. Editing a cell calls
`upsert_record`, which writes back into that record's `.md` frontmatter. Column
types: `text | number | date | select | multiselect | checkbox | url`.

## Command palette (`features/`/`components/CommandPalette.tsx`)

Built on **cmdk**, opened with **⌘K / Ctrl+K** (handled in `AppShell`). Provides
full-text search (`search`) and quick navigation/actions.

## Theming

Two themes — **dark** (default) and **light** — selected by setting
`data-theme="dark"` on `<html>` (`applyTheme` in `store.ts`); the choice persists
in `localStorage` (`nexus.theme`). All colors are **CSS variables** defined in
`src/styles/index.css`; `tailwind.config.ts` maps Tailwind color names to those
variables, so the same classes work in both themes (and future user themes).

### Theme tokens

| Variable | Dark | Light |
|----------|------|-------|
| `--bg-primary` | `#16161a` | `#ffffff` |
| `--bg-secondary` | `#1c1c21` | `#f7f7f8` |
| `--bg-tertiary` | `#24242b` | `#efeff1` |
| `--bg-hover` | `#2a2933` | `#eceaf6` |
| `--bg-active` | `#332f4d` | `#e2ddf7` |
| `--border` | `#2a2a31` | `#e6e6ea` |
| `--border-strong` | `#3a3a44` | `#d4d4da` |
| `--text-normal` | `#e6e6ea` | `#1f2024` |
| `--text-muted` | `#a0a0ad` | `#5c5f6b` |
| `--text-faint` | `#6b6b78` | `#9498a3` |
| `--text-accent` | `#a99bff` | `#6d5cff` |
| `--accent` | `#7d6bff` | `#6d5cff` |
| `--accent-hover` | `#9385ff` | `#5b49f0` |
| `--danger` / `--success` / `--warning` | `#ff6369` / `#4cc38a` / `#f0c000` | `#e5484d` / `#30a46c` / `#e2a336` |

Fonts: **Inter** (UI) and **JetBrains Mono** (code), with system fallbacks.

## Keyboard

| Shortcut | Action |
|----------|--------|
| **⌘K / Ctrl+K** | Open command palette (search + actions) |
| **`/`** (in editor) | Open slash command menu |
| **↑ / ↓ / Enter / Esc** | Navigate / select / dismiss the slash menu |

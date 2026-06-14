# Plugin Architecture (Design)

> Status: **PLANNED — not implemented.** Nothing in this document exists in the
> v0.1 codebase yet. It is a forward-looking design to be built in a later phase
> (see [09-roadmap.md](./09-roadmap.md)). It is recorded here so the v0.1 IPC and
> data-model decisions stay compatible with it.

## Goals

- Extend the editor, sidebar panels, commands, database views, and indexing —
  without forking the app.
- Keep the local-first guarantee: plugins run **sandboxed**, with explicit,
  user-granted **capabilities**. A plugin gets no ambient filesystem, network, or
  process access.
- Distribute and discover plugins through a **marketplace**, with everything
  still being plain files on disk.

## Manifest format (proposed)

Each plugin is a folder under `<vault>/.nexus/plugins/<id>/` (or a global plugins
dir) containing a `manifest.json` and a JS entry bundle:

```json
{
  "id": "com.example.word-count",
  "name": "Word Count",
  "version": "1.0.0",
  "description": "Shows live word/character counts in the status bar.",
  "author": "Example",
  "main": "dist/index.js",
  "minAppVersion": "0.4.0",
  "capabilities": ["notes:read", "ui:statusbar", "events:fs"],
  "contributes": {
    "commands": [{ "id": "wordcount.toggle", "title": "Toggle Word Count" }],
    "panels": [{ "id": "wordcount.panel", "location": "right", "title": "Stats" }]
  }
}
```

The host validates the manifest, shows the requested capabilities to the user,
and refuses to load anything requesting a capability the user has not granted.

## Sandboxed execution model (proposed)

Plugins are **untrusted code** and must not run with the app's privileges. The
intended model:

- Each plugin runs in an **isolated context** — a dedicated Web Worker (or a
  separate WebView/`iframe` with a strict CSP) — with **no direct access** to the
  DOM, `window`, the Tauri `invoke` bridge, or Node/Rust APIs.
- The only channel out is a **structured message port** to a host broker.
- The broker enforces capabilities: every request names a capability; ungranted
  requests are rejected. The broker is the sole component that may touch the real
  filesystem (always vault-relative, never outside the vault) or call backend
  commands.
- Resource guards: time/CPU budgets per call, payload-size limits, and the
  ability to kill a misbehaving worker.

```
┌─────────────┐  postMessage   ┌───────────────┐  capability-checked  ┌──────────┐
│  Plugin     │ ─────────────▶ │  Host broker  │ ───────────────────▶ │  Tauri   │
│  (Worker)   │ ◀───────────── │ (capabilities)│ ◀─────────────────── │  core    │
└─────────────┘    events      └───────────────┘     results/events   └──────────┘
   no FS / no invoke / no DOM
```

## Capability / permission API (proposed)

Capabilities are coarse, declarative, and user-approved. A first cut:

| Capability | Grants |
|------------|--------|
| `notes:read` | Read note bodies / frontmatter / metadata. |
| `notes:write` | Create/modify/delete notes (routed through the same `WriteGuard` + atomic write). |
| `index:query` | Run search, backlinks, tags, graph queries. |
| `events:fs` | Subscribe to `fs:*` / `index:*` / `sync:*` events. |
| `ui:command` | Register command-palette commands. |
| `ui:panel` | Contribute a sidebar/center panel. |
| `ui:statusbar` | Add a status-bar item. |
| `ui:editor` | Add editor decorations / slash-menu items. |
| `db:read` / `db:write` | Read/modify Notion-style databases. |
| `net:fetch` | Outbound HTTP to a manifest-declared allowlist of hosts only. |
| `storage:local` | Per-plugin key/value store (namespaced; not the vault). |

The host SDK surface a plugin imports would mirror these, e.g.
`nexus.notes.read(path)`, `nexus.commands.register(...)`,
`nexus.ui.addPanel(...)`, `nexus.events.on("fs:changed", cb)` — each call gated by
the corresponding capability.

## Lifecycle hooks (proposed)

```ts
export default {
  onload(ctx)   { /* register commands, panels, listeners */ },
  onunload(ctx) { /* tear down; release resources */ },
  onSettingsChange(ctx, settings) { /* react to user settings */ },
}
```

- **`onload`** — called once after capabilities are granted; the plugin wires up
  its contributions.
- **`onunload`** — on disable/uninstall/app-quit; the host force-revokes
  capabilities and terminates the worker.
- Optional event hooks (subject to capabilities): note open/save, vault open,
  external change, indexing complete.

## Marketplace concept (proposed)

- A signed registry index (plain JSON) listing plugins, versions, hashes, and
  declared capabilities.
- In-app browse / install / update / uninstall, with the capability prompt shown
  **before** first load.
- Installs land as ordinary folders under `.nexus/plugins/`; integrity verified
  via a content hash, so a vault remains portable and inspectable.
- Community plugins are version-pinned against `minAppVersion`.

## Open questions

- JS-only plugins vs. native (WASM) plugins for indexing/perf-critical work.
- Exact panel/editor extension contract and whether it reuses Tiptap extensions.
- Whether the [planned local REST/WebSocket API](./06-api.md) and plugins share
  one capability model (current intent: yes).

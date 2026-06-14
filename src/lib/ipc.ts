// Typed bridge to the Rust backend. When NOT running inside Tauri (e.g. `pnpm
// dev` in a plain browser) every call is served by an in-memory mock vault, so
// the full UI renders and is screenshot-verifiable without the backend.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { open as tauriOpen } from "@tauri-apps/plugin-dialog";
import type {
  Backlink,
  DbInfo,
  DbResult,
  DbSchema,
  GraphData,
  Heading,
  NoteMeta,
  RawNote,
  SearchHit,
  Suggestion,
  TagCount,
  TreeNode,
  VaultInfo,
} from "./types";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as object);

function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri) return tauriInvoke<T>(cmd, args);
  return Promise.resolve(mockCall(cmd, args) as T);
}

export const api = {
  pickFolder: async (): Promise<string | null> => {
    if (!isTauri) return "/Users/you/Nexus Vault";
    const r = await tauriOpen({ directory: true, multiple: false });
    return (Array.isArray(r) ? r[0] : r) ?? null;
  },
  openVault: (path: string) => call<VaultInfo>("open_vault", { path }),
  closeVault: () => call<void>("close_vault"),
  startupVault: () => call<string | null>("startup_vault"),
  reindex: () => call<void>("reindex"),
  getVaultTree: () => call<TreeNode>("get_vault_tree"),
  readNote: (path: string) => call<RawNote>("read_note", { path }),
  writeNote: (path: string, markdown: string, frontmatter: Record<string, unknown>) =>
    call<NoteMeta>("write_note", { path, markdown, frontmatter }),
  createNote: (parent: string, name: string) =>
    call<NoteMeta>("create_note", { parent, name }),
  createFolder: (parent: string, name: string) =>
    call<void>("create_folder", { parent, name }),
  renamePath: (from: string, to: string) => call<void>("rename_path", { from, to }),
  movePath: (from: string, toParent: string) =>
    call<string>("move_path", { from, toParent }),
  deletePath: (path: string) => call<void>("delete_path", { path }),
  search: (query: string, limit = 50) => call<SearchHit[]>("search", { query, limit }),
  searchSuggest: (prefix: string) => call<Suggestion[]>("search_suggest", { prefix }),
  getBacklinks: (path: string) => call<Backlink[]>("get_backlinks", { path }),
  getOutline: (path: string) => call<Heading[]>("get_outline", { path }),
  listTags: () => call<TagCount[]>("list_tags"),
  notesWithTag: (tag: string, includeSubtags = true) =>
    call<SearchHit[]>("notes_with_tag", { tag, includeSubtags }),
  graphGlobal: () => call<GraphData>("graph_global"),
  graphLocal: (path: string, depth = 1) => call<GraphData>("graph_local", { path, depth }),
  listDatabases: () => call<DbInfo[]>("list_databases"),
  getDatabase: (path: string) => call<DbSchema>("get_database", { path }),
  queryDatabase: (path: string) => call<DbResult>("query_database", { path }),
  upsertRecord: (dbPath: string, path: string | null, fields: Record<string, unknown>) =>
    call<NoteMeta>("upsert_record", { dbPath, path, fields }),
};

export type UnlistenFn = () => void;

export function onEvent<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (isTauri) return tauriListen<T>(name, (e) => cb(e.payload));
  return Promise.resolve(() => {});
}

// ---------------------------------------------------------------------------
// In-browser mock vault
// ---------------------------------------------------------------------------

interface MockNote {
  fm: Record<string, unknown>;
  body: string;
}

const mockNotes: Record<string, MockNote> = {
  "Welcome.md": {
    fm: { title: "Welcome to Nexus Notes", tags: ["intro"] },
    body: `# Welcome to Nexus Notes

A **local-first** knowledge base that blends the best of Obsidian and Notion.

> [!tip] Everything is yours
> Notes are plain Markdown files on your disk. No lock-in.

Start here:
- Explore the [[Knowledge Graph]]
- Open the [[Project Plan]]
- Try the #intro tag

- [x] Open a vault
- [ ] Create your first note
- [ ] Link two ideas together`,
  },
  "Projects/Project Plan.md": {
    fm: { title: "Project Plan", tags: ["project/active"], status: "in-progress" },
    body: `# Project Plan

Linked to the [[Roadmap]] and back to [[Welcome]].

## Goals
1. Ship the core editor
2. Wire up search
3. Render the graph`,
  },
  "Projects/Roadmap.md": {
    fm: { title: "Roadmap", tags: ["project"] },
    body: `# Roadmap

See the [[Project Plan]].

## Q3
- Databases
- Canvas

## Q4
- Plugins
- AI assist`,
  },
  "Concepts/Knowledge Graph.md": {
    fm: { title: "Knowledge Graph", tags: ["concept"] },
    body: `# Knowledge Graph

Connects every note. Related: [[Backlinks]] and [[Welcome]].`,
  },
  "Concepts/Backlinks.md": {
    fm: { title: "Backlinks", tags: ["concept"] },
    body: `# Backlinks

Inbound links to a note. See [[Knowledge Graph]].`,
  },
  "Daily/2026-06-14.md": {
    fm: { title: "2026-06-14", tags: ["daily"] },
    body: `# 2026-06-14

Today I set up [[Project Plan]] and reviewed the [[Roadmap]].`,
  },
  "Tasks/Design UI.md": {
    fm: { title: "Design UI", status: "doing", priority: 1, due: "2026-06-20" },
    body: "Polish the three-pane layout.",
  },
  "Tasks/Build backend.md": {
    fm: { title: "Build backend", status: "done", priority: 2, due: "2026-06-12" },
    body: "Filesystem sync + index.",
  },
  "Tasks/Write docs.md": {
    fm: { title: "Write docs", status: "todo", priority: 3, due: "2026-06-25" },
    body: "Architecture + API reference.",
  },
};

const mockDbSchema: DbSchema = {
  path: "Tasks",
  name: "Tasks",
  columns: [
    { key: "status", type: "select", options: ["todo", "doing", "done"] },
    { key: "priority", type: "number", options: [] },
    { key: "due", type: "date", options: [] },
  ],
  views: [
    { type: "table", name: "All" },
    { type: "kanban", name: "Board", groupBy: "status" },
  ],
};

const RE_LINK = /\[\[([^\[\]\n]+)\]\]/g;
const RE_TAG = /(?:^|[\s(>[])#([A-Za-z][A-Za-z0-9_/-]*)/g;

const basename = (p: string) => (p.split("/").pop() || p).replace(/\.md$/, "");
const titleOf = (p: string, n: MockNote) =>
  (n.fm.title as string) || basename(p);

function linksOf(body: string): string[] {
  return [...body.matchAll(RE_LINK)].map((m) => m[1].split(/[|#]/)[0].trim());
}
function tagsOf(n: MockNote): string[] {
  const set = new Set<string>();
  for (const m of n.body.matchAll(RE_TAG)) set.add(m[1]);
  const fmTags = n.fm.tags;
  if (Array.isArray(fmTags)) fmTags.forEach((t) => set.add(String(t)));
  return [...set];
}

function buildTree(): TreeNode {
  const root: TreeNode = { path: "", name: "Nexus Vault", isDir: true, children: [] };
  const dirNodes: Record<string, TreeNode> = { "": root };
  const ensureDir = (dir: string): TreeNode => {
    if (dirNodes[dir]) return dirNodes[dir];
    const parent = dir.includes("/") ? dir.slice(0, dir.lastIndexOf("/")) : "";
    const node: TreeNode = {
      path: dir,
      name: dir.split("/").pop() || dir,
      isDir: true,
      children: [],
    };
    ensureDir(parent).children.push(node);
    dirNodes[dir] = node;
    return node;
  };
  for (const path of Object.keys(mockNotes)) {
    const dir = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
    ensureDir(dir).children.push({
      path,
      name: titleOf(path, mockNotes[path]),
      isDir: false,
      children: [],
    });
  }
  const sort = (n: TreeNode) => {
    n.children.sort((a, b) =>
      a.isDir === b.isDir ? a.name.localeCompare(b.name) : a.isDir ? -1 : 1,
    );
    n.children.forEach(sort);
  };
  sort(root);
  return root;
}

function mockCall(cmd: string, args: Record<string, unknown>): unknown {
  const p = args.path as string;
  switch (cmd) {
    case "open_vault":
      return { root: "/Users/you/Nexus Vault", name: "Nexus Vault", noteCount: Object.keys(mockNotes).length };
    case "startup_vault":
      return null;
    case "close_vault":
    case "reindex":
    case "create_folder":
    case "rename_path":
    case "delete_path":
      return null;
    case "get_vault_tree":
      return buildTree();
    case "read_note": {
      const n = mockNotes[p];
      return { path: p, markdown: n?.body ?? "", frontmatter: n?.fm ?? {}, hash: "mock", mtimeMs: Date.now() };
    }
    case "write_note": {
      const note = mockNotes[p] ?? { fm: {}, body: "" };
      note.body = args.markdown as string;
      note.fm = args.frontmatter as Record<string, unknown>;
      mockNotes[p] = note;
      return { path: p, title: titleOf(p, note), mtimeMs: Date.now(), hash: "mock", size: note.body.length };
    }
    case "create_note": {
      const parent = (args.parent as string) || "";
      const name = (args.name as string).replace(/\.md$/, "");
      const path = parent ? `${parent}/${name}.md` : `${name}.md`;
      mockNotes[path] = { fm: { title: name }, body: `# ${name}\n\n` };
      return { path, title: name, mtimeMs: Date.now(), hash: "mock", size: 0 };
    }
    case "search": {
      const q = ((args.query as string) || "").toLowerCase();
      if (!q) return [];
      return Object.entries(mockNotes)
        .filter(([path, n]) => (titleOf(path, n) + n.body).toLowerCase().includes(q))
        .slice(0, (args.limit as number) || 50)
        .map(([path, n]) => ({ path, title: titleOf(path, n), snippet: n.body.slice(0, 80) }));
    }
    case "search_suggest": {
      const pre = ((args.prefix as string) || "").toLowerCase();
      const notes: Suggestion[] = Object.entries(mockNotes)
        .filter(([path, n]) => titleOf(path, n).toLowerCase().includes(pre))
        .slice(0, 8)
        .map(([path, n]) => ({ kind: "note", label: titleOf(path, n), path }));
      return notes;
    }
    case "get_backlinks": {
      const target = basename(p);
      return Object.entries(mockNotes)
        .filter(([, n]) => linksOf(n.body).includes(target))
        .map(([path, n]) => ({ path, title: titleOf(path, n), alias: null }));
    }
    case "get_outline": {
      const n = mockNotes[p];
      const out: Heading[] = [];
      n?.body.split("\n").forEach((line, i) => {
        const m = /^(#{1,6})\s+(.+)/.exec(line);
        if (m) out.push({ level: m[1].length, text: m[2], line: i });
      });
      return out;
    }
    case "list_tags": {
      const counts: Record<string, number> = {};
      for (const n of Object.values(mockNotes))
        for (const t of tagsOf(n)) counts[t] = (counts[t] || 0) + 1;
      return Object.entries(counts)
        .sort()
        .map(([full, count]) => ({ full, count }));
    }
    case "notes_with_tag": {
      const tag = args.tag as string;
      return Object.entries(mockNotes)
        .filter(([, n]) => tagsOf(n).some((t) => t === tag || t.startsWith(tag + "/")))
        .map(([path, n]) => ({ path, title: titleOf(path, n), snippet: "" }));
    }
    case "graph_global":
    case "graph_local": {
      const byBase: Record<string, string> = {};
      for (const path of Object.keys(mockNotes)) byBase[basename(path)] = path;
      const nodes = Object.entries(mockNotes).map(([path, n]) => ({
        id: path,
        label: titleOf(path, n),
        val: 1 + linksOf(n.body).length,
        group: path.includes("/") ? path.split("/")[0] : "root",
      }));
      const links: { source: string; target: string }[] = [];
      for (const [path, n] of Object.entries(mockNotes))
        for (const l of linksOf(n.body)) if (byBase[l]) links.push({ source: path, target: byBase[l] });
      if (cmd === "graph_local") {
        const keep = new Set<string>([p]);
        links.forEach((l) => {
          if (l.source === p) keep.add(l.target);
          if (l.target === p) keep.add(l.source);
        });
        return {
          nodes: nodes.filter((nn) => keep.has(nn.id)),
          links: links.filter((l) => keep.has(l.source) && keep.has(l.target)),
        };
      }
      return { nodes, links };
    }
    case "list_databases":
      return [{ path: "Tasks", name: "Tasks", count: 3 }];
    case "get_database":
      return mockDbSchema;
    case "query_database": {
      const rows = Object.entries(mockNotes)
        .filter(([path]) => path.startsWith("Tasks/"))
        .map(([path, n]) => ({
          path,
          title: titleOf(path, n),
          fields: { ...n.fm },
        }));
      return { schema: mockDbSchema, rows } as DbResult;
    }
    case "upsert_record": {
      const target = (args.path as string) || `Tasks/Untitled ${Date.now()}.md`;
      const note = mockNotes[target] ?? { fm: {}, body: "" };
      note.fm = { ...note.fm, ...(args.fields as Record<string, unknown>) };
      mockNotes[target] = note;
      return { path: target, title: titleOf(target, note), mtimeMs: Date.now(), hash: "mock", size: 0 };
    }
    default:
      return null;
  }
}

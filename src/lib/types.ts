// TypeScript mirror of the Rust `model.rs` structs (all camelCase).

export interface VaultInfo {
  root: string;
  name: string;
  noteCount: number;
}

export interface NoteMeta {
  path: string;
  title: string;
  mtimeMs: number;
  hash: string;
  size: number;
}

export interface TreeNode {
  path: string;
  name: string;
  isDir: boolean;
  children: TreeNode[];
}

export interface RawNote {
  path: string;
  markdown: string;
  frontmatter: Record<string, unknown>;
  hash: string;
  mtimeMs: number;
}

export interface SearchHit {
  path: string;
  title: string;
  snippet: string;
}

export interface Suggestion {
  kind: "note" | "tag";
  label: string;
  path: string | null;
}

export interface Backlink {
  path: string;
  title: string;
  alias: string | null;
}

export interface Heading {
  level: number;
  text: string;
  line: number;
}

export interface TagCount {
  full: string;
  count: number;
}

export interface GraphNode {
  id: string;
  label: string;
  val: number;
  group: string;
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface GraphData {
  nodes: GraphNode[];
  links: GraphEdge[];
}

export interface DbColumn {
  key: string;
  type: string; // text | number | date | select | multiselect | checkbox | url
  options: string[];
}

export interface DbView {
  type: string; // table | kanban
  name: string;
  groupBy?: string | null;
}

export interface DbSchema {
  path: string;
  name: string;
  columns: DbColumn[];
  views: DbView[];
}

export interface DbInfo {
  path: string;
  name: string;
  count: number;
}

export interface DbRow {
  path: string;
  title: string;
  fields: Record<string, unknown>;
}

export interface DbResult {
  schema: DbSchema;
  rows: DbRow[];
}

// --- event payloads (Rust → JS) ---

export interface FsEvent {
  path: string;
  kind: "note" | "folder";
  meta: NoteMeta | null;
}

export interface IndexProgress {
  done: number;
  total: number;
  phase: string;
}

export interface IndexReady {
  noteCount: number;
}

export interface SyncStatus {
  state: string; // idle | watching | reconciling
}

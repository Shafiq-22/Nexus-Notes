import { useState } from "react";
import type { TreeNode } from "../lib/types";
import { useVault } from "../store";
import { api } from "../lib/ipc";
import { Chevron, Doc, FolderPlus, Plus } from "../components/icons";
import clsx from "clsx";

type MenuItem = { label: string; fn: () => void; danger?: boolean };

function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  return (
    <>
      <div className="fixed inset-0 z-40" onClick={onClose} onContextMenu={(e) => { e.preventDefault(); onClose(); }} />
      <div
        className="fixed z-50 min-w-[160px] rounded-lg border border-border-strong bg-bg-secondary p-1 shadow-xl"
        style={{ left: x, top: y }}
      >
        {items.map((it) => (
          <button
            key={it.label}
            className={clsx(
              "block w-full rounded-md px-2.5 py-1.5 text-left text-sm hover:bg-bg-hover",
              it.danger ? "text-danger" : "text-text-normal",
            )}
            onClick={it.fn}
          >
            {it.label}
          </button>
        ))}
      </div>
    </>
  );
}

function parentOf(path: string) {
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
}

function Row({ node, depth }: { node: TreeNode; depth: number }) {
  const [open, setOpen] = useState(depth < 1);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const activePath = useVault((s) => s.activePath);
  const openNote = useVault((s) => s.openNote);
  const refreshTree = useVault((s) => s.refreshTree);
  const isActive = !node.isDir && activePath === node.path;

  const newNote = async (parent: string) => {
    setMenu(null);
    const name = window.prompt("New note name");
    if (!name) return;
    const meta = await api.createNote(parent, name);
    await refreshTree();
    openNote(meta.path);
  };
  const newFolder = async (parent: string) => {
    setMenu(null);
    const name = window.prompt("New folder name");
    if (!name) return;
    await api.createFolder(parent, name);
    await refreshTree();
  };
  const rename = async () => {
    setMenu(null);
    const name = window.prompt("Rename", node.name);
    if (!name) return;
    const parent = parentOf(node.path);
    const leaf = node.isDir ? name : `${name.replace(/\.md$/, "")}.md`;
    await api.renamePath(node.path, parent ? `${parent}/${leaf}` : leaf);
    await refreshTree();
  };
  const del = async () => {
    setMenu(null);
    if (!window.confirm(`Delete "${node.name}"?`)) return;
    await api.deletePath(node.path);
    await refreshTree();
  };

  const items: MenuItem[] = [
    ...(node.isDir
      ? [
          { label: "New note", fn: () => newNote(node.path) },
          { label: "New folder", fn: () => newFolder(node.path) },
        ]
      : []),
    { label: "Rename", fn: rename },
    { label: "Delete", fn: del, danger: true },
  ];

  return (
    <>
      <div
        className={clsx("side-row", isActive && "side-row-active")}
        style={{ paddingLeft: 6 + depth * 13 }}
        onClick={() => (node.isDir ? setOpen(!open) : openNote(node.path))}
        onContextMenu={(e) => {
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY });
        }}
      >
        {node.isDir ? (
          <span
            className="flex"
            style={{ transform: open ? "rotate(90deg)" : "none", transition: "transform .12s" }}
          >
            <Chevron size={13} />
          </span>
        ) : (
          <Doc size={14} />
        )}
        <span className="truncate">{node.name}</span>
      </div>
      {node.isDir && open && node.children.map((c) => <Row key={c.path} node={c} depth={depth + 1} />)}
      {menu && <ContextMenu x={menu.x} y={menu.y} items={items} onClose={() => setMenu(null)} />}
    </>
  );
}

export default function FileTree({ root }: { root: TreeNode }) {
  const refreshTree = useVault((s) => s.refreshTree);
  const openNote = useVault((s) => s.openNote);

  const newNote = async () => {
    const name = window.prompt("New note name");
    if (!name) return;
    const meta = await api.createNote("", name);
    await refreshTree();
    openNote(meta.path);
  };
  const newFolder = async () => {
    const name = window.prompt("New folder name");
    if (!name) return;
    await api.createFolder("", name);
    await refreshTree();
  };

  return (
    <div>
      <div className="flex items-center justify-between px-1 pb-1">
        <span className="text-[11px] font-semibold uppercase tracking-wide text-text-faint">Files</span>
        <div className="flex">
          <button className="icon-btn h-6 w-6" title="New note" onClick={newNote}>
            <Plus size={14} />
          </button>
          <button className="icon-btn h-6 w-6" title="New folder" onClick={newFolder}>
            <FolderPlus size={14} />
          </button>
        </div>
      </div>
      {root.children.map((child) => (
        <Row key={child.path} node={child} depth={0} />
      ))}
    </div>
  );
}

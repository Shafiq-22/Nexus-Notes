import { useEffect, useState } from "react";
import clsx from "clsx";
import { useUi, useVault } from "../store";
import FileTree from "../features/FileTree";
import { api } from "../lib/ipc";
import type { SearchHit, TagCount } from "../lib/types";
import { Doc, Star, Tag } from "./icons";

const leafName = (p: string) => (p.split("/").pop() || p).replace(/\.md$/, "");

function Section({
  title,
  items,
  icon,
}: {
  title: string;
  items: string[];
  icon: React.ReactNode;
}) {
  const openNote = useVault((s) => s.openNote);
  return (
    <div>
      <div className="px-1 pb-1 text-[11px] font-semibold uppercase tracking-wide text-text-faint">
        {title}
      </div>
      {items.map((p) => (
        <div key={p} className="side-row" onClick={() => openNote(p)} title={p}>
          {icon}
          <span className="truncate">{leafName(p)}</span>
        </div>
      ))}
    </div>
  );
}

function TagList() {
  const [tags, setTags] = useState<TagCount[]>([]);
  const [open, setOpen] = useState<string | null>(null);
  const [notes, setNotes] = useState<SearchHit[]>([]);
  const openNote = useVault((s) => s.openNote);

  useEffect(() => {
    void api.listTags().then(setTags);
  }, []);

  const toggle = async (tag: string) => {
    if (open === tag) {
      setOpen(null);
      return;
    }
    setOpen(tag);
    setNotes(await api.notesWithTag(tag, true));
  };

  if (tags.length === 0)
    return <p className="px-2 text-sm text-text-faint">No tags yet.</p>;

  return (
    <div>
      <div className="px-1 pb-1 text-[11px] font-semibold uppercase tracking-wide text-text-faint">
        Tags
      </div>
      {tags.map((t) => (
        <div key={t.full}>
          <div className="side-row" onClick={() => toggle(t.full)}>
            <Tag size={13} />
            <span className="truncate">{t.full}</span>
            <span className="ml-auto text-xs text-text-faint">{t.count}</span>
          </div>
          {open === t.full &&
            notes.map((n) => (
              <div
                key={n.path}
                className="side-row pl-7"
                onClick={() => openNote(n.path)}
              >
                <Doc size={13} />
                <span className="truncate">{n.title}</span>
              </div>
            ))}
        </div>
      ))}
    </div>
  );
}

export default function LeftSidebar() {
  const leftTab = useUi((s) => s.leftTab);
  const setLeftTab = useUi((s) => s.setLeftTab);
  const tree = useVault((s) => s.tree);
  const recent = useVault((s) => s.recent);
  const favorites = useVault((s) => s.favorites);

  return (
    <aside className="flex min-h-0 flex-col bg-bg-secondary">
      <div className="flex gap-1 border-b border-border p-2">
        <button
          className={clsx("tab flex-1 text-center", leftTab === "files" && "tab-active")}
          onClick={() => setLeftTab("files")}
        >
          Files
        </button>
        <button
          className={clsx("tab flex-1 text-center", leftTab === "tags" && "tab-active")}
          onClick={() => setLeftTab("tags")}
        >
          Tags
        </button>
      </div>
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-2 py-2">
        {leftTab === "files" ? (
          <>
            {tree && <FileTree root={tree} />}
            {favorites.length > 0 && (
              <Section title="Favorites" items={favorites} icon={<Star size={13} />} />
            )}
            {recent.length > 0 && (
              <Section title="Recent" items={recent} icon={<Doc size={13} />} />
            )}
          </>
        ) : (
          <TagList />
        )}
      </div>
    </aside>
  );
}

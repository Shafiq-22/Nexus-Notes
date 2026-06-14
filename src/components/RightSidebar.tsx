import { useEffect, useState } from "react";
import clsx from "clsx";
import { useUi, useVault } from "../store";
import { api } from "../lib/ipc";
import type { Backlink, Heading } from "../lib/types";
import { Doc } from "./icons";

function Backlinks({ path }: { path: string }) {
  const [links, setLinks] = useState<Backlink[]>([]);
  const openNote = useVault((s) => s.openNote);
  useEffect(() => {
    void api.getBacklinks(path).then(setLinks);
  }, [path]);
  if (links.length === 0)
    return <p className="px-2 py-3 text-sm text-text-faint">No backlinks yet.</p>;
  return (
    <div className="space-y-1 p-1">
      {links.map((l) => (
        <button
          key={l.path}
          className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-bg-hover"
          onClick={() => openNote(l.path)}
        >
          <Doc size={14} />
          <span className="truncate">{l.title}</span>
        </button>
      ))}
    </div>
  );
}

function Outline({ path }: { path: string }) {
  const [headings, setHeadings] = useState<Heading[]>([]);
  useEffect(() => {
    void api.getOutline(path).then(setHeadings);
  }, [path]);
  if (headings.length === 0)
    return <p className="px-2 py-3 text-sm text-text-faint">No headings.</p>;
  return (
    <div className="p-1">
      {headings.map((h, i) => (
        <button
          key={i}
          className="block w-full truncate rounded-md px-2 py-1 text-left text-sm text-text-muted hover:bg-bg-hover hover:text-text-normal"
          style={{ paddingLeft: 8 + (h.level - 1) * 12 }}
          onClick={() => window.dispatchEvent(new CustomEvent("nexus:goto-heading", { detail: h.text }))}
        >
          {h.text}
        </button>
      ))}
    </div>
  );
}

function PropRow({ name, value, onSave }: { name: string; value: unknown; onSave: (v: unknown) => void }) {
  const isBool = typeof value === "boolean";
  const isArray = Array.isArray(value);
  const [text, setText] = useState(isArray ? (value as unknown[]).join(", ") : String(value ?? ""));
  useEffect(() => {
    setText(isArray ? (value as unknown[]).join(", ") : String(value ?? ""));
  }, [value, isArray]);

  return (
    <div className="flex items-center gap-2 px-2 py-1">
      <span className="w-24 shrink-0 truncate text-xs text-text-muted">{name}</span>
      {isBool ? (
        <input type="checkbox" checked={value as boolean} onChange={(e) => onSave(e.target.checked)} />
      ) : (
        <input
          className="min-w-0 flex-1 rounded border border-border bg-bg-primary px-1.5 py-0.5 text-sm"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => onSave(isArray ? text.split(",").map((s) => s.trim()).filter(Boolean) : text)}
          onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        />
      )}
    </div>
  );
}

function Properties({ path }: { path: string }) {
  const [fm, setFm] = useState<Record<string, unknown>>({});
  useEffect(() => {
    void api.readNote(path).then((n) => setFm(n.frontmatter || {}));
  }, [path]);

  const save = async (next: Record<string, unknown>) => {
    const fresh = await api.readNote(path); // avoid clobbering the editor's body
    await api.writeNote(path, fresh.markdown, next);
    setFm(next);
  };
  const addProp = async () => {
    const k = window.prompt("Property name");
    if (k) await save({ ...fm, [k]: "" });
  };

  const entries = Object.entries(fm);
  return (
    <div className="p-1">
      {entries.length === 0 && (
        <p className="px-2 py-3 text-sm text-text-faint">No properties.</p>
      )}
      {entries.map(([k, v]) => (
        <PropRow key={k} name={k} value={v} onSave={(nv) => save({ ...fm, [k]: nv })} />
      ))}
      <button
        className="mt-1 w-full rounded-md px-2 py-1 text-left text-sm text-text-accent hover:bg-bg-hover"
        onClick={addProp}
      >
        + Add property
      </button>
    </div>
  );
}

export default function RightSidebar() {
  const rightTab = useUi((s) => s.rightTab);
  const setRightTab = useUi((s) => s.setRightTab);
  const activePath = useVault((s) => s.activePath);

  const tabs: { id: typeof rightTab; label: string }[] = [
    { id: "backlinks", label: "Backlinks" },
    { id: "outline", label: "Outline" },
    { id: "properties", label: "Properties" },
  ];

  return (
    <aside className="flex min-h-0 flex-col bg-bg-secondary">
      <div className="flex gap-1 border-b border-border p-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            className={clsx("tab flex-1 text-center", rightTab === t.id && "tab-active")}
            onClick={() => setRightTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {!activePath ? (
          <p className="px-2 py-3 text-sm text-text-faint">Open a note.</p>
        ) : rightTab === "backlinks" ? (
          <Backlinks path={activePath} />
        ) : rightTab === "outline" ? (
          <Outline path={activePath} />
        ) : (
          <Properties path={activePath} />
        )}
      </div>
    </aside>
  );
}

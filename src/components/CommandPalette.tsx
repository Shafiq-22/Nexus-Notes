import { useEffect, useState } from "react";
import { Command } from "cmdk";
import { useUi, useVault } from "../store";
import { api } from "../lib/ipc";
import type { SearchHit } from "../lib/types";
import { Database, Doc, GraphIcon, Moon, Plus } from "./icons";

interface Cmd {
  id: string;
  label: string;
  icon: React.ReactNode;
  run: () => void;
}

export default function CommandPalette() {
  const open = useUi((s) => s.paletteOpen);
  const setPalette = useUi((s) => s.setPalette);
  const toggleTheme = useUi((s) => s.toggleTheme);
  const showGraph = useVault((s) => s.showGraph);
  const openNote = useVault((s) => s.openNote);
  const openDatabase = useVault((s) => s.openDatabase);
  const refreshTree = useVault((s) => s.refreshTree);

  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setHits([]);
    }
  }, [open]);

  useEffect(() => {
    if (!query.trim()) {
      setHits([]);
      return;
    }
    let active = true;
    void api.search(query, 12).then((r) => active && setHits(r));
    return () => {
      active = false;
    };
  }, [query]);

  const close = () => setPalette(false);
  const commands: Cmd[] = [
    {
      id: "new-note",
      label: "New note…",
      icon: <Plus size={15} />,
      run: async () => {
        const name = window.prompt("New note name");
        if (name) {
          const meta = await api.createNote("", name);
          await refreshTree();
          openNote(meta.path);
        }
        close();
      },
    },
    { id: "graph", label: "Open knowledge graph", icon: <GraphIcon size={15} />, run: () => { showGraph(); close(); } },
    { id: "tasks", label: "Open Tasks database", icon: <Database size={15} />, run: () => { openDatabase("Tasks"); close(); } },
    { id: "theme", label: "Toggle dark / light theme", icon: <Moon size={15} />, run: () => { toggleTheme(); close(); } },
    { id: "reindex", label: "Reindex vault", icon: <GraphIcon size={15} />, run: () => { void api.reindex(); close(); } },
  ];
  const q = query.toLowerCase();
  const matchedCommands = commands.filter((c) => c.label.toLowerCase().includes(q));

  if (!open) return null;

  return (
    <Command.Dialog
      open={open}
      onOpenChange={setPalette}
      shouldFilter={false}
      label="Command palette"
      className="fixed inset-0 z-[100] flex items-start justify-center pt-[12vh]"
    >
      <div className="fixed inset-0 bg-black/40" onClick={close} />
      <div className="relative w-[min(640px,92vw)] overflow-hidden rounded-xl border border-border-strong bg-bg-secondary shadow-2xl">
        <Command.Input
          autoFocus
          value={query}
          onValueChange={setQuery}
          placeholder="Search notes or run a command…"
          className="w-full border-b border-border bg-transparent px-4 py-3 text-[15px] text-text-normal outline-none placeholder:text-text-faint"
        />
        <Command.List className="max-h-[50vh] overflow-y-auto p-2">
          <Command.Empty className="px-3 py-6 text-center text-sm text-text-faint">
            No results.
          </Command.Empty>

          {hits.length > 0 && (
            <Command.Group heading="Notes" className="px-1 text-[11px] font-semibold uppercase text-text-faint">
              {hits.map((h) => (
                <Command.Item
                  key={h.path}
                  value={`note:${h.path}`}
                  onSelect={() => { openNote(h.path); close(); }}
                  className="mt-1 flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm text-text-normal data-[selected=true]:bg-bg-active"
                >
                  <Doc size={15} />
                  <span className="truncate">{h.title}</span>
                  <span className="ml-auto truncate text-xs text-text-faint">{h.path}</span>
                </Command.Item>
              ))}
            </Command.Group>
          )}

          <Command.Group heading="Commands" className="mt-2 px-1 text-[11px] font-semibold uppercase text-text-faint">
            {matchedCommands.map((c) => (
              <Command.Item
                key={c.id}
                value={`cmd:${c.id}`}
                onSelect={c.run}
                className="mt-1 flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm text-text-normal data-[selected=true]:bg-bg-active"
              >
                {c.icon}
                <span>{c.label}</span>
              </Command.Item>
            ))}
          </Command.Group>
        </Command.List>
      </div>
    </Command.Dialog>
  );
}

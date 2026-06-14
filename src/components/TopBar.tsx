import { useUi, useVault } from "../store";
import { Logo } from "../features/Welcome";
import { Doc, GraphIcon, Moon, Search, Sidebar, Sun } from "./icons";
import clsx from "clsx";

export default function TopBar() {
  const vault = useVault((s) => s.vault);
  const centerView = useVault((s) => s.centerView);
  const setCenter = useVault((s) => s.setCenter);
  const showGraph = useVault((s) => s.showGraph);
  const setPalette = useUi((s) => s.setPalette);
  const theme = useUi((s) => s.theme);
  const toggleTheme = useUi((s) => s.toggleTheme);
  const leftOpen = useUi((s) => s.leftOpen);
  const setLeftOpen = useUi((s) => s.setLeftOpen);
  const rightOpen = useUi((s) => s.rightOpen);
  const setRightOpen = useUi((s) => s.setRightOpen);

  return (
    <header className="flex h-12 items-center gap-2 border-b border-border bg-bg-secondary px-3">
      <button className="icon-btn" title="Toggle file panel" onClick={() => setLeftOpen(!leftOpen)}>
        <Sidebar />
      </button>
      <div className="flex items-center gap-2 pr-1">
        <Logo size={18} />
        <span className="text-sm font-semibold">{vault?.name}</span>
        <span className="rounded bg-bg-tertiary px-1.5 py-0.5 text-[10px] font-medium text-text-faint">
          Workspace
        </span>
      </div>

      <button
        onClick={() => setPalette(true)}
        className="mx-auto flex w-[min(540px,46vw)] items-center gap-2 rounded-lg border border-border bg-bg-primary px-3 py-1.5 text-sm text-text-faint transition-colors hover:border-border-strong"
      >
        <Search size={15} />
        <span>Search notes, tags, commands…</span>
        <kbd className="ml-auto rounded bg-bg-tertiary px-1.5 py-0.5 text-[11px]">⌘K</kbd>
      </button>

      <div className="flex items-center gap-0.5 rounded-lg bg-bg-tertiary p-0.5">
        <button
          className={clsx("tab flex items-center gap-1", centerView === "editor" && "tab-active")}
          onClick={() => setCenter("editor")}
        >
          <Doc size={14} /> Editor
        </button>
        <button
          className={clsx("tab flex items-center gap-1", centerView === "graph" && "tab-active")}
          onClick={() => showGraph()}
        >
          <GraphIcon size={14} /> Graph
        </button>
      </div>

      <button className="icon-btn" title="Toggle theme" onClick={() => toggleTheme()}>
        {theme === "dark" ? <Sun /> : <Moon />}
      </button>
      <button
        className={clsx("icon-btn", rightOpen && "text-text-normal")}
        title="Toggle right panel"
        onClick={() => setRightOpen(!rightOpen)}
      >
        <Sidebar />
      </button>
    </header>
  );
}

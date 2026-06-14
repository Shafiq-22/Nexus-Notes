import { useEffect } from "react";
import { useUi, useVault } from "../store";
import TopBar from "./TopBar";
import BottomBar from "./BottomBar";
import LeftSidebar from "./LeftSidebar";
import RightSidebar from "./RightSidebar";
import CommandPalette from "./CommandPalette";
import Editor from "../features/editor/Editor";
import GraphView from "../features/GraphView";
import DatabaseView from "../features/DatabaseView";
import { Doc } from "./icons";

function EmptyCenter() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-text-faint">
      <Doc size={40} />
      <p className="text-sm">Select a note from the sidebar, or press ⌘K to search.</p>
    </div>
  );
}

export default function AppShell() {
  const centerView = useVault((s) => s.centerView);
  const activePath = useVault((s) => s.activePath);
  const leftOpen = useUi((s) => s.leftOpen);
  const rightOpen = useUi((s) => s.rightOpen);
  const setPalette = useUi((s) => s.setPalette);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPalette]);

  const showRight = rightOpen && centerView === "editor";

  return (
    <div className="grid h-full grid-rows-[auto_1fr_auto] bg-bg-primary text-text-normal">
      <TopBar />
      <div
        className="grid min-h-0"
        style={{
          gridTemplateColumns: `${leftOpen ? "260px" : "0px"} minmax(0,1fr) ${
            showRight ? "300px" : "0px"
          }`,
        }}
      >
        {leftOpen ? <LeftSidebar /> : <div />}
        <main className="min-w-0 overflow-hidden border-x border-border">
          {centerView === "editor" &&
            (activePath ? <Editor key={activePath} path={activePath} /> : <EmptyCenter />)}
          {centerView === "graph" && <GraphView />}
          {centerView === "database" && <DatabaseView />}
        </main>
        {showRight ? <RightSidebar /> : <div />}
      </div>
      <BottomBar />
      <CommandPalette />
    </div>
  );
}

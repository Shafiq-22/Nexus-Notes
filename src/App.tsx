import { useEffect } from "react";
import { api, isTauri, onEvent } from "./lib/ipc";
import { useUi, useVault } from "./store";
import AppShell from "./components/AppShell";
import Welcome from "./features/Welcome";
import type { FsEvent, IndexProgress, IndexReady, SyncStatus } from "./lib/types";

export default function App() {
  const vault = useVault((s) => s.vault);
  const openVault = useVault((s) => s.openVault);
  const refreshTree = useVault((s) => s.refreshTree);

  useEffect(() => {
    // Auto-open the sample vault in a plain browser (preview), or a vault
    // supplied via NEXUS_VAULT when running inside Tauri.
    if (!isTauri) void openVault();
    else
      void api.startupVault().then((p) => {
        if (p) void openVault(p);
      });

    const subs = [
      onEvent<FsEvent>("fs:created", () => void refreshTree()),
      onEvent<FsEvent>("fs:deleted", () => void refreshTree()),
      onEvent<FsEvent>("fs:changed", (e) =>
        window.dispatchEvent(new CustomEvent("nexus:external-change", { detail: e })),
      ),
      onEvent<IndexProgress>("index:progress", (p) => useUi.getState().setIndexProgress(p)),
      onEvent<IndexReady>("index:ready", (p) => {
        useUi.getState().setNoteCount(p.noteCount);
        useUi.getState().setIndexProgress(null);
        void refreshTree();
      }),
      onEvent<SyncStatus>("sync:status", (s) => useUi.getState().setSyncState(s.state)),
    ];
    return () => {
      subs.forEach((p) => p.then((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return vault ? <AppShell /> : <Welcome />;
}

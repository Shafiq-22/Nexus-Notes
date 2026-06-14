import { useUi, useVault } from "../store";

export default function BottomBar() {
  const syncState = useUi((s) => s.syncState);
  const indexProgress = useUi((s) => s.indexProgress);
  const noteCount = useUi((s) => s.noteCount);
  const vault = useVault((s) => s.vault);

  const syncColor =
    syncState === "watching" ? "var(--success)" : syncState === "reconciling" ? "var(--warning)" : "var(--text-faint)";

  return (
    <footer className="flex h-7 items-center gap-4 border-t border-border bg-bg-secondary px-3 text-[11px] text-text-muted">
      <span className="flex items-center gap-1.5">
        <span className="h-2 w-2 rounded-full" style={{ background: syncColor }} />
        {syncState === "watching" ? "Synced" : syncState === "reconciling" ? "Indexing…" : "Idle"}
      </span>
      {indexProgress ? (
        <span>
          Indexing {indexProgress.done}/{indexProgress.total}
        </span>
      ) : (
        <span>{noteCount || vault?.noteCount || 0} notes indexed</span>
      )}
      <span className="ml-auto truncate text-text-faint" title={vault?.root}>
        {vault?.root}
      </span>
      <span className="text-text-faint">Nexus Notes 0.1.0</span>
    </footer>
  );
}

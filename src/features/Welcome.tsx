import { useVault } from "../store";

export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
      <rect width="32" height="32" rx="8" fill="var(--accent)" />
      <g stroke="white" strokeWidth="1.6" opacity="0.9">
        <line x1="10" y1="13" x2="22" y2="11" />
        <line x1="10" y1="13" x2="17" y2="22" />
        <line x1="22" y1="11" x2="17" y2="22" />
      </g>
      <g fill="white">
        <circle cx="10" cy="13" r="3" />
        <circle cx="22" cy="11" r="2.3" />
        <circle cx="17" cy="22" r="2.6" />
      </g>
    </svg>
  );
}

export default function Welcome() {
  const openVault = useVault((s) => s.openVault);
  const loading = useVault((s) => s.loading);
  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 bg-bg-primary px-6">
      <div className="flex items-center gap-3">
        <Logo size={44} />
        <h1 className="text-3xl font-bold tracking-tight">Nexus Notes</h1>
      </div>
      <p className="max-w-md text-center text-text-muted">
        Local-first knowledge — your files, your machine. Open any folder as a vault; every
        note is a plain Markdown file on disk that you fully own.
      </p>
      <button
        onClick={() => void openVault()}
        className="rounded-lg bg-accent px-5 py-2.5 font-medium text-white transition-colors hover:bg-accent-hover"
      >
        {loading ? "Opening…" : "Open folder as vault"}
      </button>
    </div>
  );
}

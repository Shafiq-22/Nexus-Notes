import type { ReactNode } from "react";

function I({ children, size = 16 }: { children: ReactNode; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  );
}

export const Search = (p: { size?: number }) => (
  <I {...p}>
    <circle cx="11" cy="11" r="7" />
    <path d="m21 21-4.3-4.3" />
  </I>
);
export const GraphIcon = (p: { size?: number }) => (
  <I {...p}>
    <circle cx="6" cy="7" r="2.5" />
    <circle cx="18" cy="6" r="2" />
    <circle cx="13" cy="18" r="2.5" />
    <path d="M8 8l3.5 8M16.5 7.5 13.8 16M8.2 7l7.6-.6" />
  </I>
);
export const Doc = (p: { size?: number }) => (
  <I {...p}>
    <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
    <path d="M14 3v5h5M9 13h6M9 17h6" />
  </I>
);
export const Tag = (p: { size?: number }) => (
  <I {...p}>
    <path d="M3 11l8-8 9 9-8 8z" />
    <circle cx="7.5" cy="7.5" r="1.2" />
  </I>
);
export const Sun = (p: { size?: number }) => (
  <I {...p}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4 12H2M22 12h-2M5 5l1.5 1.5M17.5 17.5 19 19M19 5l-1.5 1.5M6.5 17.5 5 19" />
  </I>
);
export const Moon = (p: { size?: number }) => (
  <I {...p}>
    <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
  </I>
);
export const Plus = (p: { size?: number }) => (
  <I {...p}>
    <path d="M12 5v14M5 12h14" />
  </I>
);
export const FolderPlus = (p: { size?: number }) => (
  <I {...p}>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <path d="M12 11v4M10 13h4" />
  </I>
);
export const Chevron = (p: { size?: number }) => (
  <I {...p}>
    <path d="m9 6 6 6-6 6" />
  </I>
);
export const Star = ({ filled, size }: { filled?: boolean; size?: number }) => (
  <svg width={size ?? 16} height={size ?? 16} viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round">
    <path d="m12 3 2.9 5.9 6.1.9-4.5 4.3 1 6-5.5-2.9L6.5 20l1-6L3 9.8l6.1-.9z" />
  </svg>
);
export const Dots = (p: { size?: number }) => (
  <I {...p}>
    <circle cx="5" cy="12" r="1" />
    <circle cx="12" cy="12" r="1" />
    <circle cx="19" cy="12" r="1" />
  </I>
);
export const Sidebar = (p: { size?: number }) => (
  <I {...p}>
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="M9 4v16" />
  </I>
);
export const Database = (p: { size?: number }) => (
  <I {...p}>
    <ellipse cx="12" cy="5" rx="8" ry="3" />
    <path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
  </I>
);
export const Link = (p: { size?: number }) => (
  <I {...p}>
    <path d="M9 15l6-6M10.5 6.5 12 5a4 4 0 0 1 5.7 5.7L16 12M7.5 12 6 13.5A4 4 0 0 0 11.7 19l1.3-1.3" />
  </I>
);
export const ListIcon = (p: { size?: number }) => (
  <I {...p}>
    <path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" />
  </I>
);
export const Folder = (p: { size?: number }) => (
  <I {...p}>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  </I>
);

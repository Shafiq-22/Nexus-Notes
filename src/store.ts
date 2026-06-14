import { create } from "zustand";
import { api } from "./lib/ipc";
import type { IndexProgress, TreeNode, VaultInfo } from "./lib/types";

type CenterView = "editor" | "graph" | "database";

// ---------------------------------------------------------------------------
// Vault store: open vault, file tree, active note / view.
// ---------------------------------------------------------------------------

interface VaultState {
  vault: VaultInfo | null;
  tree: TreeNode | null;
  activePath: string | null;
  centerView: CenterView;
  activeDb: string | null;
  recent: string[];
  favorites: string[];
  loading: boolean;
  openVault: (path?: string) => Promise<void>;
  refreshTree: () => Promise<void>;
  openNote: (path: string) => void;
  openDatabase: (path: string) => void;
  showGraph: () => void;
  setCenter: (v: CenterView) => void;
  toggleFavorite: (path: string) => void;
}

const lsGet = (k: string): string[] => {
  try {
    return JSON.parse(localStorage.getItem(k) || "[]");
  } catch {
    return [];
  }
};
const lsSet = (k: string, v: string[]) => localStorage.setItem(k, JSON.stringify(v));

export const useVault = create<VaultState>((set, get) => ({
  vault: null,
  tree: null,
  activePath: null,
  centerView: "editor",
  activeDb: null,
  recent: lsGet("nexus.recent"),
  favorites: lsGet("nexus.favorites"),
  loading: false,

  openVault: async (path) => {
    set({ loading: true });
    try {
      const target = path ?? (await api.pickFolder());
      if (!target) return;
      const vault = await api.openVault(target);
      set({ vault });
      await get().refreshTree();
    } finally {
      set({ loading: false });
    }
  },

  refreshTree: async () => {
    if (!get().vault) return;
    set({ tree: await api.getVaultTree() });
  },

  openNote: (path) => {
    const recent = [path, ...get().recent.filter((p) => p !== path)].slice(0, 12);
    lsSet("nexus.recent", recent);
    set({ activePath: path, centerView: "editor", recent });
  },

  openDatabase: (path) => set({ activeDb: path, centerView: "database" }),
  showGraph: () => set({ centerView: "graph" }),
  setCenter: (centerView) => set({ centerView }),

  toggleFavorite: (path) => {
    const has = get().favorites.includes(path);
    const favorites = has
      ? get().favorites.filter((p) => p !== path)
      : [...get().favorites, path];
    lsSet("nexus.favorites", favorites);
    set({ favorites });
  },
}));

// ---------------------------------------------------------------------------
// UI store: theme, panels, command palette, status bar.
// ---------------------------------------------------------------------------

type LeftTab = "files" | "tags";
type RightTab = "backlinks" | "outline" | "properties";

interface UiState {
  theme: "dark" | "light";
  leftOpen: boolean;
  rightOpen: boolean;
  leftTab: LeftTab;
  rightTab: RightTab;
  paletteOpen: boolean;
  indexProgress: IndexProgress | null;
  syncState: string;
  noteCount: number;
  toggleTheme: () => void;
  setLeftTab: (t: LeftTab) => void;
  setRightTab: (t: RightTab) => void;
  setLeftOpen: (b: boolean) => void;
  setRightOpen: (b: boolean) => void;
  setPalette: (b: boolean) => void;
  setIndexProgress: (p: IndexProgress | null) => void;
  setSyncState: (s: string) => void;
  setNoteCount: (n: number) => void;
}

const initialTheme = (): "dark" | "light" =>
  (localStorage.getItem("nexus.theme") as "dark" | "light") || "dark";

export const applyTheme = (theme: string) =>
  document.documentElement.setAttribute("data-theme", theme);

export const useUi = create<UiState>((set, get) => ({
  theme: initialTheme(),
  leftOpen: true,
  rightOpen: true,
  leftTab: "files",
  rightTab: "backlinks",
  paletteOpen: false,
  indexProgress: null,
  syncState: "idle",
  noteCount: 0,
  toggleTheme: () => {
    const theme = get().theme === "dark" ? "light" : "dark";
    localStorage.setItem("nexus.theme", theme);
    applyTheme(theme);
    set({ theme });
  },
  setLeftTab: (leftTab) => set({ leftTab }),
  setRightTab: (rightTab) => set({ rightTab }),
  setLeftOpen: (leftOpen) => set({ leftOpen }),
  setRightOpen: (rightOpen) => set({ rightOpen }),
  setPalette: (paletteOpen) => set({ paletteOpen }),
  setIndexProgress: (indexProgress) => set({ indexProgress }),
  setSyncState: (syncState) => set({ syncState }),
  setNoteCount: (noteCount) => set({ noteCount }),
}));

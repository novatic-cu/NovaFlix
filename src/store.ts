import { create } from "zustand";
import type { DownloadProgress, IndexStats, Screen } from "./lib/types";

export interface Toast {
  id: number;
  message: string;
  tone: "info" | "success" | "error";
}

interface UiState {
  screen: Screen;
  browsePath: string;
  searchQuery: string;
  toasts: Toast[];
  live: Record<number, DownloadProgress>;
  stats: IndexStats | null;
  baseUrl: string;
  setScreen: (screen: Screen) => void;
  setBrowsePath: (path: string) => void;
  openFolder: (path: string) => void;
  setSearchQuery: (query: string) => void;
  toast: (message: string, tone?: Toast["tone"]) => void;
  dismiss: (id: number) => void;
  setLive: (progress: DownloadProgress) => void;
  clearLive: () => void;
  setStats: (stats: IndexStats | null) => void;
  setBaseUrl: (url: string) => void;
}

let toastSeed = 0;

export const useUi = create<UiState>((set) => ({
  screen: "search",
  browsePath: "/",
  searchQuery: "",
  toasts: [],
  live: {},
  stats: null,
  baseUrl: "https://visuales.uclv.cu",
  setScreen: (screen) => set({ screen }),
  setBrowsePath: (browsePath) => set({ browsePath }),
  openFolder: (path) => set({ browsePath: path, screen: "browse" }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  toast: (message, tone = "info") => {
    const id = ++toastSeed;
    set((state) => ({ toasts: [...state.toasts, { id, message, tone }] }));
    window.setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) }));
    }, 3800);
  },
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) })),
  setLive: (progress) =>
    set((state) => ({ live: { ...state.live, [progress.id]: progress } })),
  clearLive: () => set({ live: {} }),
  setStats: (stats) => set({ stats }),
  setBaseUrl: (baseUrl) => set({ baseUrl }),
}));


import { create } from "zustand";
import type { DaemonHealth, Project } from "../transport/protocol";

export function extractTokenFromHash(): string {
  if (typeof window === "undefined") {
    return "";
  }
  const hash = window.location.hash;
  if (!hash || !hash.startsWith("#")) {
    return "";
  }
  const params = new URLSearchParams(hash.slice(1));
  const raw = params.get("token")?.trim() ?? "";
  if (raw) {
    try {
      window.history.replaceState(null, "", window.location.pathname + window.location.search);
    } catch {
      // Abaikan bila history API terbatas
    }
  }
  return raw;
}

function readInitialToken(): string {
  if (typeof window !== "undefined" && window.__LATCH_TOKEN__) {
    return window.__LATCH_TOKEN__;
  }
  const fromHash = extractTokenFromHash();
  if (fromHash) {
    try {
      window.sessionStorage.setItem("latch_token", fromHash);
    } catch {
      // Abaikan bila sessionStorage tidak tersedia
    }
    return fromHash;
  }
  try {
    return window.sessionStorage.getItem("latch_token") ?? "";
  } catch {
    return "";
  }
}

interface HostsState {
  health: DaemonHealth | null;
  connected: boolean;
  projects: Project[];
  selectedProjectId: string;
  token: string;
  loading: boolean;
  error: string | null;
  setHealth: (health: DaemonHealth | null) => void;
  setConnected: (connected: boolean) => void;
  setProjects: (projects: Project[]) => void;
  addProjectLocal: (project: Project) => void;
  setSelectedProjectId: (id: string) => void;
  setToken: (token: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useHostsStore = create<HostsState>((set) => ({
  health: null,
  connected: false,
  projects: [],
  selectedProjectId: "all",
  token: readInitialToken(),
  loading: false,
  error: null,
  setHealth: (health) => set({ health }),
  setConnected: (connected) => set({ connected }),
  setProjects: (projects) => set({ projects }),
  addProjectLocal: (project) =>
    set((s) => ({
      projects: s.projects.some((p) => p.id === project.id)
        ? s.projects
        : [...s.projects, project],
    })),
  setSelectedProjectId: (selectedProjectId) => set({ selectedProjectId }),
  setToken: (token) => {
    const trimmed = token.trim();
    try {
      if (trimmed) {
        window.sessionStorage.setItem("latch_token", trimmed);
      } else {
        window.sessionStorage.removeItem("latch_token");
      }
    } catch {
      // Abaikan bila storage tidak tersedia
    }
    set({ token: trimmed, error: null });
  },
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));

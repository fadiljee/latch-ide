import { create } from "zustand";
import type { Session } from "../api/types";
import type { Locale } from "../i18n/catalog";

interface WorkbenchState {
  locale: Locale;
  sessions: Session[];
  connected: boolean;
  setLocale: (locale: Locale) => void;
  setSessions: (sessions: Session[]) => void;
  setConnected: (connected: boolean) => void;
}

export const useWorkbenchStore = create<WorkbenchState>((set) => ({
  locale: "id",
  sessions: [],
  connected: false,
  setLocale: (locale) => set({ locale }),
  setSessions: (sessions) => set({ sessions }),
  setConnected: (connected) => set({ connected }),
}));

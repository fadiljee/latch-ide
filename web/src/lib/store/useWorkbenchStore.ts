import { create } from "zustand";
import type { Session } from "../api/types";
import type { Locale } from "../i18n/catalog";

interface WorkbenchState {
  locale: Locale;
  sessions: Session[];
  setLocale: (locale: Locale) => void;
  setSessions: (sessions: Session[]) => void;
}

export const useWorkbenchStore = create<WorkbenchState>((set) => ({
  locale: "id",
  sessions: [],
  setLocale: (locale) => set({ locale }),
  setSessions: (sessions) => set({ sessions }),
}));

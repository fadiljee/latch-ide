import { create } from "zustand";
import { persist } from "zustand/middleware";

export type SessionTab = "terminal" | "chat" | "diff";
export type SplitMode = "single" | "vertical";

interface LayoutState {
  activeSessionId: string | null;
  secondarySessionId: string | null;
  splitMode: SplitMode;
  primaryTab: SessionTab;
  secondaryTab: SessionTab;
  setActiveSession: (id: string | null) => void;
  setSecondarySession: (id: string | null) => void;
  toggleSplit: () => void;
  setPrimaryTab: (tab: SessionTab) => void;
  setSecondaryTab: (tab: SessionTab) => void;
}

export const useLayoutStore = create<LayoutState>()(
  persist(
    (set, get) => ({
      activeSessionId: null,
      secondarySessionId: null,
      splitMode: "single",
      primaryTab: "terminal",
      secondaryTab: "diff",
      setActiveSession: (activeSessionId) => set({ activeSessionId }),
      setSecondarySession: (secondarySessionId) => set({ secondarySessionId }),
      toggleSplit: () =>
        set({
          splitMode: get().splitMode === "single" ? "vertical" : "single",
        }),
      setPrimaryTab: (primaryTab) => set({ primaryTab }),
      setSecondaryTab: (secondaryTab) => set({ secondaryTab }),
    }),
    {
      name: "latch_layout",
    },
  ),
);

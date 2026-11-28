import { create } from "zustand";
import { persist } from "zustand/middleware";

export type SessionTab = "terminal" | "chat" | "diff";
export type SplitMode = "single" | "vertical" | "grid" | "board";

interface LayoutState {
  activeSessionId: string | null;
  secondarySessionId: string | null;
  focusedGridSessionId: string | null;
  splitMode: SplitMode;
  primaryTab: SessionTab;
  secondaryTab: SessionTab;
  setActiveSession: (id: string | null) => void;
  setSecondarySession: (id: string | null) => void;
  setFocusedGridSession: (id: string | null) => void;
  setSplitMode: (mode: SplitMode) => void;
  toggleSplit: () => void;
  setPrimaryTab: (tab: SessionTab) => void;
  setSecondaryTab: (tab: SessionTab) => void;
}

export const useLayoutStore = create<LayoutState>()(
  persist(
    (set, get) => ({
      activeSessionId: null,
      secondarySessionId: null,
      focusedGridSessionId: null,
      splitMode: "single",
      primaryTab: "terminal",
      secondaryTab: "diff",
      setActiveSession: (activeSessionId) => set({ activeSessionId }),
      setSecondarySession: (secondarySessionId) => set({ secondarySessionId }),
      setFocusedGridSession: (focusedGridSessionId) => set({ focusedGridSessionId }),
      setSplitMode: (splitMode) => set({ splitMode }),
      toggleSplit: () => {
        const cur = get().splitMode;
        const next: SplitMode =
          cur === "single"
            ? "vertical"
            : cur === "vertical"
              ? "grid"
              : cur === "grid"
                ? "board"
                : "single";
        set({ splitMode: next });
      },
      setPrimaryTab: (primaryTab) => set({ primaryTab }),
      setSecondaryTab: (secondaryTab) => set({ secondaryTab }),
    }),
    {
      name: "latch_layout",
    },
  ),
);

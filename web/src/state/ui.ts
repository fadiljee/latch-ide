import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Locale } from "../lib/i18n/catalog";

export type AppRoute =
  | "workspace"
  | "automations"
  | "bots"
  | "plugins"
  | "hosts"
  | "usage"
  | "settings";
export type SidebarGrouping = "status" | "tree";
export type SidePanelTab = "git" | "checks" | "traffic";

interface UiState {
  locale: Locale;
  activeRoute: AppRoute;
  sidebarOpen: boolean;
  sidebarGrouping: SidebarGrouping;
  sidePanelOpen: boolean;
  sidePanelTab: SidePanelTab;
  searchQuery: string;
  commandPaletteOpen: boolean;
  newSessionOpen: boolean;
  setLocale: (locale: Locale) => void;
  setActiveRoute: (route: AppRoute) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setSidebarGrouping: (grouping: SidebarGrouping) => void;
  toggleSidePanel: () => void;
  setSidePanelOpen: (open: boolean) => void;
  setSidePanelTab: (tab: SidePanelTab) => void;
  setSearchQuery: (query: string) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setNewSessionOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      locale: "id",
      activeRoute: "workspace",
      sidebarOpen: true,
      sidebarGrouping: "status",
      sidePanelOpen: true,
      sidePanelTab: "git",
      searchQuery: "",
      commandPaletteOpen: false,
      newSessionOpen: false,
      setLocale: (locale) => set({ locale }),
      setActiveRoute: (activeRoute) => set({ activeRoute }),
      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
      setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
      setSidebarGrouping: (sidebarGrouping) => set({ sidebarGrouping }),
      toggleSidePanel: () => set((s) => ({ sidePanelOpen: !s.sidePanelOpen })),
      setSidePanelOpen: (sidePanelOpen) => set({ sidePanelOpen }),
      setSidePanelTab: (sidePanelTab) => set({ sidePanelTab, sidePanelOpen: true }),
      setSearchQuery: (searchQuery) => set({ searchQuery }),
      setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
      setNewSessionOpen: (newSessionOpen) => set({ newSessionOpen }),
    }),
    {
      name: "latch_ui",
      partialize: (state) => ({
        locale: state.locale,
        sidebarOpen: state.sidebarOpen,
        sidebarGrouping: state.sidebarGrouping,
        sidePanelOpen: state.sidePanelOpen,
        sidePanelTab: state.sidePanelTab,
      }),
    },
  ),
);

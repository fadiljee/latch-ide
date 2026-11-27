import { create } from "zustand";
import { persist } from "zustand/middleware";
import { DEFAULT_ACCENT, isValidHex } from "./accent";
import { applyTheme, type ThemeMode, type ThemeState, type ThemeStyle } from "./apply-theme";

interface ThemeStore extends ThemeState {
  resolvedMode: "dark" | "light";
  setStyle: (style: ThemeStyle) => void;
  setMode: (mode: ThemeMode) => void;
  setAccent: (accent: string) => void;
}

export const useThemeStore = create<ThemeStore>()(
  persist(
    (set, get) => ({
      style: "tonal",
      mode: "dark",
      accent: DEFAULT_ACCENT,
      resolvedMode: "dark",
      setStyle: (style) => {
        const next = { style, mode: get().mode, accent: get().accent };
        const resolvedMode = applyTheme(next, true);
        set({ style, resolvedMode });
      },
      setMode: (mode) => {
        const next = { style: get().style, mode, accent: get().accent };
        const resolvedMode = applyTheme(next, true);
        set({ mode, resolvedMode });
      },
      setAccent: (accent) => {
        if (!isValidHex(accent)) {
          return;
        }
        const next = { style: get().style, mode: get().mode, accent };
        const resolvedMode = applyTheme(next, true);
        set({ accent, resolvedMode });
      },
    }),
    {
      name: "latch_theme",
      partialize: (state) => ({
        style: state.style,
        mode: state.mode,
        accent: state.accent,
      }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          const resolved = applyTheme(
            { style: state.style, mode: state.mode, accent: state.accent },
            false,
          );
          state.resolvedMode = resolved;
        }
      },
    },
  ),
);

export function initThemeSubscription(): () => void {
  const current = useThemeStore.getState();
  const resolved = applyTheme(
    { style: current.style, mode: current.mode, accent: current.accent },
    false,
  );
  useThemeStore.setState({ resolvedMode: resolved });

  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    const st = useThemeStore.getState();
    if (st.mode === "system") {
      const nextResolved = applyTheme(
        { style: st.style, mode: st.mode, accent: st.accent },
        true,
      );
      useThemeStore.setState({ resolvedMode: nextResolved });
    }
  };
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}

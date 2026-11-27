import { computeAccentSoft, contrastOnAccent, DEFAULT_ACCENT, isValidHex } from "./accent";

export type ThemeStyle = "tonal" | "ledger" | "bento" | "glass";
export type ThemeMode = "dark" | "light" | "system";

export interface ThemeState {
  style: ThemeStyle;
  mode: ThemeMode;
  accent: string;
}

export function resolveMode(mode: ThemeMode): "dark" | "light" {
  if (mode !== "system") {
    return mode;
  }
  if (typeof window !== "undefined" && typeof window.matchMedia === "function") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "dark";
}

export function applyTheme(theme: ThemeState, animate = false): "dark" | "light" {
  const root = document.documentElement;
  const resolved = resolveMode(theme.mode);
  const safeAccent = isValidHex(theme.accent) ? theme.accent : DEFAULT_ACCENT;

  if (animate) {
    root.classList.add("theme-transition");
    window.setTimeout(() => {
      root.classList.remove("theme-transition");
    }, 260);
  }

  root.dataset.mode = resolved;
  root.dataset.style = theme.style;
  root.style.setProperty("--accent", safeAccent);
  root.style.setProperty("--accent-soft", computeAccentSoft(safeAccent));
  root.style.setProperty("--on-accent", contrastOnAccent(safeAccent));
  return resolved;
}

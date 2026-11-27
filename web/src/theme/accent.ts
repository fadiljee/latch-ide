export interface AccentPreset {
  id: string;
  label: string;
  hex: string;
}

export const ACCENT_PRESETS: readonly AccentPreset[] = [
  { id: "cobalt", label: "Cobalt", hex: "#3b82f6" },
  { id: "emerald", label: "Emerald", hex: "#10b981" },
  { id: "amber", label: "Amber", hex: "#d97706" },
  { id: "crimson", label: "Crimson", hex: "#e11d48" },
  { id: "slate", label: "Steel", hex: "#475569" },
];

export const DEFAULT_ACCENT = "#3b82f6";

export function isValidHex(hex: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(hex);
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = isValidHex(hex) ? hex.slice(1) : DEFAULT_ACCENT.slice(1);
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return [r, g, b];
}

function relativeLuminance(r: number, g: number, b: number): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

export function computeAccentSoft(hex: string): string {
  const safe = isValidHex(hex) ? hex : DEFAULT_ACCENT;
  const [r, g, b] = hexToRgb(safe);
  return `rgba(${r}, ${g}, ${b}, 0.16)`;
}

export function contrastOnAccent(hex: string): "#ffffff" | "#0f1114" {
  const [r, g, b] = hexToRgb(hex);
  const lum = relativeLuminance(r, g, b);
  return lum > 0.35 ? "#0f1114" : "#ffffff";
}

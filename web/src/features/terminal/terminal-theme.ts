import type { ITheme } from "@xterm/xterm";

export function readTerminalTheme(): ITheme {
  if (typeof window === "undefined") {
    return {};
  }
  const styles = window.getComputedStyle(document.documentElement);
  const read = (name: string) => styles.getPropertyValue(name).trim() || undefined;

  const background = read("--terminal-bg");
  const foreground = read("--terminal-fg");
  const accent = read("--accent");

  return {
    background,
    foreground,
    cursor: accent,
    selectionBackground: read("--accent-soft"),
    black: read("--bg-app"),
    red: read("--status-error"),
    green: read("--status-done"),
    yellow: read("--status-needs-you"),
    blue: read("--status-working"),
    cyan: accent,
    white: foreground,
  };
}

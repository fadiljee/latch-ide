export function isMacPlatform(): boolean {
  if (typeof navigator === "undefined") {
    return false;
  }
  return /mac/i.test(navigator.platform);
}

export function formatShortcut(macKey: string, defaultKey: string): string {
  return isMacPlatform() ? macKey : defaultKey;
}

export interface GlobalShortcutHandlers {
  onCommandPalette: () => void;
  onNewSession: () => void;
  onToggleSidebar: () => void;
  onToggleSidePanel: () => void;
  onEscape: () => void;
}

export function registerGlobalShortcuts(handlers: GlobalShortcutHandlers): () => void {
  const onKeyDown = (ev: KeyboardEvent) => {
    const mod = ev.metaKey || ev.ctrlKey;
    const key = ev.key.toLowerCase();

    if (ev.key === "Escape") {
      handlers.onEscape();
      return;
    }
    if (!mod) {
      return;
    }
    if (key === "k") {
      ev.preventDefault();
      handlers.onCommandPalette();
    } else if (key === "n" && !ev.shiftKey) {
      ev.preventDefault();
      handlers.onNewSession();
    } else if (key === "b" && !ev.shiftKey) {
      ev.preventDefault();
      handlers.onToggleSidebar();
    } else if (key === "j" && !ev.shiftKey) {
      ev.preventDefault();
      handlers.onToggleSidePanel();
    }
  };

  window.addEventListener("keydown", onKeyDown);
  return () => window.removeEventListener("keydown", onKeyDown);
}

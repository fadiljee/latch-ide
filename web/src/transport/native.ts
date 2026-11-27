export type WindowAction = "minimize" | "toggle_maximize" | "close" | "start_dragging";

export function isTauri(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  return Boolean(window.__LATCH_TAURI__ || window.__TAURI_INTERNALS__?.invoke);
}

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri()) {
    return null;
  }
  const fn = window.__TAURI_INTERNALS__?.invoke;
  if (!fn) {
    return null;
  }
  try {
    return (await fn(cmd, args)) as T;
  } catch {
    return null;
  }
}

export async function windowAction(action: WindowAction): Promise<void> {
  await invokeTauri("latch_window_action", { action });
}

export async function toggleFloatingWindow(): Promise<void> {
  await invokeTauri("latch_toggle_floating");
}

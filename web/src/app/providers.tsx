import { useEffect, type ReactNode } from "react";
import { registerGlobalShortcuts } from "../lib/keys";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore } from "../state/layout";
import { useUiStore } from "../state/ui";
import { initThemeSubscription } from "../theme/theme-store";
import { connectEventsSocket } from "../transport/daemon-client";
import { syncAllFromDaemon } from "../transport/events";

export function AppProviders({ children }: { children: ReactNode }) {
  const token = useHostsStore((s) => s.token);
  const setConnected = useHostsStore((s) => s.setConnected);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const toggleSidePanel = useUiStore((s) => s.toggleSidePanel);
  const setCommandPaletteOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);
  const commandPaletteOpen = useUiStore((s) => s.commandPaletteOpen);
  const newSessionOpen = useUiStore((s) => s.newSessionOpen);

  useEffect(() => initThemeSubscription(), []);

  useEffect(() => {
    return registerGlobalShortcuts({
      onCommandPalette: () => setCommandPaletteOpen(!commandPaletteOpen),
      onNewSession: () => setNewSessionOpen(true),
      onToggleSidebar: toggleSidebar,
      onToggleSidePanel: toggleSidePanel,
      onEscape: () => {
        if (commandPaletteOpen) {
          setCommandPaletteOpen(false);
        }
        if (newSessionOpen) {
          setNewSessionOpen(false);
        }
      },
    });
  }, [
    commandPaletteOpen,
    newSessionOpen,
    setCommandPaletteOpen,
    setNewSessionOpen,
    toggleSidePanel,
    toggleSidebar,
  ]);

  useEffect(() => {
    if (!token) {
      setConnected(false);
      return;
    }
    let active = true;
    const sync = () => {
      if (active) {
        void syncAllFromDaemon(token);
      }
    };
    sync();

    const disconnect = connectEventsSocket(
      token,
      (ev) => {
        if (ev.type === "session.created" && ev.session_id) {
          useLayoutStore.getState().setActiveSession(ev.session_id);
        }
        sync();
      },
      (isConnected) => {
        if (active) {
          setConnected(isConnected);
        }
      },
      sync,
    );

    return () => {
      active = false;
      disconnect();
    };
  }, [token, setConnected]);

  return <>{children}</>;
}

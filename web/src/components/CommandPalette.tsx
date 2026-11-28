import { useEffect, useMemo, useRef, useState } from "react";
import { CATALOGS } from "../lib/i18n/catalog";
import { useLayoutStore } from "../state/layout";
import { useSessionsStore } from "../state/sessions";
import { useUiStore } from "../state/ui";
import { toggleFloatingWindow } from "../transport/native";
import { Dialog } from "./ui/Dialog";
import { Input } from "./ui/Input";
import { Kbd } from "./Kbd";
import { StatusDot } from "./StatusDot";
import { mapSessionUiStatus } from "../state/sessions";

interface PaletteAction {
  id: string;
  label: string;
  shortcut?: string;
  run: () => void;
}

export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const toggleSidePanel = useUiStore((s) => s.toggleSidePanel);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const locale = useUiStore((s) => s.locale);
  const toggleSplit = useLayoutStore((s) => s.toggleSplit);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const byId = useSessionsStore((s) => s.byId);
  const order = useSessionsStore((s) => s.order);

  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement | null>(null);
  const t = CATALOGS[locale];

  useEffect(() => {
    if (open) {
      setQuery("");
      inputRef.current?.focus();
    }
  }, [open]);

  const actions = useMemo<PaletteAction[]>(() => {
    const base: PaletteAction[] = [
      {
        id: "new-session",
        label: t.palette.openNewSession,
        shortcut: "Ctrl+N",
        run: () => setNewSessionOpen(true),
      },
      {
        id: "toggle-sidebar",
        label: t.palette.toggleSidebar,
        shortcut: "Ctrl+B",
        run: () => toggleSidebar(),
      },
      {
        id: "toggle-sidepanel",
        label: t.palette.toggleSidePanel,
        shortcut: "Ctrl+J",
        run: () => toggleSidePanel(),
      },
      {
        id: "toggle-split",
        label: t.palette.toggleSplit,
        run: () => toggleSplit(),
      },
      {
        id: "open-agents",
        label: t.palette.openAgents,
        run: () => setActiveRoute("bots"),
      },
      {
        id: "open-plugins",
        label: t.palette.openPlugins,
        run: () => setActiveRoute("plugins"),
      },
      {
        id: "open-settings",
        label: t.palette.openSettings,
        run: () => setActiveRoute("settings"),
      },
      {
        id: "toggle-floating",
        label: t.palette.openFloating,
        run: () => {
          void toggleFloatingWindow();
        },
      },
    ];

    for (const id of order) {
      const ses = byId[id];
      if (!ses) {
        continue;
      }
      base.push({
        id: `ses-${ses.id}`,
        label: `${ses.task ?? ses.branch_name} (${ses.agent_profile})`,
        run: () => {
          setActiveSession(ses.id);
          setActiveRoute("workspace");
        },
      });
    }
    return base;
  }, [
    byId,
    order,
    setActiveRoute,
    setActiveSession,
    setNewSessionOpen,
    t.palette,
    toggleSidePanel,
    toggleSidebar,
    toggleSplit,
  ]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      return actions;
    }
    return actions.filter((a) => a.label.toLowerCase().includes(q));
  }, [actions, query]);

  return (
    <Dialog
      open={open}
      title="Command Palette"
      titleId="command-palette-title"
      onClose={() => setOpen(false)}
    >
      <Input
        ref={inputRef}
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t.palette.placeholder}
        aria-label={t.palette.placeholder}
      />

      <ul className="mt-3 max-h-72 space-y-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <li className="px-2 py-4 text-center text-xs text-fg-3">{t.palette.empty}</li>
        ) : (
          filtered.map((item) => {
            const ses = item.id.startsWith("ses-") ? byId[item.id.slice(4)] : undefined;
            return (
              <li key={item.id}>
                <button
                  type="button"
                  onClick={() => {
                    item.run();
                    setOpen(false);
                  }}
                  className="flex w-full items-center justify-between gap-2 rounded-md px-2.5 py-2 text-left text-xs text-fg-2 hover:bg-hover hover:text-fg"
                >
                  <span className="flex min-w-0 items-center gap-2">
                    {ses ? <StatusDot status={mapSessionUiStatus(ses.status)} /> : null}
                    <span className="truncate">{item.label}</span>
                  </span>
                  {item.shortcut ? <Kbd keys={item.shortcut} /> : null}
                </button>
              </li>
            );
          })
        )}
      </ul>
    </Dialog>
  );
}

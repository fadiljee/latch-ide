import { StatusDot } from "../components/StatusDot";
import { cn } from "../lib/cn";
import { CATALOGS } from "../lib/i18n/catalog";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore, type SplitMode } from "../state/layout";
import { useSessionsStore } from "../state/sessions";
import { useUiStore } from "../state/ui";
import { useThemeStore } from "../theme/theme-store";

export function StatusBar() {
  const connected = useHostsStore((s) => s.connected);
  const health = useHostsStore((s) => s.health);
  const activeSessionId = useLayoutStore((s) => s.activeSessionId);
  const splitMode = useLayoutStore((s) => s.splitMode);
  const setSplitMode = useLayoutStore((s) => s.setSplitMode);
  const activeSession = useSessionsStore((s) =>
    activeSessionId ? s.byId[activeSessionId] : undefined,
  );
  const locale = useUiStore((s) => s.locale);
  const toggleSidePanel = useUiStore((s) => s.toggleSidePanel);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const style = useThemeStore((s) => s.style);
  const resolvedMode = useThemeStore((s) => s.resolvedMode);

  const t = CATALOGS[locale];

  return (
    <footer className="flex h-[var(--statusbar-h)] shrink-0 items-center justify-between gap-3 border-t border-line bg-sidebar px-3 text-[11px] text-fg-3">
      <div className="flex items-center gap-3">
        <span className="inline-flex items-center gap-1.5">
          <StatusDot status={connected ? "done" : "needs_you"} />
          <span>{connected ? t.connection.connected : t.connection.reconnecting}</span>
        </span>
        {health ? <span className="hidden font-mono sm:inline">{health.sandbox}</span> : null}
        {activeSession ? (
          <span className="hidden truncate font-mono text-fg-2 md:inline">
            {activeSession.branch_name}
          </span>
        ) : null}
      </div>

      <div className="flex items-center gap-2">
        <div
          role="group"
          aria-label="Workspace layout mode"
          className="inline-flex items-center rounded-sm border border-line bg-app p-0.5"
        >
          {(
            [
              { id: "single", label: t.workspace.singlePane },
              { id: "vertical", label: t.workspace.splitToggle },
              { id: "grid", label: t.workspace.gridMode },
            ] as readonly { id: SplitMode; label: string }[]
          ).map((item) => (
            <button
              key={item.id}
              type="button"
              aria-pressed={splitMode === item.id}
              onClick={() => setSplitMode(item.id)}
              className={cn(
                "rounded-sm px-1.5 py-0.5 transition-colors",
                splitMode === item.id
                  ? "bg-accent-soft font-medium text-accent"
                  : "text-fg-3 hover:bg-hover hover:text-fg",
              )}
            >
              {item.label}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={toggleSidePanel}
          className="hidden rounded-sm px-2 py-0.5 hover:bg-hover hover:text-fg lg:inline-block"
        >
          {t.workspace.sidePanelToggle}
        </button>
        <button
          type="button"
          onClick={() => setActiveRoute("settings")}
          className="rounded-sm px-2 py-0.5 font-mono hover:bg-hover hover:text-fg"
        >
          {style} · {resolvedMode}
        </button>
      </div>
    </footer>
  );
}

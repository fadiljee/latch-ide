import { Button } from "../components/ui/Button";
import { IconButton } from "../components/ui/IconButton";
import { Kbd } from "../components/Kbd";
import { UsagePill } from "../components/UsagePill";
import { launchAntigravityCliQuick } from "../features/sessions/quick-launch";
import { CATALOGS } from "../lib/i18n/catalog";
import { formatShortcut } from "../lib/keys";
import { cn } from "../lib/cn";
import { useHostsStore } from "../state/hosts";
import { useUiStore, type AppRoute } from "../state/ui";
import { isTauri, windowAction } from "../transport/native";

const ROUTES: readonly AppRoute[] = [
  "workspace",
  "automations",
  "bots",
  "plugins",
  "hosts",
  "usage",
  "settings",
];

export function TitleBar() {
  const locale = useUiStore((s) => s.locale);
  const activeRoute = useUiStore((s) => s.activeRoute);
  const setLocale = useUiStore((s) => s.setLocale);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const setCommandPaletteOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);
  const projects = useHostsStore((s) => s.projects);
  const selectedProjectId = useHostsStore((s) => s.selectedProjectId);
  const setSelectedProjectId = useHostsStore((s) => s.setSelectedProjectId);

  const t = CATALOGS[locale];
  const tauriMode = isTauri();

  return (
    <header
      data-tauri-drag-region=""
      className="latch-glass-chrome flex min-h-[var(--titlebar-h)] flex-wrap items-center justify-between gap-2 border-b border-line bg-surface px-3 py-1"
    >
      <div className="flex items-center gap-2">
        <IconButton
          label={t.palette.toggleSidebar}
          active={sidebarOpen}
          onClick={toggleSidebar}
        >
          <svg
            aria-hidden="true"
            viewBox="0 0 16 16"
            className="h-3.5 w-3.5 fill-none stroke-current"
            strokeWidth="1.5"
          >
            <rect x="2" y="2.5" width="12" height="11" rx="1.5" />
            <path d="M6 2.5v11" />
          </svg>
        </IconButton>

        <h1 className="text-xs font-semibold tracking-tight text-fg">{t.appName}</h1>

        <select
          aria-label={t.newSession.projectLabel}
          value={selectedProjectId}
          onChange={(e) => setSelectedProjectId(e.target.value)}
          className="hidden rounded-sm border border-line bg-app px-2 py-0.5 text-xs text-fg-2 focus:border-accent focus:outline-none sm:inline-block"
        >
          <option value="all">{t.routes.workspace}: All</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>

      <nav aria-label="Primary navigation" className="flex flex-wrap items-center gap-1">
        {ROUTES.map((route) => {
          const active = activeRoute === route;
          return (
            <button
              key={route}
              type="button"
              onClick={() => setActiveRoute(route)}
              className={cn(
                "rounded-sm px-2.5 py-1 text-xs font-medium transition-colors",
                active ? "bg-raised text-fg" : "text-fg-3 hover:bg-hover hover:text-fg",
              )}
            >
              {t.routes[route]}
            </button>
          );
        })}
      </nav>

      <div className="flex items-center gap-1.5">
        <button
          type="button"
          onClick={() => setCommandPaletteOpen(true)}
          className="hidden items-center gap-2 rounded-sm border border-line bg-app px-2.5 py-1 text-xs text-fg-3 hover:bg-hover hover:text-fg md:inline-flex"
        >
          <span>Search</span>
          <Kbd keys={formatShortcut("Cmd+K", "Ctrl+K")} />
        </button>

        <Button
          variant="secondary"
          size="sm"
          onClick={() => void launchAntigravityCliQuick()}
        >
          {t.workspace.openAntigravityCli}
        </Button>

        <Button variant="primary" size="sm" onClick={() => setNewSessionOpen(true)}>
          {t.workspace.newSessionCta}
        </Button>

        <UsagePill onClick={() => setActiveRoute("usage")} />

        <Button
          size="sm"
          variant="ghost"
          aria-label={t.localeToggleLabel}
          onClick={() => setLocale(locale === "id" ? "en" : "id")}
        >
          {locale.toUpperCase()}
        </Button>

        {tauriMode ? (
          <div className="ml-1 flex items-center gap-1 border-l border-line pl-2">
            <IconButton label="Minimize" onClick={() => void windowAction("minimize")}>
              <svg aria-hidden="true" viewBox="0 0 16 16" className="h-3 w-3 stroke-current" strokeWidth="1.5">
                <path d="M3 8h10" />
              </svg>
            </IconButton>
            <IconButton label="Maximize" onClick={() => void windowAction("toggle_maximize")}>
              <svg
                aria-hidden="true"
                viewBox="0 0 16 16"
                className="h-3 w-3 fill-none stroke-current"
                strokeWidth="1.5"
              >
                <rect x="3.5" y="3.5" width="9" height="9" rx="1" />
              </svg>
            </IconButton>
            <IconButton label="Close" onClick={() => void windowAction("close")}>
              <svg aria-hidden="true" viewBox="0 0 16 16" className="h-3 w-3 stroke-current" strokeWidth="1.5">
                <path d="M4 4l8 8M12 4l-8 8" />
              </svg>
            </IconButton>
          </div>
        ) : null}
      </div>
    </header>
  );
}

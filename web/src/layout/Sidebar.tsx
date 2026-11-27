import { useState } from "react";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Segmented } from "../components/ui/Segmented";
import { TreeItem } from "../components/TreeItem";
import { SessionRow } from "../features/sessions/SessionRow";
import { CATALOGS } from "../lib/i18n/catalog";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore } from "../state/layout";
import { useNotificationsStore } from "../state/notifications";
import { useFilteredSessions, useGroupedSessions } from "../state/selectors";
import { useSessionsStore } from "../state/sessions";
import { useUiStore, type SidebarGrouping } from "../state/ui";
import { addProject } from "../transport/daemon-client";

export function Sidebar() {
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const grouping = useUiStore((s) => s.sidebarGrouping);
  const setGrouping = useUiStore((s) => s.setSidebarGrouping);
  const searchQuery = useUiStore((s) => s.searchQuery);
  const setSearchQuery = useUiStore((s) => s.setSearchQuery);
  const locale = useUiStore((s) => s.locale);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);

  const projects = useHostsStore((s) => s.projects);
  const token = useHostsStore((s) => s.token);
  const loading = useHostsStore((s) => s.loading);
  const addProjectLocal = useHostsStore((s) => s.addProjectLocal);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const activeSessionId = useLayoutStore((s) => s.activeSessionId);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const markRead = useSessionsStore((s) => s.markRead);
  const totalCount = useSessionsStore((s) => s.order.length);
  const grouped = useGroupedSessions();
  const filtered = useFilteredSessions();

  const [repoPath, setRepoPath] = useState("");
  const t = CATALOGS[locale];

  if (!sidebarOpen) {
    return null;
  }

  const projectById = new Map(projects.map((p) => [p.id, p]));

  const selectSession = (id: string) => {
    setActiveSession(id);
    markRead(id);
    setActiveRoute("workspace");
  };

  const handleAddRepo = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!token || !repoPath.trim()) {
      return;
    }
    try {
      const proj = await addProject(token, repoPath.trim());
      addProjectLocal(proj);
      setRepoPath("");
      pushToast("success", `Added ${proj.name}`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed adding repository");
    }
  };

  const groupingOptions: readonly { value: SidebarGrouping; label: string }[] = [
    { value: "status", label: t.sidebar.statusMode },
    { value: "tree", label: t.sidebar.treeMode },
  ];

  return (
    <aside
      aria-label={t.sidebar.heading}
      className="flex w-full shrink-0 flex-col border-b border-line bg-sidebar p-2.5 md:w-[var(--sidebar-w)] md:border-r md:border-b-0"
    >
      <div className="mb-2.5 space-y-2">
        <Input
          type="search"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder={t.sidebar.searchPlaceholder}
          aria-label={t.sidebar.searchPlaceholder}
        />
        <Segmented
          label={t.sidebar.heading}
          value={grouping}
          options={groupingOptions}
          onChange={setGrouping}
          className="w-full"
        />
      </div>

      {projects.length === 0 && token ? (
        <form onSubmit={(e) => void handleAddRepo(e)} className="mb-3 flex gap-1.5">
          <Input
            type="text"
            value={repoPath}
            onChange={(e) => setRepoPath(e.target.value)}
            placeholder={t.sidebar.addRepoPlaceholder}
            aria-label={t.sidebar.addRepoPlaceholder}
          />
          <Button type="submit" size="sm">
            +
          </Button>
        </form>
      ) : null}

      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-0.5">
        {loading && totalCount === 0 ? (
          <p className="px-2 py-4 text-xs text-fg-3" role="status">
            {t.sidebar.loading}
          </p>
        ) : filtered.length === 0 ? (
          <div className="rounded-md border border-line bg-surface p-3 text-xs">
            <p className="font-medium text-fg">
              {totalCount === 0 ? t.sidebar.emptyTitle : t.sidebar.emptyFiltered}
            </p>
            <p className="mt-1 text-fg-3">{t.sidebar.emptyBody}</p>
            <Button
              size="sm"
              variant="primary"
              className="mt-3 w-full"
              onClick={() => setNewSessionOpen(true)}
            >
              {t.workspace.newSessionCta}
            </Button>
          </div>
        ) : grouping === "status" ? (
          grouped.map((group) => (
            <section key={group.status}>
              <h2 className="mb-1 px-2 text-[11px] font-medium text-fg-3">
                {t.statusLabels[group.status]} ({group.items.length})
              </h2>
              <ul className="space-y-1">
                {group.items.map((ses) => (
                  <li key={ses.id}>
                    <SessionRow
                      session={ses}
                      projectName={projectById.get(ses.project_id)?.name}
                      selected={ses.id === activeSessionId}
                      onSelect={selectSession}
                    />
                  </li>
                ))}
              </ul>
            </section>
          ))
        ) : (
          <div className="space-y-2">
            <TreeItem label="local-daemon" meta={`${filtered.length}`} />
            {projects.map((proj) => {
              const projSessions = filtered.filter((s) => s.project_id === proj.id);
              return (
                <div key={proj.id} className="space-y-0.5">
                  <TreeItem label={proj.name} meta={proj.default_branch} depth={1} />
                  {projSessions.map((ses) => (
                    <TreeItem
                      key={ses.id}
                      label={ses.task ?? ses.branch_name}
                      meta={ses.agent_profile}
                      depth={2}
                      selected={ses.id === activeSessionId}
                      onClick={() => selectSession(ses.id)}
                    />
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </aside>
  );
}

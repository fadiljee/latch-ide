import { ModelBadge } from "../../components/ModelBadge";
import { StatusDot } from "../../components/StatusDot";
import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import { formatRelativeShort } from "../../lib/time";
import { useHostsStore } from "../../state/hosts";
import { useNotificationsStore } from "../../state/notifications";
import { useFilteredSessions } from "../../state/selectors";
import { mapSessionUiStatus, useSessionsStore } from "../../state/sessions";
import { useUsageStore } from "../../state/usage";
import { fetchSessions, postSessionAction } from "../../transport/daemon-client";
import type { Session, UiStatus } from "../../transport/protocol";

const BOARD_COLUMNS: readonly UiStatus[] = ["needs_you", "working", "error", "done", "idle"];

interface SessionBoardProps {
  token: string;
  statusLabels: Catalog["statusLabels"];
  actions: Catalog["sessionActions"];
  workspaceLabels: Catalog["workspace"];
  onOpenSession: (sessionId: string, tab: "terminal" | "diff") => void;
  onNewSession: () => void;
}

export function SessionBoard({
  token,
  statusLabels,
  actions,
  workspaceLabels,
  onOpenSession,
  onNewSession,
}: SessionBoardProps) {
  const filtered = useFilteredSessions();
  const projects = useHostsStore((s) => s.projects);
  const bySessionId = useUsageStore((s) => s.bySessionId);
  const upsertMany = useSessionsStore((s) => s.upsertMany);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const projectById = new Map(projects.map((p) => [p.id, p.name]));

  const columns: Record<UiStatus, Session[]> = {
    needs_you: [],
    working: [],
    error: [],
    done: [],
    idle: [],
  };
  for (const ses of filtered) {
    columns[mapSessionUiStatus(ses.status)].push(ses);
  }

  const handleQuickAction = (
    sessionId: string,
    action: "pause" | "resume" | "hibernate" | "kill",
  ) => {
    if (!token) {
      return;
    }
    void postSessionAction(token, sessionId, action)
      .then(() => fetchSessions(token))
      .then((list) => {
        upsertMany(list);
        pushToast("success", `Session ${action} complete`);
      })
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Action failed"));
  };

  return (
    <section
      aria-label={workspaceLabels.boardMode}
      className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 px-1">
        <div>
          <h2 className="text-sm font-semibold text-fg">{workspaceLabels.boardHeading}</h2>
          <p className="text-xs text-fg-3">{workspaceLabels.boardSubtitle}</p>
        </div>
        <Button size="sm" variant="primary" onClick={onNewSession}>
          {workspaceLabels.newSessionCta}
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-x-auto pb-2 md:grid-cols-2 xl:grid-cols-5">
        {BOARD_COLUMNS.map((colStatus) => {
          const items = columns[colStatus];
          return (
            <div
              key={colStatus}
              data-board-column={colStatus}
              className="flex min-h-0 flex-col rounded-lg border border-line bg-surface"
            >
              <header className="flex items-center justify-between gap-2 border-b border-line px-3 py-2 text-xs">
                <span className="flex items-center gap-2 font-semibold text-fg">
                  <StatusDot status={colStatus} />
                  <span>{statusLabels[colStatus]}</span>
                </span>
                <span className="rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-[10px] text-fg-3">
                  {items.length}
                </span>
              </header>

              <div className="min-h-0 flex-1 space-y-2 overflow-y-auto p-2">
                {items.length === 0 ? (
                  <p className="px-2 py-6 text-center text-[11px] text-fg-3">
                    {workspaceLabels.boardColumnEmpty}
                  </p>
                ) : (
                  items.map((ses) => {
                    const summary = bySessionId[ses.id];
                    const tokens =
                      (summary?.prompt_tokens ?? 0) + (summary?.completion_tokens ?? 0);
                    return (
                      <article
                        key={ses.id}
                        className="space-y-2 rounded-md border border-line bg-app p-2.5 text-xs transition-colors hover:border-accent/60"
                      >
                        <button
                          type="button"
                          onClick={() => onOpenSession(ses.id, "terminal")}
                          className="block w-full text-left"
                        >
                          <p className="line-clamp-2 font-medium text-fg">
                            {ses.task?.trim() || ses.branch_name}
                          </p>
                          <p className="mt-0.5 truncate font-mono text-[10px] text-fg-3">
                            {projectById.get(ses.project_id) ?? ses.branch_name}
                          </p>
                        </button>

                        <div className="flex flex-wrap items-center justify-between gap-1.5">
                          <ModelBadge profile={ses.agent_profile} />
                          <span className="font-mono text-[10px] text-fg-3">
                            {formatRelativeShort(ses.started_at)}
                          </span>
                        </div>

                        {tokens > 0 ? (
                          <p className="font-mono text-[10px] text-fg-3">
                            {(tokens / 1000).toFixed(1)}k tok · $
                            {(summary?.estimated_cost_usd ?? 0).toFixed(4)}
                          </p>
                        ) : null}

                        <div className="flex flex-wrap items-center gap-1 border-t border-line pt-2">
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => onOpenSession(ses.id, "terminal")}
                          >
                            {workspaceLabels.terminalTab}
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => onOpenSession(ses.id, "diff")}
                          >
                            {workspaceLabels.diffTab}
                          </Button>
                          {ses.status === "running" ? (
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => handleQuickAction(ses.id, "pause")}
                            >
                              {actions.pause}
                            </Button>
                          ) : null}
                          {ses.status === "paused" || ses.status === "hibernated" ? (
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => handleQuickAction(ses.id, "resume")}
                            >
                              {actions.resume}
                            </Button>
                          ) : null}
                        </div>
                      </article>
                    );
                  })
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

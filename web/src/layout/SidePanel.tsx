import { lazy, Suspense } from "react";
import { Button } from "../components/ui/Button";
import { TabBar, type TabOption } from "../components/TabBar";
import { GitPanel } from "../features/review/GitPanel";
import { CATALOGS } from "../lib/i18n/catalog";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore } from "../state/layout";
import { useNotificationsStore } from "../state/notifications";
import { useSessionsStore } from "../state/sessions";
import { useUiStore, type SidePanelTab } from "../state/ui";
import { decideApproval, fetchSessions, postSessionAction } from "../transport/daemon-client";

const TrafficView = lazy(() => import("../features/traffic/TrafficView"));

export function SidePanel() {
  const open = useUiStore((s) => s.sidePanelOpen);
  const activeTab = useUiStore((s) => s.sidePanelTab);
  const setTab = useUiStore((s) => s.setSidePanelTab);
  const locale = useUiStore((s) => s.locale);
  const activeRoute = useUiStore((s) => s.activeRoute);

  const token = useHostsStore((s) => s.token);
  const activeSessionId = useLayoutStore((s) => s.activeSessionId);
  const byId = useSessionsStore((s) => s.byId);
  const upsertMany = useSessionsStore((s) => s.upsertMany);
  const approvals = useNotificationsStore((s) => s.approvals);
  const removeApproval = useNotificationsStore((s) => s.removeApproval);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  if (!open || activeRoute !== "workspace") {
    return null;
  }

  const t = CATALOGS[locale];
  const session = activeSessionId ? byId[activeSessionId] : undefined;

  const tabs: readonly TabOption<SidePanelTab>[] = [
    { id: "git", label: t.sidePanel.gitTab },
    { id: "checks", label: t.sidePanel.checksTab, badge: approvals.length },
    { id: "traffic", label: t.sidePanel.trafficTab },
  ];

  const handleGitAction = (action: "merge" | "discard") => {
    if (!token || !session) {
      return;
    }
    void postSessionAction(
      token,
      session.id,
      action,
      action === "merge" ? { strategy: "squash" } : undefined,
    )
      .then(() => fetchSessions(token))
      .then((list) => {
        upsertMany(list);
        pushToast("success", `Session ${action} complete`);
      })
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Git action failed"));
  };

  return (
    <aside
      aria-label="Inspector panel"
      className="latch-glass-chrome hidden w-[var(--sidepanel-w)] shrink-0 flex-col border-l border-line bg-sidebar lg:flex"
    >
      <div className="border-b border-line p-2.5">
        <TabBar label="Inspector tabs" tabs={tabs} active={activeTab} onSelect={setTab} />
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {activeTab === "git" ? (
          session ? (
            <GitPanel
              token={token}
              session={session}
              labels={t.sidePanel}
              actions={t.sessionActions}
              onAction={handleGitAction}
            />
          ) : (
            <p className="text-xs text-fg-3">{t.workspace.emptyTitle}</p>
          )
        ) : activeTab === "checks" ? (
          approvals.length === 0 ? (
            <p className="text-xs text-fg-3">{t.sidePanel.noApprovals}</p>
          ) : (
            <ul className="space-y-2">
              {approvals.map((req) => (
                <li
                  key={req.id}
                  className="rounded-md border border-line bg-surface p-2.5 text-xs"
                >
                  <span className="inline-block rounded-sm bg-needs/15 px-1.5 py-0.5 font-mono text-[10px] text-needs">
                    {req.action_type}
                  </span>
                  <p className="mt-1.5 break-all font-mono text-[11px] text-fg">
                    {req.payload_summary}
                  </p>
                  <div className="mt-2 flex gap-1.5">
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => {
                        void decideApproval(token, req.id, "approved").then(() =>
                          removeApproval(req.id),
                        );
                      }}
                    >
                      {t.sidePanel.approve}
                    </Button>
                    <Button
                      size="sm"
                      variant="danger"
                      onClick={() => {
                        void decideApproval(token, req.id, "denied").then(() =>
                          removeApproval(req.id),
                        );
                      }}
                    >
                      {t.sidePanel.deny}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )
        ) : session ? (
          <Suspense
            fallback={<p className="text-xs text-fg-3">{t.workspace.loadingTelemetry}</p>}
          >
            <TrafficView
              token={token}
              sessionId={session.id}
              emptyLabel={t.sidePanel.noTraffic}
              loadingLabel={t.workspace.loadingTelemetry}
            />
          </Suspense>
        ) : (
          <p className="text-xs text-fg-3">{t.workspace.emptyTitle}</p>
        )}
      </div>
    </aside>
  );
}

import { useEffect } from "react";
import { Button } from "../../components/ui/Button";
import { StatusDot } from "../../components/StatusDot";
import { CATALOGS } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useNotificationsStore } from "../../state/notifications";
import { useFilteredSessions } from "../../state/selectors";
import { mapSessionUiStatus } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import { initThemeSubscription } from "../../theme/theme-store";
import { connectEventsSocket, decideApproval } from "../../transport/daemon-client";
import { syncAllFromDaemon } from "../../transport/events";

export function FloatingControl() {
  const token = useHostsStore((s) => s.token);
  const connected = useHostsStore((s) => s.connected);
  const setConnected = useHostsStore((s) => s.setConnected);
  const approvals = useNotificationsStore((s) => s.approvals);
  const removeApproval = useNotificationsStore((s) => s.removeApproval);
  const locale = useUiStore((s) => s.locale);
  const sessions = useFilteredSessions();
  const t = CATALOGS[locale];

  useEffect(() => initThemeSubscription(), []);

  useEffect(() => {
    if (!token) {
      return;
    }
    void syncAllFromDaemon(token);
    return connectEventsSocket(
      token,
      () => {
        void syncAllFromDaemon(token);
      },
      (ok) => setConnected(ok),
      () => {
        void syncAllFromDaemon(token);
      },
    );
  }, [token, setConnected]);

  return (
    <main className="flex h-screen flex-col overflow-hidden bg-app p-3 text-fg">
      <header
        data-tauri-drag-region=""
        className="mb-3 flex items-center justify-between border-b border-line pb-2"
      >
        <h1 className="text-xs font-semibold">{t.floating.title}</h1>
        <StatusDot status={connected ? "done" : "idle"} label={t.connection.label} />
      </header>

      <section className="mb-3 space-y-2 overflow-y-auto">
        <h2 className="text-[11px] font-medium text-fg-2">
          {t.floating.needsYou} ({approvals.length})
        </h2>
        {approvals.length === 0 ? (
          <p className="rounded-md border border-line bg-surface p-2.5 text-[11px] text-fg-3">
            {t.floating.noPending}
          </p>
        ) : (
          <ul className="space-y-1.5">
            {approvals.map((req) => (
              <li key={req.id} className="rounded-md border border-line bg-surface p-2 text-[11px]">
                <p className="truncate font-mono text-fg">{req.payload_summary}</p>
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
        )}
      </section>

      <section className="min-h-0 flex-1 overflow-y-auto">
        <h2 className="mb-1.5 text-[11px] font-medium text-fg-2">{t.sidebar.heading}</h2>
        <ul className="space-y-1">
          {sessions.slice(0, 8).map((s) => (
            <li
              key={s.id}
              className="flex items-center justify-between rounded-sm border border-line bg-surface px-2 py-1.5 text-[11px]"
            >
              <span className="flex min-w-0 items-center gap-2">
                <StatusDot status={mapSessionUiStatus(s.status)} />
                <span className="truncate">{s.task ?? s.branch_name}</span>
              </span>
              <span className="font-mono text-fg-3">{s.agent_profile}</span>
            </li>
          ))}
        </ul>
      </section>
    </main>
  );
}

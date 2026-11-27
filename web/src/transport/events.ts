import {
  fetchHealth,
  fetchPendingApprovals,
  fetchPipelines,
  fetchProjects,
  fetchSessions,
} from "./daemon-client";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore } from "../state/layout";
import { useNotificationsStore } from "../state/notifications";
import { useSessionsStore } from "../state/sessions";
import { useUsageStore } from "../state/usage";

export async function syncAllFromDaemon(token: string): Promise<void> {
  const hosts = useHostsStore.getState();
  const sessionsStore = useSessionsStore.getState();
  const layoutStore = useLayoutStore.getState();
  const notifStore = useNotificationsStore.getState();
  const usageStore = useUsageStore.getState();

  hosts.setLoading(true);
  try {
    const [health, projects, sessions, approvals, pipelines] = await Promise.all([
      fetchHealth().catch(() => null),
      fetchProjects(token),
      fetchSessions(token),
      fetchPendingApprovals(token).catch(() => []),
      fetchPipelines(token).catch(() => []),
    ]);

    if (health) {
      hosts.setHealth(health);
    }
    hosts.setProjects(projects);
    sessionsStore.upsertMany(sessions);
    notifStore.setApprovals(approvals);
    usageStore.setPipelines(pipelines);
    hosts.setError(null);

    const active = layoutStore.activeSessionId;
    const first = sessions[0];
    if ((!active || !sessions.some((s) => s.id === active)) && first) {
      layoutStore.setActiveSession(first.id);
    }
  } catch (err) {
    hosts.setError(err instanceof Error ? err.message : "Failed to connect to Latch daemon");
  } finally {
    hosts.setLoading(false);
  }
}

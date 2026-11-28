import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useLayoutStore } from "../../state/layout";
import { useNotificationsStore } from "../../state/notifications";
import { useSessionsStore } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import { createSession } from "../../transport/daemon-client";

/**
 * Meluncurkan sesi interaktif Antigravity CLI (`agy`) secara langsung pada proyek aktif,
 * atau membuka dialog sesi baru dengan profil `antigravity` bila belum ada proyek terdaftar.
 */
export async function launchAntigravityCliQuick(): Promise<void> {
  const { token, projects, selectedProjectId } = useHostsStore.getState();
  const { setPreferredProfile } = useIntegrationsStore.getState();
  const { setNewSessionOpen, setActiveRoute } = useUiStore.getState();
  const { setActiveSession, setPrimaryTab } = useLayoutStore.getState();
  const { upsertOne } = useSessionsStore.getState();
  const { pushToast } = useNotificationsStore.getState();

  const targetProject =
    (selectedProjectId !== "all"
      ? projects.find((p) => p.id === selectedProjectId)
      : undefined) ?? projects[0];

  if (!token || !targetProject) {
    setPreferredProfile("antigravity");
    setNewSessionOpen(true);
    return;
  }

  try {
    const created = await createSession(token, {
      project: targetProject.name,
      profile: "antigravity",
      task: "",
      extra_args: [],
    });
    upsertOne(created);
    setActiveSession(created.id);
    setPrimaryTab("terminal");
    setActiveRoute("workspace");
    pushToast("success", `Antigravity CLI (agy) started on ${targetProject.name}`);
  } catch (err) {
    pushToast("danger", err instanceof Error ? err.message : "Failed launching Antigravity CLI");
  }
}

import { useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import { Segmented } from "../../components/ui/Segmented";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useLayoutStore } from "../../state/layout";
import { useNotificationsStore } from "../../state/notifications";
import { useFilteredSessions } from "../../state/selectors";
import { useSessionsStore } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import {
  deleteAgent,
  setAgentRuntimeKey,
  startAgentLogin,
  upsertAgent,
} from "../../transport/daemon-client";
import type { AgentAuthMode, AgentProfile, AgentProfileStatus } from "../../transport/protocol";
import { AddAgentDialog } from "./AddAgentDialog";
import { AgentCard } from "./AgentCard";
import { FALLBACK_AGENTS } from "./fallback-agents";

type AgentFilter = "all" | "installed" | "custom";

interface BotsPageProps {
  labels: Catalog["pages"];
  agentLabels: Catalog["agents"];
  newSessionLabel: string;
}

export function BotsPage({ labels, agentLabels, newSessionLabel }: BotsPageProps) {
  const token = useHostsStore((s) => s.token);
  const liveAgents = useIntegrationsStore((s) => s.agents);
  const upsertAgentLocal = useIntegrationsStore((s) => s.upsertAgentLocal);
  const removeAgentLocal = useIntegrationsStore((s) => s.removeAgentLocal);
  const setPreferredProfile = useIntegrationsStore((s) => s.setPreferredProfile);
  const upsertSession = useSessionsStore((s) => s.upsertOne);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const setPrimaryTab = useLayoutStore((s) => s.setPrimaryTab);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const pushToast = useNotificationsStore((s) => s.pushToast);
  const sessions = useFilteredSessions();

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AgentFilter>("all");
  const [addOpen, setAddOpen] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);

  const agents = liveAgents.length > 0 ? liveAgents : FALLBACK_AGENTS;
  const connected = Boolean(token);

  const filteredAgents = useMemo(() => {
    const q = query.trim().toLowerCase();
    return agents.filter((agent) => {
      if (filter === "installed" && !agent.installed) {
        return false;
      }
      if (filter === "custom" && agent.builtin) {
        return false;
      }
      if (!q) {
        return true;
      }
      return (
        agent.id.toLowerCase().includes(q) ||
        (agent.profile.display_name ?? "").toLowerCase().includes(q) ||
        agent.profile.command.toLowerCase().includes(q) ||
        (agent.profile.description ?? "").toLowerCase().includes(q) ||
        (agent.profile.proxy_provider ?? "").toLowerCase().includes(q)
      );
    });
  }, [agents, filter, query]);

  const handleSwitchAuthMode = (agent: AgentProfileStatus, nextMode: AgentAuthMode) => {
    if (!token) {
      return;
    }
    setBusyId(agent.id);
    const updatedProfile: AgentProfile = {
      ...agent.profile,
      auth_mode: nextMode,
    };
    void upsertAgent(token, agent.id, updatedProfile)
      .then((saved) => {
        upsertAgentLocal(saved);
        pushToast("success", `${agent.id}: ${nextMode}`);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed updating agent auth mode"),
      )
      .finally(() => setBusyId(null));
  };

  const handleSaveRuntimeKey = async (agent: AgentProfileStatus, apiKey: string) => {
    if (!token) {
      return;
    }
    setBusyId(agent.id);
    try {
      const saved = await setAgentRuntimeKey(token, agent.id, apiKey);
      upsertAgentLocal(saved);
      pushToast("success", `${agent.id}: runtime key set in memory`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed setting runtime key");
    } finally {
      setBusyId(null);
    }
  };

  const handleClearRuntimeKey = async (agent: AgentProfileStatus) => {
    if (!token) {
      return;
    }
    setBusyId(agent.id);
    try {
      const saved = await setAgentRuntimeKey(token, agent.id, null);
      upsertAgentLocal(saved);
      pushToast("info", `${agent.id}: runtime key cleared`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed clearing runtime key");
    } finally {
      setBusyId(null);
    }
  };

  const handleOAuthLogin = (agent: AgentProfileStatus) => {
    if (!token) {
      return;
    }
    setBusyId(agent.id);
    void startAgentLogin(token, agent.id)
      .then((session) => {
        upsertSession(session);
        setActiveSession(session.id);
        setPrimaryTab("terminal");
        setActiveRoute("workspace");
        pushToast("info", `OAuth terminal opened for ${agent.profile.display_name || agent.id}`);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed opening OAuth login"),
      )
      .finally(() => setBusyId(null));
  };

  const handleLaunch = (agent: AgentProfileStatus) => {
    setPreferredProfile(agent.id);
    setNewSessionOpen(true);
  };

  const handleDelete = (agent: AgentProfileStatus) => {
    if (!token) {
      return;
    }
    setBusyId(agent.id);
    void deleteAgent(token, agent.id)
      .then(() => {
        removeAgentLocal(agent.id);
        pushToast("info", `Removed agent ${agent.id}`);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed deleting agent"),
      )
      .finally(() => setBusyId(null));
  };

  const handleAddAgent = async (id: string, profile: AgentProfile) => {
    if (!token) {
      return;
    }
    setBusyId("__new__");
    try {
      const saved = await upsertAgent(token, id, profile);
      upsertAgentLocal(saved);
      setAddOpen(false);
      pushToast("success", `Saved custom agent ${saved.id}`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed saving custom agent");
    } finally {
      setBusyId(null);
    }
  };

  const filterOptions = [
    { value: "all" as const, label: agentLabels.filterAll },
    { value: "installed" as const, label: agentLabels.filterInstalled },
    { value: "custom" as const, label: agentLabels.filterCustom },
  ];

  return (
    <div className="flex-1 space-y-5 overflow-y-auto p-5 text-xs">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-fg">{labels.botsHeading}</h2>
          <p className="mt-0.5 max-w-2xl text-fg-3">{labels.botsSubtitle}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="secondary" disabled={!connected} onClick={() => setAddOpen(true)}>
            {agentLabels.addCustom}
          </Button>
          <Button variant="primary" onClick={() => setNewSessionOpen(true)}>
            {newSessionLabel}
          </Button>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2.5">
        <Input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={agentLabels.searchPlaceholder}
          aria-label={agentLabels.searchPlaceholder}
          className="min-w-52 flex-1"
        />
        <Segmented
          label={agentLabels.searchPlaceholder}
          value={filter}
          options={filterOptions}
          onChange={setFilter}
        />
      </div>

      {filteredAgents.length === 0 ? (
        <div
          role="status"
          className="rounded-lg border border-line bg-surface p-6 text-center text-fg-3"
        >
          {agentLabels.emptySearch}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {filteredAgents.map((agent) => {
            const activeCount = sessions.filter(
              (s) => s.agent_profile === agent.id && s.status === "running",
            ).length;
            return (
              <AgentCard
                key={agent.id}
                agent={agent}
                activeSessions={activeCount}
                labels={agentLabels}
                connected={connected}
                busy={busyId === agent.id}
                onSwitchAuthMode={handleSwitchAuthMode}
                onSaveRuntimeKey={handleSaveRuntimeKey}
                onClearRuntimeKey={handleClearRuntimeKey}
                onOAuthLogin={handleOAuthLogin}
                onLaunch={handleLaunch}
                onDelete={handleDelete}
              />
            );
          })}
        </div>
      )}

      <AddAgentDialog
        open={addOpen}
        labels={agentLabels}
        busy={busyId === "__new__"}
        onClose={() => setAddOpen(false)}
        onSave={handleAddAgent}
      />
    </div>
  );
}

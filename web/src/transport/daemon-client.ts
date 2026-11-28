import type {
  AgentProfile,
  AgentProfileStatus,
  AntigravityCliInfo,
  ApprovalRequest,
  DaemonEventFrame,
  DaemonHealth,
  GcReport,
  GitHubRepoItem,
  GitHubStatus,
  McpServerConfig,
  MergeCheckResult,
  PipelineArenaResponse,
  PipelineRun,
  PluginConfig,
  Project,
  ScreenSnapshot,
  Session,
  SessionDetail,
  SessionDiff,
  SessionTelemetrySummary,
  TelemetryRecord,
  UpdateInfo,
} from "./protocol";

declare global {
  interface Window {
    __LATCH_API_BASE__?: string;
    __LATCH_TOKEN__?: string;
    __LATCH_TAURI__?: boolean;
    __TAURI_INTERNALS__?: {
      invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

export function apiBase(): string {
  return typeof window !== "undefined" ? (window.__LATCH_API_BASE__ ?? "") : "";
}

export function wsBase(): string {
  const base = apiBase();
  if (base.startsWith("http://")) {
    return `ws://${base.slice("http://".length)}`;
  }
  if (base.startsWith("https://")) {
    return `wss://${base.slice("https://".length)}`;
  }
  if (typeof window !== "undefined") {
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${window.location.host}`;
  }
  return "ws://127.0.0.1:7878";
}

function authHeaders(token: string): HeadersInit {
  return {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
}

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(url, init);
  if (!resp.ok) {
    let msg = `HTTP ${resp.status}`;
    try {
      const parsed = (await resp.json()) as { error?: { message?: string } };
      if (parsed.error?.message) {
        msg = parsed.error.message;
      }
    } catch {
      // Abaikan body non-JSON
    }
    throw new Error(msg);
  }
  return (await resp.json()) as T;
}

export async function fetchHealth(): Promise<DaemonHealth> {
  return fetchJson<DaemonHealth>(`${apiBase()}/api/v1/health`);
}

export async function fetchUpdateInfo(token: string): Promise<UpdateInfo> {
  return fetchJson<UpdateInfo>(`${apiBase()}/api/v1/update`, {
    headers: authHeaders(token),
  });
}

export async function fetchProjects(token: string): Promise<Project[]> {
  return fetchJson<Project[]>(`${apiBase()}/api/v1/projects`, {
    headers: authHeaders(token),
  });
}

export async function addProject(
  token: string,
  repoPath: string,
  name?: string,
): Promise<Project> {
  return fetchJson<Project>(`${apiBase()}/api/v1/projects`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({
      repo_path: repoPath,
      name: name ?? null,
      default_branch: null,
    }),
  });
}

export async function fetchSessions(token: string): Promise<Session[]> {
  return fetchJson<Session[]>(`${apiBase()}/api/v1/sessions`, {
    headers: authHeaders(token),
  });
}

export async function createSession(
  token: string,
  params: {
    project: string;
    profile: string;
    task?: string;
    base_ref?: string;
    extra_args?: string[];
  },
): Promise<Session> {
  return fetchJson<Session>(`${apiBase()}/api/v1/sessions`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({
      project: params.project,
      profile: params.profile,
      task: params.task ?? "",
      base_ref: params.base_ref ?? null,
      extra_args: params.extra_args ?? [],
    }),
  });
}

export async function fetchSessionDetail(token: string, id: string): Promise<SessionDetail> {
  return fetchJson<SessionDetail>(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}`, {
    headers: authHeaders(token),
  });
}

export async function fetchSessionDiff(token: string, id: string): Promise<SessionDiff> {
  return fetchJson<SessionDiff>(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/diff`, {
    headers: authHeaders(token),
  });
}

export async function checkSessionMerge(token: string, id: string): Promise<MergeCheckResult> {
  return fetchJson<MergeCheckResult>(
    `${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/merge-check`,
    { headers: authHeaders(token) },
  );
}

export async function fetchSessionScreen(token: string, id: string): Promise<ScreenSnapshot> {
  return fetchJson<ScreenSnapshot>(
    `${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/screen`,
    { headers: authHeaders(token) },
  );
}

export async function fetchSessionTelemetry(
  token: string,
  id: string,
): Promise<{ summary: SessionTelemetrySummary; records: TelemetryRecord[] }> {
  return fetchJson<{ summary: SessionTelemetrySummary; records: TelemetryRecord[] }>(
    `${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/telemetry`,
    { headers: authHeaders(token) },
  );
}

export async function postSessionAction(
  token: string,
  id: string,
  action: "pause" | "resume" | "hibernate" | "kill" | "discard" | "merge",
  body?: Record<string, unknown>,
): Promise<void> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/${action}`, {
    method: "POST",
    headers: authHeaders(token),
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
}

export async function sendSessionInput(token: string, id: string, data: string): Promise<void> {
  const bytes = new TextEncoder().encode(data);
  let binary = "";
  for (const b of bytes) {
    binary += String.fromCharCode(b);
  }
  const resp = await fetch(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/input`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ data_base64: btoa(binary) }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
}

export async function fetchPendingApprovals(token: string): Promise<ApprovalRequest[]> {
  return fetchJson<ApprovalRequest[]>(`${apiBase()}/api/v1/approvals?status=pending`, {
    headers: authHeaders(token),
  });
}

export async function decideApproval(
  token: string,
  id: string,
  decision: "approved" | "denied",
): Promise<ApprovalRequest> {
  return fetchJson<ApprovalRequest>(
    `${apiBase()}/api/v1/approvals/${encodeURIComponent(id)}/decision`,
    {
      method: "POST",
      headers: authHeaders(token),
      body: JSON.stringify({ decision, decided_by: "web" }),
    },
  );
}

export async function fetchPipelines(token: string): Promise<PipelineRun[]> {
  return fetchJson<PipelineRun[]>(`${apiBase()}/api/v1/pipelines`, {
    headers: authHeaders(token),
  });
}

export async function runPipeline(
  token: string,
  params: { project: string; workflow_toml: string; task: string },
): Promise<PipelineRun> {
  return fetchJson<PipelineRun>(`${apiBase()}/api/v1/pipelines`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({
      project: params.project,
      workflow_toml: params.workflow_toml,
      workflow_path: null,
      task: params.task,
    }),
  });
}

export async function fetchPipelineArena(
  token: string,
  id: string,
): Promise<PipelineArenaResponse> {
  return fetchJson<PipelineArenaResponse>(
    `${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/arena`,
    { headers: authHeaders(token) },
  );
}

export async function selectPipelineArenaWinner(
  token: string,
  id: string,
  stepId: string,
  sessionId: string,
): Promise<PipelineRun> {
  return fetchJson<PipelineRun>(
    `${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/arena/select`,
    {
      method: "POST",
      headers: authHeaders(token),
      body: JSON.stringify({ step_id: stepId, session_id: sessionId }),
    },
  );
}

export async function cancelPipeline(token: string, id: string): Promise<PipelineRun> {
  return fetchJson<PipelineRun>(
    `${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/cancel`,
    {
      method: "POST",
      headers: authHeaders(token),
    },
  );
}

export async function triggerGc(token: string, dryRun: boolean): Promise<GcReport> {
  return fetchJson<GcReport>(`${apiBase()}/api/v1/gc`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ dry_run: dryRun }),
  });
}

export async function fetchAgents(token: string): Promise<AgentProfileStatus[]> {
  return fetchJson<AgentProfileStatus[]>(`${apiBase()}/api/v1/agents`, {
    headers: authHeaders(token),
  });
}

export async function fetchAntigravityCliInfo(token: string): Promise<AntigravityCliInfo> {
  return fetchJson<AntigravityCliInfo>(`${apiBase()}/api/v1/agents/antigravity/info`, {
    headers: authHeaders(token),
  });
}

export async function upsertAgent(
  token: string,
  id: string,
  profile: AgentProfile,
): Promise<AgentProfileStatus> {
  return fetchJson<AgentProfileStatus>(`${apiBase()}/api/v1/agents/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: authHeaders(token),
    body: JSON.stringify(profile),
  });
}

export async function deleteAgent(token: string, id: string): Promise<void> {
  const resp = await fetch(`${apiBase()}/api/v1/agents/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
}

export async function setAgentRuntimeKey(
  token: string,
  id: string,
  apiKey: string | null,
): Promise<AgentProfileStatus> {
  return fetchJson<AgentProfileStatus>(
    `${apiBase()}/api/v1/agents/${encodeURIComponent(id)}/runtime-key`,
    {
      method: "POST",
      headers: authHeaders(token),
      body: JSON.stringify({ api_key: apiKey }),
    },
  );
}

export async function startAgentLogin(
  token: string,
  id: string,
  project?: string,
): Promise<Session> {
  return fetchJson<Session>(`${apiBase()}/api/v1/agents/${encodeURIComponent(id)}/login`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ project: project ?? null }),
  });
}

export async function fetchGitHubStatus(token: string): Promise<GitHubStatus> {
  return fetchJson<GitHubStatus>(`${apiBase()}/api/v1/integrations/github`, {
    headers: authHeaders(token),
  });
}

export async function fetchGitHubRepos(token: string): Promise<GitHubRepoItem[]> {
  return fetchJson<GitHubRepoItem[]>(`${apiBase()}/api/v1/integrations/github/repos`, {
    headers: authHeaders(token),
  });
}

export async function startGitHubLogin(token: string, project?: string): Promise<Session> {
  return fetchJson<Session>(`${apiBase()}/api/v1/integrations/github/login`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ project: project ?? null }),
  });
}

export async function cloneGitHubRepo(
  token: string,
  repo: string,
  targetDir?: string,
): Promise<Project> {
  return fetchJson<Project>(`${apiBase()}/api/v1/integrations/github/clone`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ repo, target_dir: targetDir ?? null }),
  });
}

export async function fetchMcpServers(token: string): Promise<McpServerConfig[]> {
  return fetchJson<McpServerConfig[]>(`${apiBase()}/api/v1/mcp`, {
    headers: authHeaders(token),
  });
}

export async function upsertMcpServer(
  token: string,
  server: McpServerConfig,
): Promise<McpServerConfig> {
  return fetchJson<McpServerConfig>(`${apiBase()}/api/v1/mcp/${encodeURIComponent(server.id)}`, {
    method: "PUT",
    headers: authHeaders(token),
    body: JSON.stringify(server),
  });
}

export async function deleteMcpServer(token: string, id: string): Promise<void> {
  const resp = await fetch(`${apiBase()}/api/v1/mcp/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
}

export async function fetchPlugins(token: string): Promise<PluginConfig[]> {
  return fetchJson<PluginConfig[]>(`${apiBase()}/api/v1/plugins`, {
    headers: authHeaders(token),
  });
}

export async function upsertPlugin(token: string, plugin: PluginConfig): Promise<PluginConfig> {
  return fetchJson<PluginConfig>(`${apiBase()}/api/v1/plugins/${encodeURIComponent(plugin.id)}`, {
    method: "PUT",
    headers: authHeaders(token),
    body: JSON.stringify(plugin),
  });
}

export async function deletePlugin(token: string, id: string): Promise<void> {
  const resp = await fetch(`${apiBase()}/api/v1/plugins/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
}

export function connectEventsSocket(
  token: string,
  onEvent: (event: DaemonEventFrame) => void,
  onStatus: (connected: boolean) => void,
  onResync?: () => void,
): () => void {
  let disposed = false;
  let activeWs: WebSocket | null = null;
  let retryTimer: number | null = null;
  let retryDelay = 500;

  const connect = () => {
    if (disposed) {
      return;
    }
    const ws = new WebSocket(`${wsBase()}/api/v1/ws`);
    activeWs = ws;

    ws.addEventListener("open", () => {
      ws.send(JSON.stringify({ type: "auth", token }));
    });

    ws.addEventListener("message", (ev) => {
      try {
        const parsed = JSON.parse(String(ev.data)) as DaemonEventFrame;
        if (parsed.type === "hello") {
          retryDelay = 500;
          onStatus(true);
          onResync?.();
        }
        onEvent(parsed);
      } catch {
        // Abaikan frame non-JSON
      }
    });

    const scheduleReconnect = () => {
      onStatus(false);
      if (disposed || retryTimer !== null) {
        return;
      }
      const delay = retryDelay;
      retryDelay = Math.min(8000, retryDelay * 2);
      retryTimer = window.setTimeout(() => {
        retryTimer = null;
        connect();
      }, delay);
    };

    ws.addEventListener("close", scheduleReconnect);
    ws.addEventListener("error", scheduleReconnect);
  };

  connect();

  return () => {
    disposed = true;
    if (retryTimer !== null) {
      window.clearTimeout(retryTimer);
    }
    activeWs?.close();
  };
}

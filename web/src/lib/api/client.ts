import type {
  ApprovalRequest,
  MergeCheckResult,
  PipelineArenaResponse,
  PipelineRun,
  Project,
  Session,
  SessionDetail,
  SessionDiff,
} from "./types";

declare global {
  interface Window {
    __LATCH_API_BASE__?: string;
    __LATCH_TOKEN__?: string;
  }
}

function apiBase(): string {
  return window.__LATCH_API_BASE__ ?? "";
}

function wsBase(): string {
  const base = apiBase();
  if (base.startsWith("http://")) {
    return `ws://${base.slice("http://".length)}`;
  }
  if (base.startsWith("https://")) {
    return `wss://${base.slice("https://".length)}`;
  }
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}`;
}

function authHeaders(token: string): HeadersInit {
  return {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
}

export async function fetchProjects(token: string): Promise<Project[]> {
  const resp = await fetch(`${apiBase()}/api/v1/projects`, { headers: authHeaders(token) });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as Project[];
}

export async function addProject(
  token: string,
  repoPath: string,
  name?: string,
): Promise<Project> {
  const resp = await fetch(`${apiBase()}/api/v1/projects`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ repo_path: repoPath, name: name || null, default_branch: null }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as Project;
}

export async function createSession(
  token: string,
  params: { project: string; profile: string; task: string; base_ref?: string },
): Promise<Session> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({
      project: params.project,
      profile: params.profile,
      task: params.task,
      base_ref: params.base_ref ?? null,
    }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as Session;
}

export async function fetchSessions(token: string): Promise<Session[]> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions`, { headers: authHeaders(token) });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as Session[];
}

export async function fetchSessionDetail(token: string, id: string): Promise<SessionDetail> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}`, {
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as SessionDetail;
}

export async function fetchSessionDiff(token: string, id: string): Promise<SessionDiff> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/diff`, {
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as SessionDiff;
}

export async function checkSessionMerge(token: string, id: string): Promise<MergeCheckResult> {
  const resp = await fetch(`${apiBase()}/api/v1/sessions/${encodeURIComponent(id)}/merge-check`, {
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as MergeCheckResult;
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

export async function fetchPendingApprovals(token: string): Promise<ApprovalRequest[]> {
  const resp = await fetch(`${apiBase()}/api/v1/approvals?status=pending`, {
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as ApprovalRequest[];
}

export async function decideApproval(
  token: string,
  id: string,
  decision: "approved" | "denied",
): Promise<ApprovalRequest> {
  const resp = await fetch(`${apiBase()}/api/v1/approvals/${encodeURIComponent(id)}/decision`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ decision, decided_by: "web" }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as ApprovalRequest;
}

export async function fetchPipelines(token: string): Promise<PipelineRun[]> {
  const resp = await fetch(`${apiBase()}/api/v1/pipelines`, { headers: authHeaders(token) });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as PipelineRun[];
}

export async function fetchPipelineArena(
  token: string,
  id: string,
): Promise<PipelineArenaResponse> {
  const resp = await fetch(`${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/arena`, {
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as PipelineArenaResponse;
}

export async function selectPipelineArenaWinner(
  token: string,
  id: string,
  stepId: string,
  sessionId: string,
): Promise<PipelineRun> {
  const resp = await fetch(`${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/arena/select`, {
    method: "POST",
    headers: authHeaders(token),
    body: JSON.stringify({ step_id: stepId, session_id: sessionId }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as PipelineRun;
}

export async function cancelPipeline(token: string, id: string): Promise<PipelineRun> {
  const resp = await fetch(`${apiBase()}/api/v1/pipelines/${encodeURIComponent(id)}/cancel`, {
    method: "POST",
    headers: authHeaders(token),
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  return (await resp.json()) as PipelineRun;
}

export function connectEventsSocket(
  token: string,
  onEvent: (event: { type: string; session_id?: string }) => void,
  onStatus: (connected: boolean) => void,
): () => void {
  const ws = new WebSocket(`${wsBase()}/api/v1/ws`);

  ws.addEventListener("open", () => {
    // Autentikasi lewat pesan pertama (tidak pernah di query string).
    ws.send(JSON.stringify({ type: "auth", token }));
  });

  ws.addEventListener("message", (ev) => {
    try {
      const parsed = JSON.parse(String(ev.data)) as { type: string; session_id?: string };
      if (parsed.type === "hello") {
        onStatus(true);
      }
      onEvent(parsed);
    } catch {
      // Abaikan frame non-JSON
    }
  });

  ws.addEventListener("close", () => onStatus(false));
  ws.addEventListener("error", () => onStatus(false));

  return () => {
    ws.close();
  };
}

export interface PtyConnection {
  sendInput: (data: string) => void;
  close: () => void;
}

export function connectPtySocket(
  token: string,
  sessionId: string,
  onOutput: (chunk: string) => void,
  onState: (connected: boolean) => void,
): PtyConnection {
  const ws = new WebSocket(`${wsBase()}/api/v1/sessions/${encodeURIComponent(sessionId)}/pty`);
  ws.binaryType = "arraybuffer";
  const decoder = new TextDecoder();

  ws.addEventListener("open", () => {
    ws.send(JSON.stringify({ type: "auth", token }));
    onState(true);
  });

  ws.addEventListener("message", (ev) => {
    if (ev.data instanceof ArrayBuffer) {
      onOutput(decoder.decode(new Uint8Array(ev.data), { stream: true }));
    } else if (typeof ev.data === "string") {
      try {
        const parsed = JSON.parse(ev.data) as { type?: string; bytes?: number };
        if (parsed.type === "dropped" && parsed.bytes) {
          onOutput(`\n[...${parsed.bytes} bytes dropped...]\n`);
        }
      } catch {
        onOutput(ev.data);
      }
    }
  });

  ws.addEventListener("close", () => onState(false));
  ws.addEventListener("error", () => onState(false));

  return {
    sendInput: (data: string) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    },
    close: () => ws.close(),
  };
}

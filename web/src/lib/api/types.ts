// Kontrak tipe API Latch Workbench disinkronkan dari model `latch-core` (`crates/core/src/model.rs`).

export type SessionStatus =
  | "running"
  | "paused"
  | "waiting_approval"
  | "hibernated"
  | "completed"
  | "tripped"
  | "failed";

export type ProxyMode = "injected" | "bypass";

export interface Project {
  id: string;
  name: string;
  repo_path: string;
  default_branch: string;
  created_at: number;
}

export interface Session {
  id: string;
  project_id: string;
  agent_profile: string;
  worktree_path: string;
  branch_name: string;
  status: SessionStatus;
  pid: number | null;
  started_at: number;
  ended_at: number | null;
  task: string | null;
  proxy_mode: ProxyMode;
  sandbox_provider: string;
}

export interface DaemonHealth {
  status: string;
  version: string;
  sandbox: string;
  pid: number;
  active_sessions: number;
}

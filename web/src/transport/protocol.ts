// Kontrak tipe API Latch IDE disinkronkan dari `crates/core/src/model.rs` dan `crates/core/src/protocol.rs`.

export type SessionStatus =
  | "running"
  | "paused"
  | "waiting_approval"
  | "hibernated"
  | "completed"
  | "tripped"
  | "failed";

export type UiStatus = "needs_you" | "working" | "error" | "done" | "idle";

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
  pipeline_run_id: string | null;
  step_id: string | null;
  exit_code: number | null;
  trip_reason: string | null;
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

export interface SessionTelemetrySummary {
  session_id: string;
  request_count: number;
  prompt_tokens: number;
  completion_tokens: number;
  estimated_cost_usd: number;
  error_count: number;
}

export interface TelemetryRecord {
  id: number | null;
  session_id: string;
  provider: string;
  model: string | null;
  prompt_tokens: number;
  completion_tokens: number;
  estimated_cost_usd: number;
  http_status: number | null;
  latency_ms: number | null;
  ttfb_ms: number | null;
  usage_source: "reported" | "estimated";
  error_kind: string | null;
  request_fingerprint: string | null;
  timestamp: number;
}

export interface SessionDetail extends Session {
  telemetry: SessionTelemetrySummary;
}

export interface DiffFileStat {
  path: string;
  old_path: string | null;
  status: "added" | "modified" | "deleted" | "renamed";
  additions: number;
  deletions: number;
}

export interface SessionDiff {
  base_branch: string;
  files: DiffFileStat[];
  patch: string;
  total_patch_bytes: number;
  truncated: boolean;
}

export interface MergeCheckResult {
  clean: boolean;
  conflicts: string[];
}

export interface ScreenSnapshot {
  session_id: string;
  cols: number;
  rows: number;
  text: string;
  ansi_bytes: number[];
}

export interface ApprovalRequest {
  id: string;
  session_id: string;
  action_type: "fs_write" | "shell_exec" | "net_egress";
  payload_summary: string;
  decision: "pending" | "approved" | "denied";
  decided_by: string | null;
  decided_at: number | null;
  created_at: number;
  expires_at: number | null;
}

export type PipelineStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "tripped"
  | "cancelled";

export interface PipelineRun {
  id: string;
  project_id: string;
  workflow_name: string;
  dag_state_json: string;
  status: PipelineStatus;
  created_at: number;
  updated_at: number | null;
  task: string | null;
  total_cost_usd: number;
}

export interface ArenaCandidate {
  profile: string;
  session_id: string | null;
  passed: boolean;
  duration_ms: number;
  diff_lines: number;
  cost_usd: number;
  output: string;
}

export interface PipelineArenaStep {
  step_id: string;
  status: string;
  winner: string | null;
  candidates: ArenaCandidate[];
}

export interface PipelineArenaResponse {
  pipeline_id: string;
  workflow_name: string;
  status: PipelineStatus;
  steps: PipelineArenaStep[];
}

export interface GcReport {
  dry_run: boolean;
  telemetry_deleted: number;
  audit_deleted: number;
  tmp_dirs_removed: number;
  log_files_removed: number;
  bytes_freed: number;
}

export interface DaemonEventFrame {
  seq?: number;
  type: string;
  session_id?: string;
  pipeline_id?: string;
  status?: string;
  approval?: ApprovalRequest;
}

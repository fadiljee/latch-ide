-- Migrasi awal Latch sesuai docs/DATA_MODEL.md §4.
-- Catatan urutan: pipeline_runs dibuat sebelum sessions karena FK pipeline_run_id.

CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at INTEGER NOT NULL
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  repo_path TEXT NOT NULL,
  default_branch TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE pipeline_runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  workflow_name TEXT NOT NULL,
  dag_state_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN
    ('pending','running','succeeded','failed','tripped','cancelled')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER,
  task TEXT,
  total_cost_usd REAL NOT NULL DEFAULT 0
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  agent_profile TEXT NOT NULL,
  worktree_path TEXT NOT NULL,
  branch_name TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN
    ('running','paused','waiting_approval','hibernated','completed','tripped','failed')),
  pid INTEGER,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  task TEXT,
  pipeline_run_id TEXT REFERENCES pipeline_runs(id) ON DELETE SET NULL,
  step_id TEXT,
  exit_code INTEGER,
  trip_reason TEXT,
  proxy_mode TEXT NOT NULL DEFAULT 'injected' CHECK (proxy_mode IN ('injected','bypass')),
  sandbox_provider TEXT NOT NULL DEFAULT 'noop'
);
CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_sessions_status ON sessions(status);

CREATE TABLE proxy_telemetry (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  provider TEXT NOT NULL,
  model TEXT,
  prompt_tokens INTEGER NOT NULL DEFAULT 0,
  completion_tokens INTEGER NOT NULL DEFAULT 0,
  estimated_cost_usd REAL NOT NULL DEFAULT 0,
  http_status INTEGER,
  latency_ms INTEGER,
  ttfb_ms INTEGER,
  usage_source TEXT NOT NULL DEFAULT 'reported' CHECK (usage_source IN ('reported','estimated')),
  error_kind TEXT,
  request_fingerprint TEXT,
  timestamp INTEGER NOT NULL
);
CREATE INDEX idx_telemetry_session_time ON proxy_telemetry(session_id, timestamp);

CREATE TABLE approval_requests (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  action_type TEXT NOT NULL CHECK (action_type IN ('fs_write','shell_exec','net_egress')),
  payload_summary TEXT NOT NULL,
  decision TEXT NOT NULL DEFAULT 'pending' CHECK (decision IN ('pending','approved','denied')),
  decided_by TEXT,
  decided_at INTEGER,
  created_at INTEGER NOT NULL,
  expires_at INTEGER
);
CREATE INDEX idx_approvals_pending ON approval_requests(decision) WHERE decision = 'pending';

CREATE TABLE audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT,
  kind TEXT NOT NULL,
  detail TEXT NOT NULL,
  timestamp INTEGER NOT NULL
);

CREATE TABLE session_snapshots (
  session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
  screen_blob BLOB,
  worktree_head TEXT,
  git_status_json TEXT,
  context_summary TEXT,
  created_at INTEGER NOT NULL
);

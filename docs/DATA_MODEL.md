# Model Data (SQLite)

Basis data: SQLite lokal melalui `rusqlite` (`bundled`). File: `~/.local/state/latch/latch.db`.

## 1. Prinsip

- Semua akses lewat crate `store`; klien tidak menyentuh DB langsung.
- Mode: `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;`
- ID: UUID v7 (TEXT). Waktu: integer epoch **milidetik** UTC.
- Enum disimpan sebagai TEXT dengan `CHECK`.
- **Tidak menyimpan rahasia:** tidak ada kolom untuk API key, header `Authorization`, atau isi prompt penuh (NFR-05). `payload_summary` harus sudah disaring/dipotong.
- Migrasi: berkas SQL berurutan di `crates/store/migrations/NNNN_nama.sql`, dicatat di `schema_migrations`. Jangan mengedit migrasi yang sudah dirilis.

## 2. Entitas Inti (dari blueprint)

### 2.1 `projects`

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | TEXT PK | UUID v7 |
| `name` | TEXT UNIQUE NOT NULL | Nama pendek proyek |
| `repo_path` | TEXT NOT NULL | Path absolut repositori utama |
| `default_branch` | TEXT NOT NULL | Branch tujuan merge, mis. `main` |
| `created_at` | INTEGER NOT NULL | epoch ms |

### 2.2 `sessions`

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | TEXT PK | UUID v7 |
| `project_id` | TEXT FK → projects | |
| `agent_profile` | TEXT NOT NULL | Nama profil di `agents.toml` |
| `worktree_path` | TEXT NOT NULL | Path absolut worktree |
| `branch_name` | TEXT NOT NULL | Mis. `latch/my-repo/a1b2c3` |
| `status` | TEXT NOT NULL | `running`, `paused`, `waiting_approval`, `hibernated`, `completed`, `tripped`, `failed` |
| `pid` | INTEGER NULL | PID proses agen (NULL bila tidak hidup) |
| `started_at` | INTEGER NOT NULL | |
| `ended_at` | INTEGER NULL | |

Kolom tambahan yang diusulkan (di luar blueprint, perlu persetujuan):

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `task` | TEXT | Deskripsi tugas (dipotong, tanpa rahasia) |
| `pipeline_run_id` | TEXT NULL FK | Bila sesi bagian dari pipeline |
| `step_id` | TEXT NULL | Id node pipeline |
| `exit_code` | INTEGER NULL | |
| `trip_reason` | TEXT NULL | `budget`, `tokens`, `error_loop`, `file_loop` |
| `proxy_mode` | TEXT | `injected` atau `bypass` |
| `sandbox_provider` | TEXT | `landlock` atau `noop` |

### 2.3 `proxy_telemetry`

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | INTEGER PK AUTOINCREMENT | |
| `session_id` | TEXT FK → sessions | |
| `provider` | TEXT | `openai`, `anthropic`, `google`, ... |
| `model` | TEXT | |
| `prompt_tokens` | INTEGER | |
| `completion_tokens` | INTEGER | |
| `estimated_cost_usd` | REAL | Dari `pricing.toml` |
| `http_status` | INTEGER | |
| `latency_ms` | INTEGER | Waktu total (dan opsional TTFB di kolom tambahan) |
| `timestamp` | INTEGER | epoch ms |

Kolom tambahan diusulkan: `ttfb_ms INTEGER`, `usage_source TEXT` (`reported` | `estimated`), `error_kind TEXT NULL`, `request_fingerprint TEXT NULL` (hash untuk deteksi loop, bukan isi).

### 2.4 `approval_requests`

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | TEXT PK | |
| `session_id` | TEXT FK | |
| `action_type` | TEXT | `fs_write`, `shell_exec`, `net_egress` |
| `payload_summary` | TEXT | Ringkasan aksi (disaring, maks 1 KB) |
| `decision` | TEXT | `pending`, `approved`, `denied` |
| `decided_by` | TEXT NULL | `tui`, `web:<client-label>`, `timeout`, `policy` |
| `decided_at` | INTEGER NULL | |

Kolom tambahan diusulkan: `created_at INTEGER NOT NULL`, `expires_at INTEGER NULL`.

### 2.5 `pipeline_runs`

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | TEXT PK | |
| `project_id` | TEXT FK | |
| `workflow_name` | TEXT | |
| `dag_state_json` | TEXT | State serial DAG (node, status, retry count, artefak) |
| `status` | TEXT | `pending`, `running`, `succeeded`, `failed`, `tripped`, `cancelled` |
| `created_at` | INTEGER | |

Kolom tambahan diusulkan: `updated_at INTEGER`, `task TEXT`, `total_cost_usd REAL`.

## 3. Tabel Tambahan yang Diusulkan

### 3.1 `schema_migrations`

| Kolom | Tipe |
| --- | --- |
| `version` | INTEGER PK |
| `name` | TEXT |
| `applied_at` | INTEGER |

### 3.2 `audit_log`

Dibutuhkan oleh acceptance criteria sandbox ("tercatat di log audit").

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `id` | INTEGER PK AUTOINCREMENT | |
| `session_id` | TEXT NULL | |
| `kind` | TEXT | `sandbox_denied`, `approval`, `trip`, `auth_failure`, `merge`, `discard`, `config_change` |
| `detail` | TEXT | Ringkasan (tanpa rahasia) |
| `timestamp` | INTEGER | |

> Catatan: Landlock tidak memberi notifikasi penolakan ke proses induk. Entri `sandbox_denied` diperoleh dari deteksi tidak langsung (mis. `strace`-less: parse error `EACCES` pada output, atau mode pengujian khusus). Lihat SECURITY §Audit.

### 3.3 `session_snapshots`

Untuk hibernasi (FR-04).

| Kolom | Tipe | Keterangan |
| --- | --- | --- |
| `session_id` | TEXT PK FK | |
| `screen_blob` | BLOB | Snapshot layar `vt100` terkompresi |
| `worktree_head` | TEXT | SHA HEAD saat hibernasi |
| `git_status_json` | TEXT | Ringkasan `git status` |
| `context_summary` | TEXT NULL | Ringkasan konteks untuk prompt "lanjutkan" |
| `created_at` | INTEGER | |

### 3.4 `session_events` (opsional)

Linimasa event sesi (transisi status, pause/resume, kill) untuk debugging dan UI riwayat.

## 4. DDL Referensi (migrasi 0001)

```sql
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
```

> `sessions.pipeline_run_id` merujuk `pipeline_runs`, jadi urutan pembuatan tabel di migrasi nyata harus menempatkan `pipeline_runs` sebelum `sessions` (atau memakai `ALTER TABLE ... ADD COLUMN` setelahnya).

## 5. Status Sesi & Transisi

| Dari | Ke | Pemicu |
| --- | --- | --- |
| (baru) | `running` | `run` sukses spawn |
| `running` | `paused` | Pengguna pause / `SIGSTOP` |
| `paused` | `running` | Resume / `SIGCONT` |
| `running` | `waiting_approval` | Aksi berisiko menunggu keputusan |
| `waiting_approval` | `running` | Approve |
| `waiting_approval` | `failed`/(kill) | Deny + agen dihentikan, atau timeout kebijakan |
| `running`/`paused`/`waiting_approval` | `tripped` | Circuit breaker |
| `running` | `hibernated` | Idle melewati ambang |
| `hibernated` | `running` | Resume |
| `running` | `completed` | Proses keluar dengan kode 0 |
| `running`/`paused` | `failed` | Exit non-nol, crash, OOM |

Status akhir: `completed`, `tripped`, `failed`. Daemon menyimpan `ended_at` pada status akhir.

## 6. Rekonsiliasi Saat Startup (NFR-06)

1. Untuk setiap sesi `running`/`paused`/`waiting_approval` dengan `pid` yang sudah tidak hidup → set `failed` (atau `hibernated` bila ada snapshot), isi `ended_at`.
2. Bandingkan `git worktree list` tiap proyek dengan sesi aktif; worktree tanpa sesi aktif = yatim → bersihkan (setelah penundaan aman dan log audit).
3. Pipeline `running` dimuat dari `dag_state_json` dan dilanjutkan atau ditandai `failed` sesuai kebijakan.

## 7. Retensi Data

| Data | Bawaan |
| --- | --- |
| `proxy_telemetry` | 30 hari, lalu diagregasi (konfigurasi `retention.telemetry_days`) |
| `audit_log` | 90 hari |
| Log PTY di disk | Rolling, maks total per sesi (mis. 50 MB) |
| `session_snapshots` | Dihapus saat sesi berstatus akhir dan dibersihkan |

## 8. Ukuran & Performa

- Penulisan telemetri di-*batch* (mis. tiap 250 ms atau 50 baris) oleh aktor store agar tidak mengganggu jalur proxy.
- Query UI memakai indeks di atas; agregat biaya per sesi/pipeline dapat dimaterialisasi bila lambat.

# Spesifikasi `workflow.toml`

Status: **Draft**. Parser berada di `crates/core`, eksekutor di `crates/dag` (FR-18, FR-19, FR-20).

## 1. Contoh Lengkap

```toml
[workflow]
name = "feature-with-verification"
max_budget_usd = 2.00

[sandbox]
allow_network = ["api.openai.com", "api.anthropic.com", "generativelanguage.googleapis.com"]
deny_paths = ["~/.ssh", "~/.aws", "**/.env*"]

[circuit_breaker]
max_consecutive_errors = 3
idle_hibernate_minutes = 15

[[steps]]
id = "plan"
type = "agent"
profile = "architect-cli"
prompt = "Susun rencana implementasi terstruktur untuk: {{task}}"

[[steps]]
id = "implement"
type = "fanout"
depends_on = ["plan"]
profiles = ["coder-claude", "coder-codex"]
prompt = "Implementasikan rencana berikut: {{steps.plan.output}}"

[[steps]]
id = "verify"
type = "shell"
depends_on = ["implement"]
command = "cargo check && cargo test"
on_fail_retry_step = "implement"
max_retries = 3
```

## 2. Seksi Tingkat Atas

### 2.1 `[workflow]` (wajib)

| Field | Tipe | Wajib | Bawaan | Keterangan |
| --- | --- | --- | --- | --- |
| `name` | string | ya | - | Pengenal pipeline (`[a-z0-9-_]+`) |
| `max_budget_usd` | float | tidak | tanpa batas | Anggaran total pipeline; melampaui → `tripped` |
| `max_parallel` | int | tidak | 2 | Batas node/sesi paralel |
| `base_ref` | string | tidak | `default_branch` proyek | Titik awal worktree |
| `description` | string | tidak | - | Dokumentasi |

### 2.2 `[sandbox]` (opsional; menimpa/memperketat config global)

| Field | Tipe | Keterangan |
| --- | --- | --- |
| `allow_network` | array string | Domain yang boleh diakses (ditegakkan oleh proxy/egress; lihat SECURITY §Jaringan) |
| `deny_paths` | array string | Path/glob yang diblokir. Mendukung `~` dan `**` |
| `read_paths` | array string | Tambahan jalur baca-saja |
| `write_paths` | array string | Tambahan jalur tulis (di luar worktree) - memerlukan persetujuan eksplisit/konfirmasi |

Aturan penggabungan: workflow **hanya boleh memperketat** kebijakan global. `allow_network` di workflow adalah *irisan* dengan global; `write_paths` di luar worktree ditolak kecuali diizinkan di config global (`allow_workflow_write_paths = true`).

### 2.3 `[circuit_breaker]` (opsional)

| Field | Tipe | Bawaan | Keterangan |
| --- | --- | --- | --- |
| `max_consecutive_errors` | int | 3 | Error/tool-call identik berturut-turut sebelum *trip* |
| `max_tokens` | int | tanpa batas | Total token per sesi |
| `max_session_budget_usd` | float | tanpa batas | Biaya per sesi |
| `max_repeated_file_ops` | int | tanpa batas | Pemanggilan file berulang tak produktif |
| `idle_hibernate_minutes` | int | nonaktif | Idle sebelum hibernasi (FR-04) |
| `action` | `"pause"` \| `"stop"` | `"stop"` | Reaksi saat *trip* |

## 3. `[[steps]]` - Node

Semua node memiliki:

| Field | Tipe | Wajib | Keterangan |
| --- | --- | --- | --- |
| `id` | string | ya | Unik dalam workflow (`[a-z0-9_]+`) |
| `type` | `"agent"` \| `"fanout"` \| `"shell"` | ya | Jenis node |
| `depends_on` | array id | tidak | Dependensi; membentuk DAG |
| `timeout_minutes` | int | tidak | Batas waktu node |
| `max_budget_usd` | float | tidak | Anggaran node |

### 3.1 `type = "agent"`

| Field | Tipe | Wajib | Keterangan |
| --- | --- | --- | --- |
| `profile` | string | ya | Nama profil di `agents.toml` |
| `prompt` | string | ya | Template minijinja |
| `reuse_worktree_of` | id | tidak | Pakai worktree node sebelumnya (estafet pada kode yang sama) |
| `output` | `"transcript"` \| `"diff"` \| `"final_message"` \| `"file:<path>"` | tidak (bawaan `final_message`) | Apa yang menjadi `{{steps.<id>.output}}` |

### 3.2 `type = "fanout"`

| Field | Tipe | Wajib | Keterangan |
| --- | --- | --- | --- |
| `profiles` | array string | ya (≥ 2) | Profil agen/model paralel, masing-masing di worktree sendiri |
| `prompt` | string | ya | Prompt yang sama untuk semua |
| `verify_command` | string | tidak | Perintah verifikasi per kandidat (bila tak ada node `shell` setelahnya) |
| `select` | `"manual"` \| `"cheapest_passing"` \| `"fastest_passing"` \| `"smallest_diff_passing"` | tidak (bawaan `manual`) | Aturan pemenang |

Keluaran: `{{steps.<id>.output}}` = output pemenang; `{{steps.<id>.candidates}}` = daftar kandidat dengan metrik.

Matriks arena (FR-20) memuat per kandidat: profil/model, lolos/gagal test, durasi, jumlah file/baris diff, token, biaya, status sesi.

### 3.3 `type = "shell"`

| Field | Tipe | Wajib | Keterangan |
| --- | --- | --- | --- |
| `command` | string | ya | Dijalankan via `sh -c` di worktree node yang didepend (atau pemenang fan-out) |
| `on_fail_retry_step` | id | tidak | Node yang dijalankan ulang saat perintah gagal |
| `max_retries` | int | tidak (bawaan 0) | Batas retry |
| `env` | tabel | tidak | Variabel lingkungan tambahan (tanpa rahasia) |
| `expect_exit` | int | tidak (0) | Kode keluar sukses |
| `feedback_max_bytes` | int | tidak (16384) | Batas stdout/stderr yang diumpankan balik |

Node `shell` berjalan dalam sandbox yang sama dengan agen (di bawah kebijakan workflow) dan **bukan** lewat proxy LLM.

## 4. Templating (minijinja)

| Variabel | Arti |
| --- | --- |
| `{{task}}` | Tugas dari `--task` |
| `{{steps.<id>.output}}` | Keluaran node sukses |
| `{{steps.<id>.status}}` | `succeeded` \| `failed` \| `skipped` |
| `{{steps.<id>.diff}}` | Diff worktree node |
| `{{retry.attempt}}` | Nomor percobaan saat ini (1..max_retries) |
| `{{retry.stdout}}`, `{{retry.stderr}}` | Keluaran terpotong dari node validasi yang gagal (hanya pada prompt retry) |
| `{{project.name}}`, `{{project.default_branch}}` | Info proyek |

Aturan:
- Render dalam mode **strict**: variabel tidak dikenal = galat validasi.
- Output otomatis dipotong (bawaan 32 KiB) dan ditandai `[...dipotong...]`.
- Isi keluaran agen diperlakukan sebagai **data tak tepercaya**: gunakan escaping yang aman saat dimasukkan ke perintah shell. Template **tidak** boleh diinterpolasikan langsung ke `command` shell; gunakan `env` atau berkas artefak.

### Prompt retry

Ketika `verify` gagal dan `on_fail_retry_step = "implement"`, prompt node `implement` dijalankan ulang dengan tambahan (bisa dikustomisasi lewat `retry_prompt` di node target):

```text
Percobaan sebelumnya gagal pada langkah "verify".
Keluaran (dipotong):
{{retry.stderr}}
Perbaiki masalahnya, lalu pastikan perintah verifikasi lolos.
```

## 5. Aturan Validasi

Dijalankan oleh `latch config check` dan saat `pipeline run`:

1. `workflow.name` valid dan `steps` tidak kosong.
2. `id` unik; `depends_on` merujuk id yang ada.
3. Graf dependensi **asiklik** (toposort `petgraph`). `on_fail_retry_step` bukan edge dependensi, tetapi target harus merupakan leluhur node asal (langsung/tak langsung).
4. `fanout.profiles` ≥ 2 dan semua profil ada di `agents.toml`.
5. `shell.max_retries > 0` hanya valid bila `on_fail_retry_step` ada.
6. Semua variabel template terselesaikan (mis. `steps.plan.output` hanya bila `plan` adalah leluhur).
7. `max_budget_usd` ≥ 0; jumlah anggaran node tidak melebihi anggaran workflow (peringatan).
8. Aturan `[sandbox]` tidak melonggarkan kebijakan global.

## 6. Semantik Eksekusi

1. Urutkan topologis; node tanpa dependensi tak terpenuhi dijalankan paralel (≤ `max_parallel`).
2. Node `latch`: buat sesi (worktree + proses) lalu tunggu selesai/trip.
3. Node `fanout`: N sesi paralel; setelah semua selesai, jalankan verifikasi per kandidat; tentukan pemenang (`select`) atau tunggu pilihan manual.
4. Node `shell`: jalankan di worktree pemenang/dependensi; sukses → lanjut; gagal → bila retry tersedia, kumpulkan stdout/stderr, jalankan ulang node target dengan prompt retry, lalu jalankan `shell` lagi; bila `max_retries` habis → pipeline `failed`.
5. Anggaran: biaya agregat dari `proxy_telemetry`; melampaui `max_budget_usd` → semua sesi di-*trip* dan pipeline `tripped`.
6. Setiap transisi disimpan ke `pipeline_runs.dag_state_json`; saat restart daemon pipeline dilanjutkan dari node belum selesai (node `running` dijalankan ulang).
7. Pembatalan (`cancel`) menghentikan semua sesi turunan dan membersihkan worktree sesuai kebijakan retensi.

## 6.1 Status Node

`pending` → `running` → (`succeeded` | `failed` | `tripped` | `skipped` | `cancelled`); selama retry node target kembali `running` dengan penghitung `attempt`.

## 7. Contoh Tambahan

### 7.1 Estafet sederhana: rencana → kode → lint

```toml
[workflow]
name = "plan-code-lint"
max_budget_usd = 1.50

[[steps]]
id = "plan"
type = "agent"
profile = "architect-cli"
prompt = "Rencana singkat untuk: {{task}}"

[[steps]]
id = "code"
type = "agent"
profile = "coder-claude"
depends_on = ["plan"]
prompt = "Implementasikan: {{steps.plan.output}}"

[[steps]]
id = "lint"
type = "shell"
depends_on = ["code"]
command = "npm run lint"
on_fail_retry_step = "code"
max_retries = 2
```

### 7.2 Arena dua model, pilih termurah yang lolos

```toml
[workflow]
name = "arena"
max_budget_usd = 3.00

[[steps]]
id = "arena"
type = "fanout"
profiles = ["coder-claude", "coder-codex"]
prompt = "{{task}}"
verify_command = "cargo test"
select = "cheapest_passing"
```

## 8. Berkas Terkait

- Profil agen: `~/.config/latch/agents.toml` (lihat [USER_GUIDE](USER_GUIDE.md#profil-agen)).
- Konfigurasi global: `~/.config/latch/config.toml`.
- Dapat disimpan di repo sebagai `.agentd/workflows/*.toml`; berkas di repo diperlakukan **tak tepercaya** (lihat SECURITY §Workflow dari repo).

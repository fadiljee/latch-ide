# Roadmap

Rencana rilis bertahap. Setiap fase memiliki **Definition of Done (DoD)** yang dapat diuji otomatis. Rincian task ada di [TASKS.md](TASKS.md).

## Ringkasan

| Fase | Versi | Tema | Requirement | Perkiraan |
| --- | --- | --- | --- | --- |
| 1 | `v0.1` | Fondasi | FR-01, FR-02, FR-05 s.d. FR-08, FR-21 | 4-6 minggu |
| 2 | `v0.2` | Observabilitas & Web | FR-03, FR-09 s.d. FR-13, FR-22, FR-23 | 5-7 minggu |
| 3 | `v0.3` | Keamanan | FR-14 s.d. FR-17 | 4-6 minggu |
| 4 | `v1.0` | Otomasi penuh | FR-04, FR-18 s.d. FR-20 | 5-7 minggu |

> Perkiraan waktu adalah asumsi untuk 1-2 engineer (atau ekuivalen kerja dengan bantuan agen) dan bukan komitmen; revisi setelah v0.1.

## Fase 1 - Fondasi (`v0.1`)

**Cakupan:** FR-01, FR-02, FR-05, FR-06, FR-07, FR-08, FR-21.

**DoD**
- Daemon Rust + TUI menjalankan ≥ 3 sesi CLI agen paralel di `git worktree` terpisah.
- State tersimpan di SQLite dan pulih setelah restart.
- Diff per worktree tampil; merge/discard berfungsi (dengan simulasi `merge-tree`).
- RAM daemon idle < 25 MB (diukur di CI).
- Binary musl dapat dibangun; `ldd` = statis.

**Risiko fase:** kompleksitas PTY lintas terminal, penanganan resize, hubungan worktree dan hook git.

**Demo keluaran:** rekaman TUI dengan 3 sesi paralel dan merge hasilnya.

## Fase 2 - Observabilitas & Web (`v0.2`)

**Cakupan:** FR-03, FR-09, FR-10, FR-11, FR-12, FR-13, FR-22, FR-23.

**DoD**
- Proxy lokal mencatat token/biaya akurat (uji terhadap mock upstream dan `usage` nyata).
- Circuit Breaker menghentikan simulasi *infinite loop* dalam < 1 detik.
- Web PWA tersemat dapat diakses dari browser laptop dan HP via SSH tunnel/Tailscale dengan bearer token.
- Notifikasi desktop dan webhook terkirim pada event selesai/butuh izin/tripped.
- Latensi tambahan proxy ≤ 2 ms (benchmark).
- Mode bypass proxy per profil berfungsi.

**Risiko fase:** CLI yang menolak proxy (OAuth/pinning), variasi format SSE antar penyedia.

## Fase 3 - Keamanan (`v0.3`)

**Cakupan:** FR-14, FR-15, FR-16, FR-17.

**DoD**
- Proses agen terkunci Landlock di worktree (+ tmp terisolasi).
- Tes otomatis: akses `~/.ssh` gagal dengan `EACCES`; tulis ke luar worktree gagal.
- Toolchain umum (`cargo`, `node`, `python`, `git`) tetap berjalan di dalam sandbox memakai preset baca.
- Alur Approve/Deny bekerja dari TUI dan Web PWA; keputusan tercatat.
- `trait SandboxProvider` dengan `NoopProvider` untuk OS non-Linux; peringatan jelas saat tak terisolasi.

**Risiko fase:** akses `.git` utama untuk worktree, intersepsi aksi untuk approval gate, jaringan per-domain.

## Fase 4 - Otomasi Penuh (`v1.0`)

**Cakupan:** FR-04, FR-18, FR-19, FR-20.

**DoD**
- `workflow.toml` mengeksekusi Planner → Fan-Out Coder → Test Gate → Retry hingga tuntas.
- Fan-out menampilkan tabel perbandingan (test, waktu, diff, biaya) dan opsi pemilihan pemenang.
- Sesi idle terhibernasi dan pulih normal.
- Pipeline pulih setelah restart daemon dari `dag_state_json`.
- Dokumentasi pengguna lengkap, rilis binary untuk Linux x86_64/aarch64 (musl).

## Pasca-v1 (di luar komitmen)

- Sandbox macOS (Seatbelt) dan Windows (AppContainer).
- Pembatasan jaringan tingkat kernel (Landlock ABI ≥ 4 untuk port TCP) dan/atau egress proxy wajib.
- Kebijakan tim (config terpusat), SSO untuk Web.
- Plugin profil agen komunitas dan galeri workflow.
- Mode multi-user pada satu daemon.

## Pemetaan FR → Fase

| FR | Fase | FR | Fase |
| --- | --- | --- | --- |
| FR-01 | v0.1 | FR-13 | v0.2 |
| FR-02 | v0.1 | FR-14 | v0.3 |
| FR-03 | v0.2 | FR-15 | v0.3 |
| FR-04 | v1.0 | FR-16 | v0.3 |
| FR-05 | v0.1 | FR-17 | v0.3 |
| FR-06 | v0.1 | FR-18 | v1.0 |
| FR-07 | v0.1 | FR-19 | v1.0 |
| FR-08 | v0.1 | FR-20 | v1.0 |
| FR-09 | v0.2 | FR-21 | v0.1 |
| FR-10 | v0.2 | FR-22 | v0.2 |
| FR-11 | v0.2 | FR-23 | v0.2 |
| FR-12 | v0.2 | | |

## Gerbang Keluar Setiap Rilis

1. Semua DoD fase lolos di CI.
2. Anggaran NFR (RAM, ukuran, latensi) tidak dilanggar.
3. `CHANGELOG.md` dan dokumen terdampak diperbarui.
4. Tes keamanan (kebocoran kredensial ke DB/log) hijau.
5. Tag rilis dan artefak binary dipublikasikan.

## Jalur Agent IDE

Milestone workbench `I0` s.d. `I5` berjalan paralel/lanjutan setelah fondasi runtime; lihat [ide/IDE_ROADMAP.md](ide/IDE_ROADMAP.md).

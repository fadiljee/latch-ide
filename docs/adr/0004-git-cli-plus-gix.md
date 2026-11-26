# ADR-0004: `git` CLI untuk worktree + `gix` untuk inspeksi

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Isolasi per sesi memakai `git worktree`, `merge-tree`, dan diff/status yang sering dipanggil UI. Dukungan fitur worktree dan `merge-tree --write-tree` di pustaka Rust belum selengkap CLI `git`; spawn proses berulang untuk diff/status mahal.

## Keputusan
- Operasi mutasi dan kompleks (`worktree add/remove`, `merge-tree`, `merge --squash`, `branch -D`) memakai **`git` CLI** via `tokio::process`.
- Inspeksi cepat (status, diff, resolusi ref) memakai **`gix`** (dengan fitur minimal), dengan fallback ke `git` CLI bila gagal.
- `git2`/`libgit2` tidak dipakai (dependensi C, fitur worktree/merge-tree terbatas).
- Versi minimum `git`: 2.38 (untuk `merge-tree --write-tree`).

## Alternatif yang dipertimbangkan
- **Hanya `git` CLI**: sederhana tetapi spawn berulang dan parsing teks.
- **Hanya `gix`/`git2`**: kurang stabil untuk worktree dan merge pada saat ini; menambah ukuran binary.

## Konsekuensi
### Positif
Stabilitas operasi kritis; inspeksi cepat.
### Negatif / risiko
Dua jalur kode yang harus konsisten; ketergantungan pada `git` terpasang; ukuran `gix` harus dipantau (R-20). Akses tulis ke `.git` utama dari worktree membutuhkan keputusan keamanan terpisah (ADR-0008).

## Referensi
FR-05, FR-08, R-02, R-20.

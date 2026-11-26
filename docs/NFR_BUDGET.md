# Anggaran Non-Fungsional & Cara Mengukur

Dokumen ini menerjemahkan NFR-01 s.d. NFR-07 menjadi anggaran terukur, metode pengukuran, dan gerbang CI.

## 1. Ringkasan Anggaran

| ID | Metrik | Batas | Metode ukur | Gerbang CI |
| --- | --- | --- | --- | --- |
| NFR-01a | RSS daemon idle | ≤ 25 MB | `/proc/<pid>/status` (`VmRSS`), juga PSS dari `smaps_rollup` | Gagal bila > 25 MB |
| NFR-01b | Overhead per sesi PTY aktif | ≤ 5 MB | (RSS daemon dengan N sesi − RSS idle) / N, tanpa proses CLI agen | Gagal bila > 5 MB |
| NFR-02 | Tambahan latensi proxy | ≤ 2 ms | Benchmark: upstream mock vs lewat proxy; p50 dan p99 | Gagal bila p99 > 2 ms (pada runner tetap) |
| NFR-03 | Batching output PTY | 8-16 ms | Tes interval frame di klien WS/TUI | Gagal di luar rentang (toleransi jitter) |
| NFR-04 | Ukuran binary musl | < 30 MB | `stat -c %s` pada artefak rilis | Gagal bila ≥ 30 MB |
| NFR-05 | Tidak ada rahasia di DB/log | 0 temuan | Tes kanari (pindai SQLite + log) | Gagal bila ada temuan |
| NFR-06 | Isolasi crash sesi | Daemon tetap hidup | Tes integrasi: `kill -9` proses agen / OOM simulasi | Gagal bila daemon mati |
| NFR-07 | Kompatibilitas OS | Linux ≥ 5.13; build di macOS/Windows | CI matrix build | Gagal bila build rusak |

## 2. Rincian Anggaran RAM

Anggaran 25 MB untuk daemon idle adalah **RSS total** dan ketat. Usulan alokasi awal (harus divalidasi lewat spike R-08):

| Komponen | Perkiraan (MB) | Catatan |
| --- | --- | --- |
| Kode & data biner yang dipetakan (sebagian resident) | 6-9 | Lazy page-in; `strip`, LTO |
| Runtime tokio (2 worker) + stack thread | 1-2 | Kurangi `worker_threads`; stack thread kecil |
| SQLite (cache + koneksi) | 2-3 | `PRAGMA cache_size=-2000` (≈ 2 MB), `mmap_size` kecil |
| axum/hyper + koneksi idle | 1-2 | Batasi koneksi |
| `reqwest` + `rustls` (pool koneksi idle) | 1-3 | Pool kecil, idle timeout pendek |
| Aset web (`rust-embed`) | 0-1 resident | Dipetakan di binary, hanya halaman yang diakses menjadi resident; jangan salin ke heap |
| Alokator & fragmentasi | 1-2 | |
| Cadangan/Margin | 3-5 | |

Aturan praktis:
- Hindari men-*decompress* seluruh aset web ke heap; layani aset terkompresi dengan `Content-Encoding` apa adanya.
- Hindari `String`/`Vec` besar yang hidup lama; gunakan `Bytes`, buffer berbatas, dan streaming.
- Pada v0.1, ukur dengan 0, 1, dan 3 sesi; catat hasilnya di `docs/perf/` setiap rilis.

### 2.1 Anggaran per sesi PTY (≤ 5 MB)

| Komponen | Bawaan | Memori |
| --- | --- | --- |
| Ring buffer output mentah | 1 MiB (dapat diatur 256 KiB-4 MiB) | ~1 MB |
| Layar `vt100` (mis. 220×60 sel + scrollback 500 baris) | - | ~0.5-1.5 MB |
| Kanal berbatas ke pelanggan (TUI/WS) | 64 frame × ≤ 16 KiB | ≤ 1 MB (hanya terisi saat klien terhubung) |
| Thread pembaca PTY (stack) | 256 KiB | ~0.25 MB |
| State sesi, buffer deteksi loop, metadata | - | < 0.5 MB |
| **Total perkiraan** | | **~3-4 MB** |

Skala dapat disetel: menaikkan ring buffer atau scrollback harus memperbarui tabel ini dan lolos tes NFR-01b.

## 3. Latensi Proxy (NFR-02)

**Definisi:** `t_via_proxy − t_langsung` pada jalur *forwarding* (dari klien ke upstream dan balik) dengan upstream lokal tiruan yang mengembalikan respons tetap; diukur sebagai selisih TTFB dan waktu per-chunk untuk SSE.

Metodologi:
1. Server mock (`hyper`) merespons SSE dengan chunk tiap 1 ms.
2. Klien benchmark mengukur 10.000 request: langsung vs lewat proxy.
3. Laporkan p50, p95, p99 selisih; gerbang pada p99 ≤ 2 ms (runner CI stabil; di luar itu laporkan sebagai peringatan).

Praktik implementasi: penerusan byte tanpa buffering penuh, parse di tugas terpisah, tanpa alokasi per chunk, jangan menulis SQLite di jalur panas.

## 4. Batching Output (NFR-03)

- Pembaca PTY mengumpulkan byte dan melepas frame setiap **8-16 ms** (bawaan 12 ms) atau bila buffer ≥ ambang (mis. 16 KiB), mana yang lebih dulu.
- Tes beban: pembangkit mencetak ≥ 10.000 baris/detik; verifikasi (a) interval frame dalam rentang, (b) memori tidak tumbuh tanpa batas, (c) klien lambat tidak memblokir PTY (drop/coalesce).

## 5. Ukuran Binary (NFR-04)

| Komponen | Anggaran awal |
| --- | --- |
| Kode Rust (semua crate + dependensi) | ≤ 18 MB |
| Aset Web (terkompresi, bundel awal ≤ 600 KB) | ≤ 3 MB (lihat [ide/IDE_NFR_BUDGET.md](ide/IDE_NFR_BUDGET.md)) |
| SQLite bundled | ~1.5 MB |
| Margin | ≥ 9 MB |

Teknik: `lto = "thin"` (atau `"fat"` bila perlu), `codegen-units = 1`, `strip = true`, `opt-level = "s"/"z"` bila dapat diterima, matikan fitur dependensi, `cargo bloat`, `cargo tree -d`.

Pemeriksaan CI:

```bash
cargo build --release --target x86_64-unknown-linux-musl
SIZE=$(stat -c %s target/x86_64-unknown-linux-musl/release/latch)
test "$SIZE" -lt $((30*1024*1024)) || { echo "binary terlalu besar: $SIZE"; exit 1; }
ldd target/x86_64-unknown-linux-musl/release/latch 2>&1 | grep -q "not a dynamic" || exit 1
```

## 6. Skrip Pengukuran (usulan di `scripts/`)

```bash
# scripts/bench-rss.sh - ukur RSS daemon idle dan per sesi
set -euo pipefail
latch daemon start --foreground &
PID=$!
sleep 3
idle=$(awk '/VmRSS/ {print $2}' /proc/$PID/status)   # kB
echo "idle_rss_kb=$idle"
test "$idle" -le $((25*1024))

for i in 1 2 3; do latch run --project bench --profile sleeper --task idle; done
sleep 3
with3=$(awk '/VmRSS/ {print $2}' /proc/$PID/status)
per=$(( (with3 - idle) / 3 ))
echo "per_session_kb=$per"
test "$per" -le $((5*1024))
kill $PID
```

`sleeper` adalah profil uji yang menjalankan proses PTY ringan (`sleep`/skrip kecil) sehingga yang diukur hanya overhead daemon.

## 7. Tes Reliabilitas (NFR-06)

1. Jalankan 3 sesi; `kill -9` salah satu proses agen → status `failed`, sesi lain lanjut, daemon hidup.
2. Simulasi OOM (cgroup memori kecil pada satu sesi, bila tersedia) → sama seperti di atas.
3. `kill -9` daemon, start ulang → rekonsiliasi: sesi mati ditandai, worktree yatim dibersihkan, riwayat utuh.

## 8. Kompatibilitas OS (NFR-07)

CI matrix: `ubuntu-latest` (glibc + musl), `ubuntu-24.04-arm` (opsional), `macos-latest`, `windows-latest` (build + tes unit non-sandbox). Tes Landlock hanya di runner Linux dengan kernel ≥ 5.13 (verifikasi `uname -r` dan dukungan ABI; lewati dengan peringatan bila tak tersedia).

## 9. Pelaporan

Setiap rilis memublikasikan tabel di `docs/perf/<versi>.md`: RSS idle, RSS per sesi, ukuran binary, latensi proxy (p50/p99), dan hasil tes batching. Regresi > 10% pada salah satu metrik memerlukan penjelasan dalam PR.

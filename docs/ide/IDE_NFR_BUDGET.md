# Anggaran Non-Fungsional - Agent IDE

Melengkapi [NFR_BUDGET.md](../NFR_BUDGET.md). **Anggaran inti runtime tidak berubah**: daemon idle ≤ 25 MB tanpa klien IDE; 5 MB per sesi PTY. Angka IDE di bawah bersifat tambahan dan harus divalidasi lewat spike.

## 1. Ringkasan

| ID | Metrik | Batas | Metode | Gerbang CI |
| --- | --- | --- | --- | --- |
| NFR-I01 | Ukuran aset web (terkompresi) | ≤ 3 MB total; bundel awal ≤ 600 KB gzip | `size-limit`/visualizer | Gagal bila melewati |
| NFR-I02 | Latensi ketikan editor | < 16 ms p95 (≤ 5.000 baris) | Playwright + Performance API | Peringatan, gagal bila > 32 ms |
| NFR-I03 | Buka file 1 MB | ≤ 200 ms | E2E | Peringatan |
| NFR-I04 | Quick open (50k berkas, indeks hangat) | < 100 ms | Benchmark `search` | Gagal bila > 150 ms |
| NFR-I05 | RAM daemon + layanan IDE | ≤ +15 MB (1 workspace, tanpa LSP) | Skrip RSS | Gagal bila > +15 MB |
| NFR-I06 | Event status → UI | < 500 ms | E2E | Peringatan |
| NFR-I07 | Keamanan jalur fs | 0 lolos | Fuzz + tes | Gagal |
| NFR-I08 | A11y | AA pada komponen inti | axe-core | Gagal pada pelanggaran serius |
| NFR-I09 | Tidak kehilangan draf saat reconnect | 100% | E2E | Gagal |
| NFR-I10 | Matriks browser | Lulus | Playwright multi-browser | Gagal |
| NFR-I11 | Lebar 360 px | Fungsi inti berjalan | Tes visual | Peringatan |
| NFR-I12 | i18n | id+en lengkap | Tes katalog | Gagal pada kunci hilang |

## 2. Anggaran RAM Daemon untuk Layanan IDE

Diukur sebagai selisih RSS dibanding daemon dengan klien IDE tertutup.

| Komponen | Perkiraan (MB) | Catatan |
| --- | --- | --- |
| Indeks nama file (50k berkas) | 3-6 | Arena string; ukur |
| Watcher inotify + state | 0.5-1 | Debounce; watch bertahap |
| Cache hunk/diff + undo (batas) | 1-3 | LRU berbatas |
| Koneksi WS klien IDE (2-3 klien) | 1-2 | Buffer berbatas |
| Pencarian teks aktif (puncak) | 1-3 | Buffer hasil berbatas, dilepas setelah selesai |
| Margin | 2-3 | |
| **Total** | **≈ 9-18** | Target ≤ 15 (lazy: dilepas saat idle) |

Aturan:
- Indeks, watcher, dan cache **dibongkar** setelah `ide.idle_unload_minutes` (bawaan 10) tanpa klien.
- Server LSP berjalan sebagai **proses terpisah**; memorinya tidak dihitung dalam NFR-I05 tetapi diberi batas `ulimit`/cgroup (bawaan 512 MB per server) dan dimatikan saat idle.
- Highlight, diff tampilan, dan seluruh rendering dilakukan di klien.

## 3. Anggaran Frontend (Peramban)

| Metrik | Target |
| --- | --- |
| Memori tab workbench (5 sesi, 1 editor) | ≤ 250 MB (diukur di Chromium) |
| Waktu interaktif (lokal) | < 1.5 s |
| Terminal xterm: 5 sel dengan output 5k baris/dtk total | Tanpa jank (> 30 fps), CPU tab < 40% |
| Render diff 2.000 baris | < 300 ms (virtualisasi) |
| Daftar 10k hasil cari | Scroll mulus (virtualisasi) |

Teknik: virtualisasi daftar, worker untuk diff/highlight berat, hentikan render terminal tak terlihat, `requestIdleCallback`/batching pembaruan store, memoisasi selektif.

## 4. Ukuran Binary
Total binary musl tetap < 30 MB (NFR-04). Aset web ≤ 3 MB terkompresi + kode `fs-service/search/workbench-api` (± 1-2 MB) wajar. Fitur `lsp` dan `native-shell` opsional; ukur tambahan masing-masing dan laporkan.

## 5. Skrip Pengukuran (usulan)

```bash
# scripts/bench-ide-rss.sh
latch daemon start --foreground & PID=$!
sleep 3; base=$(awk '/VmRSS/{print $2}' /proc/$PID/status)
# buka workspace melalui API (token uji) dan lakukan indeks + 1 pencarian
./scripts/ide-open-workspace.sh "$FIXTURE_REPO_50K"
sleep 5; with=$(awk '/VmRSS/{print $2}' /proc/$PID/status)
delta=$((with-base)); echo "ide_delta_kb=$delta"
test "$delta" -le $((15*1024))
```

Fixture repo besar (mis. 50k berkas sintetis) dibuat deterministik oleh skrip pembuat.

## 6. Benchmark Editor
Playwright mengetik 200 karakter pada file 5.000 baris dan mengukur selisih `keydown`→`paint` via `PerformanceObserver`; laporkan p50/p95. Berjalan di runner CI tetap; hasil dibandingkan dengan baseline (regresi > 20% = gagal).

## 7. Pelaporan
`docs/perf/ide-<versi>.md` per rilis: ukuran bundel per chunk, RAM delta, latensi ketik/quick open, memori tab, hasil a11y.

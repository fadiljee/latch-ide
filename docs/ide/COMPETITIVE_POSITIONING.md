# Posisi Kompetitif

> **Peringatan akurasi:** deskripsi Orca dan enowX di bawah diambil dari karakterisasi di blueprint proyek ("isolasi kode ala Orca", "efisiensi daemon/fan-out ala enowX"), bukan dari verifikasi produk terbaru. **Verifikasi fitur dan lisensi masing-masing sebelum dipublikasikan.** Tabel "Pembanding" sengaja memakai kolom "perlu verifikasi".

## 1. Ringkasan Posisi

**Agent IDE** menargetkan pengguna yang menjalankan banyak CLI agen secara paralel dan menginginkan: (1) RAM sangat rendah, (2) isolasi OS nyata, (3) kontrol biaya otomatis, (4) pipeline deterministik, (5) kerja dari HP/remote, dalam satu binary statis.

## 2. Pembanding

| Aspek | Orca (karakterisasi blueprint) | enowX (karakterisasi blueprint) | Agent IDE (target) |
| --- | --- | --- | --- |
| Isolasi per tugas | Git worktree terisolasi | perlu verifikasi | Git worktree + branch + sandbox Landlock |
| Arsitektur | Aplikasi desktop (blueprint menyebut orkestrator berbasis Electron menyedot RAM) | Daemon + fan-out | Daemon Rust + workbench web/PWA + shell webview opsional |
| Konsumsi RAM | Tinggi (menurut blueprint) | Efisien | Target ≤ 25 MB daemon idle |
| Sandbox OS | perlu verifikasi | perlu verifikasi | Landlock (Linux-first), approval gate |
| Telemetri token/biaya | perlu verifikasi | perlu verifikasi | Proxy LLM lokal + circuit breaker |
| Pipeline/DAG | perlu verifikasi | Fan-out | `workflow.toml` DAG + retry gate + arena |
| Remote/HP | perlu verifikasi | perlu verifikasi | PWA via Tailscale/SSH; review & approve dari HP |
| Editor bawaan | perlu verifikasi | perlu verifikasi | Editor ringan (CM6), bukan IDE penuh |

## 3. Pembeda yang Diklaim (harus terbukti dengan benchmark)

1. **Efisiensi:** perbandingan RSS dengan N sesi vs. aplikasi desktop (ukur sendiri, publikasikan metodologi).
2. **Keamanan:** bukti tes sandbox (`~/.ssh` ditolak) dan kanari rahasia.
3. **Perlindungan kuota:** demonstrasi circuit breaker menghentikan loop < 1 s.
4. **Deterministik:** pipeline estafet dengan retry gate yang dapat diulang.
5. **Remote-native:** review/approve dari HP tanpa relay pihak ketiga.

## 4. Apa yang **Bukan** Pembeda
- Kelengkapan editor (tidak bersaing dengan VS Code/JetBrains/Zed).
- Ekosistem ekstensi.
- Dukungan semua OS untuk sandbox pada v1.

## 5. Pernyataan Nilai (draf)

> "Jalankan lima agen coding sekaligus di laptop 8 GB, dengan agen terkunci di worktree-nya, biaya terpantau, dan review langsung dari HP, dalam satu binary Rust."

## 6. Risiko Positioning
- Produk pembanding berkembang cepat; perbarui tabel secara berkala.
- Klaim "lebih hemat RAM" harus memakai konfigurasi setara dan metodologi terbuka.
- Hindari penggunaan merek/logo pihak lain; sebut nama hanya untuk perbandingan faktual.

## 7. Tindakan Verifikasi (sebelum rilis publik)
- [ ] Telaah fitur, arsitektur, lisensi Orca dan enowX versi terbaru.
- [ ] Isi kolom "perlu verifikasi" dengan sumber (tautan, tanggal).
- [ ] Siapkan benchmark RAM reproduktif.
- [ ] Tinjau hukum merek dagang/penamaan.

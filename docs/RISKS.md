# Daftar Risiko & Mitigasi

Skala: **Kemungkinan** (R/S/T) × **Dampak** (R/S/T). Pemilik risiko diisi saat pengerjaan.

## 1. Risiko dari Blueprint

| ID | Risiko | Dampak | Mitigasi | K | D | Fase |
| --- | --- | --- | --- | --- | --- | --- |
| R-01 | **CLI agen menolak proxy lokal** (OAuth tertutup, *certificate pinning*, base URL tak dapat diubah) | Trafik tidak lewat proxy; token/biaya/loop tidak terpantau | Wajibkan FR-13 (bypass per profil) dengan fallback deteksi idle/loop lewat pola output PTY; dokumentasikan matriks kompatibilitas per CLI; uji tiap CLI di CI manual | T | S | v0.2 |
| R-02 | **Konflik Git merge pasca-paralel** | Auto-merge gagal | Simulasi `git merge-tree --write-tree` sebelum merge; blokir auto-merge bila konflik, tampilkan file konflik, opsi agen *conflict resolver*; merge berurutan dengan rebase opsional | S | S | v0.1 |
| R-03 | **Kompleksitas aturan Landlock** (toolchain gagal di sandbox) | Sandbox terlalu ketat → pengguna mematikannya | Preset baca (`/usr`, `/lib`, `/etc/ssl`, cache toolchain) dapat disesuaikan di config global; mode `learn`/`dry-run` yang menampilkan akses yang akan ditolak; dokumentasi per toolchain | T | S | v0.3 |
| R-04 | **Banjir output terminal** | Buffer membengkak, WebSocket macet | Ring buffer per sesi + log rolling di disk; batching 8-16 ms; kanal berbatas; drop/coalesce frame untuk klien lambat | S | S | v0.1 |

## 2. Risiko Tambahan (hasil analisis desain)

| ID | Risiko | Dampak | Mitigasi | K | D | Fase |
| --- | --- | --- | --- | --- | --- | --- |
| R-05 | **Worktree perlu akses tulis `.git` utama** sehingga isolasi melemah | Agen dapat mengubah ref/hook lain | ADR opsi A/B/C (SECURITY §4.3); nonaktifkan hook; cek integritas ref sebelum merge | T | T | v0.3 |
| R-06 | **Approval gate tidak dapat mencegat semua aksi dalam CLI agen** | Rasa aman palsu | Jelaskan cakupan secara eksplisit; kombinasi shim + hook agen + sandbox; jangan memasarkan sebagai "semua aksi" | T | S | v0.3 |
| R-07 | **Pembatasan domain jaringan tidak dapat ditegakkan Landlock** | Exfiltrasi lewat jaringan | Egress proxy kooperatif, Landlock ABI ≥ 4 untuk port, dokumentasi batasan | T | T | v0.3 |
| R-08 | **Anggaran RAM 25 MB sulit dicapai** (SQLite, tokio, axum, aset web, `reqwest`, `gix`) | Gagal NFR-01 | Ukur sejak v0.1; worker tokio sedikit; cache SQLite kecil; aset web dilayani dari memori-map `rust-embed` (bukan dimuat ke heap); `gix` fitur minimal; ambil keputusan dengan data benchmark | S | S | v0.1 |
| R-09 | **Ukuran binary > 30 MB** | Gagal NFR-04 | Anggaran bundle web; `cargo bloat`; `opt-level`, LTO, strip; kompresi aset; hindari dependensi besar | R | S | v0.2 |
| R-10 | **Estimasi biaya/token tidak akurat** (SSE tanpa `usage`, harga berubah, caching/batch) | Circuit breaker salah memicu atau terlambat | Utamakan `usage` dari penyedia; fallback estimasi (ditandai `estimated`); `pricing.toml` dapat diperbarui; uji dengan fixture nyata per penyedia | S | S | v0.2 |
| R-11 | **Format API penyedia berubah** (SSE, header, path) | Parser proxy rusak | Parser toleran dan terisolasi per penyedia; proxy tetap meneruskan byte walau parse gagal (*fail-open* untuk forwarding); telemetri bertanda tak tersedia | S | S | v0.2 |
| R-12 | **Proxy menambah latensi > 2 ms** | Streaming terasa lambat | Forwarding zero-copy sebisa mungkin; parse di tugas terpisah; hindari alokasi per chunk; benchmark berkelanjutan | R | S | v0.2 |
| R-13 | **Kebocoran rahasia ke log/DB** | Pelanggaran NFR-05 | Denylist header pusat, redaksi `tracing`, tes kanari otomatis | R | T | v0.2 |
| R-14 | **PTY lintas platform (ConPTY, perilaku terminal berbeda)** | Bug render, kehilangan input | Fokus Linux v1; tes sintetis; `vt100` untuk normalisasi | S | R | v0.1 |
| R-15 | **Hibernasi tidak dapat memulihkan proses persis** | Ekspektasi pengguna salah | Dokumentasi jelas; gunakan fitur resume bawaan CLI bila ada; ringkasan konteks; opsi `SIGSTOP` | T | S | v1.0 |
| R-16 | **Worktree yatim / disk penuh** | Pemborosan disk, gagal membuat sesi | Rekonsiliasi startup, batas jumlah worktree, `latch gc`, peringatan ruang disk | S | R | v0.1 |
| R-17 | **Repo besar/monorepo** (worktree lambat, disk besar) | Start sesi lambat | Dukungan sparse-checkout, `git worktree add --no-checkout` + sparse; peringatan; dokumentasi | S | S | v1.0 |
| R-18 | **Token Web dicuri** | Kontrol penuh daemon | Token acak, `0600`, rotasi, tidak di URL, cek Origin, peran read-only | R | T | v0.2 |
| R-19 | **SQLite terkunci/korup saat crash** | Kehilangan state | WAL, aktor tunggal penulis, backup periodik, `integrity_check` saat startup | R | S | v0.1 |
| R-20 | **Crate `gix` besar/berubah cepat** | Ukuran dan stabilitas | Batasi fitur; fallback `git` CLI; isolasi di `git-engine` | S | R | v0.1 |
| R-21 | **Alokator musl lambat** | Throughput proxy/PTY turun | Benchmark; evaluasi alokator alternatif hanya bila terbukti perlu | R | R | v0.2 |
| R-22 | **Cakupan terlalu luas untuk tim kecil** | Jadwal molor | Fase ketat dengan DoD; fitur P2 ditunda; bagi task kecil (TASKS.md) | T | S | semua |
| R-23 | **Pelanggaran ToS penyedia lewat proxy/otomasi** | Akun dibatasi | Proxy hanya meneruskan; bypass tersedia; dokumentasikan; pengguna bertanggung jawab atas kebijakan penyedia | R | S | v0.2 |
| R-24 | **Prompt injection dari konten repo/web** | Agen melakukan aksi tak diinginkan | Sandbox membatasi dampak; approval gate; review diff; tidak ada auto-merge/push default | T | S | v0.3 |
| R-25 | **Kelelahan persetujuan (approval fatigue)** | Pengguna menyetujui tanpa membaca | Kebijakan allow/deny per pola; hanya minta untuk aksi benar-benar berisiko; ringkasan jelas | S | S | v0.3 |

## 3. Pelacakan

Perbarui kolom status risiko pada setiap rilis minor (`Terbuka`, `Dimitigasi`, `Diterima`, `Ditutup`). Risiko dengan K×D = Tinggi×Tinggi harus memiliki pemilik dan ADR atau eksperimen (*spike*) sebelum fase terkait dimulai.

## 4. Spike yang Disarankan Lebih Awal

| Spike | Menjawab risiko | Keluaran |
| --- | --- | --- |
| Ukur RSS: tokio + axum + rusqlite + portable-pty minimal | R-08 | Angka dasar anggaran RAM |
| Proxy mock + parser SSE OpenAI/Anthropic | R-10, R-11, R-12 | Prototipe dan benchmark latensi |
| Landlock menjalankan `cargo`, `npm`, `python`, `git` | R-03, R-05 | Preset baca awal + opsi `.git` |
| Uji tiap CLI agen dengan `*_BASE_URL` | R-01 | Matriks kompatibilitas |
| Prototipe PATH shim untuk approval | R-06 | Keputusan ADR approval gate |

# ADR-0007: Proxy LLM via injeksi `*_BASE_URL`

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Untuk menghitung token/biaya dan mendeteksi loop, daemon perlu melihat lalu lintas API LLM dari CLI agen. Pendekatan MITM TLS (sertifikat CA kustom) invasif, rapuh, dan berisiko keamanan.

## Keputusan
Daemon menjalankan **reverse proxy lokal** dan **menyuntik variabel `*_BASE_URL`** (mis. `OPENAI_BASE_URL`, `ANTHROPIC_BASE_URL`) ke env sesi PTY. Sesi diidentifikasi lewat **token sesi pada path** (`/s/<token>/<provider>/...`), bukan kredensial. Proxy meneruskan header otorisasi apa adanya tanpa menyimpannya. CLI yang tidak mendukung base URL kustom, memakai OAuth tertutup, atau *certificate pinning* memakai **mode bypass** per profil (FR-13) dengan deteksi heuristik dari output PTY. Tidak ada MITM TLS.

## Alternatif yang dipertimbangkan
- **MITM TLS dengan CA lokal**: dapat menangkap semua lalu lintas tetapi rapuh, berisiko, dan gagal pada pinning.
- **Hook pustaka/LD_PRELOAD**: tidak portabel, tidak cocok binary statis/Go/Rust.
- **Membaca log CLI**: tidak seragam antar agen.

## Konsekuensi
### Positif
Sederhana, aman, tanpa sertifikat kustom; telemetri akurat untuk CLI yang kompatibel.
### Negatif / risiko
Tidak semua CLI kompatibel (R-01); butuh matriks kompatibilitas dan pemeliharaan parser per penyedia (R-11); tekanan latensi ≤ 2 ms (NFR-02).

## Referensi
FR-09 s.d. FR-13, NFR-02, NFR-05, [ARCHITECTURE.md §4.5](../ARCHITECTURE.md).

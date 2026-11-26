# ADR-0015: Layanan IDE lazy dan LSP sebagai proses terisolasi

- Status: Proposed
- Tanggal: (isi saat disetujui)

## Konteks
Anggaran daemon inti idle ≤ 25 MB tidak boleh rusak oleh fitur IDE (indeks file, watcher, pencarian, LSP). Server bahasa dapat memakai ratusan MB dan menjalankan kode proyek.

## Keputusan
1. Layanan IDE (`fs-service`, `search`, `workbench-api`) dimuat **lazy** saat workspace dibuka dan **dibongkar** setelah idle; dapat dimatikan lewat fitur Cargo/konfigurasi.
2. **LSP dijalankan sebagai proses terpisah** di dalam sandbox per worktree, dengan batas memori dan penutupan saat idle; tidak dibundel; konfigurasi hanya dari config pengguna.
3. Seluruh rendering/highlight di klien; tidak ada highlight sisi server.
4. Anggaran RAM IDE terpisah dari anggaran inti (NFR-I05).

## Alternatif yang dipertimbangkan
- **Selalu aktif**: sederhana, tetapi melanggar anggaran saat tidak dipakai.
- **LSP in-process**: tidak memungkinkan (server eksternal), mengabaikan isolasi.
- **Tanpa LSP**: lebih ringan; dipertahankan sebagai default (nonaktif sampai diaktifkan).

## Konsekuensi
### Positif
Anggaran inti aman; keamanan LSP lebih baik.
### Negatif / risiko
Latensi pembukaan pertama (indeks dingin); kompleksitas siklus hidup; perlu pengukuran disiplin.

## Referensi
NFR-I05, FR-I80, IR-08, IR-15.

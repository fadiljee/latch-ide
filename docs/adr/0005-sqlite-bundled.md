# ADR-0005: SQLite `bundled` sebagai penyimpanan state

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Daemon perlu persistensi (proyek, sesi, telemetri, approval, pipeline) yang pulih setelah restart, tanpa server database terpisah dan tanpa dependensi sistem.

## Keputusan
Gunakan **SQLite** melalui `rusqlite` dengan fitur `bundled`, mode WAL, akses melalui satu aktor penulis (thread khusus) di crate `store`. Migrasi berurutan berbentuk berkas SQL. Tidak ada rahasia yang disimpan (NFR-05).

## Alternatif yang dipertimbangkan
- **sled/redb/RocksDB**: tidak sekaya query/migrasi; manfaat terbatas.
- **PostgreSQL**: dependensi eksternal; berlebihan.
- **File JSON**: rentan korupsi, tak ada transaksi.
- **`sqlx`**: menambah ukuran dan kompleksitas async; SQLite sinkron cukup dengan `spawn_blocking`/aktor.

## Konsekuensi
### Positif
Satu berkas, transaksional, tanpa instalasi, pulih saat crash.
### Negatif / risiko
Menambah ±1-2 MB ke binary; perlu disiplin agar penulisan tidak memblokir runtime; cache perlu dibatasi demi RAM (R-08).

## Referensi
FR-02, NFR-01, NFR-05, NFR-06, [DATA_MODEL.md](../DATA_MODEL.md).

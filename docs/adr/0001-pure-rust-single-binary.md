# ADR-0001: Rust murni, single static binary

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Orkestrator berbasis Electron memakai RAM besar saat banyak sesi berjalan. Target produk: RAM idle ≤ 25 MB, binary < 30 MB, tanpa dependensi runtime eksternal (Node.js, Electron, Docker), dan mudah dipasang di VPS/laptop terbatas.

## Keputusan
Daemon, CLI, dan TUI ditulis 100% Rust dalam **satu binary** dengan subcommand (`daemon`, `tui`, `run`, ...). Antarmuka web adalah aset statis yang ditanam (ADR-0006). Build rilis Linux memakai target `musl` dengan `rustls` dan `rusqlite` `bundled`.

## Alternatif yang dipertimbangkan
- **Electron/Tauri**: RAM dan ukuran besar; bertentangan dengan tujuan produk.
- **Go**: single binary mudah, tetapi runtime/GC memakai RAM lebih besar dan ekosistem sandbox/PTY/TUI berbeda; Rust dipilih untuk kontrol memori dan `ratatui`.
- **Banyak binary terpisah (daemon + klien)**: lebih fleksibel tetapi distribusi lebih rumit; satu binary menyederhanakan instalasi.

## Konsekuensi
### Positif
Distribusi sederhana, RAM rendah, tanpa GC, keamanan memori.
### Negatif / risiko
Waktu kompilasi lebih lama; kurva belajar Rust; ukuran binary harus dijaga ketat (R-09); alokator musl dapat lebih lambat (R-21); `unsafe` terbatas pada `sandbox`/`pty-pool`.

## Referensi
NFR-01, NFR-04, FR-01; [TECH_STACK.md](../TECH_STACK.md).

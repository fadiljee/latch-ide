# Contributing

Terima kasih sudah tertarik berkontribusi pada `Latch (`latch-ide`)`.

## Prasyarat

- Rust stable terbaru (`rustup`), plus target `x86_64-unknown-linux-musl` untuk build statis
- `git` ≥ 2.38 (butuh `git merge-tree --write-tree`)
- Node.js ≥ 20 dan npm (hanya untuk mengembangkan `web/`; tidak dibutuhkan saat runtime)
- Opsional: `mold` atau `lld`, `cargo-nextest`, `cargo-bloat`, `cargo-deny`
- Linux kernel ≥ 5.13 untuk menguji sandbox Landlock penuh

## Setup

```bash
git clone <repo-url> Latch (`latch-ide`) && cd Latch (`latch-ide`)

# Linker cepat (opsional) - .cargo/config.toml
# [target.x86_64-unknown-linux-gnu]
# linker = "clang"
# rustflags = ["-C", "link-arg=-fuse-ld=mold"]

cargo build
cargo nextest run --workspace

# Web (opsional)
cd web && npm ci && npm run dev
```

## Alur kerja

1. Pilih task dari `docs/TASKS.md` atau buat issue untuk diskusi.
2. Buat branch `feat/<crate>-<ringkas>`.
3. Tulis tes terlebih dahulu bila memungkinkan.
4. Jalankan:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

5. Perbarui dokumen dan `CHANGELOG.md` (bagian `Unreleased`).
6. Buka Pull Request memakai templat di bawah.

## Gaya kode

Ikuti bagian 4 dan 6 di [`AGENTS.md`](AGENTS.md). Ringkasnya: tanpa `unwrap()` di produksi, `tracing` bukan `println!`, tidak ada kebocoran kredensial di log, dan jaga dependensi tetap ramping.

## Pesan commit

[Conventional Commits](https://www.conventionalcommits.org/): `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`. Scope = nama crate, misalnya `feat(dag): add retry gate`.

## Templat Pull Request

```markdown
## Ringkasan
<apa dan kenapa>

## Requirement terkait
FR-xx / NFR-xx / T-xx

## Cara uji
<perintah / langkah>

## Dampak performa & ukuran
RSS idle: ... MB | ukuran binary: ... MB | catatan: ...

## Checklist
- [ ] fmt, clippy, tes lolos
- [ ] Dokumen diperbarui
- [ ] CHANGELOG diperbarui
- [ ] Tidak ada rahasia/log sensitif
```

## Melaporkan celah keamanan

Jangan membuka issue publik. Kirim laporan privat ke kontak keamanan yang tercantum di `docs/SECURITY.md` (bagian *Pelaporan kerentanan*).

## Kode etik

Bersikap sopan dan konstruktif. Kritik kode, bukan orangnya.

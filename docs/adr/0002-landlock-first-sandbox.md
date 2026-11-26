# ADR-0002: Sandbox Linux-first dengan Landlock

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Agen otonom berisiko membaca kredensial atau merusak file di luar proyek. Dibutuhkan isolasi tanpa overhead kontainer, tanpa daemon tambahan, dan tanpa privilese root.

## Keputusan
Gunakan **Landlock** (kernel ≥ 5.13) sebagai sandbox filesystem pada v1, dibungkus di balik `trait SandboxProvider` (FR-16) sehingga macOS (Seatbelt) dan Windows (AppContainer) dapat ditambahkan nanti. OS lain memakai `NoopProvider` dengan peringatan jelas. Pembatasan jaringan per domain **tidak** diklaim dapat ditegakkan Landlock (lihat SECURITY §6).

## Alternatif yang dipertimbangkan
- **Docker/Podman**: dependensi eksternal, overhead, butuh daemon/privilese.
- **bubblewrap/firejail**: dependensi eksternal dan perbedaan distro.
- **seccomp saja**: tidak membatasi akses path secara ergonomis.
- **User namespaces + mount namespaces**: kuat tetapi kompleks dan sering dinonaktifkan di beberapa distro.

## Konsekuensi
### Positif
Tanpa root, tanpa dependensi, overhead mendekati nol, bawaan kernel; abstraksi memudahkan perluasan OS.
### Negatif / risiko
Hanya Linux pada v1; aturan allowlist rumit untuk toolchain (R-03); tidak bisa mengecualikan sub-path dari induk yang diizinkan; tidak ada filter domain; tidak ada notifikasi penolakan ke induk (audit terbatas); perlu penanganan akses `.git` (ADR-0008).

## Referensi
FR-14 s.d. FR-16, NFR-07, [SECURITY.md](../SECURITY.md).

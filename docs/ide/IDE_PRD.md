# Product Requirements Document - Agent IDE

**Produk:** Agent IDE (Workbench) di atas `Latch (`latch-ide`)`
**Status:** Draft v1
**Terkait:** [IDE_REQUIREMENTS](IDE_REQUIREMENTS.md) · [IDE_UX_SPEC](IDE_UX_SPEC.md) · [IDE_ARCHITECTURE](IDE_ARCHITECTURE.md) · [SCOPE_CHANGES](SCOPE_CHANGES.md)

## 1. Ringkasan

Agent IDE adalah **workbench berbasis web** yang disajikan oleh daemon Rust, tempat pengembang **memerintah banyak agen coding paralel, memantau, meninjau diff per hunk, mengoreksi, dan menggabungkan hasil**, dengan editor, terminal, pencarian, dan panel git yang ringan. Ia mewarisi isolasi worktree, sandbox, telemetri biaya, dan pipeline dari runtime.

Rumusan singkat: *"Mission control untuk agen coding, dengan editor secukupnya untuk menyelesaikan pekerjaan."*

## 2. Masalah

| # | Masalah | Dampak |
| --- | --- | --- |
| I1 | Menjalankan 3-5 agen paralel berarti 3-5 jendela terminal tanpa gambaran gabungan | Sulit tahu mana yang selesai, macet, atau butuh izin |
| I2 | Meninjau hasil agen lewat `git diff` di terminal melelahkan | Review dangkal, bug lolos |
| I3 | Umpan balik ke agen berupa copy-paste manual (potongan kode, error, instruksi) | Lambat, konteks hilang |
| I4 | IDE berbasis Electron berat saat dijalankan bersama banyak agen | Laptop terbatas melambat |
| I5 | Bekerja dari mesin jauh atau HP tidak mulus | Agen idle menunggu persetujuan |
| I6 | Biaya dan loop agen tidak terlihat di tempat kerja utama | Kuota terbakar |

## 3. Visi & Tujuan

**Visi.** Workbench agen paling ringan dan paling aman, yang bekerja sama baik di laptop maupun VPS+HP.

**Tujuan terukur**
1. Dari buka workbench sampai 3 agen berjalan paralel: ≤ 60 detik untuk pengguna baru, ≤ 10 detik untuk pengguna berpengalaman.
2. Meninjau diff multi-file dan menerima/menolak per hunk tanpa meninggalkan workbench.
3. Mengirim umpan balik ke agen dari baris/hunk yang dipilih dengan ≤ 3 klik.
4. Workbench tetap responsif (input editor < 16 ms) dengan 5 sesi aktif.
5. Overhead daemon akibat layanan IDE ≤ anggaran di IDE_NFR_BUDGET; anggaran inti runtime tidak dilanggar.
6. Seluruh fitur kerja inti dapat dipakai dari layar HP (review + approve + kirim instruksi).

## 4. Persona

| Persona | Kebutuhan di IDE |
| --- | --- |
| **Dina (individu, laptop 8 GB)** | Beberapa agen tanpa RAM habis; review cepat |
| **Rafi (power user, VPS)** | Workbench di browser terhubung ke VPS; pintasan keyboard; palet perintah |
| **Tim engineering** | Pipeline/arena visual, kebijakan seragam, biaya per sesi |
| **Reviewer di perjalanan** | Review + approve dari HP |

## 5. Ruang Lingkup

### 5.1 In-Scope
- **Mission Control:** papan sesi, grid terminal agen, status, biaya, notifikasi.
- **Panel agen:** prompt/chat, konteks (file/seleksi), tugas templat, kirim ke banyak agen (fan-out dari UI).
- **Review:** diff side-by-side/inline, terima/tolak per hunk, komentar kembali ke agen, simulasi merge, merge/discard.
- **Editor ringan:** buka/ubah/simpan file di worktree, sorot perubahan agen, multi-tab, penyorotan sintaks.
- **Explorer, quick open, pencarian global.**
- **Terminal:** terminal agen (PTY) dan terminal shell tersandbox.
- **Panel git:** status, stage, commit, branch, log ringan.
- **Visual pipeline:** lihat DAG, status node, arena.
- **LSP opsional:** diagnostik, go-to-definition, hover (lazy, tersandbox).
- **Palet perintah, pintasan, tema, pengaturan.**
- **Mobile layout:** review, approve, kirim instruksi.
- **Shell native opsional** (webview sistem).

### 5.2 Tidak Akan Dibangun (mengikat)
- Debugger, profiler, notebook, designer UI.
- Kompatibilitas ekstensi VS Code / marketplace.
- Refactoring semantik mendalam, indexing kode berskala besar sebagai fitur sentral.
- Terminal multiplexer penuh (tmux replacement) dan manajer SSH.
- Kolaborasi *real-time multi-user* (kursor bersama) pada v1.
- Cloud/SaaS multi-tenant.
- Menjadi pengganti editor utama untuk penulisan kode manual intensif.

## 6. User Stories & Kriteria Penerimaan

### IUS-1 Mission Control
*Sebagai developer, saya ingin melihat semua sesi agen dalam satu papan agar tahu mana yang perlu perhatian.*
- Papan menampilkan kolom: Antre, Berjalan, Butuh Tindakan (approval/tripped/gagal), Siap Review, Selesai.
- Kartu menampilkan agen, tugas, durasi, biaya, jumlah file berubah, indikator aktivitas terakhir.
- Perubahan status muncul < 500 ms tanpa refresh.

### IUS-2 Review per Hunk
*Sebagai developer, saya ingin menerima atau menolak perubahan agen per hunk.*
- Diff multi-file dengan tampilan side-by-side dan inline.
- Aksi per hunk: terima, tolak (revert hunk di worktree), komentar.
- Hasil review dapat di-*commit* dan di-*merge* dengan simulasi konflik sebelumnya.

### IUS-3 Umpan Balik ke Agen
*Sebagai developer, saya ingin memilih baris/hunk lalu mengirim instruksi ke agen tanpa copy-paste.*
- Seleksi kode → "Kirim ke agen..." membawa path, rentang baris, dan isi sebagai konteks.
- Komentar review dikumpulkan lalu dikirim sebagai satu pesan terstruktur.
- Pesan masuk ke sesi yang dipilih (PTY: diketik sebagai input; mode terstruktur: pesan pengguna).

### IUS-4 Koreksi Manual Cepat
*Sebagai developer, saya ingin memperbaiki beberapa baris sendiri.*
- Edit di editor, simpan ke worktree; perubahan tampil di diff.
- Bila agen sedang menulis berkas yang sama, ada peringatan dan resolusi konflik yang jelas (lihat IDE_ARCHITECTURE §6).

### IUS-5 Fan-Out dari UI
*Sebagai developer, saya ingin mengirim satu prompt ke beberapa agen dan membandingkan hasil.*
- Pilih ≥ 2 profil, jalankan; tampilan arena (test, durasi, diff, biaya) dengan tombol "Pakai kandidat ini".

### IUS-6 Kerja dari HP
*Sebagai developer yang jauh dari laptop, saya ingin meninjau dan menyetujui dari HP.*
- Tata letak mobile: daftar sesi, diff vertikal, approve/reject, kirim pesan singkat, lihat layar terminal teks.
- Approve → sesi lanjut < 500 ms.

### IUS-7 Navigasi Cepat
*Sebagai developer, saya ingin menemukan file/simbol/teks dengan cepat.*
- Quick open fuzzy dalam < 100 ms untuk repo ≤ 50k file; pencarian teks global dengan hasil bertahap.

## 7. Prinsip UX

1. Status dan butuh-tindakan selalu terlihat.
2. Review lebih penting daripada menulis.
3. Semua aksi punya pintasan dan masuk palet perintah.
4. Tidak ada modal yang menghalangi pantauan agen lain.
5. Mobile adalah warga kelas satu untuk review dan kontrol.

## 8. Metrik Keberhasilan

| Metrik | Target |
| --- | --- |
| Waktu buka workbench → daftar sesi tampil | < 1.5 s (lokal) |
| Latensi ketikan di editor | < 16 ms p95 |
| Waktu buka file 1 MB | < 200 ms |
| Waktu quick open (50k file) | < 100 ms setelah indeks hangat |
| Ukuran aset web terkompresi | ≤ 3 MB |
| Waktu klik Approve → sesi lanjut | < 500 ms |
| Review hunk (terima/tolak) → worktree terbarui | < 300 ms |

## 9. Asumsi & Batasan
- Runtime v0.2 (server web, WS, token) sudah tersedia.
- Pengguna memakai browser modern (Chromium/Firefox/Safari 16+) atau shell native opsional.
- LSP bergantung pada server bahasa yang dipasang pengguna; tidak dibundel.
- Nama dan perilaku produk pembanding (Orca, enowX) diambil dari karakterisasi di blueprint dan perlu diverifikasi sebelum klaim publik.

## 10. Pertanyaan Terbuka
1. Seberapa jauh mode agen terstruktur (AGENT_PROTOCOL) menjadi jalur utama dibanding PTY?
2. Apakah review per hunk memodifikasi worktree langsung atau memakai staging layer? (IDE_ARCHITECTURE §7)
3. Perlu shell native sejak awal atau cukup browser/PWA? (ADR-0014)
4. Bahasa UI: Indonesia + Inggris sejak awal?
5. Lisensi dan nama final.

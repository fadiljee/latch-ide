# Requirements - Agent IDE

Prioritas: **P0** wajib MVP IDE · **P1** pembeda inti · **P2** penyempurnaan. Fase merujuk milestone di [IDE_ROADMAP](IDE_ROADMAP.md).

## Persyaratan Fungsional

### A. Workbench Shell & Navigasi
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I01 | Workbench Web | Aplikasi web (React+TS) disajikan daemon di `/`, dengan layout panel yang dapat diubah ukuran dan disimpan | P0 | I1 |
| FR-I02 | Command Palette | Palet perintah global (fuzzy) untuk semua aksi | P0 | I1 |
| FR-I03 | Keybindings | Pintasan bawaan + kustomisasi di `keybindings.json` | P1 | I2 |
| FR-I04 | Tema | Terang/gelap/sistem + tema kustom (token warna) | P1 | I2 |
| FR-I05 | Penyimpanan Layout/Preferensi | Preferensi per pengguna di daemon (bukan localStorage saja) agar konsisten antar perangkat | P1 | I2 |
| FR-I06 | Pencarian Global Cepat (Quick Open) | Fuzzy file open dengan indeks daemon | P0 | I2 |

### B. Mission Control & Manajemen Sesi
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I10 | Papan Sesi | Kanban: Antre, Berjalan, Butuh Tindakan, Siap Review, Selesai | P0 | I1 |
| FR-I11 | Grid Terminal Agen | Tampilan beberapa terminal agen sekaligus (1x1, 2x2, fokus+strip) | P0 | I1 |
| FR-I12 | Kartu Sesi | Status, agen, tugas, durasi, biaya, token, file berubah, aktivitas terakhir | P0 | I1 |
| FR-I13 | Buat Sesi dari UI | Dialog: proyek, profil, tugas, anggaran, base ref, templat | P0 | I1 |
| FR-I14 | Inbox Tindakan | Daftar terpusat: approval, tripped, gagal, siap review, dengan aksi cepat | P0 | I1 |
| FR-I15 | Dashboard Biaya & Telemetri | Grafik biaya/token per sesi, per proyek, per hari | P1 | I3 |
| FR-I16 | Riwayat Sesi | Pencarian dan pembukaan kembali sesi lama (read-only) | P1 | I3 |

### C. Panel Agen (Prompt/Chat)
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I20 | Kirim Pesan ke Agen | Kirim teks ke sesi (PTY: input; terstruktur: pesan pengguna) | P0 | I1 |
| FR-I21 | Lampirkan Konteks | `@file`, `@folder`, seleksi editor, diff, hasil test sebagai konteks terstruktur | P1 | I3 |
| FR-I22 | Templat Tugas | Templat prompt dengan variabel (mis. "perbaiki test gagal") | P1 | I3 |
| FR-I23 | Fan-Out dari UI | Pilih ≥ 2 profil untuk satu prompt; membuka arena | P1 | I4 |
| FR-I24 | Mode Agen Terstruktur | Jalur ACP/MCP-like untuk agen yang mendukung: pesan, tool call, permintaan izin terstruktur | P1 | I4 |
| FR-I25 | Antrean Pesan | Pesan saat agen sibuk dapat diantre/disela dengan aturan jelas | P2 | I4 |

### D. Review & Diff
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I30 | Diff Multi-File | Daftar file + diff side-by-side dan inline | P0 | I1 |
| FR-I31 | Terima/Tolak per Hunk | Revert/stage per hunk di worktree dengan undo | P0 | I2 |
| FR-I32 | Komentar Review | Komentar pada baris/hunk; kumpulkan dan kirim ke agen sebagai satu pesan | P1 | I3 |
| FR-I33 | Merge dari UI | Cek konflik (`merge-tree`), squash/merge, discard dengan konfirmasi | P0 | I1 |
| FR-I34 | Penanda Perubahan Agen | Gutter/penanda perubahan langsung di editor | P1 | I2 |
| FR-I35 | Ringkasan Perubahan | Statistik file/baris, daftar file berisiko (lockfile, CI, `.env`, skrip) | P1 | I3 |
| FR-I36 | Perbandingan Antar Kandidat | Diff antar sesi/arena | P2 | I4 |

### E. Editor
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I40 | Editor Ringan | CodeMirror 6: buka/ubah/simpan, multi-tab, penyorotan sintaks bahasa umum, pencarian dalam file | P0 | I2 |
| FR-I41 | Konkurensi Aman | `ETag`/versi pada tulis; deteksi perubahan oleh agen; resolusi 3-arah | P0 | I2 |
| FR-I42 | Read-only Mode Otomatis | File yang sedang aktif ditulis agen ditandai; opsi "ambil alih" | P1 | I2 |
| FR-I43 | Penanganan File Besar/Biner | Batas ukuran, mode baca-saja, pratinjau gambar | P1 | I2 |
| FR-I44 | Sticky Context Mobile | Mode edit sederhana di HP | P2 | I5 |

### F. Explorer, Pencarian, Navigasi
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I50 | File Explorer | Pohon file worktree (menghormati `.gitignore`), buat/rename/hapus | P0 | I2 |
| FR-I51 | Pencarian Teks Global | Regex/literal, filter glob, hasil bertahap (streaming) | P1 | I3 |
| FR-I52 | Simbol Dasar | Outline dan "go to symbol" berbasis tree-sitter/regex ringan atau LSP | P2 | I5 |
| FR-I53 | Pantau Perubahan File | Watcher menyinkronkan explorer/editor dengan perubahan dari agen | P0 | I2 |

### G. Terminal
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I60 | Terminal Agen | Attach xterm.js ke PTY sesi (baca/tulis) | P0 | I1 |
| FR-I61 | Terminal Shell Tersandbox | Shell di worktree dengan kebijakan sandbox yang sama | P1 | I3 |
| FR-I62 | Terminal Host (opsional) | Shell tanpa sandbox, butuh scope token `shell-host` dan aktivasi eksplisit | P2 | I5 |
| FR-I63 | Tautan Teks Terminal | `file:line` di output terminal membuka editor | P1 | I3 |

### H. Git
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I70 | Panel Git | Status, stage/unstage, commit, branch aktif | P1 | I3 |
| FR-I71 | Log & Blame Ringan | Riwayat commit file/sesi, blame baris | P2 | I5 |
| FR-I72 | Rebase/Sinkron dengan Base | Perbarui worktree dari base terbaru dengan deteksi konflik | P1 | I4 |
| FR-I73 | Resolusi Konflik Assisted | Tampilan 3-arah + opsi "minta agen menyelesaikan" | P2 | I5 |

### I. LSP (Opsional)
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I80 | Host LSP Tersandbox | Daemon menjalankan server bahasa di dalam sandbox per worktree, lazy, dengan batas memori | P2 | I5 |
| FR-I81 | Diagnostik, Hover, Definisi | Fitur LSP dasar di editor | P2 | I5 |
| FR-I82 | Registri Server Bahasa | Konfigurasi `lsp.toml` (perintah per bahasa) | P2 | I5 |

### J. Pipeline Visual
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I90 | Viewer DAG | Graf node `workflow.toml` dengan status, biaya, retry live | P1 | I4 |
| FR-I91 | Editor Workflow | Edit `workflow.toml` dengan validasi langsung dan peta ke graf | P2 | I5 |
| FR-I92 | Arena UI | Matriks perbandingan + aksi pilih pemenang | P1 | I4 |

### K. Remote, Mobile, Native
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I100 | Layout Mobile | Tata letak responsif untuk review/approve/pesan | P0 | I1 |
| FR-I101 | PWA & Push | Instalasi PWA, notifikasi event | P1 | I1 |
| FR-I102 | `latch ide` | Perintah membuka workbench (browser default atau shell native) dengan token otomatis | P0 | I1 |
| FR-I103 | Shell Native Opsional | Jendela webview sistem (wry/tao) tanpa Chromium bawaan | P2 | I5 |
| FR-I104 | Multi-Workspace | Beberapa proyek dalam satu workbench | P1 | I3 |

### L. Ekstensibilitas
| ID | Requirement | Deskripsi | P | Milestone |
| --- | --- | --- | --- | --- |
| FR-I110 | Perintah Kustom | Perintah palet dari `commands.toml` yang menjalankan shell/pipeline/templat | P2 | I5 |
| FR-I111 | Templat & Profil Komunitas | Impor templat tugas/profil/workflow dari berkas | P2 | I5 |
| FR-I112 | Plugin UI (eksperimental) | Panel pihak ketiga tersandbox (iframe + postMessage) | P2 | pasca |

## Persyaratan Non-Fungsional

| ID | Kategori | Target | Verifikasi |
| --- | --- | --- | --- |
| NFR-I01 | Ukuran aset web | ≤ 3 MB terkompresi total; bundle awal (shell+papan) ≤ 600 KB gzip; editor dimuat malas | Gerbang CI ukuran bundle |
| NFR-I02 | Latensi ketikan | < 16 ms p95 di editor (file ≤ 5.000 baris) | Benchmark browser headless |
| NFR-I03 | Waktu buka file | ≤ 200 ms untuk 1 MB | Tes e2e |
| NFR-I04 | Quick open | < 100 ms setelah indeks hangat (≤ 50k file) | Benchmark `search` |
| NFR-I05 | RAM daemon + layanan IDE | ≤ +15 MB di atas anggaran inti saat 1 workspace dibuka (tanpa server LSP); inti idle tetap ≤ 25 MB saat tidak ada klien IDE | Skrip RSS |
| NFR-I06 | Latensi event UI | Event status → UI < 500 ms | Tes e2e |
| NFR-I07 | Keamanan tulis | Semua operasi file dibatasi di akar worktree (tanpa lolos symlink/`..`) | Tes keamanan |
| NFR-I08 | Aksesibilitas | WCAG 2.1 AA untuk komponen inti; navigasi keyboard penuh | Audit axe + manual |
| NFR-I09 | Ketahanan UI | Reconnect otomatis, tidak kehilangan draf edit | Tes e2e |
| NFR-I10 | Kompatibilitas browser | Chrome/Edge/Firefox 2 versi terakhir, Safari ≥ 16 | Matriks e2e |
| NFR-I11 | Layar kecil | Fungsi inti dapat dipakai pada lebar 360 px | Tes visual |
| NFR-I12 | i18n | Bahasa Indonesia dan Inggris | Tes katalog |

## Keterlacakan

| Area | Bergantung pada runtime |
| --- | --- |
| Mission Control | FR-02, FR-06, FR-10, FR-21..23 |
| Review/Merge | FR-08, git-engine |
| Fan-Out/Pipeline UI | FR-18..FR-20 |
| Approval UI | FR-17 |
| Terminal | FR-06 (pty-pool) |
| Auth/scope | NFR-05, API token |

# Pintasan Keyboard Bawaan

Kustomisasi di `~/.config/latch/keybindings.json`. `Mod` = `Ctrl` (Linux/Windows) atau `Cmd` (macOS). Semua aksi juga tersedia di Palet Perintah.

## Global
| Pintasan | Aksi |
| --- | --- |
| `Mod+Shift+P` | Palet perintah |
| `Mod+P` | Quick open file |
| `Mod+Shift+F` | Cari teks global |
| `Mod+B` | Tampil/sembunyi rail kiri |
| `Mod+J` | Tampil/sembunyi panel bawah |
| `Mod+\` | Tampil/sembunyi panel agen |
| `Mod+1..5` | Pindah ke Papan / Review / Editor / Terminal / Pipeline |
| `Alt+A` | Buka Inbox Tindakan |
| `Alt+N` | Sesi baru |
| `Mod+,` | Pengaturan |
| `?` | Bantuan pintasan |

## Mission Control
| Pintasan | Aksi |
| --- | --- |
| `←/→/↑/↓` atau `h/j/k/l` | Navigasi kartu |
| `Enter` | Buka sesi |
| `p` | Pause/Resume |
| `x` | Hentikan (konfirmasi) |
| `y` / `n` | Approve / Deny pada kartu terpilih |
| `r` | Buka Review |

## Review
| Pintasan | Aksi |
| --- | --- |
| `j` / `k` | Hunk berikutnya / sebelumnya |
| `]` / `[` | File berikutnya / sebelumnya |
| `a` | Terima hunk |
| `r` | Tolak hunk |
| `c` | Komentar pada hunk/baris |
| `Mod+Enter` | Kirim komentar terkumpul ke agen |
| `s` | Ganti Side-by-side / Inline |
| `Mod+Z` | Urungkan aksi hunk terakhir |
| `m` | Buka dialog merge |

## Editor
| Pintasan | Aksi |
| --- | --- |
| `Mod+S` | Simpan |
| `Mod+F` / `Mod+H` | Cari / ganti dalam file |
| `Mod+G` | Ke baris |
| `Mod+D` | Pilih kemunculan berikut |
| `Alt+↑/↓` | Pindah baris |
| `Mod+/` | Komentari baris |
| `Mod+Shift+Enter` | Kirim seleksi ke agen |
| `F12` | Ke definisi (bila LSP aktif) |
| `Mod+W` | Tutup tab |

## Terminal
| Pintasan | Aksi |
| --- | --- |
| `Mod+Shift+\`` | Terminal shell baru |
| `Mod+Shift+C/V` | Salin/tempel |
| `Mod+K` | Bersihkan tampilan (tidak mengirim ke agen) |
| `Mod+Alt+←/→` | Pindah antar terminal |

## Catatan
- Pintasan yang bentrok dengan browser (mis. `Mod+W`, `Mod+T`) mungkin tidak dapat dicegat; di shell native dan PWA terpasang cakupannya lebih luas.
- Mode Vim/Emacs untuk editor: opsional (ekstensi CodeMirror), pasca-v1.

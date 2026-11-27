# Latch IDE: UI Spec untuk Rust + Tauri v2 + React 19

Pelengkap `UI-SPEC-agent-desktop.md` (spec umum gaya Orca + enowX). Dokumen ini menerjemahkannya ke **stack Latch IDE**:

| Lapisan | Stack |
|---|---|
| Daemon/engine | Rust (tokio, axum, SQLite, portable-pty, vt100) di `crates/` |
| Shell desktop | Tauri v2 di `desktop/` |
| UI | React 19 + TypeScript strict + Vite + Tailwind v4 + Zustand di `web/` |
| TUI opsional | Ratatui di `crates/tui/` |

Semua rekomendasi library di bawah adalah **usulan**; sesuaikan dengan apa yang sudah ada di repo.

---

## 1. Keputusan arsitektur UI

1. **Daemon = sumber kebenaran.** UI hanya merender state dan mengirim perintah. Sesi, PTY, status agent hidup di daemon, jadi UI boleh ditutup tanpa mematikan sesi (sama seperti enowX).
2. **Dua jalur transport:**
   - **WebSocket/SSE ke axum** untuk aliran data (byte PTY, event status, pesan chat).
   - **Tauri `invoke`** hanya untuk hal native: tray, jendela, shortcut global, dialog file, notifikasi.
   Dengan begitu `web/` bisa jalan juga di browser biasa (mode remote/server) tanpa Tauri.
3. **Terminal: xterm.js di frontend**, menerima byte mentah dari `portable-pty` lewat WebSocket. `vt100` tetap dipakai di daemon untuk snapshot layar (mis. agent membaca "apa yang tampil di layar", atau preview sesi di sidebar).
4. **Tema lewat CSS variables + atribut data**, bukan class per komponen. Ganti gaya/tema = ganti atribut di `<html>`.
5. **Code-splitting wajib.** Target ~86 KiB gzip hanya realistis untuk shell awal. Terminal, diff, canvas, notes, traffic di-`React.lazy`.

---

## 2. Struktur `web/`

```
web/
├── index.html
├── vite.config.ts
├── tsconfig.json                     # strict: true, noUncheckedIndexedAccess: true
├── package.json
├── AGENTS.md                         # aturan untuk coding agent (lihat bagian 12)
├── src/
│   ├── main.tsx
│   ├── app/
│   │   ├── App.tsx
│   │   ├── AppShell.tsx              # grid: TitleBar / Sidebar / Workspace / SidePanel / StatusBar
│   │   ├── routes.ts                 # view aktif: workspace | bots | automations | hosts | usage | plugins | settings
│   │   └── providers.tsx             # transport, tema, shortcut
│   │
│   ├── styles/
│   │   ├── index.css                 # @import "tailwindcss"; @theme; layer base
│   │   ├── tokens.css                # token dasar (spasi, radius, font, motion)
│   │   ├── palettes/
│   │   │   ├── dark.css              # [data-mode="dark"]
│   │   │   └── light.css             # [data-mode="light"]
│   │   └── styles/
│   │       ├── tonal.css             # [data-style="tonal"]
│   │       ├── ledger.css            # [data-style="ledger"]
│   │       ├── bento.css             # [data-style="bento"]
│   │       └── glass.css             # [data-style="glass"] (dengan fallback, lihat bagian 8)
│   │
│   ├── theme/
│   │   ├── theme-store.ts            # style, mode, accent, palette, bg image (persist)
│   │   ├── apply-theme.ts            # tulis data-* + --accent ke <html>
│   │   └── accent.ts                 # hitung --accent-soft, kontras teks di atas aksen
│   │
│   ├── transport/
│   │   ├── daemon-client.ts          # WebSocket + reconnect + keepalive + backoff
│   │   ├── protocol.ts               # tipe pesan (selaraskan dengan crate Rust, lihat bagian 6)
│   │   ├── native.ts                 # pembungkus invoke(); no-op di browser
│   │   └── events.ts                 # dispatcher event daemon → store
│   │
│   ├── state/                        # Zustand, satu store per domain
│   │   ├── hosts.ts
│   │   ├── sessions.ts               # byId, status, unread, hibernated
│   │   ├── messages.ts               # chat per sesi (append-only + windowing)
│   │   ├── layout.ts                 # pohon split + tab, persist
│   │   ├── ui.ts                     # sidebar/side panel terbuka, ukuran, view aktif
│   │   ├── usage.ts                  # meter limit per akun
│   │   ├── notifications.ts
│   │   └── selectors.ts              # selector memo (needsYouCount, sessionsByWorktree, ...)
│   │
│   ├── lib/
│   │   ├── cn.ts                     # gabung class
│   │   ├── time.ts                   # "49m", "now", "4d 6h"
│   │   ├── keys.ts                   # registry shortcut + deteksi platform
│   │   └── ids.ts
│   │
│   ├── components/
│   │   ├── ui/                       # primitives: Button, IconButton, Input, Toggle, Segmented,
│   │   │                             #   Dialog, Popover, Tooltip, DropdownMenu, ContextMenu, Toast
│   │   ├── StatusDot.tsx
│   │   ├── ModelBadge.tsx
│   │   ├── UsagePill.tsx
│   │   ├── TreeItem.tsx
│   │   ├── TabBar.tsx
│   │   ├── SplitLayout.tsx           # render pohon split dari state/layout
│   │   ├── CommandPalette.tsx
│   │   └── Kbd.tsx
│   │
│   ├── layout/
│   │   ├── TitleBar.tsx              # drag region + tab global + meter
│   │   ├── Sidebar.tsx
│   │   ├── Workspace.tsx
│   │   ├── SidePanel.tsx
│   │   └── StatusBar.tsx
│   │
│   └── features/
│       ├── sessions/                 # SessionRow, ChatView, Composer, ModelPicker, NewSessionDialog
│       ├── terminal/                 # TerminalView (lazy), pty-stream.ts, terminal-theme.ts
│       ├── review/                   # GitPanel, DiffView (lazy), ConflictRadar, CompareView
│       ├── browser/                  # BrowserPanel, DesignMode
│       ├── traffic/                  # (lazy)
│       ├── bots/                     # BotsPage, TaskBoard
│       ├── automations/
│       ├── hosts/
│       ├── usage/
│       ├── plugins/
│       ├── notes/                    # (lazy)
│       ├── canvas/                   # (lazy)
│       ├── settings/
│       └── floating/                 # FloatingControl (entry terpisah, bagian 7)
│
└── tests/
    ├── visual/                       # Playwright: screenshot per gaya × mode
    └── unit/                         # Vitest: store, time, accent
```

Entry terpisah untuk jendela floating: `web/floating.html` + `src/features/floating/main.tsx` (konfigurasi Vite multi-page).

---

## 3. Token Tailwind v4 (CSS-first)

Tailwind v4 membaca token dari CSS (`@theme`), bukan `tailwind.config.js`. Pola: komponen memakai utilitas semantik (`bg-surface`, `text-muted`) yang menunjuk ke CSS variable; variable-nya diganti per mode/gaya.

`src/styles/index.css`:

```css
@import "tailwindcss";
@import "./tokens.css";
@import "./palettes/dark.css";
@import "./palettes/light.css";
@import "./styles/tonal.css";
@import "./styles/ledger.css";
@import "./styles/bento.css";
@import "./styles/glass.css";

@theme inline {
  --color-app: var(--bg-app);
  --color-sidebar: var(--bg-sidebar);
  --color-surface: var(--bg-surface);
  --color-raised: var(--bg-surface-raised);
  --color-hover: var(--bg-hover);
  --color-selected: var(--bg-selected);
  --color-line: var(--border-subtle);
  --color-line-strong: var(--border-strong);
  --color-fg: var(--text-primary);
  --color-fg-2: var(--text-secondary);
  --color-fg-3: var(--text-muted);
  --color-accent: var(--accent);
  --color-accent-soft: var(--accent-soft);
  --color-working: var(--status-working);
  --color-needs: var(--status-needs-you);
  --color-done: var(--status-done);
  --color-idle: var(--status-idle);
  --color-danger: var(--status-error);

  --radius-sm: var(--r-sm);
  --radius-md: var(--r-md);
  --radius-lg: var(--r-lg);
  --radius-xl: var(--r-xl);

  --font-sans: var(--font-ui);
  --font-mono: var(--font-code);
}
```

`src/styles/tokens.css` (nilai tetap, tidak berubah per mode):

```css
:root {
  --font-ui: "Inter", system-ui, sans-serif;
  --font-code: "JetBrains Mono", ui-monospace, monospace;
  --ease: cubic-bezier(.2, .8, .2, 1);
  --dur-fast: 120ms;
  --dur-panel: 180ms;
  --dur-theme: 240ms;
  --sidebar-w: 260px;
  --sidepanel-w: 340px;
  --titlebar-h: 40px;
  --statusbar-h: 26px;
}
```

`src/styles/palettes/dark.css` (contoh; Light dibuat setara):

```css
[data-mode="dark"] {
  --bg-app: #0f1114;
  --bg-sidebar: #13161a;
  --bg-surface: #171b20;
  --bg-surface-raised: #1d2228;
  --bg-hover: #1f252c;
  --bg-selected: color-mix(in oklab, var(--accent) 14%, transparent);
  --border-subtle: #242a31;
  --border-strong: #323a43;
  --text-primary: #e7eaee;
  --text-secondary: #a7afb9;
  --text-muted: #6f7782;
  --status-working: #4da3ff;
  --status-needs-you: #ff9f43;
  --status-done: #3ecf8e;
  --status-idle: #6f7782;
  --status-error: #ff5d6c;
  --terminal-bg: #0b0d10;
}
```

`src/styles/styles/bento.css` (contoh gaya: hanya geometri, tanpa warna baru):

```css
[data-style="bento"] {
  --r-sm: 8px; --r-md: 12px; --r-lg: 16px; --r-xl: 20px;
  --gap-panel: 8px;                         /* jarak antar kartu */
  --panel-border: 0px;
  --panel-shadow: 0 1px 2px rgb(0 0 0 / .25), 0 8px 24px rgb(0 0 0 / .18);
}
[data-style="ledger"] {
  --r-sm: 2px; --r-md: 3px; --r-lg: 4px; --r-xl: 6px;
  --gap-panel: 0px;
  --panel-border: 1px;                      /* garis pemisah tegas */
  --panel-shadow: none;
}
```

Komponen kerangka memakai variable gaya, misalnya panel:

```tsx
<section className="bg-surface rounded-lg border-[length:var(--panel-border)] border-line
                    shadow-[var(--panel-shadow)] m-[var(--gap-panel)]" />
```

### Aturan token (masukkan ke AGENTS.md)
- Dilarang warna hex/rgb langsung di komponen; hanya utilitas semantik.
- Terminal selalu memakai `--terminal-bg`, gelap di kedua mode.
- Status selalu lewat `StatusDot` (satu sumber visual).

---

## 4. Theme manager

`theme/apply-theme.ts`:

```ts
export type ThemeState = {
  style: "tonal" | "ledger" | "bento" | "glass";
  mode: "dark" | "light" | "system";
  accent: string;            // hex
};

export function applyTheme(t: ThemeState) {
  const root = document.documentElement;
  const mode =
    t.mode === "system"
      ? matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
      : t.mode;
  root.dataset.mode = mode;
  root.dataset.style = t.style;
  root.style.setProperty("--accent", t.accent);
  root.style.setProperty("--accent-soft", `color-mix(in oklab, ${t.accent} 16%, transparent)`);
}
```

- Panggil sekali sebelum render pertama (di `index.html` lewat script kecil membaca `localStorage`) agar tidak ada flash tema.
- Dengarkan `matchMedia` saat mode = system.
- Transisi: tambahkan class sementara `theme-transition` (`transition: background-color var(--dur-theme), color var(--dur-theme)`) lalu lepas setelah 260 ms, jangan transisi permanen di semua elemen.

---

## 5. State Zustand dan alur event

```ts
// state/sessions.ts
type Status = "working" | "needs_you" | "done" | "idle" | "error";
type Session = {
  id: string; worktreeId: string; agent: string; model: string;
  title: string; status: Status; unread: number;
  startedAt: number; hibernated: boolean;
};

type SessionsState = {
  byId: Record<string, Session>;
  order: string[];
  upsert: (s: Session) => void;
  setStatus: (id: string, status: Status) => void;
};
```

Aturan:
- Satu event dari daemon mengubah satu store; UI lain **menurunkan** dari selector (`needsYouCount`), jangan menyalin state.
- Data frekuensi tinggi (byte PTY) **tidak** masuk Zustand; langsung ke instance xterm lewat `pty-stream.ts`.
- Chat besar: simpan per sesi, render dengan virtualisasi (`@tanstack/react-virtual`), dan hanya muat bagian terbaru.
- Persist hanya yang perlu (`layout`, `ui`, `theme`), bukan sesi (itu milik daemon).
- Gunakan selector sempit (`useSessions(s => s.byId[id])`) agar baris sidebar tidak re-render semua.

---

## 6. Kontrak transport (daemon ↔ web)

Definisikan tipe sekali, di Rust, lalu hasilkan TypeScript (mis. dengan `ts-rs` atau `specta`) supaya tidak ada drift. Usulan pesan:

```
Server → UI
  session.upsert       { session }
  session.status       { id, status }
  session.output       { id, bytes }          # biner, bukan JSON
  chat.append          { sessionId, message }
  usage.update         { accountId, sessionPct, sessionResetAt, weekPct, weekResetAt }
  git.changes          { sessionId, files[] }
  conflict.warning     { sessionId, otherSessionId, file, lines }

UI → Server
  session.create | session.send | session.kill | session.hibernate | session.wake
  session.resize       { id, cols, rows }
  git.commit | git.push | git.discard
  review.comment       { sessionId, file, line, text }   # dikirim ke agent
```

Kebutuhan klien: reconnect otomatis dengan backoff, keepalive/ping, dan "resync penuh" setelah reconnect (minta snapshot state) agar UI konsisten setelah koneksi putus (penting untuk server jauh).

---

## 6b. Terminal

- Paket: `@xterm/xterm` + addon `fit` dan `webgl`; sediakan fallback renderer jika WebGL bermasalah.
- Alur: `pty-stream.ts` menulis byte dari WebSocket ke `term.write()`; input dan `resize` dikirim balik ke daemon.
- Satu instance xterm per sesi, dibuang saat tab tidak terlihat lama dan dibangun ulang dari snapshot `vt100` + scrollback dari daemon (membantu memori dan "hibernate").
- Tema terminal: baca token `--terminal-*`, jangan ikut mode terang.
- Pertimbangkan uji awal WebGL di WebKitGTK di mesin target; bila tidak stabil, jadikan renderer biasa sebagai default di Linux.

---

## 7. Shell Tauri v2 (`desktop/`)

### Jendela
```jsonc
// tauri.conf.json (potongan)
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Latch",
        "width": 1440, "height": 900, "minWidth": 960, "minHeight": 600,
        "decorations": false,          // title bar kustom (React)
        "transparent": false           // true hanya untuk gaya glass, lihat bagian 8
      },
      {
        "label": "floating",
        "url": "floating.html",
        "width": 360, "height": 360,
        "decorations": false, "transparent": true,
        "alwaysOnTop": true, "skipTaskbar": true, "visible": false
      }
    ]
  }
}
```

- Title bar kustom: beri `data-tauri-drag-region` pada area kosong TitleBar; tombol minimize/maximize/close memanggil API window. Uji perilaku resize tepi jendela di Linux bila `decorations: false`.
- Simpan posisi/ukuran dengan plugin window-state.

### Plugin yang dibutuhkan
`global-shortcut` (floating control), `notification`, `window-state`, `dialog`, `fs` (secukupnya), `opener`, plus fitur tray bawaan Tauri v2.

### Floating control
- Shortcut global (`Ctrl+Shift+Space` / `⌘⇧Space`) menampilkan/menyembunyikan jendela `floating`.
- Isinya halaman React terpisah (`floating.html`) yang berbagi `transport/` dan `state/` yang sama, tetapi dimuat ringan (tanpa terminal/diff).
- Menutup lewat `Esc` atau klik di luar; fokus kembali ke aplikasi sebelumnya.

### Tray
- Menu: daftar sesi "needs you", buka aplikasi, keluar.
- Aplikasi tetap hidup di tray saat jendela ditutup; daemon jalan terus.
- Badge angka: tidak semua platform/tray mendukungnya (khususnya Linux), jadi siapkan **varian ikon** (`tray-0`, `tray-1`, `tray-2`, `tray-9+`) dan tukar ikon saat hitungan berubah.

### Daemon
Shell Tauri menjalankan/menyambung ke daemon (sidecar atau proses sistem) dan memberi UI alamat + token lokal. Untuk remote, UI menyambung ke daemon di server lewat SSH tunnel/Tailscale; kode `web/` sama.

---

## 8. Catatan khusus Linux (WebKitGTK)

Karena shell memakai WebView OS (WebKitGTK), ada perbedaan dibanding Chromium. Hal yang perlu diuji sejak awal:

| Area | Risiko | Saran |
|---|---|---|
| **Gaya Liquid Glass** | Tidak ada vibrancy native di Linux; `backdrop-filter` dan jendela transparan bergantung compositor dan versi WebKitGTK | Jadikan `glass` opsional; deteksi dukungan, fallback ke `tonal`; jangan jadikan default di Linux |
| **WebGL xterm** | Bisa tidak stabil di beberapa driver | Fallback renderer biasa |
| **Shortcut global** | Di Wayland umumnya tidak tersedia untuk aplikasi biasa | Sediakan alternatif lewat tray/aksi menu dan beri tahu pengguna di Settings |
| **Tray** | Butuh dukungan AppIndicator di desktop environment | Dokumentasikan dependensi; fallback tanpa tray |
| **Font** | Font sistem berbeda antar distro | Bundel Inter + JetBrains Mono (woff2, subset) |
| **Skala HiDPI** | Variasi antar compositor | Uji 100/125/150/200% |
| **Drag-drop file** | Perilaku event berbeda antara WebView dan jendela | Uji dari file manager langsung ke Composer |
| **Fitur CSS baru** | Versi WebKitGTK lebih lama bisa belum mendukung (mis. beberapa fungsi warna) | Tetapkan versi minimum, cek `@supports`, sediakan fallback `color-mix` |

Rilis `.AppImage` dan `.deb`: ingat bahwa versi WebKitGTK yang dipakai mengikuti sistem pengguna (khususnya pada `.deb`), jadi rentang versi yang didukung sebaiknya dicatat di dokumentasi.

---

## 9. Dependensi frontend yang disarankan

| Kebutuhan | Paket usulan | Catatan |
|---|---|---|
| State | `zustand` | sudah dipakai |
| Split pane | `react-resizable-panels` | resize + persist |
| Headless UI | `@radix-ui/*` atau `react-aria-components` | dialog, popover, menu, tooltip |
| Command palette | `cmdk` | |
| Ikon | `lucide-react` (import per ikon) | tree-shakeable |
| Virtualisasi | `@tanstack/react-virtual` | chat, tabel traffic |
| Terminal | `@xterm/xterm` + addon | lazy chunk |
| Diff | komponen diff sendiri atau library khusus | lazy chunk |
| Markdown (chat/notes) | `react-markdown` + syntax highlighter ringan | lazy |
| Tauri | `@tauri-apps/api` v2 + plugin terkait | |
| Tes | `vitest`, `@playwright/test` | screenshot per gaya × mode |

Anggaran ukuran (usulan): shell awal ≤ 100 KiB gzip, tiap chunk lazy ≤ 150 KiB gzip. Pantau dengan `vite build --report` atau analyzer di CI.

---

## 10. Pemetaan komponen → file (ringkas)

| Elemen UI (dari spec umum) | File |
|---|---|
| Title bar + tab Search/Tasks/Automations/Bots + meter | `layout/TitleBar.tsx`, `components/UsagePill.tsx` |
| Tree Hosts → Projects → Worktree → Session | `layout/Sidebar.tsx`, `components/TreeItem.tsx`, `features/sessions/SessionRow.tsx` |
| Badge model + waktu | `components/ModelBadge.tsx`, `lib/time.ts` |
| Chat + tool-call lipat + diff inline | `features/sessions/ChatView.tsx` |
| Composer (@, /, mode, model) | `features/sessions/Composer.tsx`, `ModelPicker.tsx` |
| Split + tab | `components/SplitLayout.tsx`, `TabBar.tsx`, `state/layout.ts` |
| Side panel (Git, Checks, Files, Ports, Browser, Traffic) | `layout/SidePanel.tsx` + `features/*` |
| Fan-out + Compare | `features/review/CompareView.tsx`, `features/sessions/NewSessionDialog.tsx` |
| Command palette | `components/CommandPalette.tsx`, `lib/keys.ts` |
| Halaman Bots | `features/bots/BotsPage.tsx`, `TaskBoard.tsx` |
| Floating control | `features/floating/*`, `web/floating.html` |
| Empat gaya + palet + aksen | `styles/*`, `theme/*` |

---

## 11. Urutan pengerjaan (disesuaikan stack)

1. Vite + React 19 + Tailwind v4 + `styles/` + `theme/` (satu gaya, dua mode) + tes screenshot dasar
2. `AppShell` dengan title bar kustom di Tauri (`data-tauri-drag-region`) + tombol window
3. Primitives (`components/ui/`) di atas Radix/React Aria
4. `transport/daemon-client.ts` + tipe protokol yang dihasilkan dari Rust; mock server untuk pengembangan UI
5. Store `hosts`, `sessions`, `layout` + Sidebar + `SessionRow`/`StatusDot`
6. `SplitLayout` + `TabBar` (persist)
7. `ChatView` + `Composer` (virtualisasi sejak awal)
8. `TerminalView` (lazy) tersambung ke PTY
9. `CommandPalette` + registry shortcut
10. Side panel: Git/diff, Files, Ports, Browser
11. Tiga gaya sisanya + uji visual semua kombinasi
12. Halaman Bots, Automations, Hosts, Usage, Settings
13. Tray, floating control, notifikasi, rilis AppImage/.deb

---

## 12. Isi `web/AGENTS.md` (untuk coding agent)

```
# Aturan UI web/
- Stack: React 19, TypeScript strict, Vite, Tailwind v4 (CSS-first), Zustand.
- Dilarang warna literal di komponen. Pakai utilitas semantik (bg-surface, text-fg-2, ...).
- Dilarang menaruh byte PTY atau data frekuensi tinggi di Zustand.
- Satu store per domain; komponen memakai selector sempit.
- Fitur berat (terminal, diff, canvas, notes, traffic) wajib React.lazy.
- Semua tipe pesan daemon berasal dari tipe yang dihasilkan; jangan menulis ulang manual.
- Panggilan native hanya lewat transport/native.ts; kode harus tetap jalan di browser tanpa Tauri.
- Setiap komponen baru: uji visual untuk 4 gaya × 2 mode.
- Perintah: pnpm dev | pnpm build | pnpm test | pnpm test:visual
```

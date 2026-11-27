# Aturan UI `web/`

- Stack: React 19, TypeScript strict (`noUncheckedIndexedAccess: true`), Vite, Tailwind v4 (CSS-first), Zustand.
- Dilarang warna literal (`#hex`, `rgb(...)`, atau utilitas palet Tailwind langsung) di komponen `.tsx`. Pakai utilitas semantik (`bg-surface`, `text-fg-2`, `border-line`, `bg-accent`, dll.).
- Dilarang inline `<script>` atau `dangerouslySetInnerHTML` (CSP ketat: `script-src 'self'`).
- Dilarang karakter em dash (`—`) pada teks UI dan katalog i18n.
- Dilarang menaruh byte PTY atau data frekuensi tinggi di Zustand; alirkan langsung lewat `features/terminal/pty-stream.ts`.
- Satu store per domain di `src/state/`; komponen memakai selector sempit.
- Fitur berat (terminal `@xterm/xterm`, diff viewer, comparison arena, traffic log) wajib `React.lazy`.
- Panggilan native Tauri hanya lewat `src/transport/native.ts`; kode wajib tetap berjalan di browser biasa tanpa Tauri.
- Semua kontrol interaktif menggunakan elemen semantik (`<button>`, `<label htmlFor="...">`, fokus terlihat, dukungan keyboard, tanpa `autoFocus`).
- Perintah wajib: `npm run lint`, `npm run typecheck`, `npm run test`, `npm run build`.

# Architecture Decision Records (ADR)

Satu berkas per keputusan arsitektur penting. Format ringkas (MADR). Jangan mengedit ADR yang sudah `Accepted`; ganti dengan ADR baru yang berstatus `Supersedes`.

## Indeks

| No. | Judul | Status |
| --- | --- | --- |
| [0001](0001-pure-rust-single-binary.md) | Rust murni, single static binary | Accepted |
| [0002](0002-landlock-first-sandbox.md) | Sandbox Linux-first dengan Landlock | Accepted |
| [0003](0003-no-relay-server-v1.md) | Tanpa server relay di v1 | Accepted |
| [0004](0004-git-cli-plus-gix.md) | `git` CLI untuk worktree + `gix` untuk inspeksi | Accepted |
| [0005](0005-sqlite-bundled.md) | SQLite `bundled` sebagai penyimpanan state | Accepted |
| [0006](0006-embedded-web-pwa.md) | Web PWA tersemat, bukan Electron/Tauri | Accepted |
| [0007](0007-proxy-via-base-url-injection.md) | Proxy LLM via injeksi `*_BASE_URL` | Accepted |

ADR yang direncanakan (runtime): 0008 (akses `.git` dari worktree), 0009 (mekanisme approval gate), 0010 (lisensi), 0011 (nama final produk).

### ADR Agent IDE (lihat [`../ide/`](../ide/))

| No. | Judul | Status |
| --- | --- | --- |
| [0012](0012-workbench-as-web-app-served-by-daemon.md) | Workbench sebagai web app yang disajikan daemon | Proposed |
| [0013](0013-codemirror-6-instead-of-monaco.md) | CodeMirror 6, bukan Monaco | Proposed |
| [0014](0014-optional-native-shell-via-system-webview.md) | Shell native opsional via webview sistem | Proposed |
| [0015](0015-lazy-ide-services-and-process-isolated-lsp.md) | Layanan IDE lazy, LSP proses terisolasi | Proposed |
| [0016](0016-dual-agent-integration-pty-and-structured.md) | Integrasi agen ganda: PTY + terstruktur | Proposed |
| [0017](0017-scoped-tokens-and-guarded-write-surface.md) | Token ber-scope dan permukaan tulis terjaga | Proposed |

## Templat

```markdown
# ADR-NNNN: Judul

- Status: Proposed | Accepted | Deprecated | Superseded by ADR-XXXX
- Tanggal: YYYY-MM-DD
- Pengambil keputusan:

## Konteks
## Keputusan
## Alternatif yang dipertimbangkan
## Konsekuensi
### Positif
### Negatif / risiko
## Referensi
```

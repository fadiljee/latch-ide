# ADR-0016: Integrasi agen ganda: PTY universal + mode terstruktur

- Status: Proposed (perlu spike IT-005)
- Tanggal: (isi saat disetujui)

## Konteks
PTY mendukung semua CLI tetapi tidak memberi struktur (pesan, tool call, izin). Approval gate (FR-17) sulit dipenuhi hanya dengan PTY. Beberapa agen mulai mendukung protokol terstruktur (mis. ACP; **verifikasi dukungan terkini**).

## Keputusan
Pertahankan **PTY sebagai jalur universal** dan tambahkan **mode terstruktur** opsional per profil lewat crate `agent-proto`, awalnya mengadaptasi **ACP** bila spike membuktikan kelayakan dan dukungan agen nyata. Server MCP internal opsional untuk menyediakan alat workbench. Pesan UI ke agen bersifat terstruktur dan di-*render* ke teks untuk PTY dengan templat per profil. Permintaan izin dari protokol terstruktur dipetakan ke approval gate tanpa menurunkan sandbox.

## Alternatif yang dipertimbangkan
- **Hanya PTY**: sederhana; UX dan approval terbatas.
- **Hanya terstruktur**: tidak kompatibel dengan banyak CLI.
- **Protokol kustom sendiri**: hanya bila ACP tidak sesuai; tidak ada adopsi eksternal.

## Konsekuensi
### Positif
UX kaya dan approval akurat untuk agen yang mendukung; tidak mengorbankan kompatibilitas.
### Negatif / risiko
Dua jalur kode; protokol dapat berubah; matriks kompatibilitas harus dipelihara. Mitigasi: adaptor terisolasi, spike awal, ADR dapat direvisi.

## Referensi
AGENT_PROTOCOL, FR-I24, FR-17, R-06, IR-10.

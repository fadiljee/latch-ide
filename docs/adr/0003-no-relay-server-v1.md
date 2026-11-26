# ADR-0003: Tanpa server relay di v1

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Pengguna ingin mengendalikan daemon dari HP atau mesin lain. Membangun server relay milik sendiri menambah beban infrastruktur, biaya, dan permukaan serangan, serta mendekati SaaS multi-tenant yang merupakan non-goal v1.

## Keputusan
Remote hanya melalui **SSH port forwarding** atau **Tailscale/WireGuard**. Daemon bind ke `127.0.0.1` secara bawaan dan dilindungi bearer token acak lokal. Tidak ada port publik mentah.

## Alternatif yang dipertimbangkan
- **Relay/cloud sendiri**: butuh akun, TLS publik, moderasi; ditolak untuk v1.
- **Expose langsung ke internet dengan token**: risiko tinggi; ditolak.
- **Cloudflare Tunnel/ngrok**: bergantung layanan pihak ketiga; tidak dilarang pengguna, tetapi tidak didukung resmi.

## Konsekuensi
### Positif
Tanpa infrastruktur, keamanan lebih sederhana, sesuai *local-first*.
### Negatif / risiko
Pengguna harus menyiapkan Tailscale/SSH; Web Push butuh HTTPS (diselesaikan dengan sertifikat Tailscale); pengalaman onboarding HP kurang mulus.

## Referensi
FR-03, FR-22, NFR-05, [SECURITY.md §9](../SECURITY.md).

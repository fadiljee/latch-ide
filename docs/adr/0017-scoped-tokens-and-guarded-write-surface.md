# ADR-0017: Token ber-scope dan permukaan tulis yang dijaga

- Status: Proposed
- Tanggal: (isi saat disetujui)

## Konteks
Workbench memungkinkan klien web menulis file, membuka terminal, dan mengelola LSP. Satu bearer token "serba bisa" (seperti pada API kontrol-saja) menjadi risiko besar bila bocor, terutama token di HP.

## Keputusan
1. Token memiliki **scope**: `read`, `control`, `write`, `shell-sandbox`, `shell-host`, `admin`; dibuat/dicabut via `latch token`, disimpan sebagai hash, dapat kedaluwarsa.
2. Token HP/remote disarankan `read,control`.
3. `shell-host` **nonaktif default** dan hanya untuk koneksi lokal kecuali dikonfigurasi eksplisit.
4. Semua operasi file lewat `fs-service` dengan `openat2 RESOLVE_BENEATH`, larangan tulis `.git/`, batas ukuran, dan audit tanpa isi.
5. Bearer + validasi `Origin`/`Host`; tidak ada cookie otentikasi.

## Alternatif yang dipertimbangkan
- **Satu token penuh**: sederhana, tetapi radius ledakan besar.
- **Cookie sesi + CSRF**: lebih rumit dan rentan DNS rebinding bila salah konfigurasi.
- **mTLS/WebAuthn**: lebih kuat, ditunda pasca-v1 (dapat ditinjau).

## Konsekuensi
### Positif
Radius ledakan lebih kecil; kebijakan jelas untuk perangkat berbeda.
### Negatif / risiko
Kompleksitas UX token; perlu tes 403 menyeluruh; pengguna dapat salah memberi scope.

## Referensi
IDE_SECURITY, IDE_API §1, IR-06, IT-102.

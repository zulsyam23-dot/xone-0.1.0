# Hello Hook Plugin

Plugin sederhana untuk uji coba upload/install plugin di Xone.

## Isi paket
- `manifest.json`: metadata plugin.
- `hooks.conf`: definisi hook yang akan dibaca Xone.

## Shortcut bawaan
- `Ctrl+Alt+H`: menampilkan message di status bar.
- `Ctrl+Alt+R`: kirim command ke terminal.
- `Ctrl+Alt+G`: buka file `Cargo.toml`.

## Cara pakai (mekanisme sekarang)
1. Salin isi `hooks.conf` ke `<workspace>/.xone/hooks.conf`.
2. Jalankan/restart Xone.
3. Coba shortcut di atas.

## Catatan
Ini adalah paket contoh untuk validasi alur upload. Saat ini Xone belum punya installer plugin resmi, jadi integrasi masih via `hooks.conf`.

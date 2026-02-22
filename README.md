## editor
![image alt](https://github.com/zulsyam23-dot/xone-0.1.0/blob/196a64c08f5fdc373af4e7839bca5372852f16c6/Screenshot%202026-02-22%20132943.png)
## terminal
![image alt](https://github.com/zulsyam23-dot/xone-0.1.0/blob/5f8a47d3f7cae370cfc69d04d4a82618a6b10c4a/Screenshot%202026-02-22%20134829.png)
## pengaturan
![image alt](https://github.com/zulsyam23-dot/xone-0.1.0/blob/b82016443a8aaa62e008a1b4459ed53e154f2c49/Screenshot%202026-02-22%20134849.png)
 # Xone Workspace Guide

## Ringkasan
Xone adalah editor TUI dengan panel Explorer, Editor, Terminal, Settings, dan AI Chat dalam satu workspace.

## Persyaratan

Sebelum menjalankan project ini, pastikan sudah menginstall:

- Rust
- Git
- Windows Terminal (disarankan)

---

## Install Rust

1. Download Rust dari: https://rustup.rs
2. Jalankan installer `.exe`
3. Pilih opsi default
4. Restart komputer setelah selesai

## Install Git

Download Git untuk Windows:
https://git-scm.com/download/win




## Clone Repository

Buka PowerShell atau Windows Terminal:

- git clone https://github.com/zulsyam23-dot/xone-0.1.0.git
- cd xone-0.1.0




## Build Project

- cargo build --release
- cargo run




## ⚠ Troubleshooting

### Cargo tidak dikenali
Pastikan Rust sudah terinstall dan komputer sudah direstart.

### Error saat build
Pastikan koneksi internet aktif dan Rust terupdate:



## Fitur Utama
- Editor teks dengan syntax highlight, suggestion, bookmark, undo/redo.
- Explorer file/folder terintegrasi dengan workspace.
- Terminal multi-tab + scrollback (`Ctrl+PgUp/PgDn`).
- AI Chat panel (kiri) dengan provider yang bisa diganti cepat.
- Hook command kustom via `.xone/hooks.conf`.

## Kelebihan
- Cepat untuk workflow keyboard-only.
- Semua panel dalam satu layar (tidak perlu pindah aplikasi).
- Konfigurasi local-per-workspace (mudah dipindah/versi kontrol).
- Bisa mulai lokal penuh dengan Ollama tanpa API key cloud.

## Mulai Cepat
1. Konfigurasi AI di `.xone/ai/ai.conf`.
2. Buka Settings (`F2`) untuk switch profile AI cepat (`1/2/3`).
3. Buka AI Chat (`F6`) dan kirim prompt.
![image alt](https://github.com/zulsyam23-dot/xone-0.1.0/blob/b470cf40ebcf76718e9c44ec95e9b012f2b8b7a6/Screenshot%202026-02-22%20134210.png)
5. Gunakan `Ctrl+Enter` (insert) atau `Alt+Enter` (replace selection) dari jawaban AI.

## Dokumentasi Lanjutan
- Konfigurasi workspace: `.xone/README.md`
- Konfigurasi AI: `.xone/ai/README.md`
- Arsitektur source: `src/README.md`
- Panduan dan uji: `docs/README.md`

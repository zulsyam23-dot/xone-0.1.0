# src Folder

## Tujuan
Sumber kode utama aplikasi Xone.

## Fitur yang Sudah Terintegrasi
- TUI multi-panel: Explorer, Editor, Terminal, Settings, AI Chat.
- Routing input keyboard terpusat.
- Integrasi AI chat dengan multi-provider.

## Kelebihan Arsitektur
- Pemisahan modul `app` dan `core` menjaga boundary jelas.
- Core filesystem reusable dan aman terhadap root workspace.
- UI render terpusat memudahkan konsistensi tampilan.

## Cara Menavigasi Kode
1. Entry point: `main.rs`
2. Orkestrasi aplikasi: `src/app`
3. Workspace/filesystem core: `src/core`

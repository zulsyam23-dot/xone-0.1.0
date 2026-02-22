# src/app/terminal Folder

## Tujuan
Menangani terminal emulation, session PTY, parser, dan widget terminal.

## Pencapaian Saat Ini
- Tab terminal aktif dengan navigasi tab.
- Scrollback terminal aktif dan bisa dinavigasi.
- Sinkronisasi cwd terminal ke explorer tersedia.

## Cara Pemakaian
1. Logika session di `mod.rs`.
2. Integrasi widget terminal di `widget.rs`.
3. Rendering state terminal di `state.rs`.

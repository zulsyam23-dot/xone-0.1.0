# src/app Folder

## Tujuan
Implementasi fitur aplikasi tingkat UI, interaksi, dan workflow pengguna.

## Fitur
- Editor: highlight, suggestion, bookmark, undo/redo, paste rewrite.
- Explorer: navigasi tree + create file/folder.
- Terminal: multi-tab, scrollback, sync cwd.
- Settings: pengaturan UI + konfigurasi AI cepat.
- AI Chat:
  - panel kiri (mode 50% saat aktif)
  - streaming response (Ollama)
  - context editor/selection
  - aksi cepat ke editor (`Ctrl+Enter` insert, `Alt+Enter` replace)

## Kelebihan
- Workflow coding end-to-end dalam satu TUI.
- Konfigurasi AI bisa dilakukan dari Settings.
- Integrasi context editor membuat output AI lebih relevan.

## File Kunci
- `mod.rs`: state/app loop/orchestrator.
- `ui/mod.rs`: rendering panel dan status.
- `input_router.rs`: routing keyboard.
- `terminal_controller.rs`: kontrol terminal.
- `settings_panel.rs`: logika settings.

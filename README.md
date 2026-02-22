# Xone Workspace Guide

## Ringkasan
Xone adalah editor TUI dengan panel Explorer, Editor, Terminal, Settings, dan AI Chat dalam satu workspace.

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
4. Gunakan `Ctrl+Enter` (insert) atau `Alt+Enter` (replace selection) dari jawaban AI.

## Dokumentasi Lanjutan
- Konfigurasi workspace: `.xone/README.md`
- Konfigurasi AI: `.xone/ai/README.md`
- Arsitektur source: `src/README.md`
- Panduan dan uji: `docs/README.md`

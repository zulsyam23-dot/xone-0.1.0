//! Module: src/app/editor_clipboard.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use super::helpers::{ensure_cursor_visible, line_len, shift_bookmarks_on_remove_range};
use super::{App, AppError, Focus};

impl App {
    pub(super) fn run_editor_shortcut<F>(&mut self, global: bool, action: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut Self),
    {
        if !self.prepare_editor_shortcut(global)? {
            return Ok(());
        }
        action(self);
        Ok(())
    }

    fn prepare_editor_shortcut(&mut self, global: bool) -> Result<bool, AppError> {
        if !global && !matches!(self.focus, Focus::Editor) {
            self.message = Some("Shortcut ini khusus Editor".to_string());
            return Ok(false);
        }

        if !self.editor.has_open_buffer() {
            let selected_file = self
                .explorer
                .items
                .get(self.explorer.selected)
                .filter(|item| !item.is_dir)
                .map(|item| item.path.clone());
            if let Some(path) = selected_file {
                self.open_editor_path(&path)?;
            }
        }
        if !self.editor.has_open_buffer() {
            self.message = Some("Tidak ada file terbuka di editor".to_string());
            return Ok(false);
        }
        if global {
            self.set_focus(Focus::Editor);
        }
        Ok(true)
    }

    pub(super) fn select_all_editor(&mut self) {
        let Some(buffer) = self.editor.current() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        let line_count = buffer.lines.len();
        if line_count == 0 {
            self.message = Some("Buffer kosong".to_string());
            return;
        }
        self.start_selection(Focus::Editor, 0);
        self.update_selection(Focus::Editor, line_count.saturating_sub(1));
        self.message = Some(format!("Editor: {} baris terseleksi", line_count));
    }

    pub(super) fn copy_editor_selection(&mut self) {
        let Some(copied) = self.collect_editor_selection_text() else {
            return;
        };
        let Some(buffer) = self.editor.current() else {
            return;
        };
        let last = buffer.lines.len().saturating_sub(1);
        let (start, end) = self
            .selection_range(Focus::Editor)
            .map(|(s, e)| (s.min(last), e.min(last)))
            .unwrap_or_else(|| {
                let row = buffer.cursor_row.min(last);
                (row, row)
            });
        self.store_clipboard_text(copied);
        self.message = Some(format!("Tersalin {} baris", end.saturating_sub(start) + 1));
    }

    pub(super) fn paste_editor_clipboard(&mut self) {
        let Some(content) = self.load_clipboard_text() else {
            self.message = Some("Clipboard kosong".to_string());
            return;
        };
        self.paste_editor_text(&content);
    }

    pub(super) fn delete_editor_selection(&mut self) {
        let Some(buffer) = self.editor.current() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if buffer.lines.is_empty() {
            self.message = Some("Buffer kosong".to_string());
            return;
        }
        let last = buffer.lines.len().saturating_sub(1);
        let (start, end) = self
            .selection_range(Focus::Editor)
            .map(|(s, e)| (s.min(last), e.min(last)))
            .unwrap_or_else(|| {
                let row = buffer.cursor_row.min(last);
                (row, row)
            });
        self.delete_editor_line_range(start, end);
    }

    fn collect_editor_selection_text(&mut self) -> Option<String> {
        let Some(buffer) = self.editor.current() else {
            self.message = Some("Tidak ada buffer".to_string());
            return None;
        };
        if buffer.lines.is_empty() {
            self.message = Some("Buffer kosong".to_string());
            return None;
        }
        let last = buffer.lines.len().saturating_sub(1);
        let (start, end) = self
            .selection_range(Focus::Editor)
            .map(|(s, e)| (s.min(last), e.min(last)))
            .unwrap_or_else(|| {
                let row = buffer.cursor_row.min(last);
                (row, row)
            });
        Some(buffer.lines[start..=end].join(buffer.line_ending.as_str()))
    }

    fn delete_editor_line_range(&mut self, start: usize, end: usize) {
        let view_height = self.editor_view_height;
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if buffer.lines.is_empty() {
            self.message = Some("Buffer kosong".to_string());
            return;
        }
        let end = end.min(buffer.lines.len().saturating_sub(1));
        let start = start.min(end);
        let remove_count = end.saturating_sub(start) + 1;

        buffer.push_undo();
        if remove_count >= buffer.lines.len() {
            buffer.lines.clear();
            buffer.lines.push(String::new());
            buffer.bookmarks.clear();
            buffer.cursor_row = 0;
            buffer.cursor_col = 0;
            buffer.scroll = 0;
        } else {
            buffer.lines.drain(start..=end);
            shift_bookmarks_on_remove_range(buffer, start, end);

            if buffer.cursor_row > end {
                buffer.cursor_row -= remove_count;
            } else if buffer.cursor_row >= start {
                buffer.cursor_row = start.min(buffer.lines.len().saturating_sub(1));
            }
            buffer.cursor_col = buffer
                .cursor_col
                .min(line_len(&buffer.lines[buffer.cursor_row]));
        }
        buffer.dirty = true;
        ensure_cursor_visible(buffer, view_height);
        self.selection = None;
        self.message = Some(format!("Dihapus {} baris", remove_count));
    }

    pub(super) fn store_clipboard_text(&mut self, text: String) {
        self.clipboard = text.clone();
        if text.is_empty() {
            return;
        }
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    pub(super) fn load_clipboard_text(&self) -> Option<String> {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        if self.clipboard.is_empty() {
            None
        } else {
            Some(self.clipboard.clone())
        }
    }
}

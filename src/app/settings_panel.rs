//! Module: src/app/settings_panel.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use crossterm::event::{KeyCode, KeyEvent};

use super::helpers::{char_to_byte_index, ensure_cursor_visible, insert_text};
use super::{language, App, Focus};

const SETTINGS_LAST_INDEX: usize = 14;

impl App {
    pub(super) fn open_settings_panel(&mut self) {
        if !matches!(self.focus, Focus::Settings) {
            self.focus_before_settings = self.focus;
        }
        self.set_focus(Focus::Settings);
        self.message = Some(
            "Settings: Enter/Space toggle, Up/Down pilih, F6 AI Chat, Esc/Ctrl+O/F2 untuk tutup"
                .to_string(),
        );
    }

    pub(super) fn close_settings_panel(&mut self) {
        if !matches!(self.focus, Focus::Settings) {
            return;
        }
        let target = if matches!(self.focus_before_settings, Focus::Settings) {
            Focus::Editor
        } else {
            self.focus_before_settings
        };
        self.set_focus(target);
        self.message = Some("Kembali dari Settings".to_string());
    }

    pub(super) fn toggle_settings_panel(&mut self) {
        if matches!(self.focus, Focus::Settings) {
            self.close_settings_panel();
        } else {
            self.open_settings_panel();
        }
    }

    pub(super) fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.settings.selected = self.settings.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                self.settings.selected = (self.settings.selected + 1).min(SETTINGS_LAST_INDEX);
            }
            KeyCode::PageUp => {
                self.settings.shortcut_page = self.settings.shortcut_page.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.settings.shortcut_page = self.settings.shortcut_page.saturating_add(1);
            }
            KeyCode::Home => {
                self.settings.shortcut_page = 0;
            }
            KeyCode::Left | KeyCode::Right => {
                match self.settings.selected {
                    2 => self.cycle_appearance(),
                    3 => self.cycle_accent(),
                    4 => self.cycle_tab_theme(),
                    5 => self.cycle_syntax_theme(),
                    6 => self.cycle_density(),
                    10 => self.cycle_ai_profile(),
                    11 => self.cycle_ai_provider(),
                    12 => self.cycle_ai_base_url(),
                    13 => self.cycle_ai_model(),
                    _ => {}
                }
            }
            KeyCode::Char('1') => self.set_ai_profile(super::AiProfile::Groq),
            KeyCode::Char('2') => self.set_ai_profile(super::AiProfile::Together),
            KeyCode::Char('3') => self.set_ai_profile(super::AiProfile::OpenRouter),
            KeyCode::Delete if self.settings.selected == 14 => self.clear_ai_api_key(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.toggle_selected_setting();
            }
            _ => {}
        }
    }

    fn toggle_selected_setting(&mut self) {
        match self.settings.selected {
            0 => {
                self.settings.syntax_highlight = !self.settings.syntax_highlight;
                self.message = Some(format!(
                    "Syntax Highlight: {}",
                    if self.settings.syntax_highlight {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
            }
            1 => {
                self.settings.code_suggestions = !self.settings.code_suggestions;
                self.suggestion_index = 0;
                self.message = Some(format!(
                    "Code Suggestion: {}",
                    if self.settings.code_suggestions {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
            }
            2 => {
                self.cycle_appearance();
            }
            3 => {
                self.cycle_accent();
            }
            4 => {
                self.cycle_tab_theme();
            }
            5 => {
                self.cycle_syntax_theme();
            }
            6 => {
                self.cycle_density();
            }
            7 => {
                self.toggle_terminal_fx();
            }
            8 => {
                self.toggle_terminal_command_sync();
            }
            9 => {
                self.toggle_hard_mode();
            }
            10 => self.cycle_ai_profile(),
            11 => self.cycle_ai_provider(),
            12 => self.cycle_ai_base_url(),
            13 => self.cycle_ai_model(),
            14 => self.set_ai_api_key_from_clipboard(),
            _ => {}
        }
    }

    fn set_ai_profile(&mut self, profile: super::AiProfile) {
        self.settings.ai_profile = profile;
        let previous_key = self.ai_chat.config.api_key.clone();
        self.ai_chat.config = super::config_for_profile(profile, &previous_key);
        self.sync_ai_flags_from_config();
        self.persist_ai_config(format!("AI Profile: {}", profile.label()));
    }

    fn cycle_ai_profile(&mut self) {
        let next = self.settings.ai_profile.next();
        self.set_ai_profile(next);
    }

    fn cycle_ai_provider(&mut self) {
        let next = "openai_compatible";
        self.ai_chat.config.provider = next.to_string();
        self.sync_ai_flags_from_config();
        self.persist_ai_config(format!("AI Provider: {}", next));
    }

    fn cycle_ai_base_url(&mut self) {
        let current = self.ai_chat.config.base_url.trim().to_ascii_lowercase();
        let next = if current.contains("api.groq.com") {
            "https://api.together.xyz"
        } else if current.contains("api.together.xyz") {
            "https://openrouter.ai/api"
        } else {
            "https://api.groq.com/openai"
        };
        self.ai_chat.config.base_url = next.to_string();
        self.sync_ai_flags_from_config();
        self.persist_ai_config(format!("AI Base URL: {}", next));
    }

    fn cycle_ai_model(&mut self) {
        let model = self.ai_chat.config.model.trim().to_ascii_lowercase();
        let next = if model == "llama-3.1-8b-instant" {
            "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo"
        } else if model == "meta-llama/meta-llama-3.1-8b-instruct-turbo" {
            "openai/gpt-4o-mini"
        } else {
            "llama-3.1-8b-instant"
        };
        self.ai_chat.config.model = next.to_string();
        self.persist_ai_config(format!("AI Model: {}", next));
    }

    fn set_ai_api_key_from_clipboard(&mut self) {
        let Some(value) = self.load_clipboard_text() else {
            self.message = Some("Clipboard kosong".to_string());
            return;
        };
        let key = value.trim();
        if key.is_empty() {
            self.message = Some("API key dari clipboard kosong".to_string());
            return;
        }
        self.ai_chat.config.api_key = key.to_string();
        self.sync_ai_flags_from_config();
        self.persist_ai_config("AI API key diperbarui dari clipboard".to_string());
    }

    fn clear_ai_api_key(&mut self) {
        self.ai_chat.config.api_key.clear();
        self.sync_ai_flags_from_config();
        self.persist_ai_config("AI API key dibersihkan".to_string());
    }

    fn sync_ai_flags_from_config(&mut self) {
        self.settings.ai_profile = super::infer_ai_profile(&self.ai_chat.config);
        self.ai_chat.api_key_present = !self.ai_chat.config.api_key.trim().is_empty();
    }

    fn persist_ai_config(&mut self, ok_message: String) {
        match super::save_ai_config(self.workspace.root(), &self.ai_chat.config) {
            Ok(_) => {
                self.message = Some(ok_message);
            }
            Err(error) => {
                self.message = Some(format!("Gagal simpan AI config: {}", error));
            }
        }
    }

    fn cycle_appearance(&mut self) {
        let next = self.settings.appearance.next();
        self.settings.appearance = next;
        self.theme.set_preset(next);
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!("Appearance: {}", next.label()));
            }
            Err(error) => {
                self.message = Some(format!(
                    "Appearance: {} (gagal simpan config: {})",
                    next.label(),
                    error
                ));
            }
        }
    }

    fn cycle_accent(&mut self) {
        let next = self.settings.accent.next();
        self.settings.accent = next;
        self.theme.set_accent(next);
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!("Accent: {}", next.label()));
            }
            Err(error) => {
                self.message = Some(format!("Accent: {} (gagal simpan config: {})", next.label(), error));
            }
        }
    }

    fn cycle_tab_theme(&mut self) {
        let next = self.settings.tab_theme.next();
        self.settings.tab_theme = next;
        self.theme.set_tab_theme(next);
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!("Tab Theme: {}", next.label()));
            }
            Err(error) => {
                self.message = Some(format!(
                    "Tab Theme: {} (gagal simpan config: {})",
                    next.label(),
                    error
                ));
            }
        }
    }

    fn cycle_syntax_theme(&mut self) {
        let next = self.settings.syntax_theme.next();
        self.settings.syntax_theme = next;
        self.theme.set_syntax_theme(next);
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!("Syntax Theme: {}", next.label()));
            }
            Err(error) => {
                self.message = Some(format!(
                    "Syntax Theme: {} (gagal simpan config: {})",
                    next.label(),
                    error
                ));
            }
        }
    }

    fn cycle_density(&mut self) {
        let next = self.settings.density.next();
        self.settings.density = next;
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!("Density: {}", next.label()));
            }
            Err(error) => {
                self.message =
                    Some(format!("Density: {} (gagal simpan config: {})", next.label(), error));
            }
        }
    }

    fn toggle_terminal_fx(&mut self) {
        self.settings.terminal_fx = !self.settings.terminal_fx;
        self.terminal.set_enhanced_prompt(self.settings.terminal_fx);
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!(
                    "Terminal Style FX: {}",
                    if self.settings.terminal_fx { "ON" } else { "OFF" }
                ));
            }
            Err(error) => {
                self.message = Some(format!(
                    "Terminal Style FX gagal disimpan: {}",
                    error
                ));
            }
        }
    }

    fn toggle_hard_mode(&mut self) {
        self.settings.hard_mode = !self.settings.hard_mode;
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!(
                    "Hard Mode: {} (mouse {})",
                    if self.settings.hard_mode { "ON" } else { "OFF" },
                    if self.settings.hard_mode { "OFF" } else { "ON" }
                ));
            }
            Err(error) => {
                self.message = Some(format!("Hard Mode gagal disimpan: {}", error));
            }
        }
    }

    fn toggle_terminal_command_sync(&mut self) {
        self.settings.terminal_command_sync = !self.settings.terminal_command_sync;
        self.terminal_command_tracking_valid = self.settings.terminal_command_sync;
        if !self.settings.terminal_command_sync {
            self.terminal_command_buffer.clear();
        }
        match super::save_ui_preferences(
            self.workspace.root(),
            self.settings.appearance,
            self.settings.accent,
            self.settings.tab_theme,
            self.settings.syntax_theme,
            self.settings.density,
            self.settings.terminal_fx,
            self.settings.terminal_command_sync,
            self.settings.hard_mode,
            self.settings.adaptive_terminal_height,
        ) {
            Ok(_) => {
                self.message = Some(format!(
                    "Terminal Command Auto Sync: {}",
                    if self.settings.terminal_command_sync { "ON" } else { "OFF" }
                ));
            }
            Err(error) => {
                self.message = Some(format!(
                    "Terminal Command Auto Sync gagal disimpan: {}",
                    error
                ));
            }
        }
    }

    pub(super) fn current_editor_suggestions(&self) -> Vec<language::Suggestion> {
        if !self.settings.code_suggestions {
            return Vec::new();
        }
        let Some(buffer) = self.editor.current() else {
            return Vec::new();
        };
        let Some(line) = buffer.lines.get(buffer.cursor_row) else {
            return Vec::new();
        };
        let byte = char_to_byte_index(line, buffer.cursor_col);
        let before_cursor = &line[..byte];
        language::suggestions_for_line(buffer.language, before_cursor)
    }

    pub(super) fn current_editor_suggestion_entry(
        &self,
    ) -> Option<(language::Suggestion, usize, usize)> {
        let suggestions = self.current_editor_suggestions();
        if suggestions.is_empty() {
            return None;
        }
        let total = suggestions.len();
        let index = self.suggestion_index % total;
        Some((suggestions[index], index, total))
    }

    pub(super) fn next_editor_suggestion(&mut self) {
        if !self.settings.code_suggestions {
            self.message = Some("Code suggestion dimatikan di Settings".to_string());
            return;
        }
        let suggestions = self.current_editor_suggestions();
        if suggestions.is_empty() {
            self.message = Some("Tidak ada suggestion di posisi ini".to_string());
            return;
        }
        self.suggestion_index = (self.suggestion_index + 1) % suggestions.len();
        let selected = suggestions[self.suggestion_index];
        self.message = Some(format!(
            "Suggestion {}/{}: {}",
            self.suggestion_index + 1,
            suggestions.len(),
            selected.label
        ));
    }

    pub(super) fn apply_editor_suggestion(&mut self) {
        if !self.settings.code_suggestions {
            self.message = Some("Code suggestion dimatikan di Settings".to_string());
            return;
        }
        let Some((suggestion, index, total)) = self.current_editor_suggestion_entry() else {
            self.message = Some("Tidak ada suggestion di posisi ini".to_string());
            return;
        };
        let view_height = self.editor_view_height;
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        buffer.push_undo();
        insert_text(buffer, suggestion.insert);
        buffer.dirty = true;
        ensure_cursor_visible(buffer, view_height);
        self.selection = None;
        self.message = Some(format!(
            "Suggestion diterapkan ({}/{}): {}",
            index + 1,
            total,
            suggestion.label
        ));
        self.suggestion_index = 0;
    }
}



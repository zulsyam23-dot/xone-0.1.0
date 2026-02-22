//! Shortcut panel + custom keybind persistence.

use crossterm::event::{KeyCode, KeyEvent};

use super::{control, App, Focus};

impl App {
    pub(super) fn open_shortcuts_panel(&mut self) {
        if !matches!(self.focus, Focus::Shortcuts) {
            self.focus_before_shortcuts = self.focus;
        }
        self.shortcuts_capture_mode = false;
        self.set_focus(Focus::Shortcuts);
        self.message = Some(
            "Shortcuts: Up/Down pilih, Enter ubah, Delete reset default, Esc/F3 tutup".to_string(),
        );
    }

    pub(super) fn close_shortcuts_panel(&mut self) {
        if !matches!(self.focus, Focus::Shortcuts) {
            return;
        }
        self.shortcuts_capture_mode = false;
        let target = if self.editor.has_open_buffer() {
            Focus::Editor
        } else {
            Focus::Explorer
        };
        self.set_focus(target);
        self.message = Some("Kembali dari Shortcuts".to_string());
    }

    pub(super) fn toggle_shortcuts_panel(&mut self) {
        if matches!(self.focus, Focus::Shortcuts) {
            self.close_shortcuts_panel();
        } else {
            self.open_shortcuts_panel();
        }
    }

    pub(super) fn handle_shortcuts_key(&mut self, key: KeyEvent) {
        if self.shortcuts_capture_mode {
            if key.code == KeyCode::Esc {
                self.shortcuts_capture_mode = false;
                self.message = Some("Capture shortcut dibatalkan".to_string());
                return;
            }
            let Some(binding) = control::binding_from_event(&key) else {
                return;
            };
            let actions = control::customizable_actions();
            if actions.is_empty() {
                return;
            }
            let index = self.shortcuts_selected.min(actions.len().saturating_sub(1));
            let action = actions[index];

            if let Some(other_index) = self
                .shortcut_bindings
                .iter()
                .position(|(existing_action, existing_binding)| {
                    *existing_action != action && *existing_binding == binding
                })
            {
                let other_action = self.shortcut_bindings[other_index].0;
                self.shortcut_bindings[other_index].1 = control::default_binding(other_action);
            }
            if let Some((_, selected_binding)) = self
                .shortcut_bindings
                .iter_mut()
                .find(|(existing_action, _)| *existing_action == action)
            {
                *selected_binding = binding;
            }
            self.shortcuts_capture_mode = false;
            super::save_shortcut_bindings(self.workspace.root(), &self.shortcut_bindings);
            self.message = Some(format!(
                "Shortcut {} => {}",
                control::action_label(action),
                control::binding_to_text(binding)
            ));
            return;
        }

        let total = control::customizable_actions().len();
        match key.code {
            KeyCode::Up => {
                self.shortcuts_selected = self.shortcuts_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                self.shortcuts_selected = (self.shortcuts_selected + 1).min(total.saturating_sub(1));
            }
            KeyCode::Enter => {
                self.shortcuts_capture_mode = true;
                self.message = Some("Tekan kombinasi tombol baru untuk action ini".to_string());
            }
            KeyCode::Delete | KeyCode::Backspace => {
                if total == 0 {
                    return;
                }
                let action = control::customizable_actions()
                    [self.shortcuts_selected.min(total.saturating_sub(1))];
                if let Some((_, binding)) = self
                    .shortcut_bindings
                    .iter_mut()
                    .find(|(existing_action, _)| *existing_action == action)
                {
                    *binding = control::default_binding(action);
                }
                super::save_shortcut_bindings(self.workspace.root(), &self.shortcut_bindings);
                self.message = Some(format!("Shortcut {} di-reset default", control::action_label(action)));
            }
            _ => {}
        }
    }

    pub(super) fn resolve_control_action(&self, key: &KeyEvent) -> Option<control::ControlAction> {
        if let Some((action, _)) = self
            .shortcut_bindings
            .iter()
            .find(|(_, binding)| binding.matches(key))
        {
            return Some(*action);
        }
        control::action_for(key)
    }
}

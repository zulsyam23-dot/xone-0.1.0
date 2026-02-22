//! Module: src/app/input_router.rs
//! Catatan: router input dipisah supaya `mod.rs` nggak jadi gudang kabel keyboard.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{
    control, is_editor_text_input_key, is_explorer_action_key, is_prompt_bypass_control, App,
    AppError, Focus,
};

pub(super) fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if matches!(key.kind, KeyEventKind::Release) {
        return Ok(());
    }
    if matches!(app.focus, Focus::Editor) {
        if !is_editor_text_input_key(&key) {
            app.flush_pending_editor_paste_if_ready(true);
        }
    } else {
        app.flush_pending_editor_paste_if_ready(true);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
    {
        app.open_new_terminal_tab();
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && matches!(key.code, KeyCode::Char('f') | KeyCode::Char('F'))
    {
        app.open_workspace_search();
        return Ok(());
    }
    if app.handle_workspace_search_key(key) {
        return Ok(());
    }
    if key.modifiers.is_empty() && matches!(key.code, KeyCode::F(6)) {
        app.focus_ai_chat();
        return Ok(());
    }
    if key.modifiers.is_empty() && matches!(key.code, KeyCode::F(1)) {
        app.toggle_about_panel();
        return Ok(());
    }
    if key.modifiers.is_empty() && matches!(key.code, KeyCode::F(3)) {
        app.toggle_shortcuts_panel();
        return Ok(());
    }
    if matches!(key.code, KeyCode::F(8))
        && app.settings.hard_mode
        && app.explorer_prompt.is_none()
        && !app.terminal_search_mode
        && !matches!(app.focus, Focus::Settings | Focus::Shortcuts | Focus::About)
    {
        app.cycle_keyboard_focus(key.modifiers.contains(KeyModifiers::SHIFT));
        return Ok(());
    }
    if matches!(key.code, KeyCode::F(2)) {
        let action = control::ControlAction::OpenSettings;
        apply_control_action(app, &key, action)?;
        return Ok(());
    }
    if app.apply_command_hook(&key)? {
        return Ok(());
    }
    if matches!(app.focus, Focus::Settings) {
        if key.code == KeyCode::Esc {
            app.close_settings_panel();
            return Ok(());
        }
        if let Some(action) = app.resolve_control_action(&key) {
            apply_control_action(app, &key, action)?;
            return Ok(());
        }
        app.handle_settings_key(key);
        return Ok(());
    }
    if matches!(app.focus, Focus::Shortcuts) {
        if key.code == KeyCode::Esc || key.code == KeyCode::F(3) {
            app.close_shortcuts_panel();
            return Ok(());
        }
        app.handle_shortcuts_key(key);
        return Ok(());
    }
    if app.explorer_prompt.is_some() {
        if let Some(action) = app.resolve_control_action(&key) {
            if app.should_skip_control_action(&key, action) {
                return Ok(());
            }
            if is_prompt_bypass_control(action) {
                app.explorer_prompt = None;
                apply_control_action(app, &key, action)?;
                return Ok(());
            }
        }
        app.handle_explorer_prompt_key(key)?;
        return Ok(());
    }
    if matches!(app.focus, Focus::Terminal) {
        if app.terminal_search_mode && app.handle_terminal_search_key(key) {
            return Ok(());
        }
        if let Some(action) = app.resolve_control_action(&key) {
            if is_global_focus_action(action) {
                apply_control_action(app, &key, action)?;
                return Ok(());
            }
        }
        if key.code == KeyCode::Esc {
            if app.editor.has_open_buffer() {
                app.set_focus(Focus::Editor);
            } else {
                app.set_focus(Focus::Explorer);
            }
            return Ok(());
        }
        app.handle_terminal_key(key);
        return Ok(());
    }
    if matches!(app.focus, Focus::AiChat) {
        if key.code == KeyCode::Tab {
            if app.editor.has_open_buffer() {
                app.set_focus(Focus::Editor);
            }
            return Ok(());
        }
        app.handle_ai_chat_key(key);
        return Ok(());
    }
    if matches!(app.focus, Focus::About) {
        app.handle_about_key(key);
        return Ok(());
    }
    if key.code == KeyCode::Esc {
        app.handle_escape_action();
        return Ok(());
    }
    if matches!(app.focus, Focus::Editor)
        && key.modifiers.is_empty()
        && matches!(key.code, KeyCode::Char('\t'))
    {
        app.handle_editor_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
        return Ok(());
    }
    if let Some(action) = app.resolve_control_action(&key) {
        if is_explorer_only_action(action) && !matches!(app.focus, Focus::Explorer) {
            // Shortcut create file/folder hanya valid saat fokus Explorer.
        } else {
            apply_control_action(app, &key, action)?;
            return Ok(());
        }
    }
    if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
        app.request_quit(false);
        return Ok(());
    }
    if matches!(app.focus, Focus::Explorer) {
        if key.code == KeyCode::Tab {
            if app.editor.has_open_buffer() {
                app.set_focus(Focus::Editor);
            }
            return Ok(());
        }
        if matches!(key.code, KeyCode::Char(_))
            && !key.modifiers.contains(KeyModifiers::CONTROL)
            && app.editor.has_open_buffer()
            && !is_explorer_action_key(key.code)
        {
            app.set_focus(Focus::Editor);
            app.handle_editor_key(key)?;
            return Ok(());
        }
    }

    match app.focus {
        Focus::Explorer => app.handle_explorer_key(key)?,
        Focus::AiChat => app.handle_ai_chat_key(key),
        Focus::About => app.handle_about_key(key),
        Focus::Editor => app.handle_editor_key(key)?,
        Focus::Settings | Focus::Shortcuts | Focus::Terminal => {}
    }
    Ok(())
}

fn apply_control_action(
    app: &mut App,
    key: &KeyEvent,
    action: control::ControlAction,
) -> Result<(), AppError> {
    if app.should_skip_control_action(key, action) {
        return Ok(());
    }
    app.mark_control_action(action);
    app.apply_control(action)
}

fn is_global_focus_action(action: control::ControlAction) -> bool {
    matches!(
        action,
        control::ControlAction::FocusExplorer
            | control::ControlAction::FocusEditor
            | control::ControlAction::OpenSettings
            | control::ControlAction::ToggleExplorer
            | control::ControlAction::ToggleTerminalPanel
    )
}

fn is_explorer_only_action(action: control::ControlAction) -> bool {
    matches!(
        action,
        control::ControlAction::CreateFile | control::ControlAction::CreateFolder
    )
}

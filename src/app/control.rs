//! Module: src/app/control.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    FocusExplorer,
    FocusEditor,
    OpenSettings,
    ToggleExplorer,
    ToggleTerminalPanel,
    Save,
    CloseBuffer,
    Quit,
    Undo,
    Redo,
    ToggleBookmark,
    NextBookmark,
    PrevBookmark,
    NextBuffer,
    PrevBuffer,
    EditorSelectAll,
    EditorCopy,
    EditorPaste,
    EditorCut,
    EditorDeleteSelection,
    GlobalEditorSelectAll,
    GlobalEditorCopy,
    GlobalEditorPaste,
    GlobalEditorCut,
    GlobalEditorDeleteSelection,
    AcceptSuggestion,
    NextSuggestion,
    CreateFile,
    CreateFolder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl ShortcutBinding {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        normalize_ascii_letter(key.code) == normalize_ascii_letter(self.code)
            && key.modifiers == self.modifiers
    }
}

pub fn action_for(key: &KeyEvent) -> Option<ControlAction> {
    action_for_code(key.code, key.modifiers).or_else(|| {
        if !key.modifiers.is_empty() {
            return None;
        }
        normalize_control_char(key.code)
            .and_then(|code| action_for_code(code, KeyModifiers::CONTROL))
    })
}

fn action_for_code(code: KeyCode, modifiers: KeyModifiers) -> Option<ControlAction> {
    if matches!(code, KeyCode::F(2)) && modifiers.is_empty() {
        return Some(ControlAction::OpenSettings);
    }
    if modifiers.is_empty() {
        return match code {
            KeyCode::Char('n') => Some(ControlAction::CreateFile),
            KeyCode::Char('N') => Some(ControlAction::CreateFolder),
            _ => None,
        };
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        let normalized = normalize_ascii_letter(code);
        return match normalized {
            KeyCode::Char('e') => Some(ControlAction::FocusExplorer),
            KeyCode::Char('i') => Some(ControlAction::FocusEditor),
            KeyCode::Char(',') => Some(ControlAction::OpenSettings),
            KeyCode::Char('o') => Some(ControlAction::OpenSettings),
            KeyCode::Char('b') => Some(ControlAction::ToggleExplorer),
            KeyCode::Char('t') => Some(ControlAction::ToggleTerminalPanel),
            KeyCode::Char('s') => Some(ControlAction::Save),
            KeyCode::Char('w') => Some(ControlAction::CloseBuffer),
            KeyCode::Char('q') => Some(ControlAction::Quit),
            KeyCode::Char('z') => Some(ControlAction::Undo),
            KeyCode::Char('y') => Some(ControlAction::Redo),
            KeyCode::Char('m') => Some(ControlAction::ToggleBookmark),
            KeyCode::Char('n') => Some(ControlAction::NextBookmark),
            KeyCode::PageDown => Some(ControlAction::NextBuffer),
            KeyCode::PageUp => Some(ControlAction::PrevBuffer),
            KeyCode::Right => Some(ControlAction::NextBuffer),
            KeyCode::Left => Some(ControlAction::PrevBuffer),
            KeyCode::Char('p') => Some(ControlAction::PrevBookmark),
            KeyCode::Char('a') => Some(ControlAction::EditorSelectAll),
            KeyCode::Char('c') => Some(ControlAction::EditorCopy),
            KeyCode::Char('v') => Some(ControlAction::EditorPaste),
            KeyCode::Char('x') => Some(ControlAction::EditorCut),
            KeyCode::Char('d') => Some(ControlAction::EditorDeleteSelection),
            KeyCode::Char('j') => Some(ControlAction::AcceptSuggestion),
            KeyCode::Char('k') => Some(ControlAction::NextSuggestion),
            KeyCode::Insert => Some(ControlAction::EditorCopy),
            KeyCode::Tab => Some(ControlAction::FocusEditor),
            KeyCode::BackTab => Some(ControlAction::PrevBuffer),
            _ => None,
        };
    }

    if modifiers.contains(KeyModifiers::ALT) {
        let normalized = normalize_ascii_letter(code);
        return match normalized {
            KeyCode::Char('a') => Some(ControlAction::GlobalEditorSelectAll),
            KeyCode::Char('c') => Some(ControlAction::GlobalEditorCopy),
            KeyCode::Char('v') => Some(ControlAction::GlobalEditorPaste),
            KeyCode::Char('x') => Some(ControlAction::GlobalEditorCut),
            KeyCode::Char('d') => Some(ControlAction::GlobalEditorDeleteSelection),
            _ => None,
        };
    }

    if modifiers.contains(KeyModifiers::SHIFT) {
        return match code {
            KeyCode::Insert => Some(ControlAction::GlobalEditorPaste),
            KeyCode::Delete => Some(ControlAction::GlobalEditorCut),
            _ => None,
        };
    }

    None
}

fn normalize_ascii_letter(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(ch) => KeyCode::Char(ch.to_ascii_lowercase()),
        other => other,
    }
}

fn normalize_control_char(code: KeyCode) -> Option<KeyCode> {
    let KeyCode::Char(ch) = code else {
        return None;
    };
    let value = ch as u32;
    if !(1..=26).contains(&value) {
        return None;
    }
    let letter = char::from_u32(u32::from(b'a') + value - 1)?;
    Some(KeyCode::Char(letter))
}

const CUSTOMIZABLE_ACTIONS: [ControlAction; 18] = [
    ControlAction::FocusExplorer,
    ControlAction::FocusEditor,
    ControlAction::OpenSettings,
    ControlAction::ToggleExplorer,
    ControlAction::ToggleTerminalPanel,
    ControlAction::Save,
    ControlAction::CloseBuffer,
    ControlAction::Undo,
    ControlAction::Redo,
    ControlAction::NextBuffer,
    ControlAction::PrevBuffer,
    ControlAction::EditorCopy,
    ControlAction::EditorPaste,
    ControlAction::EditorCut,
    ControlAction::AcceptSuggestion,
    ControlAction::NextSuggestion,
    ControlAction::CreateFile,
    ControlAction::CreateFolder,
];

pub fn customizable_actions() -> &'static [ControlAction] {
    &CUSTOMIZABLE_ACTIONS
}

pub fn action_label(action: ControlAction) -> &'static str {
    match action {
        ControlAction::FocusExplorer => "Focus Explorer",
        ControlAction::FocusEditor => "Focus Editor",
        ControlAction::OpenSettings => "Open Settings",
        ControlAction::ToggleExplorer => "Toggle Explorer",
        ControlAction::ToggleTerminalPanel => "Toggle Terminal",
        ControlAction::Save => "Save File",
        ControlAction::CloseBuffer => "Close Buffer",
        ControlAction::Undo => "Undo",
        ControlAction::Redo => "Redo",
        ControlAction::NextBuffer => "Next Buffer",
        ControlAction::PrevBuffer => "Prev Buffer",
        ControlAction::EditorCopy => "Copy",
        ControlAction::EditorPaste => "Paste",
        ControlAction::EditorCut => "Cut",
        ControlAction::AcceptSuggestion => "Accept Suggestion",
        ControlAction::NextSuggestion => "Next Suggestion",
        ControlAction::CreateFile => "Create File",
        ControlAction::CreateFolder => "Create Folder",
        _ => "Action",
    }
}

pub fn action_config_key(action: ControlAction) -> &'static str {
    match action {
        ControlAction::FocusExplorer => "focus_explorer",
        ControlAction::FocusEditor => "focus_editor",
        ControlAction::OpenSettings => "open_settings",
        ControlAction::ToggleExplorer => "toggle_explorer",
        ControlAction::ToggleTerminalPanel => "toggle_terminal",
        ControlAction::Save => "save",
        ControlAction::CloseBuffer => "close_buffer",
        ControlAction::Undo => "undo",
        ControlAction::Redo => "redo",
        ControlAction::NextBuffer => "next_buffer",
        ControlAction::PrevBuffer => "prev_buffer",
        ControlAction::EditorCopy => "copy",
        ControlAction::EditorPaste => "paste",
        ControlAction::EditorCut => "cut",
        ControlAction::AcceptSuggestion => "accept_suggestion",
        ControlAction::NextSuggestion => "next_suggestion",
        ControlAction::CreateFile => "create_file",
        ControlAction::CreateFolder => "create_folder",
        _ => "unknown",
    }
}

pub fn action_from_config_key(value: &str) -> Option<ControlAction> {
    match value.trim().to_ascii_lowercase().as_str() {
        "focus_explorer" => Some(ControlAction::FocusExplorer),
        "focus_editor" => Some(ControlAction::FocusEditor),
        "open_settings" => Some(ControlAction::OpenSettings),
        "toggle_explorer" => Some(ControlAction::ToggleExplorer),
        "toggle_terminal" => Some(ControlAction::ToggleTerminalPanel),
        "save" => Some(ControlAction::Save),
        "close_buffer" => Some(ControlAction::CloseBuffer),
        "undo" => Some(ControlAction::Undo),
        "redo" => Some(ControlAction::Redo),
        "next_buffer" => Some(ControlAction::NextBuffer),
        "prev_buffer" => Some(ControlAction::PrevBuffer),
        "copy" => Some(ControlAction::EditorCopy),
        "paste" => Some(ControlAction::EditorPaste),
        "cut" => Some(ControlAction::EditorCut),
        "accept_suggestion" => Some(ControlAction::AcceptSuggestion),
        "next_suggestion" => Some(ControlAction::NextSuggestion),
        "create_file" => Some(ControlAction::CreateFile),
        "create_folder" => Some(ControlAction::CreateFolder),
        _ => None,
    }
}

pub fn default_binding(action: ControlAction) -> ShortcutBinding {
    let text = match action {
        ControlAction::FocusExplorer => "ctrl+e",
        ControlAction::FocusEditor => "ctrl+i",
        ControlAction::OpenSettings => "ctrl+o",
        ControlAction::ToggleExplorer => "ctrl+b",
        ControlAction::ToggleTerminalPanel => "ctrl+t",
        ControlAction::Save => "ctrl+s",
        ControlAction::CloseBuffer => "ctrl+w",
        ControlAction::Undo => "ctrl+z",
        ControlAction::Redo => "ctrl+y",
        ControlAction::NextBuffer => "ctrl+pgdn",
        ControlAction::PrevBuffer => "ctrl+pgup",
        ControlAction::EditorCopy => "ctrl+c",
        ControlAction::EditorPaste => "ctrl+v",
        ControlAction::EditorCut => "ctrl+x",
        ControlAction::AcceptSuggestion => "ctrl+j",
        ControlAction::NextSuggestion => "ctrl+k",
        ControlAction::CreateFile => "n",
        ControlAction::CreateFolder => "shift+n",
        _ => "ctrl+o",
    };
    parse_binding(text).unwrap_or(ShortcutBinding {
        code: KeyCode::Char('o'),
        modifiers: KeyModifiers::CONTROL,
    })
}

pub fn binding_to_text(binding: ShortcutBinding) -> String {
    let mut parts: Vec<String> = Vec::new();
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("shift".to_string());
    }
    let key = match binding.code {
        KeyCode::Char(ch) => ch.to_ascii_lowercase().to_string(),
        KeyCode::F(n) => format!("f{}", n),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "backtab".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::PageUp => "pgup".to_string(),
        KeyCode::PageDown => "pgdn".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::Esc => "esc".to_string(),
        _ => "key".to_string(),
    };
    parts.push(key);
    parts.join("+")
}

pub fn binding_from_event(key: &KeyEvent) -> Option<ShortcutBinding> {
    match key.code {
        KeyCode::Null | KeyCode::CapsLock | KeyCode::ScrollLock | KeyCode::NumLock => None,
        _ => Some(ShortcutBinding {
            code: normalize_ascii_letter(key.code),
            modifiers: key.modifiers,
        }),
    }
}

pub fn parse_binding(value: &str) -> Option<ShortcutBinding> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut modifiers = KeyModifiers::NONE;
    let mut code = None;
    for part in trimmed.split('+') {
        let token = part.trim().to_ascii_lowercase();
        match token.as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "tab" => code = Some(KeyCode::Tab),
            "backtab" => code = Some(KeyCode::BackTab),
            "enter" => code = Some(KeyCode::Enter),
            "backspace" => code = Some(KeyCode::Backspace),
            "delete" | "del" => code = Some(KeyCode::Delete),
            "insert" | "ins" => code = Some(KeyCode::Insert),
            "pgup" | "pageup" => code = Some(KeyCode::PageUp),
            "pgdn" | "pagedown" => code = Some(KeyCode::PageDown),
            "up" => code = Some(KeyCode::Up),
            "down" => code = Some(KeyCode::Down),
            "left" => code = Some(KeyCode::Left),
            "right" => code = Some(KeyCode::Right),
            "home" => code = Some(KeyCode::Home),
            "end" => code = Some(KeyCode::End),
            "esc" | "escape" => code = Some(KeyCode::Esc),
            "comma" => code = Some(KeyCode::Char(',')),
            _ if token.starts_with('f') && token.len() > 1 => {
                let num = token[1..].parse::<u8>().ok()?;
                code = Some(KeyCode::F(num));
            }
            _ if token.len() == 1 => {
                code = Some(KeyCode::Char(token.chars().next()?));
            }
            _ => return None,
        }
    }
    Some(ShortcutBinding {
        code: normalize_ascii_letter(code?),
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn ctrl_shortcuts_are_mapped() {
        assert_eq!(
            action_for(&key(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(ControlAction::EditorSelectAll)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('E'), KeyModifiers::CONTROL)),
            Some(ControlAction::FocusExplorer)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char(','), KeyModifiers::CONTROL)),
            Some(ControlAction::OpenSettings)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('o'), KeyModifiers::CONTROL)),
            Some(ControlAction::OpenSettings)
        );
        assert_eq!(
            action_for(&key(KeyCode::F(2), KeyModifiers::NONE)),
            Some(ControlAction::OpenSettings)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(ControlAction::EditorCopy)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            Some(ControlAction::EditorPaste)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            Some(ControlAction::EditorCut)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(ControlAction::EditorDeleteSelection)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('j'), KeyModifiers::CONTROL)),
            Some(ControlAction::AcceptSuggestion)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('k'), KeyModifiers::CONTROL)),
            Some(ControlAction::NextSuggestion)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('t'), KeyModifiers::CONTROL)),
            Some(ControlAction::ToggleTerminalPanel)
        );
    }

    #[test]
    fn alt_and_insert_fallback_shortcuts_are_mapped() {
        assert_eq!(
            action_for(&key(KeyCode::Char('v'), KeyModifiers::ALT)),
            Some(ControlAction::GlobalEditorPaste)
        );
        assert_eq!(
            action_for(&key(KeyCode::Insert, KeyModifiers::SHIFT)),
            Some(ControlAction::GlobalEditorPaste)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('d'), KeyModifiers::ALT)),
            Some(ControlAction::GlobalEditorDeleteSelection)
        );
        assert_eq!(
            action_for(&key(KeyCode::Delete, KeyModifiers::SHIFT)),
            Some(ControlAction::GlobalEditorCut)
        );
    }

    #[test]
    fn create_item_shortcuts_are_mapped() {
        assert_eq!(
            action_for(&key(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(ControlAction::CreateFile)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('N'), KeyModifiers::SHIFT)),
            Some(ControlAction::CreateFolder)
        );
    }

    #[test]
    fn raw_control_chars_are_mapped_to_ctrl_shortcuts() {
        assert_eq!(
            action_for(&key(KeyCode::Char('\u{5}'), KeyModifiers::NONE)),
            Some(ControlAction::FocusExplorer)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('\u{9}'), KeyModifiers::NONE)),
            Some(ControlAction::FocusEditor)
        );
        assert_eq!(
            action_for(&key(KeyCode::Char('\u{14}'), KeyModifiers::NONE)),
            Some(ControlAction::ToggleTerminalPanel)
        );
    }
}

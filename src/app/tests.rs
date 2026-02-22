//! Module: src/app/tests.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use super::*;
use std::fs;

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("xone_test_{}_{}", name, nanos));
    dir
}

fn make_buffer(line: &str) -> EditorBuffer {
    EditorBuffer {
        path: PathBuf::from("dummy.txt"),
        language: language::Language::PlainText,
        lines: vec![line.to_string()],
        cursor_row: 0,
        cursor_col: line.chars().count(),
        scroll: 0,
        hscroll: 0,
        dirty: false,
        bookmarks: BTreeSet::new(),
        line_ending: LineEnding::Lf,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
    }
}

#[test]
fn detects_line_endings() {
    assert_eq!(detect_line_ending("a\r\nb\r\n"), LineEnding::Crlf);
    assert_eq!(detect_line_ending("a\nb\n"), LineEnding::Lf);
}

#[test]
fn undo_redo_restores_content() {
    let mut buffer = make_buffer("");
    insert_char(&mut buffer, 'x');
    assert_eq!(buffer.lines[0], "x");

    assert!(buffer.undo());
    assert_eq!(buffer.lines[0], "");

    assert!(buffer.redo());
    assert_eq!(buffer.lines[0], "x");
}

#[test]
fn bookmark_toggle_and_navigation_wraps() {
    let mut buffer = make_buffer("line");
    buffer.lines = vec!["a".into(), "b".into(), "c".into()];
    buffer.cursor_row = 0;
    assert!(buffer.toggle_bookmark());
    buffer.cursor_row = 2;
    assert!(buffer.toggle_bookmark());

    buffer.cursor_row = 2;
    assert_eq!(buffer.next_bookmark_row(), Some(0));
    assert_eq!(buffer.prev_bookmark_row(), Some(0));
}

#[test]
fn bookmarks_shift_when_lines_change() {
    let mut buffer = make_buffer("x");
    buffer.lines = vec!["a".into(), "b".into(), "c".into()];
    buffer.bookmarks.insert(1);
    buffer.bookmarks.insert(2);

    shift_bookmarks_on_insert(&mut buffer, 1);
    assert!(buffer.bookmarks.contains(&2));
    assert!(buffer.bookmarks.contains(&3));

    shift_bookmarks_on_remove(&mut buffer, 2, Some(1));
    assert!(buffer.bookmarks.contains(&1));
    assert!(buffer.bookmarks.contains(&2));
}

#[test]
fn insert_text_supports_multiline_paste() {
    let mut buffer = make_buffer("abcd");
    buffer.cursor_col = 2;
    insert_text(&mut buffer, "X\nY");
    assert_eq!(buffer.lines, vec!["abX".to_string(), "Ycd".to_string()]);
    assert_eq!(buffer.cursor_row, 1);
    assert_eq!(buffer.cursor_col, 1);
}

#[test]
fn bookmarks_shift_when_line_range_removed() {
    let mut buffer = make_buffer("x");
    buffer.lines = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
    buffer.bookmarks.insert(0);
    buffer.bookmarks.insert(2);
    buffer.bookmarks.insert(4);

    shift_bookmarks_on_remove_range(&mut buffer, 1, 3);

    assert!(buffer.bookmarks.contains(&0));
    assert!(buffer.bookmarks.contains(&1));
    assert_eq!(buffer.bookmarks.len(), 2);
}

#[test]
fn auto_pair_and_skip_closer_work() {
    let mut buffer = make_buffer("");
    assert!(try_insert_auto_pair(&mut buffer, '('));
    assert_eq!(buffer.lines[0], "()");
    assert_eq!(buffer.cursor_col, 1);

    assert!(try_skip_existing_closer(&mut buffer, ')'));
    assert_eq!(buffer.cursor_col, 2);
}

#[test]
fn enter_applies_auto_indent() {
    let mut buffer = make_buffer("{");
    insert_newline(&mut buffer);
    assert_eq!(buffer.lines, vec!["{".to_string(), "    ".to_string()]);
    assert_eq!(buffer.cursor_row, 1);
    assert_eq!(buffer.cursor_col, 4);
}

#[test]
fn plain_newline_keeps_existing_alignment() {
    let mut buffer = make_buffer("{abc");
    buffer.cursor_col = 1;
    insert_newline_plain(&mut buffer);
    assert_eq!(buffer.lines, vec!["{".to_string(), "abc".to_string()]);
    assert_eq!(buffer.cursor_row, 1);
    assert_eq!(buffer.cursor_col, 0);
}

#[test]
fn outdent_removes_leading_spaces() {
    let mut buffer = make_buffer("    let x = 1;");
    buffer.cursor_col = 4;
    outdent_current_line(&mut buffer);
    assert_eq!(buffer.lines[0], "let x = 1;");
    assert_eq!(buffer.cursor_col, 0);
}

#[test]
fn backspace_removes_bracket_pair() {
    let mut buffer = make_buffer("()");
    buffer.cursor_col = 1;
    backspace(&mut buffer);
    assert_eq!(buffer.lines[0], "");
    assert_eq!(buffer.cursor_col, 0);
}

#[test]
fn explorer_action_keys_are_reserved_in_explorer() {
    assert!(is_explorer_action_key(KeyCode::Char('n')));
    assert!(is_explorer_action_key(KeyCode::Char('N')));
    assert!(!is_explorer_action_key(KeyCode::Char('x')));
}

#[test]
fn raw_control_fallback_event_detection_is_reliable() {
    let raw_ctrl_v = KeyEvent::new(KeyCode::Char('\u{16}'), KeyModifiers::NONE);
    assert!(is_raw_control_fallback_event(&raw_ctrl_v));

    let ctrl_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert!(!is_raw_control_fallback_event(&ctrl_v));

    let plain_char = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
    assert!(!is_raw_control_fallback_event(&plain_char));
}

#[test]
fn editor_text_input_key_detection_is_precise() {
    assert!(is_editor_text_input_key(&KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::NONE
    )));
    assert!(is_editor_text_input_key(&KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE
    )));
    assert!(is_editor_text_input_key(&KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::NONE
    )));
    assert!(!is_editor_text_input_key(&KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::SHIFT
    )));
    assert!(!is_editor_text_input_key(&KeyEvent::new(
        KeyCode::Char('v'),
        KeyModifiers::CONTROL
    )));
}

#[test]
fn debounce_flags_clipboard_and_suggestion_actions_only() {
    assert!(is_debounced_control_action(
        control::ControlAction::EditorPaste
    ));
    assert!(is_debounced_control_action(
        control::ControlAction::GlobalEditorPaste
    ));
    assert!(is_debounced_control_action(
        control::ControlAction::EditorCopy
    ));
    assert!(is_debounced_control_action(
        control::ControlAction::AcceptSuggestion
    ));
    assert!(!is_debounced_control_action(
        control::ControlAction::FocusEditor
    ));
    assert!(!is_debounced_control_action(
        control::ControlAction::NextBuffer
    ));
}

#[test]
fn paste_fingerprint_is_stable_for_same_payload() {
    let a = paste_fingerprint("abc\n123");
    let b = paste_fingerprint("abc\n123");
    let c = paste_fingerprint("abc\n124");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn duplicate_paste_is_ignored_in_short_window() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);

    app.paste_editor_text("abc");
    app.paste_editor_text("abc");

    let buffer = app.editor.current().expect("buffer should exist");
    assert_eq!(buffer.lines, vec!["abc".to_string()]);
}

#[test]
fn extract_latest_fenced_code_block_picks_last_block() {
    let input = "halo\n```rs\nlet a = 1;\n```\ntext\n```js\nconst b = 2;\n```";
    let block = extract_latest_fenced_code_block(input).expect("latest block should exist");
    assert_eq!(block, "const b = 2;");
}

#[test]
fn extract_latest_fenced_code_block_ignores_empty_block() {
    let input = "```rs\nlet ok = true;\n```\n```\n\n```";
    let block = extract_latest_fenced_code_block(input).expect("non-empty block should exist");
    assert_eq!(block, "let ok = true;");
}

#[test]
fn paste_event_goes_directly_without_auto_indent_side_effect() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);
    app.handle_paste_event("a\n\tb\nc".to_string());

    let buffer = app.editor.current().expect("buffer should exist");
    assert_eq!(
        buffer.lines,
        vec!["a".to_string(), "    b".to_string(), "c".to_string()]
    );
}

#[test]
fn plain_insert_mode_disables_auto_indent_and_auto_pair() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer("{"));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);
    app.editor_plain_paste_mode_until = Some(Instant::now() + Duration::from_secs(1));

    app.handle_editor_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .expect("enter should be handled");
    app.handle_editor_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .expect("tab should be handled");
    app.handle_editor_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE))
        .expect("char should be handled");
    app.flush_pending_editor_paste_if_ready(true);

    let buffer = app.editor.current().expect("buffer should exist");
    assert_eq!(buffer.lines, vec!["{".to_string(), "    (".to_string()]);
    assert_eq!(buffer.cursor_row, 1);
    assert_eq!(buffer.cursor_col, 5);
}

#[test]
fn plain_insert_mode_holds_input_until_flushed() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);
    app.editor_plain_paste_mode_until = Some(Instant::now() + Duration::from_secs(1));

    app.handle_editor_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
        .expect("char should be buffered");

    let before_flush = app.editor.current().expect("buffer should exist");
    assert_eq!(before_flush.lines, vec!["".to_string()]);

    app.flush_pending_editor_paste_if_ready(true);
    let after_flush = app.editor.current().expect("buffer should exist");
    assert_eq!(after_flush.lines, vec!["a".to_string()]);
}

#[test]
fn pending_paste_loading_info_shows_while_waiting_flush() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);
    app.editor_plain_paste_mode_until = Some(Instant::now() + Duration::from_secs(1));

    app.handle_editor_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        .expect("char should be buffered");

    let loading = app
        .pending_editor_paste_loading_text()
        .expect("loading text should exist");
    assert!(loading.starts_with("paste "));
    assert!(loading.contains("rewrite xone"));
}

#[test]
fn rewrite_editor_paste_applies_xone_standard_for_code() {
    let raw = "  <div>  \r\n    <span>ok</span>\t\r\n  </div>\r\n\r\n";
    let rewritten = rewrite_editor_paste(raw, language::Language::Html);
    assert_eq!(rewritten, "    <div>\n    <span>ok</span>\n    </div>");
}

#[test]
fn rewrite_editor_paste_keeps_plain_text_indent_shape() {
    let raw = "  hello\n world";
    let rewritten = rewrite_editor_paste(raw, language::Language::PlainText);
    assert_eq!(rewritten, "  hello\n world");
}

#[test]
fn rewrite_editor_paste_keeps_xone_indent_when_already_standard() {
    let raw = "    fn main() {\n        println!(\"x\");\n    }\n";
    let rewritten = rewrite_editor_paste(raw, language::Language::Rust);
    assert_eq!(
        rewritten,
        "    fn main() {\n        println!(\"x\");\n    }"
    );
}

#[test]
fn rewrite_editor_paste_does_not_round_python_indent() {
    let raw = "   if True:\n      print('x')\n";
    let rewritten = rewrite_editor_paste(raw, language::Language::Python);
    assert_eq!(rewritten, "   if True:\n      print('x')");
}

#[test]
fn rewrite_editor_paste_does_not_round_yaml_indent() {
    let raw = " root:\n   child: 1\n";
    let rewritten = rewrite_editor_paste(raw, language::Language::Yaml);
    assert_eq!(rewritten, " root:\n   child: 1");
}

#[test]
fn raw_tab_char_event_in_editor_is_treated_as_indent() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");

    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);

    app.handle_key(KeyEvent::new(KeyCode::Char('\t'), KeyModifiers::NONE))
        .expect("raw tab should be handled");

    let buffer = app.editor.current().expect("buffer should exist");
    assert_eq!(buffer.lines, vec!["    ".to_string()]);
    assert_eq!(buffer.cursor_col, 4);
}

#[test]
fn ctrl_alt_t_is_not_mapped_as_toggle_terminal_control_action() {
    let action = control::action_for(&KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    ));
    assert_eq!(action, None);
}

#[test]
fn ui_preferences_roundtrip_persists_all_flags() {
    let dir = unique_temp_dir("ui_prefs");
    fs::create_dir_all(&dir).expect("temp dir should be created");

    save_ui_preferences(
        &dir,
        style::AppearancePreset::HighContrast,
        style::AccentPreset::Emerald,
        style::TabThemePreset::Vivid,
        style::SyntaxThemePreset::Neon,
        style::UiDensity::Compact,
        false,
        false,
        true,
        true,
    )
    .expect("preferences should be saved");

    let (
        appearance,
        accent,
        tab_theme,
        syntax_theme,
        density,
        terminal_fx,
        terminal_command_sync,
        hard_mode,
        adaptive_terminal_height,
    ) =
        load_ui_preferences(&dir);
    assert_eq!(appearance, style::AppearancePreset::HighContrast);
    assert_eq!(accent, style::AccentPreset::Emerald);
    assert_eq!(tab_theme, style::TabThemePreset::Vivid);
    assert_eq!(syntax_theme, style::SyntaxThemePreset::Neon);
    assert_eq!(density, style::UiDensity::Compact);
    assert!(!terminal_fx);
    assert!(!terminal_command_sync);
    assert!(hard_mode);
    assert!(adaptive_terminal_height);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ctrl_o_opens_settings_even_when_terminal_focused() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.set_focus(Focus::Terminal);

    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
        .expect("ctrl+o should be handled");

    assert!(matches!(app.focus, Focus::Settings));
}

#[test]
fn create_file_shortcut_is_ignored_when_terminal_focused() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.set_focus(Focus::Terminal);

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .expect("terminal n should be handled as terminal input");

    assert!(matches!(app.focus, Focus::Terminal));
    assert!(app.explorer_prompt.is_none());
}

#[test]
fn create_file_shortcut_is_ignored_when_editor_focused() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Editor);

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .expect("editor n should insert text");

    let buffer = app.editor.current().expect("buffer should exist");
    assert_eq!(buffer.lines, vec!["n".to_string()]);
    assert!(app.explorer_prompt.is_none());
}

#[test]
fn esc_from_about_returns_to_main_panel() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.open_about_panel();
    assert!(matches!(app.focus, Focus::About));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .expect("esc should close about");

    assert!(matches!(app.focus, Focus::Editor | Focus::Explorer));
}

#[test]
fn esc_from_shortcuts_returns_to_main_panel() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.open_shortcuts_panel();
    assert!(matches!(app.focus, Focus::Shortcuts));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .expect("esc should close shortcuts");

    assert!(matches!(app.focus, Focus::Editor | Focus::Explorer));
}

#[test]
fn explorer_can_enter_empty_folder_and_go_back_to_parent() {
    let root = unique_temp_dir("empty_folder_nav");
    let empty = root.join("kosong");
    fs::create_dir_all(&empty).expect("empty folder should be created");

    let workspace = Workspace::new(root.clone()).expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.set_focus(Focus::Explorer);
    app.explorer_root = root.clone();
    app.last_explorer_refresh = None;
    app.refresh_explorer().expect("explorer should refresh");

    let idx = app
        .explorer
        .items
        .iter()
        .position(|item| item.is_dir && item.name == "kosong")
        .expect("empty folder should be visible");
    app.explorer.selected = idx;

    app.handle_explorer_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .expect("enter should navigate into empty folder");
    assert_eq!(app.explorer_root, empty);

    app.handle_explorer_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        .expect("backspace should navigate to parent");
    assert_eq!(app.explorer_root, root);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn hard_mode_f8_cycles_focus_forward() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.settings.hard_mode = true;
    app.explorer_visible = true;
    app.terminal_visible = true;
    app.left_panel = LeftPanel::Explorer;
    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Explorer);

    app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE))
        .expect("f8 should cycle focus");
    assert!(matches!(app.focus, Focus::Editor));

    app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE))
        .expect("f8 should cycle focus");
    assert!(matches!(app.focus, Focus::Terminal));
}

#[test]
fn hard_mode_shift_f8_cycles_focus_backward() {
    let workspace = Workspace::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .expect("workspace should be created");
    let mut app = App::new(workspace).expect("app should initialize");
    app.settings.hard_mode = true;
    app.explorer_visible = true;
    app.terminal_visible = true;
    app.left_panel = LeftPanel::Explorer;
    app.editor.buffers.push(make_buffer(""));
    app.editor.active = 0;
    app.set_focus(Focus::Explorer);

    app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::SHIFT))
        .expect("shift+f8 should cycle focus backward");
    assert!(matches!(app.focus, Focus::AiChat));
}

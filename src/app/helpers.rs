//! Module: src/app/helpers.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

use crate::core::{FolderEntry, Node};

use super::{control, ConsoleHandle, EditorBuffer, ExplorerItem, LineEnding};

pub(super) fn save_external_file(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

pub(super) fn point_in_rect(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

#[cfg(windows)]
pub(super) fn configure_windows_input_mode() -> io::Result<Option<(ConsoleHandle, u32)>> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_EXTENDED_FLAGS, ENABLE_MOUSE_INPUT,
        ENABLE_QUICK_EDIT_MODE, STD_INPUT_HANDLE,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle == std::ptr::null_mut() || handle == INVALID_HANDLE_VALUE {
            return Ok(None);
        }

        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return Ok(None);
        }

        let original = mode;
        mode &= !ENABLE_QUICK_EDIT_MODE;
        mode |= ENABLE_EXTENDED_FLAGS | ENABLE_MOUSE_INPUT;
        let _ = SetConsoleMode(handle, mode);
        Ok(Some((handle, original)))
    }
}

#[cfg(not(windows))]
pub(super) fn configure_windows_input_mode() -> io::Result<Option<(ConsoleHandle, u32)>> {
    Ok(None)
}

#[cfg(windows)]
pub(super) fn restore_windows_input_mode(state: Option<(ConsoleHandle, u32)>) {
    use windows_sys::Win32::System::Console::SetConsoleMode;
    if let Some((handle, mode)) = state {
        unsafe {
            let _ = SetConsoleMode(handle, mode);
        }
    }
}

#[cfg(not(windows))]
pub(super) fn restore_windows_input_mode(_state: Option<(ConsoleHandle, u32)>) {}

pub(super) fn insert_char(buffer: &mut EditorBuffer, c: char) {
    buffer.push_undo();
    let line = &mut buffer.lines[buffer.cursor_row];
    let byte = char_to_byte_index(line, buffer.cursor_col);
    line.insert(byte, c);
    buffer.cursor_col += 1;
    buffer.dirty = true;
}

pub(super) fn insert_text(buffer: &mut EditorBuffer, text: &str) {
    if text.is_empty() {
        return;
    }

    let normalized = text.replace("\r\n", "\n").replace('\t', INDENT);
    let row = buffer.cursor_row;
    let col = buffer.cursor_col;
    let byte = char_to_byte_index(&buffer.lines[row], col);
    let left = buffer.lines[row][..byte].to_string();
    let right = buffer.lines[row][byte..].to_string();
    let parts: Vec<&str> = normalized.split('\n').collect();

    if parts.len() == 1 {
        buffer.lines[row] = format!("{}{}{}", left, parts[0], right);
        buffer.cursor_col = col + line_len(parts[0]);
        return;
    }

    buffer.lines[row] = format!("{}{}", left, parts[0]);
    let insert_at = row + 1;
    for _ in 0..parts.len().saturating_sub(1) {
        shift_bookmarks_on_insert(buffer, insert_at);
    }
    for (index, part) in parts.iter().enumerate().skip(1) {
        let position = insert_at + (index - 1);
        if index + 1 == parts.len() {
            buffer.lines.insert(position, format!("{}{}", part, right));
        } else {
            buffer.lines.insert(position, (*part).to_string());
        }
    }
    buffer.cursor_row = row + parts.len().saturating_sub(1);
    buffer.cursor_col = line_len(parts.last().copied().unwrap_or(""));
}

pub(super) fn insert_newline(buffer: &mut EditorBuffer) {
    buffer.push_undo();
    let row = buffer.cursor_row;
    let byte = char_to_byte_index(&buffer.lines[row], buffer.cursor_col);
    let left = buffer.lines[row][..byte].to_string();
    let right = buffer.lines[row][byte..].to_string();

    buffer.lines[row] = left.clone();
    let base_indent = leading_indent(&left);
    let extra_indent = if should_increase_indent(&left) {
        INDENT
    } else {
        ""
    };
    let new_line = format!("{}{}", base_indent, extra_indent);

    shift_bookmarks_on_insert(buffer, row + 1);
    buffer.lines.insert(row + 1, new_line.clone());
    buffer.cursor_row = row + 1;
    buffer.cursor_col = line_len(&new_line);

    if !right.is_empty() {
        let right_starts_closer = right.chars().next().map(is_closing_char).unwrap_or(false);
        if extra_indent.is_empty() || !right_starts_closer {
            buffer.lines[row + 1] = right;
            buffer.cursor_col = 0;
        } else {
            shift_bookmarks_on_insert(buffer, row + 2);
            buffer.lines.insert(row + 2, right);
        }
    }
    buffer.dirty = true;
}

pub(super) fn insert_newline_plain(buffer: &mut EditorBuffer) {
    buffer.push_undo();
    let row = buffer.cursor_row;
    let byte = char_to_byte_index(&buffer.lines[row], buffer.cursor_col);
    let left = buffer.lines[row][..byte].to_string();
    let right = buffer.lines[row][byte..].to_string();

    buffer.lines[row] = left;
    shift_bookmarks_on_insert(buffer, row + 1);
    buffer.lines.insert(row + 1, right);
    buffer.cursor_row = row + 1;
    buffer.cursor_col = 0;
    buffer.dirty = true;
}
pub(super) fn backspace(buffer: &mut EditorBuffer) {
    if buffer.cursor_col > 0 {
        let line = &buffer.lines[buffer.cursor_row];
        let prev_char = char_at(line, buffer.cursor_col - 1);
        let next_char = char_at(line, buffer.cursor_col);
        let remove_pair = matches_pair(prev_char, next_char);

        buffer.push_undo();
        let line = &mut buffer.lines[buffer.cursor_row];
        let byte = char_to_byte_index(line, buffer.cursor_col);
        let prev = char_to_byte_index(line, buffer.cursor_col - 1);
        if remove_pair {
            let next = char_to_byte_index(line, buffer.cursor_col + 1);
            line.replace_range(prev..next, "");
        } else {
            line.replace_range(prev..byte, "");
        }
        buffer.cursor_col -= 1;
        buffer.dirty = true;
    } else if buffer.cursor_row > 0 {
        buffer.push_undo();
        shift_bookmarks_on_remove(buffer, buffer.cursor_row, Some(buffer.cursor_row - 1));
        let current = buffer.lines.remove(buffer.cursor_row);
        buffer.cursor_row -= 1;
        let line = &mut buffer.lines[buffer.cursor_row];
        let col = line_len(line);
        line.push_str(&current);
        buffer.cursor_col = col;
        buffer.dirty = true;
    }
}

pub(super) fn delete_char(buffer: &mut EditorBuffer) {
    let line_len = line_len(&buffer.lines[buffer.cursor_row]);
    if buffer.cursor_col < line_len {
        buffer.push_undo();
        let line = &mut buffer.lines[buffer.cursor_row];
        let start = char_to_byte_index(line, buffer.cursor_col);
        let end = char_to_byte_index(line, buffer.cursor_col + 1);
        line.replace_range(start..end, "");
        buffer.dirty = true;
    } else if buffer.cursor_row + 1 < buffer.lines.len() {
        buffer.push_undo();
        shift_bookmarks_on_remove(buffer, buffer.cursor_row + 1, Some(buffer.cursor_row));
        let next = buffer.lines.remove(buffer.cursor_row + 1);
        buffer.lines[buffer.cursor_row].push_str(&next);
        buffer.dirty = true;
    }
}

pub(super) const INDENT: &str = "    ";
pub(super) const INDENT_WIDTH: usize = 4;

pub(super) fn insert_indent(buffer: &mut EditorBuffer) {
    buffer.push_undo();
    let line = &mut buffer.lines[buffer.cursor_row];
    let byte = char_to_byte_index(line, buffer.cursor_col);
    line.insert_str(byte, INDENT);
    buffer.cursor_col += INDENT_WIDTH;
    buffer.dirty = true;
}

pub(super) fn outdent_current_line(buffer: &mut EditorBuffer) {
    let row = buffer.cursor_row;
    let remove_chars = {
        let line = &buffer.lines[row];
        leading_outdent_chars(line)
    };
    if remove_chars == 0 {
        return;
    }
    buffer.push_undo();
    let line = &mut buffer.lines[row];
    let end = char_to_byte_index(line, remove_chars);
    line.replace_range(0..end, "");
    buffer.cursor_col = buffer.cursor_col.saturating_sub(remove_chars);
    buffer.dirty = true;
}

pub(super) fn leading_outdent_chars(line: &str) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.chars()
        .take(INDENT_WIDTH)
        .take_while(|c| *c == ' ')
        .count()
}

pub(super) fn leading_indent(line: &str) -> String {
    line.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

pub(super) fn should_increase_indent(left: &str) -> bool {
    left.trim_end()
        .chars()
        .last()
        .map(|ch| matches!(ch, '{' | '[' | '(' | ':'))
        .unwrap_or(false)
}

pub(super) fn try_insert_auto_pair(buffer: &mut EditorBuffer, c: char) -> bool {
    let Some(close) = matching_closer(c) else {
        return false;
    };
    buffer.push_undo();
    let line = &mut buffer.lines[buffer.cursor_row];
    let byte = char_to_byte_index(line, buffer.cursor_col);
    line.insert(byte, c);
    line.insert(byte + c.len_utf8(), close);
    buffer.cursor_col += 1;
    buffer.dirty = true;
    true
}

pub(super) fn try_skip_existing_closer(buffer: &mut EditorBuffer, c: char) -> bool {
    if !is_closing_char(c) {
        return false;
    }
    let line = &buffer.lines[buffer.cursor_row];
    if char_at(line, buffer.cursor_col) == Some(c) {
        buffer.cursor_col += 1;
        return true;
    }
    false
}

pub(super) fn matching_closer(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

pub(super) fn is_closing_char(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '"' | '\'')
}

pub(super) fn matches_pair(left: Option<char>, right: Option<char>) -> bool {
    matches!(
        (left, right),
        (Some('('), Some(')'))
            | (Some('['), Some(']'))
            | (Some('{'), Some('}'))
            | (Some('"'), Some('"'))
            | (Some('\''), Some('\''))
    )
}

pub(super) fn char_at(line: &str, char_index: usize) -> Option<char> {
    line.chars().nth(char_index)
}

pub(super) fn shift_bookmarks_on_insert(buffer: &mut EditorBuffer, inserted_at: usize) {
    if buffer.bookmarks.is_empty() {
        return;
    }
    let mut updated = BTreeSet::new();
    for row in &buffer.bookmarks {
        if *row >= inserted_at {
            updated.insert(*row + 1);
        } else {
            updated.insert(*row);
        }
    }
    buffer.bookmarks = updated;
}

pub(super) fn shift_bookmarks_on_remove(
    buffer: &mut EditorBuffer,
    removed_row: usize,
    merge_to: Option<usize>,
) {
    if buffer.bookmarks.is_empty() {
        return;
    }
    let mut updated = BTreeSet::new();
    for row in &buffer.bookmarks {
        if *row == removed_row {
            if let Some(target) = merge_to {
                updated.insert(target);
            }
        } else if *row > removed_row {
            updated.insert(*row - 1);
        } else {
            updated.insert(*row);
        }
    }
    buffer.bookmarks = updated;
}

pub(super) fn shift_bookmarks_on_remove_range(buffer: &mut EditorBuffer, start: usize, end: usize) {
    if buffer.bookmarks.is_empty() {
        return;
    }
    if end < start {
        return;
    }
    let removed = end - start + 1;
    let mut updated = BTreeSet::new();
    for row in &buffer.bookmarks {
        if *row < start {
            updated.insert(*row);
        } else if *row > end {
            updated.insert(*row - removed);
        }
    }
    buffer.bookmarks = updated;
}

pub(super) fn ensure_cursor_visible(buffer: &mut EditorBuffer, height: usize) {
    let height = height.max(1);
    if buffer.cursor_row < buffer.scroll {
        buffer.scroll = buffer.cursor_row;
    } else if buffer.cursor_row >= buffer.scroll + height {
        buffer.scroll = buffer.cursor_row.saturating_sub(height - 1);
    }
}

pub(super) fn flatten_tree(root: &FolderEntry, depth: usize) -> Vec<ExplorerItem> {
    let mut items = Vec::new();
    for node in &root.children {
        collect_node(node, depth, &mut items);
    }
    items
}

pub(super) fn collect_node(node: &Node, depth: usize, items: &mut Vec<ExplorerItem>) {
    match node {
        Node::File(file) => items.push(ExplorerItem {
            path: file.path.clone(),
            name: file.name.clone(),
            depth,
            is_dir: false,
        }),
        Node::Folder(folder) => {
            items.push(ExplorerItem {
                path: folder.path.clone(),
                name: folder.name.clone(),
                depth,
                is_dir: true,
            });
            for child in &folder.children {
                collect_node(child, depth + 1, items);
            }
        }
    }
}

pub(super) fn detect_line_ending(content: &str) -> LineEnding {
    if content.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

pub(super) fn line_len(line: &str) -> usize {
    line.chars().count()
}

pub(super) fn visual_width_until(input: &str, char_index: usize) -> usize {
    const TAB_WIDTH: usize = 4;
    let mut width = 0usize;
    for ch in input.chars().take(char_index) {
        if ch == '\t' {
            width += TAB_WIDTH;
            continue;
        }
        width += UnicodeWidthChar::width(ch).unwrap_or(1);
    }
    width
}

pub(super) fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| text.len())
}

pub(super) fn normalize_single_line_text(input: &str) -> String {
    input.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

pub(super) fn tab_name(buffer: &EditorBuffer) -> String {
    buffer
        .path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| buffer.path.to_string_lossy().to_string())
}

pub(super) fn is_explorer_action_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::Char('n') | KeyCode::Char('N'))
}

pub(super) fn is_prompt_bypass_control(action: control::ControlAction) -> bool {
    matches!(
        action,
        control::ControlAction::FocusExplorer
            | control::ControlAction::FocusEditor
            | control::ControlAction::OpenSettings
            | control::ControlAction::ToggleExplorer
            | control::ControlAction::ToggleTerminalPanel
            | control::ControlAction::CreateFile
            | control::ControlAction::CreateFolder
            | control::ControlAction::Quit
    )
}

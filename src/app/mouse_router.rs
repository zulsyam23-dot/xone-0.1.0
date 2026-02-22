//! Module: src/app/mouse_router.rs
//! Catatan: semua drama klik-drag mouse diparkir di sini biar `mod.rs` lebih waras.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::helpers::{ensure_cursor_visible, line_len, point_in_rect};
use super::{App, Focus};

pub(super) fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.settings.hard_mode {
        return;
    }
    if matches!(app.focus, Focus::Settings | Focus::Shortcuts | Focus::About) {
        return;
    }
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.mouse_dragging = true;
            begin_mouse_selection(app, mouse.column, mouse.row);
        }
        MouseEventKind::ScrollUp => {
            handle_terminal_wheel(app, mouse.column, mouse.row, true);
        }
        MouseEventKind::ScrollDown => {
            handle_terminal_wheel(app, mouse.column, mouse.row, false);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            update_mouse_selection(app, mouse.column, mouse.row);
        }
        MouseEventKind::Moved => {
            if app.mouse_dragging {
                update_mouse_selection(app, mouse.column, mouse.row);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.mouse_dragging = false;
            update_mouse_selection(app, mouse.column, mouse.row);
        }
        _ => {}
    }
}

fn begin_mouse_selection(app: &mut App, x: u16, y: u16) {
    if mouse_terminal_hit(app, x, y) {
        app.set_focus(Focus::Terminal);
        app.mouse_dragging = false;
        app.selection = None;
        return;
    }
    if let Some(row) = mouse_explorer_row(app, x, y) {
        app.set_focus(Focus::Explorer);
        if !app.explorer.items.is_empty() {
            app.explorer.selected = row.min(app.explorer.items.len().saturating_sub(1));
            app.start_selection(Focus::Explorer, app.explorer.selected);
        }
        return;
    }

    if let Some(row) = mouse_editor_row(app, x, y) {
        app.set_focus(Focus::Editor);
        let mut selected_row = None;
        if let Some(buffer) = app.editor.current_mut() {
            let row = row.min(buffer.lines.len().saturating_sub(1));
            buffer.cursor_row = row;
            buffer.cursor_col = buffer.cursor_col.min(line_len(&buffer.lines[buffer.cursor_row]));
            ensure_cursor_visible(buffer, app.editor_view_height);
            selected_row = Some(row);
        }
        if let Some(row) = selected_row {
            app.start_selection(Focus::Editor, row);
        }
        return;
    }

    app.selection = None;
}

fn update_mouse_selection(app: &mut App, x: u16, y: u16) {
    let Some(selection) = app.selection else {
        return;
    };
    match selection.panel {
        Focus::Explorer => {
            if let Some(row) = mouse_explorer_row(app, x, y) {
                let row = row.min(app.explorer.items.len().saturating_sub(1));
                app.explorer.selected = row;
                app.update_selection(Focus::Explorer, row);
            }
        }
        Focus::Editor => {
            if let Some(row) = mouse_editor_row(app, x, y) {
                if let Some(buffer) = app.editor.current_mut() {
                    let row = row.min(buffer.lines.len().saturating_sub(1));
                    buffer.cursor_row = row;
                    buffer.cursor_col = buffer.cursor_col.min(line_len(&buffer.lines[row]));
                    ensure_cursor_visible(buffer, app.editor_view_height);
                    app.update_selection(Focus::Editor, row);
                }
            }
        }
        Focus::Settings | Focus::Shortcuts | Focus::Terminal | Focus::AiChat | Focus::About => {}
    }
}

fn mouse_explorer_row(app: &App, x: u16, y: u16) -> Option<usize> {
    let rect = app.ui_regions.explorer?;
    if !point_in_rect(rect, x, y) {
        return None;
    }
    Some((y - rect.y) as usize)
}

fn mouse_editor_row(app: &App, x: u16, y: u16) -> Option<usize> {
    let rect = app.ui_regions.editor?;
    if !point_in_rect(rect, x, y) {
        return None;
    }
    let local = (y - rect.y) as usize;
    let buffer = app.editor.current()?;
    Some((buffer.scroll + local).min(buffer.lines.len().saturating_sub(1)))
}

fn mouse_terminal_hit(app: &App, x: u16, y: u16) -> bool {
    let Some(rect) = app.ui_regions.terminal else {
        return false;
    };
    point_in_rect(rect, x, y)
}

fn handle_terminal_wheel(app: &mut App, x: u16, y: u16, up: bool) {
    if !app.terminal_visible || !mouse_terminal_hit(app, x, y) {
        return;
    }
    app.set_focus(Focus::Terminal);
    let step = app
        .ui_regions
        .terminal
        .map(|r| (r.height as usize / 3).max(1).min(10))
        .unwrap_or(3);
    if up {
        app.terminal.scrollback_up(step);
    } else {
        app.terminal.scrollback_down(step);
    }
    app.dirty.terminal = true;
    app.dirty.ui = true;
}

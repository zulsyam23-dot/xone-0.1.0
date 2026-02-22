//! Module: src/app/terminal_controller.rs
//! Catatan: rumah baru untuk logika terminal, biar mod.rs gak jadi kos-kosan overkapasitas.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, Focus};

const CWD_MARKER_START: &str = "XONE_CWD:";
const CWD_MARKER_END: &str = ":XONE_CWD";

pub(super) fn paste_terminal_text(app: &mut App, content: &str) {
    if !app.terminal.is_running() {
        if let Err(error) = app.terminal.ensure_started(&app.explorer_root) {
            app.message = Some(format!("Gagal membuka terminal: {}", error));
            return;
        }
    }
    // Terminal input should primarily use CR to keep cursor alignment stable.
    let text = content.replace("\r\n", "\r").replace('\n', "\r");
    let _ = app.terminal.send(&text);
}

pub(super) fn handle_terminal_key(app: &mut App, key: KeyEvent) {
    if !app.terminal.is_running() {
        if let Err(error) = app.terminal.ensure_started(&app.explorer_root) {
            app.message = Some(format!("Gagal membuka terminal: {}", error));
            return;
        }
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char('t') | KeyCode::Char('T'))
    {
        open_new_terminal_tab(app);
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Right)
    {
        app.terminal.next_tab();
        app.terminal_command_buffer.clear();
        app.terminal_command_tracking_valid = true;
        resync_active_terminal_view(app);
        app.message = Some(format!(
            "Terminal tab {}/{}",
            app.terminal.active_index() + 1,
            app.terminal.tab_count()
        ));
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Left)
    {
        app.terminal.prev_tab();
        app.terminal_command_buffer.clear();
        app.terminal_command_tracking_valid = true;
        resync_active_terminal_view(app);
        app.message = Some(format!(
            "Terminal tab {}/{}",
            app.terminal.active_index() + 1,
            app.terminal.tab_count()
        ));
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        app.terminal_command_tracking_valid = false;
        if let KeyCode::Char(c) = key.code {
            let lower = c.to_ascii_lowercase() as u8;
            if lower.is_ascii_lowercase() {
                let code = (lower - b'a' + 1) as u32;
                if let Some(ch) = char::from_u32(code) {
                    let _ = app.terminal.send(&ch.to_string());
                }
                return;
            }
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.terminal_command_buffer.push(c);
            }
            let _ = app.terminal.send(&c.to_string());
        }
        KeyCode::Enter => {
            let _ = app.terminal.send("\r");
            sync_explorer_from_terminal_command(app);
            app.terminal_command_buffer.clear();
            app.terminal_command_tracking_valid = true;
            request_cwd_snapshot(app);
        }
        KeyCode::Backspace => {
            app.terminal_command_buffer.pop();
            let _ = app.terminal.send("\x7f");
        }
        KeyCode::Tab => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\t");
        }
        KeyCode::Up => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[A");
        }
        KeyCode::Down => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[B");
        }
        KeyCode::Left => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[D");
        }
        KeyCode::Right => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[C");
        }
        KeyCode::Home => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[1~");
        }
        KeyCode::End => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[4~");
        }
        KeyCode::Delete => {
            app.terminal_command_tracking_valid = false;
            let _ = app.terminal.send("\x1b[3~");
        }
        _ => {}
    }
}

pub(super) fn redraw_terminal_after_resize(app: &mut App) {
    if !app.terminal_visible || !app.terminal.is_running() {
        return;
    }
    let now = Instant::now();
    if let Some(last) = app.last_terminal_resize_redraw {
        if now.duration_since(last) < Duration::from_millis(120) {
            return;
        }
    }
    // Ctrl+L + CR meminta shell me-redraw buffer + menormalkan posisi input ke prompt line.
    let _ = app.terminal.send("\x0c\r");
    app.last_terminal_resize_redraw = Some(now);
}

pub(super) fn open_new_terminal_tab(app: &mut App) {
    match app.terminal.new_tab(&app.explorer_root) {
        Ok(index) => {
            let root = app.explorer_root.clone();
            update_terminal_cwd(app, &root);
            app.terminal_visible = true;
            app.set_focus(Focus::Terminal);
            resync_active_terminal_view(app);
            app.message = Some(format!(
                "Terminal tab baru dibuka ({}/{})",
                index + 1,
                app.terminal.tab_count()
            ));
        }
        Err(error) => {
            app.message = Some(format!("Gagal membuka tab terminal: {}", error));
        }
    }
}

pub(super) fn handle_terminal_search_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.terminal_search_mode = false;
            app.terminal_search_query.clear();
            app.message = Some("Terminal search dibatalkan".to_string());
            true
        }
        KeyCode::Enter => {
            let query = app.terminal_search_query.trim().to_string();
            if query.is_empty() {
                app.message = Some("Terminal search kosong".to_string());
            } else {
                let (count, line) = app.terminal.search_scrollback(&query);
                if count == 0 {
                    app.message = Some(format!("Terminal search: '{}' tidak ditemukan", query));
                } else {
                    let preview = line
                        .as_deref()
                        .map(|v| trim_preview(v, 56))
                        .unwrap_or_else(|| "-".to_string());
                    app.message = Some(format!(
                        "Terminal search '{}' ditemukan {} kali | terakhir: {}",
                        query, count, preview
                    ));
                }
            }
            app.terminal_search_mode = false;
            app.terminal_search_query.clear();
            true
        }
        KeyCode::Backspace => {
            app.terminal_search_query.pop();
            true
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.terminal_search_query.push(c);
            true
        }
        _ => true,
    }
}

pub(super) fn toggle_terminal_panel(app: &mut App) {
    if !app.terminal_visible {
        if let Err(error) = app.terminal.ensure_started(&app.explorer_root) {
            app.message = Some(format!("Gagal membuka terminal: {}", error));
            return;
        }
        let root = app.explorer_root.clone();
        update_terminal_cwd(app, &root);
        app.terminal_visible = true;
        app.set_focus(Focus::Terminal);
        if app.message.is_none() {
            app.message = Some("Terminal dibuka".to_string());
        }
        return;
    }
    if !matches!(app.focus, Focus::Terminal) {
        app.set_focus(Focus::Terminal);
        app.message = Some("Terminal aktif".to_string());
    } else {
        app.terminal_visible = false;
        if matches!(app.focus, Focus::Terminal) {
            if app.editor.has_open_buffer() {
                app.set_focus(Focus::Editor);
            } else {
                app.set_focus(Focus::Explorer);
            }
        }
        app.message = Some("Terminal disembunyikan".to_string());
    }
}

pub(super) fn sync_terminal_cwd(app: &mut App) -> bool {
    let Some((tab_index, path)) = app.terminal.take_cwd_update() else {
        return false;
    };

    if !path.exists() {
        return false;
    }

    // Path dari terminal parser sudah dibersihkan/canonicalized pada jalur utama.
    let new_canonical = path;
    if app.explorer_root == new_canonical {
        return false;
    }

    if tab_index != app.terminal.active_index() {
        return false;
    }

    // Jika cwd terminal keluar dari root workspace saat ini, pindahkan root workspace
    // agar explorer tetap bisa mengikuti direktori aktif terminal.
    if app.workspace.resolve(&new_canonical).is_err()
        && app.workspace.root() != new_canonical.as_path()
    {
        if app.workspace.set_root(new_canonical.clone()).is_err() {
            app.message = Some(format!(
                "Gagal sinkron root workspace ke: {}",
                new_canonical.display()
            ));
            return false;
        }
    }

    app.explorer_root = new_canonical.clone();
    app.pending_terminal_cwd = Some(new_canonical.clone());
    // Sinkronisasi dari terminal harus menang atas debounce explorer biasa.
    app.last_explorer_refresh = None;

    if let Err(error) = app.refresh_explorer() {
        app.message = Some(format!("Gagal refresh explorer: {}", error));
        false
    } else {
        if matches!(app.focus, Focus::Terminal) {
            app.message = Some(format!(
                "Explorer sinkron (tab {}/{}): {}",
                app.terminal.active_index() + 1,
                app.terminal.tab_count(),
                new_canonical.display()
            ));
        }
        true
    }
}

pub(super) fn update_terminal_cwd(app: &mut App, path: &Path) {
    if !app.terminal.is_running() {
        return;
    }
    let value = path.to_string_lossy().to_string();
    if value.is_empty() {
        return;
    }
    let cmd = format!(
        "Set-Location -LiteralPath '{}'\r\n",
        value.replace('\'', "''")
    );
    let _ = app.terminal.send(&cmd);
}

fn resync_active_terminal_view(app: &mut App) {
    app.dirty.terminal = true;
    app.dirty.ui = true;
    // Redraw + return keeps cursor from drifting after tab/context changes.
    let _ = app.terminal.send("\x0c\r");
    app.last_terminal_resize_redraw = Some(Instant::now());
}

fn request_cwd_snapshot(app: &mut App) {
    if !app.terminal.is_running() {
        return;
    }
    // Fallback sync: minta shell mencetak cwd setelah command dieksekusi.
    let cmd = format!(
        "Write-Output \"{}$($PWD.Path){}\"\r\n",
        CWD_MARKER_START, CWD_MARKER_END
    );
    let _ = app.terminal.send(&cmd);
}

fn sync_explorer_from_terminal_command(app: &mut App) {
    if !app.settings.terminal_command_sync {
        return;
    }
    if !app.terminal_command_tracking_valid {
        return;
    }
    let command = app.terminal_command_buffer.trim();
    let Some(target) = parse_terminal_cd_target(command, &app.explorer_root) else {
        return;
    };
    let Ok(new_canonical) = target.canonicalize() else {
        return;
    };
    if !new_canonical.is_dir() {
        return;
    }

    if app.workspace.resolve(&new_canonical).is_err() {
        if app.workspace.set_root(new_canonical.clone()).is_err() {
            return;
        }
    }
    app.explorer_root = new_canonical.clone();
    app.pending_terminal_cwd = Some(new_canonical);
    app.last_explorer_refresh = None;
    let _ = app.refresh_explorer();
}

fn parse_terminal_cd_target(command: &str, base: &Path) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let prefixes = ["cd ", "chdir ", "set-location ", "sl ", "pushd "];
    let prefix = prefixes.iter().find(|p| lower.starts_with(**p))?;
    let raw = trimmed[prefix.len()..].trim();
    if raw.is_empty() {
        return None;
    }
    let arg = raw.trim_matches('"').trim_matches('\'');
    if arg.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(arg);
    Some(if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    })
}

fn trim_preview(input: &str, max: usize) -> String {
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max {
        return input.to_string();
    }
    if max <= 3 {
        return ".".repeat(max);
    }
    let mut out = String::new();
    for ch in chars.into_iter().take(max - 3) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

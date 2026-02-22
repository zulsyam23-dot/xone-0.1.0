//! Module: src/app/explorer_controller.rs
//! Catatan: semua alur explorer dipisah ke sini biar `mod.rs` gak jadi terminal bus antarkota.

use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::helpers::{char_to_byte_index, line_len};
use super::{App, AppError, ExplorerPromptKind, ExplorerPromptState, Focus};

pub(super) fn handle_explorer_key(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Up => {
            if !app.explorer.items.is_empty() && app.explorer.selected > 0 {
                app.explorer.selected -= 1;
            }
        }
        KeyCode::Down => {
            if !app.explorer.items.is_empty() && app.explorer.selected + 1 < app.explorer.items.len() {
                app.explorer.selected += 1;
            }
        }
        KeyCode::Enter => {
            if app.explorer.items.is_empty() {
                return Ok(());
            }
            let path = app.explorer.items[app.explorer.selected].path.clone();
            let is_dir = app.explorer.items[app.explorer.selected].is_dir;
            if is_dir {
                enter_directory(app, &path)?;
            } else {
                app.open_editor_path(&path)?;
                app.set_focus(Focus::Editor);
            }
        }
        KeyCode::Right => {
            if app.explorer.items.is_empty() {
                return Ok(());
            }
            let path = app.explorer.items[app.explorer.selected].path.clone();
            if app.explorer.items[app.explorer.selected].is_dir {
                enter_directory(app, &path)?;
            }
        }
        KeyCode::Backspace | KeyCode::Left => {
            move_to_parent_directory(app)?;
        }
        _ => {}
    }
    Ok(())
}

fn enter_directory(app: &mut App, path: &Path) -> Result<(), AppError> {
    app.explorer_root = path.to_path_buf();
    app.refresh_explorer()?;
    app.update_terminal_cwd(path);
    if app.explorer.items.is_empty() {
        app.message = Some(format!("Masuk folder kosong: {}", path.display()));
    } else {
        app.message = Some(format!("Masuk folder: {}", path.display()));
    }
    Ok(())
}

fn move_to_parent_directory(app: &mut App) -> Result<(), AppError> {
    let Some(parent) = app.explorer_root.parent() else {
        return Ok(());
    };
    let resolved = if app.workspace.resolve(parent).is_ok() {
        true
    } else {
        app.workspace.set_root(parent.to_path_buf()).is_ok()
    };
    if !resolved {
        return Ok(());
    }
    app.explorer_root = parent.to_path_buf();
    app.refresh_explorer()?;
    let root = app.explorer_root.clone();
    app.update_terminal_cwd(&root);
    if app.explorer.items.is_empty() {
        app.message = Some(format!("Naik ke folder kosong: {}", root.display()));
    } else {
        app.message = Some(format!("Naik ke folder: {}", root.display()));
    }
    Ok(())
}

pub(super) fn start_explorer_prompt(app: &mut App, kind: ExplorerPromptKind) {
    app.set_focus(Focus::Explorer);
    app.explorer_prompt = Some(ExplorerPromptState {
        kind,
        value: String::new(),
        cursor_col: 0,
    });
    app.message = Some(match kind {
        ExplorerPromptKind::File => "Buat file baru (Enter simpan, Esc batal)".to_string(),
        ExplorerPromptKind::Folder => "Buat folder baru (Enter simpan, Esc batal)".to_string(),
    });
}

pub(super) fn handle_explorer_prompt_key(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    let is_paste = (matches!(key.code, KeyCode::Char('v'))
        && (key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)))
        || (matches!(key.code, KeyCode::Insert) && key.modifiers.contains(KeyModifiers::SHIFT));
    if is_paste {
        let Some(text) = app.load_clipboard_text() else {
            app.message = Some("Clipboard kosong".to_string());
            return Ok(());
        };
        app.paste_into_explorer_prompt(&text);
        return Ok(());
    }

    let is_copy = (matches!(key.code, KeyCode::Char('c'))
        && (key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)))
        || (matches!(key.code, KeyCode::Insert) && key.modifiers.contains(KeyModifiers::CONTROL));
    if is_copy {
        if let Some(prompt) = &app.explorer_prompt {
            app.store_clipboard_text(prompt.value.clone());
            app.message = Some("Teks prompt tersalin".to_string());
        }
        return Ok(());
    }

    let Some(prompt) = app.explorer_prompt.as_mut() else {
        return Ok(());
    };
    match key.code {
        KeyCode::Esc => {
            app.explorer_prompt = None;
            app.message = Some("Tambah file/folder dibatalkan".to_string());
        }
        KeyCode::Enter => {
            submit_explorer_prompt(app)?;
        }
        KeyCode::Left => {
            if prompt.cursor_col > 0 {
                prompt.cursor_col -= 1;
            }
        }
        KeyCode::Right => {
            let len = line_len(&prompt.value);
            if prompt.cursor_col < len {
                prompt.cursor_col += 1;
            }
        }
        KeyCode::Home => prompt.cursor_col = 0,
        KeyCode::End => prompt.cursor_col = line_len(&prompt.value),
        KeyCode::Backspace => {
            if prompt.cursor_col > 0 {
                let end = char_to_byte_index(&prompt.value, prompt.cursor_col);
                let start = char_to_byte_index(&prompt.value, prompt.cursor_col - 1);
                prompt.value.replace_range(start..end, "");
                prompt.cursor_col -= 1;
            }
        }
        KeyCode::Delete => {
            let len = line_len(&prompt.value);
            if prompt.cursor_col < len {
                let start = char_to_byte_index(&prompt.value, prompt.cursor_col);
                let end = char_to_byte_index(&prompt.value, prompt.cursor_col + 1);
                prompt.value.replace_range(start..end, "");
            }
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                let byte = char_to_byte_index(&prompt.value, prompt.cursor_col);
                prompt.value.insert(byte, c);
                prompt.cursor_col += 1;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn submit_explorer_prompt(app: &mut App) -> Result<(), AppError> {
    let Some(prompt) = app.explorer_prompt.take() else {
        return Ok(());
    };
    let name = prompt.value.trim();
    if name.is_empty() {
        app.message = Some("Nama file/folder tidak boleh kosong".to_string());
        return Ok(());
    }
    let target = app.explorer_root.join(name);
    let resolved = match app.workspace.resolve(&target) {
        Ok(path) => path,
        Err(error) => {
            app.message = Some(format!("Gagal membuat path: {}", error));
            return Ok(());
        }
    };
    if resolved.exists() {
        app.message = Some("Path sudah ada".to_string());
        return Ok(());
    }

    let create_result = match prompt.kind {
        ExplorerPromptKind::File => app.workspace.create_file(&resolved, "").map(|_| {
            app.message = Some(format!("File dibuat: {}", name));
        }),
        ExplorerPromptKind::Folder => app.workspace.create_folder(&resolved).map(|_| {
            app.message = Some(format!("Folder dibuat: {}", name));
        }),
    };
    if let Err(error) = create_result {
        app.message = Some(format!("Gagal membuat item: {}", error));
        return Ok(());
    }
    if let Err(error) = app.refresh_explorer() {
        app.message = Some(format!("Item dibuat, tapi refresh explorer gagal: {}", error));
        return Ok(());
    }
    select_explorer_path(app, &resolved);
    Ok(())
}

pub(super) fn select_explorer_path(app: &mut App, path: &Path) {
    if let Some(index) = app.explorer.items.iter().position(|item| item.path == path) {
        app.explorer.selected = index;
    }
}

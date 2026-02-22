//! Module: src/app/ui/mod.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::terminal::PseudoTerminal;
use super::{language, tab_name, AiRole, App, EditorBuffer, Focus, LeftPanel};

pub(super) fn draw(app: &mut App, frame: &mut ratatui::Frame) {
    app.ui_regions = super::UiRegions::default();
    let size = frame.size();
    let bottom_height = if matches!(app.settings.density, super::style::UiDensity::Compact) {
        1
    } else {
        2
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(bottom_height),
        ])
        .split(size);

    draw_header(app, frame, rows[0]);
    draw_main(app, frame, rows[1]);
    draw_status(app, frame, rows[2]);
    draw_command(app, frame, rows[3]);
}

fn draw_header(app: &App, frame: &mut ratatui::Frame, area: Rect) {
    let panel = focus_label(app.focus);
    let root_raw = app.workspace.root().to_string_lossy().to_string();
    let root_max = area.width.saturating_sub(24) as usize;
    let root = ellipsize_middle(&root_raw, root_max.max(10));

    let line = Line::from(vec![
        Span::styled(" xone ", app.theme.brand()),
        Span::styled(" ", app.theme.header()),
        Span::styled(root, app.theme.text()),
        Span::styled(" | ", app.theme.separator()),
        Span::styled("mode:", app.theme.muted()),
        Span::styled(" ", app.theme.muted()),
        Span::styled(panel, app.theme.accent().add_modifier(Modifier::BOLD)),
    ]);
    let widget = Paragraph::new(line).style(app.theme.header());
    frame.render_widget(widget, area);
}

fn draw_tabs(app: &App, frame: &mut ratatui::Frame, area: Rect) {
    let mut spans = Vec::new();
    let mut used = 0usize;
    let max_cols = area.width.max(1) as usize;
    if app.editor.buffers.is_empty() {
        spans.push(Span::styled(" no open file ", app.theme.tab_inactive()));
    } else {
        for (index, buffer) in app.editor.buffers.iter().enumerate() {
            let dirty = if buffer.dirty { "*" } else { "" };
            let label = format!(
                " [{}] {}{} ",
                index + 1,
                ellipsize_end(&tab_name(buffer), 20),
                dirty
            );
            let label_width = label.chars().count();
            if used + label_width >= max_cols {
                if used < max_cols {
                    spans.push(Span::styled(" ...", app.theme.muted()));
                }
                break;
            }
            let style = if index == app.editor.active {
                app.theme.tab_active()
            } else {
                app.theme.tab_inactive()
            };
            spans.push(Span::styled(label, style));
            used += label_width;
            if index + 1 < app.editor.buffers.len() {
                if used + 1 >= max_cols {
                    break;
                }
                spans.push(Span::styled(" ", app.theme.separator()));
                used += 1;
            }
        }
    }
    let widget = Paragraph::new(Line::from(spans)).style(app.theme.panel_alt());
    frame.render_widget(widget, area);
}

fn draw_main(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    if matches!(app.focus, Focus::Settings) {
        draw_settings(app, frame, area);
        return;
    }
    if matches!(app.focus, Focus::Shortcuts) {
        draw_shortcuts(app, frame, area);
        return;
    }
    if matches!(app.focus, Focus::About) {
        draw_about(app, frame, area);
        return;
    }
    let main_area = if app.explorer_visible {
        let explorer_width = if matches!(app.left_panel, LeftPanel::AiChat) {
            50
        } else {
            match app.settings.density {
                super::style::UiDensity::Compact => {
                    if area.width >= 160 {
                        28
                    } else if area.width >= 120 {
                        36
                    } else {
                        42
                    }
                }
                super::style::UiDensity::Comfortable => {
                    if area.width >= 160 {
                        32
                    } else if area.width >= 120 {
                        40
                    } else {
                        45
                    }
                }
            }
        };
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(explorer_width),
                Constraint::Percentage(100 - explorer_width),
            ])
            .split(area);
        if matches!(app.left_panel, LeftPanel::AiChat) {
            draw_ai_chat(app, frame, columns[0]);
        } else {
            draw_explorer(app, frame, columns[0]);
        }
        columns[1]
    } else {
        area
    };
    if app.terminal_visible {
        // Mode stabil/default: split tetap agar layout konsisten.
        let editor_percent: u16 = if matches!(app.settings.density, super::style::UiDensity::Compact) {
            60
        } else {
            55
        };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(editor_percent),
                Constraint::Percentage(100 - editor_percent),
            ])
            .split(main_area);
        draw_editor(app, frame, rows[0]);
        draw_terminal(app, frame, rows[1]);
    } else {
        draw_editor(app, frame, main_area);
    }
}

fn draw_terminal(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let active = matches!(app.focus, Focus::Terminal);
    let title = if active {
        format!(
            "Terminal [ACTIVE] [tab {}/{}]",
            app.terminal.active_index() + 1,
            app.terminal.tab_count()
        )
    } else {
        format!(
            "Terminal [tab {}/{}]",
            app.terminal.active_index() + 1,
            app.terminal.tab_count()
        )
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if active {
            app.theme.border_active()
        } else {
            app.theme.border()
        })
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.ui_regions.terminal = Some(inner);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if !app.terminal.is_running() {
        if let Err(error) = app.terminal.ensure_started(&app.explorer_root) {
            app.message = Some(format!("Gagal membuka terminal: {}", error));
            return;
        }
        let root = app.explorer_root.clone();
        app.update_terminal_cwd(&root);
    }
    app.terminal.resize(inner.height, inner.width);
    let widget = PseudoTerminal::new(app.terminal.screen());
    frame.render_widget(widget, inner);
}

fn draw_settings(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Settings [ACTIVE]")
        .border_style(app.theme.border_active())
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);

    let header = Line::from(vec![
        Span::styled("Settings", app.theme.accent()),
        Span::styled("  ", app.theme.separator()),
        Span::styled("Up/Down pilih", app.theme.muted()),
        Span::styled("  ", app.theme.separator()),
        Span::styled("Enter toggle", app.theme.muted()),
        Span::styled("  ", app.theme.separator()),
        Span::styled("Left/Right ubah opsi", app.theme.muted()),
        Span::styled("  ", app.theme.separator()),
        Span::styled("Esc/F2 tutup", app.theme.muted()),
    ]);
    frame.render_widget(Paragraph::new(header).style(app.theme.panel_alt()), rows[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(63), Constraint::Percentage(37)])
        .split(rows[1]);

    let items = vec![
        (
            "Syntax Highlight by Extension",
            app.settings.syntax_highlight,
            "Deteksi bahasa otomatis dari extension file",
        ),
        (
            "Code Suggestion",
            app.settings.code_suggestions,
            "Saran snippet konteks (Ctrl+K pilih, Ctrl+J terapkan)",
        ),
        (
            "Appearance Preset",
            true,
            app.settings.appearance.label(),
        ),
        ("Accent Color", true, app.settings.accent.label()),
        ("Tab Color Theme", true, app.settings.tab_theme.label()),
        ("Syntax Color Theme", true, app.settings.syntax_theme.label()),
        ("UI Density", true, app.settings.density.label()),
        (
            "Terminal Style FX",
            app.settings.terminal_fx,
            if app.settings.terminal_fx {
                "Prompt visual modern aktif"
            } else {
                "Prompt standar PowerShell"
            },
        ),
        (
            "Terminal Command Auto Sync",
            app.settings.terminal_command_sync,
            if app.settings.terminal_command_sync {
                "Parse command cd/sl/pushd untuk sinkron explorer"
            } else {
                "Hanya sinkron dari marker shell"
            },
        ),
        (
            "Hard Mode",
            app.settings.hard_mode,
            if app.settings.hard_mode {
                "Mouse dinonaktifkan (keyboard only)"
            } else {
                "Mode normal (mouse aktif)"
            },
        ),
        (
            "AI Profile",
            true,
            app.settings.ai_profile.label(),
        ),
        (
            "AI Provider",
            true,
            app.ai_chat.config.provider.as_str(),
        ),
        (
            "AI Base URL",
            true,
            app.ai_chat.config.base_url.as_str(),
        ),
        (
            "AI Model",
            true,
            app.ai_chat.config.model.as_str(),
        ),
        (
            "AI API Key",
            app.ai_chat.api_key_present,
            if app.ai_chat.api_key_present {
                "Tersimpan (masked) - Enter dari clipboard, Delete hapus"
            } else {
                "Belum diisi - Daftar key provider lalu Enter set dari clipboard"
            },
        ),
    ];
    let category_for_index = |index: usize| -> &'static str {
        match index {
            0..=1 => "Editor",
            2..=6 => "UI",
            7..=9 => "Terminal",
            10..=14 => "AI",
            _ => "General",
        }
    };
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(index, (label, enabled, detail))| {
            let category = category_for_index(index);
            let marker = match category {
                "Editor" => "EDT",
                "UI" => "UI ",
                "Terminal" => "TRM",
                "AI" => "AI ",
                _ => "GEN",
            };
            let marker_style = match category {
                "Editor" => app.theme.success().add_modifier(Modifier::BOLD),
                "Terminal" => app.theme.warning().add_modifier(Modifier::BOLD),
                "AI" => app.theme.accent().add_modifier(Modifier::BOLD),
                _ => app.theme.accent().add_modifier(Modifier::BOLD),
            };
            let line = Line::from(vec![
                Span::styled(format!(" {} ", marker), marker_style),
                Span::styled(format!("{} ", label), app.theme.text()),
                Span::styled(format!("- {}", detail), app.theme.muted()),
                Span::styled(
                    format!("  [{}]", if *enabled { "ON" } else { "OFF" }),
                    if *enabled {
                        app.theme.success()
                    } else {
                        app.theme.warning()
                    },
                ),
            ]);
            let mut item = ListItem::new(line);
            if index == app.settings.selected {
                item = item.style(app.theme.selection());
            }
            item
        })
        .collect();
    let list = List::new(list_items)
        .style(app.theme.panel())
        .block(Block::default().borders(Borders::ALL).title("Opsi"));
    frame.render_widget(list, body[0]);

    let (selected_label, _, selected_detail) = items
        .get(app.settings.selected.min(items.len().saturating_sub(1)))
        .unwrap_or(&("N/A", false, "N/A"));
    let selected_category = category_for_index(app.settings.selected);
    let mut details: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Detail", app.theme.accent()),
            Span::styled("  ", app.theme.separator()),
            Span::styled(*selected_label, app.theme.text().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Kategori", app.theme.accent()),
            Span::styled(": ", app.theme.separator()),
            Span::styled(selected_category, app.theme.muted()),
        ]),
        Line::from(Span::styled(*selected_detail, app.theme.muted())),
        Line::from(Span::styled("", app.theme.text())),
        Line::from(vec![
            Span::styled("AI Config", app.theme.accent()),
            Span::styled(": .xone/ai/ai.conf", app.theme.muted()),
        ]),
        Line::from(vec![
            Span::styled("API Key", app.theme.accent()),
            Span::styled(
                if app.ai_chat.api_key_present {
                    ": tersedia"
                } else {
                    ": belum diisi"
                },
                if app.ai_chat.api_key_present {
                    app.theme.success()
                } else {
                    app.theme.warning()
                },
            ),
        ]),
        Line::from(Span::styled(
            "Daftar key: OpenAI/Groq/Together/OpenRouter/Mistral",
            app.theme.muted(),
        )),
        Line::from(Span::styled("Quick: 1=ollama 2=openai 3=groq", app.theme.muted())),
    ];
    if !app.ai_chat.api_key_present && !app.ai_chat.config.provider.eq_ignore_ascii_case("ollama") {
        details.push(Line::from(Span::styled(
            "Provider ini butuh API key. Isi via clipboard pada item AI API Key.",
            app.theme.warning().add_modifier(Modifier::BOLD),
        )));
    }
    let detail_widget = Paragraph::new(Text::from(details))
        .style(app.theme.panel_alt())
        .block(Block::default().borders(Borders::ALL).title("Info"))
        .wrap(Wrap { trim: true });
    frame.render_widget(detail_widget, body[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Status ", app.theme.muted()),
        Span::styled(
            format!(
                "syntax={} suggestion={} appearance={} accent={} density={} termfx={} hard={} ai_profile={} ai_key={}",
                if app.settings.syntax_highlight {
                    "ON"
                } else {
                    "OFF"
                },
                if app.settings.code_suggestions {
                    "ON"
                } else {
                    "OFF"
                },
                app.settings.appearance.as_str(),
                app.settings.accent.as_str(),
                app.settings.density.as_str(),
                if app.settings.terminal_fx { "on" } else { "off" },
                if app.settings.hard_mode { "on" } else { "off" },
                app.settings.ai_profile.label(),
                if app.ai_chat.api_key_present { "yes" } else { "no" }
            ),
            app.theme.accent().add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(app.theme.panel_alt());
    frame.render_widget(footer, rows[2]);
}

fn draw_about(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("About Xone [ACTIVE]")
        .border_style(app.theme.border_active())
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let lines = build_about_lines(app);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(inner);

    let hero = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("XONE", app.theme.accent().add_modifier(Modifier::BOLD)),
            Span::styled("  ", app.theme.separator()),
            Span::styled(
                "Terminal Editor + Workspace Orchestrator",
                app.theme.text().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Keyboard-first. Fast feedback. Focused flow.", app.theme.muted()),
            Span::styled("  ", app.theme.separator()),
            Span::styled("F1/Esc close", app.theme.tab_inactive()),
        ]),
    ]))
    .style(app.theme.panel_alt());
    frame.render_widget(hero, rows[0]);

    if rows[1].width >= 96 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(34), Constraint::Min(1)])
            .split(rows[1]);

        let info = Paragraph::new(Text::from(vec![
            Line::from(Span::styled("PROFILE", app.theme.accent().add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("", app.theme.text())),
            Line::from(vec![
                Span::styled("Root ", app.theme.muted()),
                Span::styled(
                    ellipsize_middle(&app.workspace.root().to_string_lossy(), 24),
                    app.theme.text(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Theme ", app.theme.muted()),
                Span::styled(app.settings.appearance.label(), app.theme.text()),
            ]),
            Line::from(vec![
                Span::styled("Accent ", app.theme.muted()),
                Span::styled(app.settings.accent.label(), app.theme.text()),
            ]),
            Line::from(vec![
                Span::styled("AI ", app.theme.muted()),
                Span::styled(
                    format!("{} / {}", app.settings.ai_profile.label(), app.ai_chat.config.provider),
                    app.theme.text(),
                ),
            ]),
            Line::from(Span::styled("", app.theme.text())),
            Line::from(Span::styled("TIPS", app.theme.accent().add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("Ctrl+E Explorer", app.theme.muted())),
            Line::from(Span::styled("Ctrl+I Editor", app.theme.muted())),
            Line::from(Span::styled("Ctrl+T Terminal", app.theme.muted())),
            Line::from(Span::styled("F3 Shortcuts", app.theme.muted())),
            Line::from(Span::styled("F6 AI Chat", app.theme.muted())),
        ]))
        .style(app.theme.panel_alt())
        .block(Block::default().borders(Borders::ALL).title("Identity"));
        frame.render_widget(info, cols[0]);

        let max_lines = cols[1].height.saturating_sub(2) as usize;
        let total = lines.len();
        let max_scroll = total.saturating_sub(max_lines);
        app.about_scroll = app.about_scroll.min(max_scroll);
        let start = app.about_scroll.min(total);
        let end = (start + max_lines).min(total);
        let visible = if start < end {
            lines[start..end].to_vec()
        } else {
            vec![Line::from(Span::styled("", app.theme.text()))]
        };
        frame.render_widget(
            Paragraph::new(Text::from(visible))
                .style(app.theme.panel_alt())
                .block(Block::default().borders(Borders::ALL).title("Story"))
                .wrap(Wrap { trim: false }),
            cols[1],
        );
    } else {
        let max_lines = rows[1].height as usize;
        let total = lines.len();
        let max_scroll = total.saturating_sub(max_lines);
        app.about_scroll = app.about_scroll.min(max_scroll);
        let start = app.about_scroll.min(total);
        let end = (start + max_lines).min(total);
        let visible = if start < end {
            lines[start..end].to_vec()
        } else {
            vec![Line::from(Span::styled("", app.theme.text()))]
        };
        frame.render_widget(
            Paragraph::new(Text::from(visible))
                .style(app.theme.panel_alt())
                .wrap(Wrap { trim: false }),
            rows[1],
        );
    }

    let total = lines.len().max(1);
    let shown = app.about_scroll + 1;
    let footer = Line::from(vec![
        Span::styled("Scroll ", app.theme.muted()),
        Span::styled("Up/Down/PgUp/PgDn/Home", app.theme.tab_inactive()),
        Span::styled("  ", app.theme.separator()),
        Span::styled(format!("line {}/{}", shown.min(total), total), app.theme.muted()),
    ]);
    frame.render_widget(Paragraph::new(footer).style(app.theme.panel_alt()), rows[2]);
}

fn build_about_lines(app: &App) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for raw in &app.about_lines {
        let line = raw.trim_end();
        if let Some(rest) = line.strip_prefix("### ") {
            out.push(Line::from(Span::styled(
                format!("{} {}", ">", rest),
                app.theme.warning().add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            out.push(Line::from(Span::styled(
                format!("{} {}", ">>", rest),
                app.theme.accent().add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            out.push(Line::from(Span::styled(
                format!("{} {}", ">>>", rest),
                app.theme.accent().add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            out.push(Line::from(vec![
                Span::styled("• ", app.theme.accent()),
                Span::styled(rest.to_string(), app.theme.text()),
            ]));
            continue;
        }
        if line.starts_with("```") {
            out.push(Line::from(Span::styled(
                "────────────────────────",
                app.theme.separator(),
            )));
            continue;
        }
        if line.is_empty() {
            out.push(Line::from(Span::styled("", app.theme.text())));
        } else {
            out.push(Line::from(Span::styled(line.to_string(), app.theme.text())));
        }
    }
    if out.is_empty() {
        out.push(Line::from(Span::styled(
            "About belum tersedia. Tambahkan about/xone-profile.md",
            app.theme.warning(),
        )));
    }
    out
}

fn draw_shortcuts(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Shortcuts [ACTIVE]")
        .border_style(app.theme.border_active())
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let header = if app.shortcuts_capture_mode {
        "Capture Mode: tekan kombinasi tombol baru, Esc batal"
    } else {
        "Up/Down pilih | Enter ubah | Delete reset default | Esc/F3 tutup"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(header, app.theme.muted()))).style(app.theme.panel_alt()),
        rows[0],
    );

    let actions = super::control::customizable_actions();
    let selected = app.shortcuts_selected.min(actions.len().saturating_sub(1));
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let binding = app
                .shortcut_bindings
                .iter()
                .find(|(a, _)| a == action)
                .map(|(_, b)| super::control::binding_to_text(*b))
                .unwrap_or_else(|| super::control::binding_to_text(super::control::default_binding(*action)));
            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<22}", super::control::action_label(*action)),
                    app.theme.text(),
                ),
                Span::styled(" -> ", app.theme.separator()),
                Span::styled(binding, app.theme.accent().add_modifier(Modifier::BOLD)),
            ]);
            let mut item = ListItem::new(line);
            if index == selected {
                item = item.style(app.theme.selection());
            }
            item
        })
        .collect();
    frame.render_widget(
        List::new(items)
            .style(app.theme.panel())
            .block(Block::default().borders(Borders::ALL).title("Keymap")),
        rows[1],
    );
}

fn draw_explorer(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let focused = matches!(app.focus, Focus::Explorer);
    let scope = app.workspace.relative(&app.explorer_root);
    let scope_text = if scope.as_os_str().is_empty() {
        ".".to_string()
    } else {
        scope.to_string_lossy().to_string()
    };
    let total = app.explorer.items.len();
    let folder_count = app.explorer.items.iter().filter(|item| item.is_dir).count();
    let file_count = total.saturating_sub(folder_count);
    let unit = if total == 1 { "item" } else { "items" };
    let title = format!(
        "Explorer{} | {} {} | {}D {}F | {}",
        if focused { " [ACTIVE]" } else { "" },
        total,
        unit,
        folder_count,
        file_count,
        scope_text
    );
    let title = fit_title(&title, area.width);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if focused {
            app.theme.border_active()
        } else {
            app.theme.border()
        });
    let list_area = block.inner(area);
    app.ui_regions.explorer = Some(list_area);
    let selected_range = app.selection_range(Focus::Explorer);
    let items: Vec<ListItem> = if app.explorer.items.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (tidak ada file/folder)",
            app.theme.muted(),
        )))]
    } else {
        app.explorer
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let connector = explorer_connector(&app.explorer.items, index);
                let name = if item.is_dir {
                    format!("{}/", item.name)
                } else {
                    item.name.clone()
                };
                let row_budget = area.width.saturating_sub(7) as usize;
                let connector_width = connector.chars().count();
                let section_tag_width = 4usize;
                let name = ellipsize_end(
                    &name,
                    row_budget
                        .saturating_sub(connector_width)
                        .saturating_sub(section_tag_width),
                );
                let name_style = if item.is_dir {
                    app.theme.accent().add_modifier(Modifier::BOLD)
                } else {
                    app.theme.text()
                };
                let section_tag = if item.is_dir { "D  " } else { "F  " };
                let section_style = if item.is_dir {
                    app.theme.accent().add_modifier(Modifier::BOLD)
                } else {
                    app.theme.warning().add_modifier(Modifier::BOLD)
                };
                let mut list_item = ListItem::new(Line::from(vec![
                    Span::styled(section_tag, section_style),
                    Span::styled(connector, app.theme.separator()),
                    Span::styled(name, name_style),
                ]));
                if in_range(selected_range, index) {
                    list_item = list_item.style(app.theme.selection());
                }
                list_item
            })
            .collect()
    };
    let list = List::new(items)
        .block(block)
        .style(app.theme.panel())
        .highlight_style(app.theme.selection().add_modifier(Modifier::BOLD))
        .highlight_symbol(" > ");
    let mut state = ratatui::widgets::ListState::default();
    if !app.explorer.items.is_empty() {
        state.select(Some(app.explorer.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_ai_chat(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let focused = matches!(app.focus, Focus::AiChat);
    let title = if focused {
        format!(
            "x1.AI [ACTIVE] [{}:{}]{}",
            app.ai_chat.config.provider,
            app.ai_chat.config.model,
            if app.ai_chat.inflight { " [RUNNING]" } else { "" }
        )
    } else {
        format!(
            "x1.AI [{}:{}]{}",
            app.ai_chat.config.provider,
            app.ai_chat.config.model,
            if app.ai_chat.inflight { " [RUNNING]" } else { "" }
        )
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(fit_title(&title, area.width))
        .border_style(if focused {
            app.theme.border_active()
        } else {
            app.theme.border()
        })
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.ui_regions.explorer = None;
    if inner.height < 3 {
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(2)])
        .split(inner);

    let status = if app.ai_chat.inflight {
        "LIVE"
    } else {
        "IDLE"
    };
    let mode = if app.ai_chat.read_only_mode {
        "REVIEW"
    } else {
        "DIRECT"
    };
    let hero = Line::from(vec![
        Span::styled(" x1.AI ", app.theme.accent().add_modifier(Modifier::BOLD)),
        Span::styled(" studio chat", app.theme.text().add_modifier(Modifier::BOLD)),
        Span::styled("  ", app.theme.separator()),
        Span::styled(
            format!("{}  {}  {}", status, mode, app.settings.ai_profile.label()),
            if app.ai_chat.inflight {
                app.theme.warning().add_modifier(Modifier::BOLD)
            } else {
                app.theme.muted()
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(hero).style(app.theme.panel_alt()), rows[0]);

    let transcript_width = rows[1].width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = build_ai_transcript_lines(app, transcript_width.max(16));
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Belum ada percakapan di x1.AI",
            app.theme.muted(),
        )));
    }
    let max_lines = rows[1].height as usize;
    let total = lines.len();
    let base_start = total.saturating_sub(max_lines);
    let start = base_start.saturating_sub(app.ai_chat.scroll);
    let end = (start + max_lines).min(total);
    let visible = if start < end {
        lines[start..end].to_vec()
    } else {
        vec![Line::from(Span::styled("", app.theme.text()))]
    };
    frame.render_widget(
        Paragraph::new(Text::from(visible))
            .style(app.theme.panel_alt())
            .block(Block::default().borders(Borders::ALL).title("Transcript"))
            .wrap(Wrap { trim: false }),
        rows[1],
    );

    let prefix = " compose> ";
    let hint = if app.ai_chat.inflight {
        if rows[2].width < 44 {
            "  processing"
        } else {
            "  x1.AI memproses...  Esc keluar"
        }
    } else if !app.ai_chat.api_key_present && !app.ai_chat.config.provider.eq_ignore_ascii_case("ollama")
    {
        if rows[2].width < 60 {
            "  No key: cek Settings"
        } else {
            "  No API key: daftar provider dulu (lihat Settings)"
        }
    } else {
        if rows[2].width < 74 {
            "  Enter kirim  Ctrl+Enter insert  Ctrl+Shift+C copy code"
        } else {
            "  Enter kirim  Ctrl+Enter insert  Alt+Enter replace  Ctrl+Y confirm  Ctrl+R mode  Ctrl+Shift+C copy code  Esc keluar"
        }
    };
    let prefix_width = prefix.chars().count();
    let composer_width = rows[2].width as usize;
    let hint_max = composer_width.saturating_sub(prefix_width + 8);
    let hint_text = if composer_width < 28 {
        String::new()
    } else {
        ellipsize_end(hint, hint_max.max(8))
    };
    let hint_width = hint_text.chars().count();
    let mut input_width = composer_width
        .saturating_sub(prefix_width + hint_width)
        .max(1);
    if input_width < 16 && composer_width > prefix_width + 16 {
        input_width = 16;
    }
    let (visible_input, visible_cursor_col) = prompt_visible_input(
        &app.ai_chat.input,
        app.ai_chat.cursor_col,
        input_width,
    );
    let input_line = Line::from(vec![
        Span::styled(prefix, app.theme.tab_active()),
        Span::styled(visible_input, app.theme.text()),
        Span::styled(hint_text, app.theme.muted()),
    ]);
    frame.render_widget(
        Paragraph::new(input_line).style(app.theme.command()),
        rows[2],
    );
    if focused {
        let max_x = rows[2].width.saturating_sub(1) as usize;
        let cursor_x = (prefix_width + visible_cursor_col).min(max_x);
        frame.set_cursor(rows[2].x + cursor_x as u16, rows[2].y);
    }

    if let Some(preview) = app.ai_chat.pending_apply.as_ref() {
        let mode_label = if preview.replace_selection {
            "replace selection"
        } else {
            "insert at cursor"
        };
        let mut preview_lines = vec![Line::from(vec![
            Span::styled(" REVIEW ", app.theme.warning().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}  ", mode_label), app.theme.muted()),
        ])];
        for diff in preview.diff_lines.iter().take(4) {
            let style = if diff.starts_with('+') {
                app.theme.syntax_value()
            } else if diff.starts_with('-') {
                app.theme.warning()
            } else {
                app.theme.muted()
            };
            preview_lines.push(Line::from(Span::styled(
                ellipsize_end(diff, rows[1].width.saturating_sub(4) as usize),
                style,
            )));
        }
        frame.render_widget(
            Paragraph::new(Text::from(preview_lines)).style(app.theme.panel_alt()),
            Rect {
                x: rows[1].x.saturating_add(1),
                y: rows[1].y,
                width: rows[1].width.saturating_sub(2),
                height: 5.min(rows[1].height),
            },
        );
    }
}

fn build_ai_transcript_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for msg in &app.ai_chat.messages {
        let (tag, tag_style, body_style) = match msg.role {
            AiRole::User => (
                " YOU ",
                app.theme.accent().add_modifier(Modifier::BOLD),
                app.theme.text(),
            ),
            AiRole::Assistant => (
                " x1.AI ",
                app.theme.tab_active().add_modifier(Modifier::BOLD),
                app.theme.text().add_modifier(Modifier::BOLD),
            ),
        };
        lines.push(Line::from(vec![
            Span::styled(tag, tag_style),
            Span::styled(" ", app.theme.separator()),
            Span::styled(
                if matches!(msg.role, AiRole::Assistant) {
                    "assistant"
                } else {
                    "user"
                },
                app.theme.muted(),
            ),
        ]));

        let mut in_code = false;
        let mut code_lang = language::Language::PlainText;
        for raw in msg.content.lines() {
            if raw.trim_start().starts_with("```") {
                if in_code {
                    in_code = false;
                    code_lang = language::Language::PlainText;
                } else {
                    in_code = true;
                    code_lang = language_from_fence(raw);
                }
                lines.push(Line::from(Span::styled(
                    "  ----------------- code -----------------",
                    app.theme.separator(),
                )));
                continue;
            }

            let (prefix, style, text) = if in_code {
                ("   | ", app.theme.syntax_value(), raw.to_string())
            } else if let Some(rest) = raw.strip_prefix("### ") {
                (
                    "   > ",
                    app.theme.warning().add_modifier(Modifier::BOLD),
                    rest.to_string(),
                )
            } else if let Some(rest) = raw.strip_prefix("## ") {
                (
                    "   > ",
                    app.theme.accent().add_modifier(Modifier::BOLD),
                    rest.to_string(),
                )
            } else if let Some(rest) = raw.strip_prefix("# ") {
                (
                    "   > ",
                    app.theme.accent().add_modifier(Modifier::BOLD),
                    rest.to_string(),
                )
            } else if let Some(rest) = raw.strip_prefix("- ") {
                ("   - ", app.theme.text(), rest.to_string())
            } else {
                ("   ", body_style, raw.to_string())
            };

            let max_text = width.saturating_sub(prefix.chars().count()).max(8);
            for chunk in wrap_text_simple(&text, max_text) {
                let mut spans = vec![Span::styled(prefix.to_string(), app.theme.separator())];
                if in_code && app.settings.syntax_highlight {
                    let base = style;
                    for token in cached_highlight_line(code_lang, &chunk) {
                        let token_style = syntax_style_for(app, token.kind, base, false);
                        spans.push(Span::styled(token.text, token_style));
                    }
                } else {
                    spans.push(Span::styled(chunk, style));
                }
                lines.push(Line::from(spans));
            }
        }
        lines.push(Line::from(Span::styled("", app.theme.text())));
    }
    lines
}

fn language_from_fence(line: &str) -> language::Language {
    let lang = line
        .trim_start()
        .trim_start_matches("```")
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match lang.as_str() {
        "rs" | "rust" => language::Language::Rust,
        "js" | "javascript" | "node" => language::Language::JavaScript,
        "ts" | "typescript" | "tsx" => language::Language::TypeScript,
        "html" => language::Language::Html,
        "css" => language::Language::Css,
        "json" => language::Language::Json,
        "md" | "markdown" => language::Language::Markdown,
        "py" | "python" => language::Language::Python,
        "c" => language::Language::C,
        "cpp" | "c++" | "cxx" => language::Language::Cpp,
        "csharp" | "cs" => language::Language::CSharp,
        "java" => language::Language::Java,
        "go" | "golang" => language::Language::Go,
        "yaml" | "yml" => language::Language::Yaml,
        "toml" => language::Language::Toml,
        "sh" | "bash" | "zsh" | "shell" => language::Language::Shell,
        "ps1" | "powershell" | "pwsh" => language::Language::PowerShell,
        _ => language::Language::PlainText,
    }
}

fn wrap_text_simple(input: &str, max_width: usize) -> Vec<String> {
    if input.is_empty() {
        return vec![" ".to_string()];
    }
    let max_width = max_width.max(1);
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max_width {
        return vec![input.to_string()];
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_width).min(chars.len());
        out.push(chars[start..end].iter().collect());
        start = end;
    }
    out
}

fn draw_editor(app: &mut App, frame: &mut ratatui::Frame, area: Rect) {
    let focused = matches!(app.focus, Focus::Editor);
    let tab_count = app.editor.buffers.len();
    let title = match app.editor.current() {
        Some(buffer) => {
            let name = app
                .workspace
                .relative(&buffer.path)
                .to_string_lossy()
                .to_string();
            let dirty = if buffer.dirty { "*" } else { "" };
            let lang = language::language_label(buffer.language);
            format!(
                "Editor{} | {} | {} tabs | {}{}",
                if focused { " [ACTIVE]" } else { "" },
                lang,
                tab_count,
                name,
                dirty
            )
        }
        None => format!(
            "Editor{} - (tidak ada file)",
            if focused { " [ACTIVE]" } else { "" }
        ),
    };
    let title = fit_title(&title, area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if focused {
            app.theme.border_active()
        } else {
            app.theme.border()
        })
        .style(app.theme.panel_alt());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        app.editor_view_height = 1;
        app.editor_view_width = 1;
        return;
    }

    let editor_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    let tabs_area = editor_rows[0];
    let content_area = editor_rows[1];
    app.ui_regions.editor = Some(content_area);
    draw_tabs(app, frame, tabs_area);

    app.editor_view_height = content_area.height.max(1) as usize;
    app.editor_view_width = content_area.width.max(1) as usize;

    if let Some(cursor_row) = app.editor.current().map(|buffer| buffer.cursor_row) {
        let visual_col = app.cached_visual_width_for_active_cursor();
        let gutter = editor_gutter_width(cursor_row);
        let visible_cols = app.editor_view_width.saturating_sub(gutter).max(1);
        if let Some(buffer) = app.editor.current_mut() {
            if visual_col < buffer.hscroll {
                buffer.hscroll = visual_col;
            } else if visual_col >= buffer.hscroll + visible_cols {
                buffer.hscroll = visual_col.saturating_sub(visible_cols - 1);
            }
        }
    }

    let content = match app.editor.current() {
        Some(buffer) => editor_text(app, buffer, content_area.height as usize, content_area.width as usize),
        None => Text::from(Line::from("Buka file dari Explorer")),
    };

    let paragraph = Paragraph::new(content)
        .style(app.theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, content_area);

    if let Some((cursor_row, scroll)) = app.editor.current().map(|b| (b.cursor_row, b.scroll)) {
        if matches!(app.focus, Focus::Editor) && cursor_row >= scroll {
            let y = cursor_row - scroll;
            if y < content_area.height as usize {
                let gutter = editor_gutter_width(cursor_row);
                let max_x = content_area.width.saturating_sub(1) as usize;
                let visual_col = app.cached_visual_width_for_active_cursor();
                let hscroll = app.editor.current().map(|b| b.hscroll).unwrap_or(0);
                let x_col = (gutter + visual_col.saturating_sub(hscroll)).min(max_x);
                frame.set_cursor(content_area.x + x_col as u16, content_area.y + y as u16);
            }
        }
    }
}

fn editor_text(app: &App, buffer: &EditorBuffer, height: usize, width: usize) -> Text<'static> {
    let mut lines = Vec::new();
    let start = buffer.scroll.min(buffer.lines.len());
    let end = (start + height).min(buffer.lines.len());
    let selected_range = app.selection_range(Focus::Editor);
    for (index, line) in buffer.lines[start..end].iter().enumerate() {
        let row = start + index;
        let number = row + 1;
        let bookmark = if buffer.bookmarks.contains(&row) {
            "*"
        } else {
            " "
        };
        let content_style = if matches!(app.focus, Focus::Editor) && row == buffer.cursor_row {
            app.theme.text().add_modifier(Modifier::BOLD)
        } else {
            app.theme.text()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:>4}", number), app.theme.gutter()),
            Span::raw(" "),
            Span::styled(
                bookmark,
                if bookmark == "*" {
                    app.theme.warning()
                } else {
                    app.theme.gutter()
                },
            ),
            Span::raw(" "),
        ]));
        if let Some(last) = lines.last_mut() {
            let content_spans = editor_content_spans(
                app,
                buffer,
                row,
                line,
                in_range(selected_range, row),
                content_style,
            );
            let gutter = editor_gutter_width(row);
            let visible_cols = width.saturating_sub(gutter);
            last.spans
                .extend(clip_spans_horizontal(content_spans, buffer.hscroll, visible_cols));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("", app.theme.text())));
    }
    Text::from(lines)
}

fn editor_content_spans(
    app: &App,
    buffer: &EditorBuffer,
    row: usize,
    line: &str,
    selected: bool,
    base_style: Style,
) -> Vec<Span<'static>> {
    const INDENT_WIDTH: usize = 4;
    let mut spans = Vec::new();

    let split = line
        .char_indices()
        .find_map(|(idx, ch)| {
            if ch != ' ' && ch != '\t' {
                Some(idx)
            } else {
                None
            }
        })
        .unwrap_or(line.len());
    let indent = &line[..split];
    let rest = &line[split..];

    let mut level = 0usize;
    let mut pending_spaces = 0usize;
    for ch in indent.chars() {
        match ch {
            '\t' => {
                if pending_spaces > 0 {
                    spans.push(Span::styled(
                        " ".repeat(pending_spaces),
                        maybe_selected_indent_style(app, level, row, selected),
                    ));
                    level += 1;
                    pending_spaces = 0;
                }
                spans.push(Span::styled(
                    " ".repeat(INDENT_WIDTH),
                    maybe_selected_indent_style(app, level, row, selected),
                ));
                level += 1;
            }
            ' ' => {
                pending_spaces += 1;
                if pending_spaces == INDENT_WIDTH {
                    spans.push(Span::styled(
                        " ".repeat(INDENT_WIDTH),
                        maybe_selected_indent_style(app, level, row, selected),
                    ));
                    level += 1;
                    pending_spaces = 0;
                }
            }
            _ => break,
        }
    }
    if pending_spaces > 0 {
        spans.push(Span::styled(
            " ".repeat(pending_spaces),
            maybe_selected_indent_style(app, level, row, selected),
        ));
    }

    let text_style = if selected {
        base_style.patch(app.theme.selection().add_modifier(Modifier::BOLD))
    } else {
        base_style
    };

    if rest.is_empty() {
        if spans.is_empty() {
            spans.push(Span::styled("".to_string(), text_style));
        }
    } else if app.settings.syntax_highlight {
        for token in cached_highlight_line(buffer.language, rest) {
            let token_style = syntax_style_for(app, token.kind, base_style, selected);
            spans.push(Span::styled(
                expand_tabs(token.text.as_str(), INDENT_WIDTH),
                token_style,
            ));
        }
    } else {
        spans.push(Span::styled(expand_tabs(rest, INDENT_WIDTH), text_style));
    }
    spans
}

fn cached_highlight_line(language: language::Language, line: &str) -> Vec<language::Token> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Vec<language::Token>>>> = OnceLock::new();
    const MAX_CACHE_ENTRIES: usize = 8192;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    language.hash(&mut hasher);
    line.hash(&mut hasher);
    let key = hasher.finish();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(tokens) = guard.get(&key) {
            return tokens.clone();
        }
    }

    let tokens = language::highlight_line(language, line);

    if let Ok(mut guard) = cache.lock() {
        if guard.len() >= MAX_CACHE_ENTRIES {
            guard.clear();
        }
        guard.insert(key, tokens.clone());
    }

    tokens
}

fn syntax_style_for(app: &App, kind: language::TokenKind, base: Style, selected: bool) -> Style {
    let highlighted = match kind {
        language::TokenKind::Plain => base,
        language::TokenKind::Keyword => base.patch(app.theme.syntax_keyword()),
        language::TokenKind::Type => base.patch(app.theme.syntax_type()),
        language::TokenKind::String => base.patch(app.theme.syntax_string()),
        language::TokenKind::Number => base.patch(app.theme.syntax_number()),
        language::TokenKind::Comment => base.patch(app.theme.syntax_comment()),
        language::TokenKind::Tag => base.patch(app.theme.syntax_tag()),
        language::TokenKind::Attribute => base.patch(app.theme.syntax_attribute()),
        language::TokenKind::Value => base.patch(app.theme.syntax_value()),
        language::TokenKind::Operator => base.patch(app.theme.syntax_operator()),
    };
    if selected {
        highlighted.patch(app.theme.selection().add_modifier(Modifier::BOLD))
    } else {
        highlighted
    }
}

fn maybe_selected_indent_style(app: &App, level: usize, row: usize, selected: bool) -> Style {
    let style = app.theme.indent_level(level + row);
    if selected {
        style.patch(app.theme.selection().add_modifier(Modifier::BOLD))
    } else {
        style
    }
}

fn draw_status(app: &App, frame: &mut ratatui::Frame, area: Rect) {
    if area.width < 110 {
        draw_status_compact(app, frame, area);
        return;
    }

    let max_cols = area.width.max(1) as usize;
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;

    append_span(
        &mut spans,
        &mut used,
        max_cols,
        "FOCUS ".to_string(),
        app.theme.muted(),
    );
    append_span(
        &mut spans,
        &mut used,
        max_cols,
        focus_label(app.focus).to_string(),
        app.theme.accent().add_modifier(Modifier::BOLD),
    );

    if let Some(buffer) = app.editor.current() {
        let name = app
            .workspace
            .relative(&buffer.path)
            .to_string_lossy()
            .to_string();
        let dirty = if buffer.dirty { "*" } else { "" };
        let lang = language::language_label(buffer.language);
        append_separator(&mut spans, &mut used, max_cols, app);
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!(
                "{}{}  Ln {}  Col {}  [{}]",
                name,
                dirty,
                buffer.cursor_row + 1,
                buffer.cursor_col + 1,
                lang,
            ),
            app.theme.text(),
        );
        let mark_flag = if buffer.bookmarks.contains(&buffer.cursor_row) {
            "on"
        } else {
            "off"
        };
        append_separator(&mut spans, &mut used, max_cols, app);
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!("bookmark {} ({})", mark_flag, buffer.bookmarks.len()),
            app.theme.muted(),
        );
    }

    if let Some((start, end)) = app.selection_range(app.focus) {
        let count = end.saturating_sub(start) + 1;
        let unit = if count > 1 { "lines" } else { "line" };
        append_separator(&mut spans, &mut used, max_cols, app);
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!("select {} {}", count, unit),
            app.theme.accent().add_modifier(Modifier::BOLD),
        );
    }

    if let Some(loading) = app.pending_editor_paste_loading_text() {
        append_separator(&mut spans, &mut used, max_cols, app);
        append_span(&mut spans, &mut used, max_cols, loading, app.theme.muted());
    }

    if let Some(message) = &app.message {
        if !message.is_empty() {
            append_separator(&mut spans, &mut used, max_cols, app);
            append_span(
                &mut spans,
                &mut used,
                max_cols,
                message.clone(),
                status_message_style(app, message),
            );
        }
    }

    append_separator(&mut spans, &mut used, max_cols, app);
    append_span(
        &mut spans,
        &mut used,
        max_cols,
        format!(
            "syntax={} sugg={}",
            if app.settings.syntax_highlight {
                "ON"
            } else {
                "OFF"
            },
            if app.settings.code_suggestions {
                "ON"
            } else {
                "OFF"
            }
        ),
        app.theme.muted(),
    );

    if app.settings.code_suggestions {
        if let Some((suggestion, index, total)) = app.current_editor_suggestion_entry() {
            append_separator(&mut spans, &mut used, max_cols, app);
            append_span(
                &mut spans,
                &mut used,
                max_cols,
                "hint ".to_string(),
                app.theme.muted(),
            );
            append_span(
                &mut spans,
                &mut used,
                max_cols,
                format!(
                    "{}/{} {} (Ctrl+K/Ctrl+J)",
                    index + 1,
                    total,
                    ellipsize_end(suggestion.label, 26)
                ),
                app.theme.accent(),
            );
        }
    }

    let widget = Paragraph::new(Line::from(spans)).style(app.theme.status());
    frame.render_widget(widget, area);
}

fn draw_status_compact(app: &App, frame: &mut ratatui::Frame, area: Rect) {
    let max_cols = area.width.max(1) as usize;
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;

    append_span(
        &mut spans,
        &mut used,
        max_cols,
        format!("{} ", focus_label(app.focus)),
        app.theme.accent().add_modifier(Modifier::BOLD),
    );

    if let Some(buffer) = app.editor.current() {
        let name = app.workspace.relative(&buffer.path).to_string_lossy().to_string();
        append_separator(&mut spans, &mut used, max_cols, app);
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!(
                "{}  {}:{}",
                ellipsize_end(&name, 26),
                buffer.cursor_row + 1,
                buffer.cursor_col + 1
            ),
            app.theme.text(),
        );
    }

    if let Some(message) = &app.message {
        if !message.is_empty() {
            append_separator(&mut spans, &mut used, max_cols, app);
            append_span(
                &mut spans,
                &mut used,
                max_cols,
                ellipsize_end(message, 36),
                status_message_style(app, message),
            );
        }
    }

    let widget = Paragraph::new(Line::from(spans)).style(app.theme.status());
    frame.render_widget(widget, area);
}

fn draw_command(app: &App, frame: &mut ratatui::Frame, area: Rect) {
    if matches!(app.focus, Focus::Settings) {
        let widget = Paragraph::new(Line::from(vec![
            Span::styled(" Up/Down ", app.theme.tab_inactive()),
            Span::styled(" pilih ", app.theme.muted()),
            Span::styled(" Left/Right ", app.theme.tab_inactive()),
            Span::styled(" ubah opsi UI ", app.theme.muted()),
            Span::styled(" Enter/Space ", app.theme.tab_inactive()),
            Span::styled(" toggle ", app.theme.muted()),
            Span::styled(" Esc/Ctrl+O/F2 ", app.theme.tab_inactive()),
            Span::styled(" keluar settings ", app.theme.muted()),
        ]))
        .style(app.theme.command());
        frame.render_widget(widget, area);
        return;
    }
    if matches!(app.focus, Focus::Shortcuts) {
        let widget = Paragraph::new(Line::from(vec![
            Span::styled(" Shortcuts ", app.theme.muted()),
            Span::styled(" Up/Down ", app.theme.tab_inactive()),
            Span::styled(" pilih ", app.theme.muted()),
            Span::styled(" Enter ", app.theme.tab_inactive()),
            Span::styled(" ubah key ", app.theme.muted()),
            Span::styled(" Delete ", app.theme.tab_inactive()),
            Span::styled(" reset ", app.theme.muted()),
            Span::styled(" Esc/F3 ", app.theme.tab_inactive()),
            Span::styled(" tutup ", app.theme.muted()),
        ]))
        .style(app.theme.command());
        frame.render_widget(widget, area);
        return;
    }

    if let Some(prompt) = &app.explorer_prompt {
        let title = match prompt.kind {
            super::ExplorerPromptKind::File => " New File ",
            super::ExplorerPromptKind::Folder => " New Folder ",
        };
        let prefix = format!("{} ", title);
        let hint = "  Enter simpan  Esc batal";
        let prefix_width = prefix.chars().count();
        let hint_width = hint.chars().count();
        let total_width = area.width.max(1) as usize;
        let input_width = total_width.saturating_sub(prefix_width + hint_width);
        let (visible_input, visible_cursor_col) =
            prompt_visible_input(&prompt.value, prompt.cursor_col, input_width.max(1));
        let line = Line::from(vec![
            Span::styled(prefix, app.theme.tab_active()),
            Span::styled(visible_input, app.theme.text()),
            Span::styled(hint, app.theme.muted()),
        ]);
        let widget = Paragraph::new(line).style(app.theme.command());
        frame.render_widget(widget, area);
        let max_x = area.width.saturating_sub(1) as usize;
        let cursor_x = (prefix_width + visible_cursor_col).min(max_x);
        frame.set_cursor(area.x + cursor_x as u16, area.y);
        return;
    }

    if matches!(app.focus, Focus::Terminal) && app.terminal_search_mode {
        let prefix = " Terminal Search ";
        let hint = "  Enter cari  Esc batal";
        let prefix_width = prefix.chars().count();
        let hint_width = hint.chars().count();
        let total_width = area.width.max(1) as usize;
        let input_width = total_width.saturating_sub(prefix_width + hint_width);
        let (visible_input, visible_cursor_col) =
            prompt_visible_input(&app.terminal_search_query, app.terminal_search_query.chars().count(), input_width.max(1));
        let line = Line::from(vec![
            Span::styled(prefix, app.theme.tab_active()),
            Span::styled(visible_input, app.theme.text()),
            Span::styled(hint, app.theme.muted()),
        ]);
        let widget = Paragraph::new(line).style(app.theme.command());
        frame.render_widget(widget, area);
        let max_x = area.width.saturating_sub(1) as usize;
        let cursor_x = (prefix_width + visible_cursor_col).min(max_x);
        frame.set_cursor(area.x + cursor_x as u16, area.y);
        return;
    }
    if app.workspace_search_mode {
        let prefix = " Workspace Search ";
        let hint = "  Enter cari  Up/Down pindah  Esc batal";
        let prefix_width = prefix.chars().count();
        let hint_width = hint.chars().count();
        let total_width = area.width.max(1) as usize;
        let input_width = total_width.saturating_sub(prefix_width + hint_width);
        let (visible_input, visible_cursor_col) = prompt_visible_input(
            &app.workspace_search_query,
            app.workspace_search_query.chars().count(),
            input_width.max(1),
        );
        let line = Line::from(vec![
            Span::styled(prefix, app.theme.tab_active()),
            Span::styled(visible_input, app.theme.text()),
            Span::styled(hint, app.theme.muted()),
        ]);
        let widget = Paragraph::new(line).style(app.theme.command());
        frame.render_widget(widget, area);
        let max_x = area.width.saturating_sub(1) as usize;
        let cursor_x = (prefix_width + visible_cursor_col).min(max_x);
        frame.set_cursor(area.x + cursor_x as u16, area.y);
        return;
    }

    let shortcuts = if area.width >= 160 {
        vec![
            ("Ctrl+E", "Explorer"),
            ("Ctrl+I", "Editor"),
            ("Ctrl+O/F2", "Settings"),
            ("F3", "Shortcuts"),
            ("Ctrl+T", "Terminal"),
            ("F6", "x1.AI"),
            ("Ctrl+Alt+T", "New Term Tab"),
            ("Ctrl+Shift+F", "Search"),
            ("Ctrl+Left/Right", "Term Tab"),
            ("Esc", "Cancel"),
            ("Ctrl+K/J", "Suggestion"),
            ("Ctrl+A C V X D", "Editor"),
            ("Alt+A C V X D", "Global"),
            ("Shift+Ins/Del", "Paste/Cut"),
            ("n / N", "New File/Folder"),
        ]
    } else if area.width >= 120 {
        vec![
            ("Ctrl+E", "Explorer"),
            ("Ctrl+I", "Editor"),
            ("Ctrl+O/F2", "Settings"),
            ("F3", "Shortcuts"),
            ("Ctrl+T", "Terminal"),
            ("F6", "x1.AI"),
            ("Ctrl+Alt+T", "New TTab"),
            ("Ctrl+Shift+F", "Search"),
            ("Ctrl+Left/Right", "TTab"),
            ("Ctrl+Tab", "Tabs"),
            ("Ctrl+K/J", "Suggestion"),
            ("n / N", "New"),
            ("Esc", "Cancel"),
        ]
    } else {
        vec![
            ("Ctrl+E", "Explorer"),
            ("Ctrl+I", "Editor"),
            ("Ctrl+T", "Term"),
            ("F3", "Keys"),
            ("F6", "x1.AI"),
            ("Ctrl+Alt+T", "NewTab"),
            ("Ctrl+Shift+F", "Find"),
            ("Ctrl+Left/Right", "Tab"),
            ("Ctrl+O/F2", "Set"),
            ("Ctrl+K/J", "Hint"),
            ("n/N", "New"),
            ("Esc", "Cancel"),
        ]
    };
    let max_cols = area.width.max(1) as usize;
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for (index, (key, label)) in shortcuts.iter().enumerate() {
        if index > 0 {
            append_span(
                &mut spans,
                &mut used,
                max_cols,
                "  ".to_string(),
                app.theme.command(),
            );
        }
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!(" {} ", key),
            app.theme.tab_inactive(),
        );
        append_span(
            &mut spans,
            &mut used,
            max_cols,
            format!(" {}", label),
            app.theme.muted(),
        );
    }
    let widget = Paragraph::new(Line::from(spans)).style(app.theme.command());
    frame.render_widget(widget, area);
}

fn focus_label(focus: Focus) -> &'static str {
    match focus {
        Focus::Explorer => "Explorer",
        Focus::AiChat => "x1.AI",
        Focus::Editor => "Editor",
        Focus::Settings => "Settings",
        Focus::Shortcuts => "Shortcuts",
        Focus::Terminal => "Terminal",
        Focus::About => "About",
    }
}

fn status_message_style(app: &App, message: &str) -> Style {
    if message.starts_with("Gagal") || message.starts_with("Error") || message.contains("gagal") {
        app.theme.warning().add_modifier(Modifier::BOLD)
    } else {
        app.theme.muted()
    }
}

fn fit_title(title: &str, panel_width: u16) -> String {
    ellipsize_end(title, panel_width.saturating_sub(4) as usize)
}

fn editor_gutter_width(row: usize) -> usize {
    (row + 1).to_string().len().max(4) + 3
}

fn expand_tabs(input: &str, tab_width: usize) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch == '\t' {
            for _ in 0..tab_width {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn clip_spans_horizontal(
    spans: Vec<Span<'static>>,
    mut skip_cols: usize,
    mut take_cols: usize,
) -> Vec<Span<'static>> {
    if take_cols == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for span in spans {
        if take_cols == 0 {
            break;
        }
        let text: Vec<char> = span.content.chars().collect();
        if text.is_empty() {
            continue;
        }
        if skip_cols >= text.len() {
            skip_cols -= text.len();
            continue;
        }
        let start = skip_cols;
        let end = (start + take_cols).min(text.len());
        let sliced: String = text[start..end].iter().collect();
        if !sliced.is_empty() {
            out.push(Span::styled(sliced, span.style));
            take_cols -= end - start;
        }
        skip_cols = 0;
    }
    out
}

fn append_separator(
    spans: &mut Vec<Span<'static>>,
    used: &mut usize,
    max_cols: usize,
    app: &App,
) {
    append_span(
        spans,
        used,
        max_cols,
        " | ".to_string(),
        app.theme.separator(),
    );
}

fn append_span(
    spans: &mut Vec<Span<'static>>,
    used: &mut usize,
    max_cols: usize,
    text: String,
    style: Style,
) {
    if *used >= max_cols || text.is_empty() {
        return;
    }
    let text_len = text.chars().count();
    if *used + text_len <= max_cols {
        spans.push(Span::styled(text, style));
        *used += text_len;
        return;
    }

    let remain = max_cols.saturating_sub(*used);
    if remain == 0 {
        return;
    }
    if remain <= 3 {
        spans.push(Span::styled(".".repeat(remain), style));
        *used += remain;
        return;
    }
    let keep = remain - 3;
    let mut clipped = String::with_capacity(remain);
    for ch in text.chars().take(keep) {
        clipped.push(ch);
    }
    clipped.push_str("...");
    spans.push(Span::styled(clipped, style));
    *used = max_cols;
}

fn prompt_visible_input(input: &str, cursor_col: usize, max_width: usize) -> (String, usize) {
    let max_width = max_width.max(1);
    let chars: Vec<char> = input.chars().collect();
    let total = chars.len();
    let cursor = cursor_col.min(total);
    if total <= max_width {
        return (input.to_string(), cursor);
    }
    let mut start = if cursor >= max_width {
        cursor - max_width + 1
    } else {
        0
    };
    if start + max_width > total {
        start = total.saturating_sub(max_width);
    }
    let end = (start + max_width).min(total);
    let visible: String = chars[start..end].iter().collect();
    let visible_cursor = cursor.saturating_sub(start);
    (visible, visible_cursor)
}

fn ellipsize_end(input: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max {
        return input.to_string();
    }
    if max <= 3 {
        return ".".repeat(max);
    }
    let keep = max - 3;
    let mut out = String::with_capacity(max);
    for ch in chars.into_iter().take(keep) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn ellipsize_middle(input: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max {
        return input.to_string();
    }
    if max <= 3 {
        return ".".repeat(max);
    }
    let keep = max - 3;
    let left = keep / 2;
    let right = keep - left;
    let mut out = String::with_capacity(max);
    for ch in chars.iter().take(left) {
        out.push(*ch);
    }
    out.push_str("...");
    for ch in chars.iter().skip(chars.len().saturating_sub(right)) {
        out.push(*ch);
    }
    out
}

fn explorer_connector(items: &[super::ExplorerItem], index: usize) -> String {
    let depth = items[index].depth;
    let mut prefix = String::new();

    for ancestor_depth in 0..depth {
        if has_next_sibling(items, index, ancestor_depth) {
            prefix.push_str("|   ");
        } else {
            prefix.push_str("    ");
        }
    }

    let branch = if has_next_sibling(items, index, depth) {
        "+-- "
    } else {
        "`-- "
    };
    prefix.push_str(branch);
    prefix
}

fn has_next_sibling(items: &[super::ExplorerItem], index: usize, depth: usize) -> bool {
    for item in items.iter().skip(index + 1) {
        if item.depth < depth {
            break;
        }
        if item.depth == depth {
            return true;
        }
    }
    false
}

fn in_range(range: Option<(usize, usize)>, row: usize) -> bool {
    range
        .map(|(start, end)| row >= start && row <= end)
        .unwrap_or(false)
}

//! Module: src/app/terminal/state.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::widget::{Cell, PseudoTerminal, Screen};

pub fn handle<S: Screen>(term: &PseudoTerminal<S>, area: Rect, buf: &mut Buffer) {
    let cols = area.width;
    let rows = area.height;
    let col_start = area.x;
    let row_start = area.y;
    // Exclusive bounds for buffer coordinates.
    let area_cols = area.x.saturating_add(area.width);
    let area_rows = area.y.saturating_add(area.height);
    let screen = term.screen();

    for row in 0..rows {
        for col in 0..cols {
            let buf_col = col + col_start;
            let buf_row = row + row_start;

            if buf_row >= area_rows || buf_col >= area_cols {
                continue;
            }

            if let Some(screen_cell) = screen.cell(row, col) {
                let cell = buf.get_mut(buf_col, buf_row);
                screen_cell.apply(cell);
            } else {
                // Defensive clear: saat resize cepat, beberapa sel bisa kosong;
                // paksa blank agar tidak mewarisi warna/simbol frame sebelumnya.
                let cell = buf.get_mut(buf_col, buf_row);
                cell.reset();
                cell.set_symbol(" ");
            }
        }
    }

    if !screen.hide_cursor() && term.cursor.show {
        let (c_row, c_col) = screen.cursor_position();
        if c_row < rows
            && c_col < cols
            && (c_row + row_start) < area_rows
            && (c_col + col_start) < area_cols
        {
            let c_cell = buf.get_mut(c_col + col_start, c_row + row_start);
            if let Some(cell) = screen.cell(c_row, c_col) {
                if cell.has_contents() {
                    let style = term.cursor.overlay_style;
                    c_cell.set_style(style);
                } else {
                    let symbol = &term.cursor.symbol;
                    let style = term.cursor.style;
                    c_cell.set_symbol(symbol);
                    c_cell.set_style(style);
                }
            }
        }
    }
}

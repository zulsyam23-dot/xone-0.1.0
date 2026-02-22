//! Module: src/app/terminal/vt100_imp.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use ratatui::style::{Modifier, Style};

use super::widget::{Cell, Screen};

impl Screen for vt100::Screen {
    type C = vt100::Cell;

    fn cell(&self, row: u16, col: u16) -> Option<&Self::C> {
        self.cell(row, col)
    }

    fn hide_cursor(&self) -> bool {
        self.hide_cursor()
    }

    fn cursor_position(&self) -> (u16, u16) {
        self.cursor_position()
    }
}

impl Cell for vt100::Cell {
    fn has_contents(&self) -> bool {
        self.has_contents()
    }

    fn apply(&self, cell: &mut ratatui::buffer::Cell) {
        fill_buf_cell(self, cell)
    }
}

fn fill_buf_cell(screen_cell: &vt100::Cell, buf_cell: &mut ratatui::buffer::Cell) {
    let fg: ratatui::style::Color = TermColor::from(screen_cell.fgcolor()).into();
    let bg: ratatui::style::Color = TermColor::from(screen_cell.bgcolor()).into();
    let symbol = if screen_cell.has_contents() {
        screen_cell.contents()
    } else {
        " "
    };
    buf_cell.set_symbol(symbol);
    let mut style = Style::reset();
    if screen_cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if screen_cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if screen_cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if screen_cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }
    buf_cell.set_style(style);
    buf_cell.set_fg(fg);
    buf_cell.set_bg(bg);
}

#[allow(dead_code)]
enum TermColor {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl From<vt100::Color> for TermColor {
    fn from(value: vt100::Color) -> Self {
        match value {
            vt100::Color::Default => Self::Reset,
            vt100::Color::Idx(i) => Self::Indexed(i),
            vt100::Color::Rgb(r, g, b) => Self::Rgb(r, g, b),
        }
    }
}

impl From<TermColor> for vt100::Color {
    fn from(value: TermColor) -> Self {
        match value {
            TermColor::Reset => Self::Default,
            TermColor::Black => Self::Idx(0),
            TermColor::Red => Self::Idx(1),
            TermColor::Green => Self::Idx(2),
            TermColor::Yellow => Self::Idx(3),
            TermColor::Blue => Self::Idx(4),
            TermColor::Magenta => Self::Idx(5),
            TermColor::Cyan => Self::Idx(6),
            TermColor::Gray => Self::Idx(7),
            TermColor::DarkGray => Self::Idx(8),
            TermColor::LightRed => Self::Idx(9),
            TermColor::LightGreen => Self::Idx(10),
            TermColor::LightYellow => Self::Idx(11),
            TermColor::LightBlue => Self::Idx(12),
            TermColor::LightMagenta => Self::Idx(13),
            TermColor::LightCyan => Self::Idx(14),
            TermColor::White => Self::Idx(15),
            TermColor::Rgb(r, g, b) => Self::Rgb(r, g, b),
            TermColor::Indexed(i) => Self::Idx(i),
        }
    }
}

impl From<TermColor> for ratatui::style::Color {
    fn from(value: TermColor) -> Self {
        match value {
            TermColor::Reset => Self::Reset,
            TermColor::Black => Self::Black,
            TermColor::Red => Self::Red,
            TermColor::Green => Self::Green,
            TermColor::Yellow => Self::Yellow,
            TermColor::Blue => Self::Blue,
            TermColor::Magenta => Self::Magenta,
            TermColor::Cyan => Self::Cyan,
            TermColor::Gray => Self::Gray,
            TermColor::DarkGray => Self::DarkGray,
            TermColor::LightRed => Self::LightRed,
            TermColor::LightGreen => Self::LightGreen,
            TermColor::LightYellow => Self::LightYellow,
            TermColor::LightBlue => Self::LightBlue,
            TermColor::LightMagenta => Self::LightMagenta,
            TermColor::LightCyan => Self::LightCyan,
            TermColor::White => Self::White,
            TermColor::Rgb(r, g, b) => Self::Rgb(r, g, b),
            TermColor::Indexed(i) => Self::Indexed(i),
        }
    }
}

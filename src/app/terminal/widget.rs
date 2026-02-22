//! Module: src/app/terminal/widget.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Clear, Widget};

use super::state;

pub trait Screen {
    type C: Cell;
    fn cell(&self, row: u16, col: u16) -> Option<&Self::C>;
    fn hide_cursor(&self) -> bool;
    fn cursor_position(&self) -> (u16, u16);
}

pub trait Cell {
    fn has_contents(&self) -> bool;
    fn apply(&self, cell: &mut ratatui::buffer::Cell);
}

pub struct PseudoTerminal<'a, S> {
    screen: &'a S,
    pub(crate) block: Option<Block<'a>>,
    style: Option<Style>,
    pub(crate) cursor: Cursor,
}

pub struct Cursor {
    pub(crate) show: bool,
    pub(crate) symbol: String,
    pub(crate) style: Style,
    pub(crate) overlay_style: Style,
}

#[allow(dead_code)]
impl Cursor {
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = symbol.into();
        self
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub const fn overlay_style(mut self, overlay_style: Style) -> Self {
        self.overlay_style = overlay_style;
        self
    }

    pub const fn visibility(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    pub fn show(&mut self) {
        self.show = true;
    }

    pub fn hide(&mut self) {
        self.show = false;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            show: true,
            symbol: "\u{2588}".into(),
            style: Style::default().fg(Color::Gray),
            overlay_style: Style::default().add_modifier(Modifier::REVERSED),
        }
    }
}

#[allow(dead_code)]
impl<'a, S: Screen> PseudoTerminal<'a, S> {
    pub fn new(screen: &'a S) -> Self {
        PseudoTerminal {
            screen,
            block: None,
            style: None,
            cursor: Cursor::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = cursor;
        self
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub const fn screen(&self) -> &S {
        self.screen
    }
}

impl<S: Screen> Widget for PseudoTerminal<'_, S> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let area = self.block.as_ref().map_or(area, |b| {
            let inner_area = b.inner(area);
            b.clone().render(area, buf);
            inner_area
        });
        if let Some(style) = self.style {
            for y in area.y..area.y.saturating_add(area.height) {
                for x in area.x..area.x.saturating_add(area.width) {
                    buf.get_mut(x, y).set_style(style);
                }
            }
        }
        state::handle(&self, area, buf);
    }
}

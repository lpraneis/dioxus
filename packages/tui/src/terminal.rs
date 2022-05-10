use crossterm::cursor::MoveTo;
use crossterm::style::{
    Attribute, Attributes, Print, SetAttributes, SetBackgroundColor, SetForegroundColor,
};
use crossterm::QueueableCommand;
use euclid::{Box2D, Point2D, Size2D};
use packed_simd::*;

use crate::style::{convert, RinkColor};
use crate::RenderingMode;

pub(crate) struct Terminal {
    grid: TerminalGrid,
    pub out: std::io::Stdout,
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            grid: TerminalGrid::default(),
            out: std::io::stdout(),
        }
    }
}

impl Terminal {
    pub fn resize(&mut self, width: u16, height: u16) {
        self.grid.resize(width, height);
    }

    pub fn size(&mut self) -> Size2D<u16, u16> {
        self.grid.size()
    }
}

pub(crate) struct TerminalGrid {
    state: Vec<Vec<PackedState>>,
}

impl Default for TerminalGrid {
    fn default() -> Self {
        let (width, height) = crossterm::terminal::size().unwrap_or_default();
        Self {
            state: vec![vec![PackedState::default(); width as usize]; height as usize],
        }
    }
}

impl TerminalGrid {
    fn get(&self, x: u16, y: u16) -> Option<&PackedState> {
        self.state.get(y as usize)?.get(x as usize)
    }

    fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut PackedState> {
        self.state.get_mut(y as usize)?.get_mut(x as usize)
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.state.resize(
            height as usize,
            vec![PackedState::default(); width as usize],
        );
        for row in &mut self.state {
            row.resize(width as usize, PackedState::default());
        }
    }

    pub fn size(&self) -> Size2D<u16, u16> {
        Size2D::new(self.state[0].len() as u16, self.state.len() as u16)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PackedState {
    symbol: String,
    // values caped to u8, but stored as u16 to prevent overflow
    bg_color: u16x4,
    fg_color: u16x4,
    attributes: Attributes,
}

impl PackedState {
    pub fn set_bg_color(&mut self, color: RinkColor) {
        blend(color, &mut self.bg_color);
        // allow text to bleed through the background
        if !self.symbol.is_empty() {
            blend(color, &mut self.fg_color);
        }
    }

    pub fn set_fg_color(&mut self, color: RinkColor) {
        blend(color, &mut self.fg_color);
    }

    pub fn set_symbol(&mut self, new: String) {
        self.symbol = new;
    }

    pub fn set_attribute(&mut self, attribute: Attribute) {
        self.attributes.set(attribute);
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.attributes = attributes;
    }

    fn write(&self, stdout: &mut std::io::Stdout, mode: RenderingMode) {
        stdout
            .queue(SetBackgroundColor(convert(mode, self.bg_color)))
            .unwrap();
        stdout
            .queue(SetForegroundColor(convert(mode, self.fg_color)))
            .unwrap();
        stdout.queue(SetAttributes(self.attributes)).unwrap();
        stdout.queue(Print(&self.symbol)).unwrap();
    }
}

#[inline(always)]
fn blend(color: RinkColor, on: &mut u16x4) {
    // (color * alpha + on * (255 - alpha)) / 255
    *on *= 255 - color.alpha;
    *on += color.rgb * color.alpha;
    *on /= 255;
}

pub(crate) struct RegionMask<'a> {
    terminal: &'a mut Terminal,
    dirty: &'a [Box2D<u16, u16>],
}

impl<'a> RegionMask<'a> {
    pub fn new(terminal: &'a mut Terminal, dirty: &'a [Box2D<u16, u16>]) -> Self {
        // clear the dirty region
        let size = terminal.grid.size();
        for x in 0..size.width {
            for y in 0..size.height {
                if dirty
                    .iter()
                    .any(|r| r.contains(Point2D::new(x as u16, y as u16)))
                {
                    terminal.out.queue(MoveTo(x, y)).unwrap();
                    PackedState::default().write(&mut terminal.out, RenderingMode::BaseColors);
                }
            }
        }

        Self { terminal, dirty }
    }

    pub fn get_mut(&mut self, loc: Point2D<u16, u16>) -> Option<&mut PackedState> {
        if self.dirty.iter().any(|r| r.contains(loc)) {
            self.terminal.grid.get_mut(loc.x, loc.y)
        } else {
            None
        }
    }

    pub fn commit(&mut self, mode: RenderingMode) {
        let size = self.terminal.grid.size();
        for x in 0..size.width {
            for y in 0..size.height {
                if self
                    .dirty
                    .iter()
                    .any(|r| r.contains(Point2D::new(x as u16, y as u16)))
                {
                    if let Some(cell) = self.terminal.grid.get(x, y) {
                        self.terminal.out.queue(MoveTo(x, y)).unwrap();
                        cell.write(&mut self.terminal.out, mode);
                    }
                }
            }
        }
    }

    pub fn intersects(&self, other: &Box2D<u16, u16>) -> bool {
        self.dirty.iter().any(|r| r.intersects(other))
    }
}

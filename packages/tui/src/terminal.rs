use std::io::Write;

use crossterm::cursor::MoveTo;
use crossterm::style::{
    Attribute, Attributes, Print, SetAttributes, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, QueueableCommand};
use euclid::{Box2D, Point2D, Size2D, Vector2D};
use packed_simd::*;

use crate::style::{convert, RinkColor};
use crate::RenderingMode;

pub(crate) struct Terminal {
    grid: TerminalGrid,
    pub out: std::io::Stdout,
}

impl Default for Terminal {
    fn default() -> Self {
        let mut out = std::io::stdout();
        execute!(out, Clear(ClearType::All)).unwrap();
        Self {
            grid: TerminalGrid::default(),
            out,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackedState {
    symbol: String,
    // values caped to u8, but stored as u16 to prevent overflow
    bg_color: u16x4,
    fg_color: u16x4,
    attributes: Attributes,
}

impl Default for PackedState {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            bg_color: Default::default(),
            fg_color: Default::default(),
            attributes: Default::default(),
        }
    }
}

impl PackedState {
    pub fn set_bg_color(&mut self, color: RinkColor) {
        blend(color, &mut self.bg_color);
        // allow text to bleed through the background
        if self.symbol.chars().any(|c| !c.is_whitespace()) {
            blend(color, &mut self.fg_color);
        }
    }

    pub fn set_fg_color(&mut self, color: RinkColor) {
        blend(color, &mut self.fg_color);
    }

    pub fn set_symbol(&mut self, new: String) {
        self.symbol = new;
    }

    #[allow(dead_code)]
    pub fn set_attribute(&mut self, attribute: Attribute) {
        self.attributes.set(attribute);
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.attributes = attributes;
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
    changed: TerminalGrid,
    offset: Vector2D<u16, u16>,
    dirty: &'a [Box2D<u16, u16>],
}

impl<'a> RegionMask<'a> {
    pub fn new(terminal: &'a mut Terminal, dirty: &'a [Box2D<u16, u16>]) -> Self {
        let mut changed = TerminalGrid::default();
        let min_x = dirty.iter().map(|r| r.min.x).min().unwrap_or(0);
        let max_x = dirty.iter().map(|r| r.max.x).max().unwrap_or(0);
        let min_y = dirty.iter().map(|r| r.min.y).min().unwrap_or(0);
        let max_y = dirty.iter().map(|r| r.max.y).max().unwrap_or(0);
        changed.resize(max_x - min_x, max_y - min_y);
        Self {
            terminal,
            changed,
            offset: Vector2D::new(min_x, min_y),
            dirty,
        }
    }

    pub fn get_mut(&mut self, mut loc: Point2D<u16, u16>) -> Option<&mut PackedState> {
        if self.dirty.iter().any(|r| contains_inclusive(r, loc)) {
            loc -= self.offset;
            self.changed.get_mut(loc.x, loc.y)
        } else {
            None
        }
    }

    pub fn commit(&mut self, mode: RenderingMode) {
        let size = self.terminal.grid.size();
        let mut brush = TerminalBrush::new(mode);
        for y in 0..size.height {
            for x in 0..size.width {
                if self
                    .dirty
                    .iter()
                    .any(|r| contains_inclusive(r, Point2D::new(x as u16, y as u16)))
                {
                    if let Some(cell) = self.terminal.grid.get_mut(x, y) {
                        let new = self
                            .changed
                            .get(x - self.offset.x, y - self.offset.y)
                            .unwrap();
                        if cell != new {
                            brush.paint(&mut self.terminal.out, &new, Point2D::new(x, y));
                            *cell = new.clone();
                        }
                    }
                }
            }
        }
        self.terminal.out.flush().unwrap();
    }

    pub fn intersects(&self, other: &Box2D<u16, u16>) -> bool {
        self.dirty.iter().any(|r| r.intersects(other))
    }
}

struct TerminalBrush {
    mode: RenderingMode,
    bg_color: u16x4,
    fg_color: u16x4,
    attributes: Attributes,
    run: String,
    run_bg_color: u16x4,
    run_fg_color: u16x4,
    run_attributes: Attributes,
    run_pos: Option<Point2D<u16, u16>>,
}

impl TerminalBrush {
    fn new(mode: RenderingMode) -> Self {
        Self {
            bg_color: Default::default(),
            fg_color: Default::default(),
            attributes: Default::default(),
            run_bg_color: Default::default(),
            run_fg_color: Default::default(),
            run_attributes: Default::default(),
            mode,
            run: String::new(),
            run_pos: None,
        }
    }

    fn paint(&mut self, out: &mut std::io::Stdout, cell: &PackedState, loc: Point2D<u16, u16>) {
        let is_after_run = self
            .run_pos
            .map(|p| p.x + self.run.len() as u16 == loc.x && p.y == loc.y)
            .unwrap_or(true);
        // if all attributes are the same, we can use the same run
        if is_after_run
            && self.bg_color == cell.bg_color
            && self.fg_color == cell.fg_color
            && self.attributes == cell.attributes
        {
            self.run += &cell.symbol;
        } else {
            if let Some(pos) = self.run_pos {
                out.queue(MoveTo(pos.x, pos.y)).unwrap();
                if self.run_bg_color != self.bg_color {
                    self.bg_color = self.run_bg_color;
                    out.queue(SetBackgroundColor(convert(self.mode, self.bg_color)))
                        .unwrap();
                }
                if self.run_fg_color != self.fg_color {
                    self.fg_color = self.run_fg_color;
                    out.queue(SetForegroundColor(convert(self.mode, self.fg_color)))
                        .unwrap();
                }
                if self.run_attributes != self.attributes {
                    self.attributes = self.run_attributes;
                    out.queue(SetAttributes(self.attributes)).unwrap();
                }
                out.queue(Print(&self.run)).unwrap();

                self.run = String::new();
                self.run_pos = None;
            }

            // start a new run
            self.run_bg_color = cell.bg_color;
            self.run_fg_color = cell.fg_color;
            self.run_attributes = cell.attributes;
            self.run_pos = Some(loc);
            self.run += &cell.symbol;
        }
    }
}

fn contains_inclusive(b: &Box2D<u16, u16>, p: Point2D<u16, u16>) -> bool {
    b.min.x <= p.x && p.x < b.max.x && b.min.y <= p.y && p.y < b.max.y
}

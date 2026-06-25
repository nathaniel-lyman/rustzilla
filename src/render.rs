use crate::sprite::{Sprite, Style};

/// One terminal cell: a character plus how it should be styled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Cell {
    fn blank() -> Cell {
        Cell {
            ch: ' ',
            style: Style::default(),
        }
    }
}

/// An in-memory grid of styled cells. Row-major, `width * height` cells.
pub struct Frame {
    pub width: u16,
    pub height: u16,
    cells: Vec<Cell>,
}

impl Frame {
    pub fn new(width: u16, height: u16) -> Frame {
        Frame {
            width,
            height,
            cells: vec![Cell::blank(); width as usize * height as usize],
        }
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// The character at a cell (style ignored).
    pub fn cell(&self, x: u16, y: u16) -> char {
        self.cells[self.idx(x, y)].ch
    }

    /// The full styled cell.
    pub fn styled(&self, x: u16, y: u16) -> Cell {
        self.cells[self.idx(x, y)]
    }

    /// Set a cell to a plain (unstyled) character.
    pub fn set(&mut self, x: u16, y: u16, c: char) {
        let i = self.idx(x, y);
        self.cells[i] = Cell {
            ch: c,
            style: Style::default(),
        };
    }

    /// Draw a sprite at integer cell (ox, oy), carrying the sprite's style.
    /// Spaces in the sprite are transparent. Cells outside the frame are clipped.
    pub fn draw_sprite(&mut self, ox: i32, oy: i32, sprite: &Sprite) {
        for (dy, row) in sprite.rendered_rows().iter().enumerate() {
            let y = oy + dy as i32;
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            for (dx, c) in row.chars().enumerate() {
                if c == ' ' {
                    continue;
                }
                let x = ox + dx as i32;
                if x < 0 || x >= self.width as i32 {
                    continue;
                }
                let i = self.idx(x as u16, y as u16);
                self.cells[i] = Cell {
                    ch: c,
                    style: sprite.style,
                };
            }
        }
    }

    /// Cells that differ from `prev`, as (x, y, new_cell).
    /// Assumes both frames share dimensions.
    pub fn diff(&self, prev: &Frame) -> Vec<(u16, u16, Cell)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.styled(x, y);
                if c != prev.styled(x, y) {
                    out.push((x, y, c));
                }
            }
        }
        out
    }
}

use crossterm::style::{
    Attribute, Color as CtColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};
use crossterm::{cursor, execute, queue, terminal};
use std::io::{Stdout, Write};

use crate::sprite::Color;

fn to_ct(color: Color) -> CtColor {
    match color {
        Color::Red => CtColor::Rgb {
            r: 0xD8,
            g: 0x4A,
            b: 0x4A,
        },
        Color::Yellow => CtColor::Rgb {
            r: 0xF2,
            g: 0xC6,
            b: 0x41,
        },
        Color::Green => CtColor::Rgb {
            r: 0x4F,
            g: 0xCF,
            b: 0x6F,
        },
        Color::Cyan => CtColor::Rgb {
            r: 0x49,
            g: 0xD0,
            b: 0xE0,
        },
        Color::Blue => CtColor::Rgb {
            r: 0x5B,
            g: 0x8C,
            b: 0xFF,
        },
        Color::White => CtColor::Rgb {
            r: 0xF2,
            g: 0xF2,
            b: 0xF2,
        },
        Color::Orange => CtColor::Rgb {
            r: 0xE8,
            g: 0x90,
            b: 0x2F,
        },
        Color::Black => CtColor::Rgb {
            r: 0x14,
            g: 0x14,
            b: 0x14,
        },
        Color::Grey => CtColor::Rgb {
            r: 0x8A,
            g: 0x93,
            b: 0xA0,
        },
        Color::Belly => CtColor::Rgb {
            r: 0xC9,
            g: 0xD0,
            b: 0xD8,
        },
    }
}

/// Enables raw mode + alternate screen on creation and unconditionally
/// restores the terminal in `Drop` — so cleanup runs on normal exit AND
/// during panic unwinding.
pub struct TerminalGuard {
    stdout: Stdout,
}

impl TerminalGuard {
    pub fn enter() -> std::io::Result<TerminalGuard> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(TerminalGuard { stdout })
    }

    pub fn stdout(&mut self) -> &mut Stdout {
        &mut self.stdout
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore; ignore errors during teardown.
        let _ = execute!(self.stdout, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

use crate::sprite::PixelSprite;

/// One terminal cell as two vertically-stacked pixels (None = water).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelCell {
    pub top: Option<Color>,
    pub bottom: Option<Color>,
}

impl PixelCell {
    fn blank() -> PixelCell {
        PixelCell {
            top: None,
            bottom: None,
        }
    }
}

/// An in-memory grid of pixel cells. Addressed in cells; the pixel canvas is
/// `width × height*2` (each cell holds a top and bottom pixel).
pub struct PixelFrame {
    pub width: u16,
    pub height: u16,
    cells: Vec<PixelCell>,
}

impl PixelFrame {
    pub fn new(width: u16, height: u16) -> PixelFrame {
        PixelFrame {
            width,
            height,
            cells: vec![PixelCell::blank(); width as usize * height as usize],
        }
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// Read the pixel at pixel coords (py: 0..height*2). Top half = even py.
    pub fn pixel(&self, px: u16, py: u16) -> Option<Color> {
        if px >= self.width || py >= self.height * 2 {
            return None;
        }
        let cell = self.cells[self.idx(px, py / 2)];
        if py.is_multiple_of(2) {
            cell.top
        } else {
            cell.bottom
        }
    }

    /// Set the pixel at pixel coords; out-of-range is clipped (no panic).
    pub fn set_pixel(&mut self, px: i32, py: i32, color: Color) {
        if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 * 2 {
            return;
        }
        let i = self.idx(px as u16, (py as u16) / 2);
        if py % 2 == 0 {
            self.cells[i].top = Some(color);
        } else {
            self.cells[i].bottom = Some(color);
        }
    }

    /// Draw a sprite with its top-left pixel at cell (ox, oy) -> pixel (ox, 2*oy).
    /// Transparent pixels are skipped; everything clips to the frame.
    pub fn draw_sprite(&mut self, ox: i32, oy: i32, sprite: &PixelSprite) {
        for (row, pixels) in sprite.rendered_rows().iter().enumerate() {
            for (col, px) in pixels.iter().enumerate() {
                if let Some(color) = px {
                    self.set_pixel(ox + col as i32, 2 * oy + row as i32, *color);
                }
            }
        }
    }

    /// Cells that differ from `prev`, as (x, y, new_cell).
    /// Assumes both frames share dimensions.
    pub fn diff(&self, prev: &PixelFrame) -> Vec<(u16, u16, PixelCell)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.idx(x, y);
                if self.cells[i] != prev.cells[i] {
                    out.push((x, y, self.cells[i]));
                }
            }
        }
        out
    }

    /// Every cell as a change — forces a full repaint (startup / post-resize).
    pub fn full_changes(&self) -> Vec<(u16, u16, PixelCell)> {
        let mut out = Vec::with_capacity(self.cells.len());
        for y in 0..self.height {
            for x in 0..self.width {
                out.push((x, y, self.cells[self.idx(x, y)]));
            }
        }
        out
    }
}

/// Water background as a crossterm truecolor (matches `raster::BG`).
fn water() -> CtColor {
    CtColor::Rgb {
        r: 0x0A,
        g: 0x14,
        b: 0x28,
    }
}

fn half_rgb(c: Option<Color>) -> CtColor {
    c.map(to_ct).unwrap_or_else(water)
}

/// Write changed cells as half-blocks: fg = top pixel, bg = bottom pixel,
/// transparent halves render as water. One reset at the end (every cell sets
/// both fg and bg, so styles never bleed between cells).
pub fn flush_pixels(out: &mut Stdout, changes: &[(u16, u16, PixelCell)]) -> std::io::Result<()> {
    for (x, y, cell) in changes {
        queue!(out, cursor::MoveTo(*x, *y))?;
        queue!(out, SetForegroundColor(half_rgb(cell.top)))?;
        queue!(out, SetBackgroundColor(half_rgb(cell.bottom)))?;
        queue!(out, Print('▀'))?;
    }
    queue!(out, ResetColor)?;
    out.flush()
}

/// Write only the cells that changed since the previous frame, applying each
/// cell's bold/color and resetting afterwards so styles never bleed.
pub fn flush_diff(out: &mut Stdout, changes: &[(u16, u16, Cell)]) -> std::io::Result<()> {
    for (x, y, cell) in changes {
        queue!(out, cursor::MoveTo(*x, *y))?;
        if cell.style.bold {
            queue!(out, SetAttribute(Attribute::Bold))?;
        }
        if let Some(color) = cell.style.color {
            queue!(out, SetForegroundColor(to_ct(color)))?;
        }
        queue!(out, Print(cell.ch))?;
        if cell.style.bold || cell.style.color.is_some() {
            queue!(out, SetAttribute(Attribute::Reset), ResetColor)?;
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::{Color, Sprite, Style};

    #[test]
    fn pixel_frame_starts_transparent() {
        let f = PixelFrame::new(2, 2);
        assert_eq!(f.pixel(0, 0), None);
        assert_eq!(f.pixel(1, 3), None); // bottom half of the bottom-right cell
    }

    #[test]
    fn set_pixel_targets_top_and_bottom_halves() {
        let mut f = PixelFrame::new(1, 1);
        f.set_pixel(0, 0, Color::Cyan); // even py -> top
        f.set_pixel(0, 1, Color::Red); // odd py -> bottom
        assert_eq!(f.pixel(0, 0), Some(Color::Cyan));
        assert_eq!(f.pixel(0, 1), Some(Color::Red));
    }

    #[test]
    fn set_pixel_clips_out_of_range() {
        let mut f = PixelFrame::new(1, 1);
        f.set_pixel(5, 5, Color::Cyan); // out of bounds: no panic, no write
        f.set_pixel(-1, -1, Color::Cyan);
        assert_eq!(f.pixel(0, 0), None);
    }

    #[test]
    fn draw_sprite_places_pixels_and_skips_transparent() {
        let mut f = PixelFrame::new(3, 2); // canvas 3x4 px
        let s = crate::sprite::PixelSprite::from_art(
            &[".b", "k."],
            &[('b', Color::Cyan), ('k', Color::Black)],
        );
        f.draw_sprite(0, 0, &s); // top-left at pixel (0,0)
        assert_eq!(f.pixel(0, 0), None); // '.' transparent
        assert_eq!(f.pixel(1, 0), Some(Color::Cyan)); // 'b' at row0,col1
        assert_eq!(f.pixel(0, 1), Some(Color::Black)); // 'k' at row1,col0 -> py=1 (bottom half)
    }

    #[test]
    fn draw_sprite_transparent_pixel_does_not_erase() {
        let mut f = PixelFrame::new(2, 1);
        f.set_pixel(0, 0, Color::Cyan);
        let s = crate::sprite::PixelSprite::from_art(&["."], &[]); // single transparent pixel
        f.draw_sprite(0, 0, &s);
        assert_eq!(f.pixel(0, 0), Some(Color::Cyan)); // untouched
    }

    #[test]
    fn diff_reports_a_cell_when_either_half_changes() {
        let prev = PixelFrame::new(1, 1);
        let mut next = PixelFrame::new(1, 1);
        next.set_pixel(0, 1, Color::Red); // only the bottom half changes
        let changes = next.diff(&prev);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].2.bottom, Some(Color::Red));
    }

    #[test]
    fn full_changes_returns_every_cell() {
        let f = PixelFrame::new(3, 2);
        assert_eq!(f.full_changes().len(), 6);
    }

    #[test]
    fn blank_frame_is_all_spaces() {
        let f = Frame::new(3, 2);
        assert_eq!(f.cell(0, 0), ' ');
        assert_eq!(f.cell(2, 1), ' ');
    }

    #[test]
    fn draw_sprite_places_chars_and_skips_spaces() {
        let mut f = Frame::new(10, 3);
        let s = Sprite::new(vec!["ab".into()]);
        f.draw_sprite(2, 1, &s);
        assert_eq!(f.cell(2, 1), 'a');
        assert_eq!(f.cell(3, 1), 'b');
        // A leading space in a sprite row must not erase background.
        let bg = Sprite::new(vec![" Z".into()]);
        f.draw_sprite(2, 1, &bg);
        assert_eq!(f.cell(2, 1), 'a'); // space did not overwrite
        assert_eq!(f.cell(3, 1), 'Z');
    }

    #[test]
    fn draw_clips_out_of_bounds() {
        let mut f = Frame::new(4, 2);
        let s = Sprite::new(vec!["XXXX".into()]);
        f.draw_sprite(2, 0, &s); // half off the right edge
        assert_eq!(f.cell(2, 0), 'X');
        assert_eq!(f.cell(3, 0), 'X');
        // No panic = clipping worked.
    }

    #[test]
    fn diff_reports_only_changed_cells() {
        let prev = Frame::new(3, 1);
        let mut next = Frame::new(3, 1);
        next.set(1, 0, 'o');
        let changes = next.diff(&prev);
        assert_eq!(
            changes,
            vec![(
                1,
                0,
                Cell {
                    ch: 'o',
                    style: Style::default()
                }
            )]
        );
    }

    #[test]
    fn draw_sprite_carries_style() {
        let mut f = Frame::new(8, 1);
        let s = Sprite::new(vec!["ab".into()]).bold().colored(Color::Red);
        f.draw_sprite(0, 0, &s);
        assert_eq!(f.styled(0, 0).ch, 'a');
        assert!(f.styled(0, 0).style.bold);
        assert_eq!(f.styled(0, 0).style.color, Some(Color::Red));
    }

    #[test]
    fn diff_detects_style_change_even_when_char_is_same() {
        // Same glyph, different style → still a change to repaint.
        let mut prev = Frame::new(2, 1);
        prev.draw_sprite(0, 0, &Sprite::new(vec!["x".into()]));
        let mut next = Frame::new(2, 1);
        next.draw_sprite(0, 0, &Sprite::new(vec!["x".into()]).bold());
        let changes = next.diff(&prev);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].2.style.bold);
    }
}

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
    Attribute, Color as CtColor, Print, ResetColor, SetAttribute, SetForegroundColor,
};
use crossterm::{cursor, execute, queue, terminal};
use std::io::{Stdout, Write};

use crate::sprite::Color;

fn to_ct(color: Color) -> CtColor {
    match color {
        Color::Red => CtColor::Red,
        Color::Yellow => CtColor::Yellow,
        Color::Green => CtColor::Green,
        Color::Cyan => CtColor::Cyan,
        Color::Blue => CtColor::Blue,
        Color::White => CtColor::White,
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

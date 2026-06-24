use crate::sprite::Sprite;

/// An in-memory grid of characters. Row-major, `width * height` cells.
pub struct Frame {
    pub width: u16,
    pub height: u16,
    cells: Vec<char>,
}

impl Frame {
    pub fn new(width: u16, height: u16) -> Frame {
        Frame {
            width,
            height,
            cells: vec![' '; width as usize * height as usize],
        }
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn cell(&self, x: u16, y: u16) -> char {
        self.cells[self.idx(x, y)]
    }

    pub fn set(&mut self, x: u16, y: u16, c: char) {
        let i = self.idx(x, y);
        self.cells[i] = c;
    }

    /// Draw a sprite at integer cell (ox, oy). Spaces in the sprite are
    /// transparent. Cells outside the frame are clipped.
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
                self.set(x as u16, y as u16, c);
            }
        }
    }

    /// Cells that differ from `prev`, as (x, y, new_char).
    /// Assumes both frames share dimensions.
    pub fn diff(&self, prev: &Frame) -> Vec<(u16, u16, char)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.cell(x, y);
                if c != prev.cell(x, y) {
                    out.push((x, y, c));
                }
            }
        }
        out
    }
}

use crossterm::{cursor, execute, queue, style::Print, terminal};
use std::io::{Stdout, Write};

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

/// Write only the cells that changed since the previous frame.
pub fn flush_diff(out: &mut Stdout, changes: &[(u16, u16, char)]) -> std::io::Result<()> {
    for (x, y, c) in changes {
        queue!(out, cursor::MoveTo(*x, *y), Print(c))?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Sprite;

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
        assert_eq!(changes, vec![(1, 0, 'o')]);
    }
}

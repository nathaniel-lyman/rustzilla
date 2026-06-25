#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

/// Foreground colors we use. Kept terminal-agnostic here; `render` maps these
/// to crossterm colors so the pure sprite layer needs no terminal dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Cyan,
    Blue,
    White,
    Orange,
    Black,
    Grey,
    Belly,
}

/// A grid of optional palette colors (None = transparent). Authored in pixels,
/// facing right and upright; `facing`/`flip_v` are applied at render time.
///
/// Invariant: `pixels` is rectangular — every row has the same length.
/// `from_art` guarantees this by right-padding ragged rows with `None`; any code
/// that mutates `pixels` directly must preserve the equal-length rows (the
/// `every_entity_sprite_is_well_formed` test is the safety net).
#[derive(Clone, Debug)]
pub struct PixelSprite {
    /// Rectangular grid of pixels (None = transparent); see the type invariant.
    pub pixels: Vec<Vec<Option<Color>>>,
    pub facing: Facing,
    pub flip_v: bool,
}

impl PixelSprite {
    /// Build from palette-indexed string rows. Any char not in `map` (including
    /// '.' and ' ') is transparent. Ragged rows are right-padded with None.
    /// A mistyped palette char becomes transparent rather than panicking; the
    /// `every_entity_sprite_is_well_formed` test is the safety net for blanks.
    pub fn from_art(rows: &[&str], map: &[(char, Color)]) -> PixelSprite {
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let lookup = |c: char| map.iter().find(|(k, _)| *k == c).map(|(_, v)| *v);
        let pixels = rows
            .iter()
            .map(|row| {
                let mut r: Vec<Option<Color>> = row.chars().map(lookup).collect();
                r.resize(width, None);
                r
            })
            .collect();
        PixelSprite {
            pixels,
            facing: Facing::Right,
            flip_v: false,
        }
    }

    pub fn width(&self) -> usize {
        self.pixels.first().map(|r| r.len()).unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.pixels.len()
    }

    /// Width in terminal cells (1 cell = 1 pixel wide).
    pub fn cell_w(&self) -> usize {
        self.width()
    }

    /// Height in terminal cells (1 cell = 2 pixels tall, rounded up).
    pub fn cell_h(&self) -> usize {
        self.height().div_ceil(2)
    }

    /// Pixel rows with facing (column mirror) then flip_v (row reversal) applied.
    pub fn rendered_rows(&self) -> Vec<Vec<Option<Color>>> {
        let mut rows: Vec<Vec<Option<Color>>> = match self.facing {
            Facing::Right => self.pixels.clone(),
            Facing::Left => self
                .pixels
                .iter()
                .map(|r| r.iter().rev().copied().collect())
                .collect(),
        };
        if self.flip_v {
            rows.reverse();
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_art_maps_palette_and_treats_unknown_as_transparent() {
        let s = PixelSprite::from_art(&[".b.", "bkb"], &[('b', Color::Cyan), ('k', Color::Black)]);
        assert_eq!(s.pixels[0], vec![None, Some(Color::Cyan), None]);
        assert_eq!(
            s.pixels[1],
            vec![Some(Color::Cyan), Some(Color::Black), Some(Color::Cyan)]
        );
    }

    #[test]
    fn from_art_pads_ragged_rows_to_max_width() {
        let s = PixelSprite::from_art(&["bb", "b"], &[('b', Color::Cyan)]);
        assert_eq!(s.width(), 2);
        assert_eq!(s.pixels[1], vec![Some(Color::Cyan), None]); // padded with None
    }

    #[test]
    fn rendered_rows_mirror_columns_when_facing_left() {
        let mut s = PixelSprite::from_art(&["bk."], &[('b', Color::Cyan), ('k', Color::Black)]);
        s.facing = Facing::Left;
        // Column-reversed: ".kb"
        assert_eq!(
            s.rendered_rows()[0],
            vec![None, Some(Color::Black), Some(Color::Cyan)]
        );
    }

    #[test]
    fn pixel_sprite_rendered_rows_flip_vertically() {
        let mut s = PixelSprite::from_art(&["b", "k"], &[('b', Color::Cyan), ('k', Color::Black)]);
        s.flip_v = true;
        assert_eq!(s.rendered_rows()[0], vec![Some(Color::Black)]);
        assert_eq!(s.rendered_rows()[1], vec![Some(Color::Cyan)]);
    }

    #[test]
    fn cell_dimensions_round_height_up() {
        let s = PixelSprite::from_art(&["b", "b", "b"], &[('b', Color::Cyan)]); // 1x3 px
        assert_eq!(s.cell_w(), 1);
        assert_eq!(s.cell_h(), 2); // ceil(3/2)
    }
}

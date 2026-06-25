#[derive(Clone, Copy, Debug, PartialEq)]
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

/// How a sprite is rendered: bold and/or colored. Default is plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub color: Option<Color>,
}

/// A small fixed grid of characters, drawn from `rows` (which are authored
/// facing right, upright). `facing`/`flip_v`/`style` are applied at render time.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub rows: Vec<String>,
    pub facing: Facing,
    pub flip_v: bool,
    pub style: Style,
}

impl Sprite {
    pub fn new(rows: Vec<String>) -> Sprite {
        Sprite {
            rows,
            facing: Facing::Right,
            flip_v: false,
            style: Style::default(),
        }
    }

    /// Builder: render this sprite bold.
    pub fn bold(mut self) -> Sprite {
        self.style.bold = true;
        self
    }

    /// Builder: render this sprite in `color`.
    pub fn colored(mut self, color: Color) -> Sprite {
        self.style.color = Some(color);
        self
    }

    pub fn width(&self) -> usize {
        self.rows
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Rows with facing/flip applied, ready to draw.
    pub fn rendered_rows(&self) -> Vec<String> {
        let mut rows: Vec<String> = match self.facing {
            Facing::Right => self.rows.clone(),
            Facing::Left => self.rows.iter().map(|r| mirror_row(r)).collect(),
        };
        if self.flip_v {
            rows.reverse();
        }
        rows
    }
}

/// Reverse a row and swap direction-sensitive glyphs so a left-facing
/// fish still looks like a fish.
fn mirror_row(row: &str) -> String {
    row.chars().rev().map(mirror_char).collect()
}

fn mirror_char(c: char) -> char {
    match c {
        '<' => '>',
        '>' => '<',
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '/' => '\\',
        '\\' => '/',
        '{' => '}',
        '}' => '{',
        other => other,
    }
}

/// A grid of optional palette colors (None = transparent). Authored in pixels,
/// facing right and upright; `facing`/`flip_v` are applied at render time.
#[derive(Clone, Debug)]
pub struct PixelSprite {
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

    #[test]
    fn dimensions_come_from_rows() {
        let s = Sprite::new(vec!["><(((°>".into(), " ~~".into()]);
        assert_eq!(s.width(), 7);
        assert_eq!(s.height(), 2);
    }

    #[test]
    fn rendered_rows_mirror_when_facing_left() {
        let mut s = Sprite::new(vec!["<°)))><".into()]);
        s.facing = Facing::Left;
        // Mirrored: reverse the row, then swap paired glyphs 1:1.
        // Length is always preserved (mirroring never adds/removes chars).
        assert_eq!(s.rendered_rows()[0], "><(((°>");
    }

    #[test]
    fn rendered_rows_flip_vertically() {
        let mut s = Sprite::new(vec!["top".into(), "bot".into()]);
        s.flip_v = true;
        assert_eq!(
            s.rendered_rows(),
            vec!["bot".to_string(), "top".to_string()]
        );
    }
}

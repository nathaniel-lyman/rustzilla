#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Facing {
    Left,
    Right,
}

/// A small fixed grid of characters, drawn from `rows` (which are authored
/// facing right, upright). `facing`/`flip_v` are applied at render time.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub rows: Vec<String>,
    pub facing: Facing,
    pub flip_v: bool,
}

impl Sprite {
    pub fn new(rows: Vec<String>) -> Sprite {
        Sprite {
            rows,
            facing: Facing::Right,
            flip_v: false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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

use crate::font8x8::FONT8X8_BASIC;
use crate::sprite::{Color, Style};

/// Tank background (deep blue-black), packed 0x00RRGGBB.
pub const BG: u32 = 0x000A_1428;

/// Map a logical color to a packed 0x00RRGGBB pixel. Symmetric with
/// `render::to_ct` (which maps the same enum to crossterm).
pub fn rgb(color: Color) -> u32 {
    match color {
        Color::Red => 0x00E0_4040,
        Color::Yellow => 0x00E0_C040,
        Color::Green => 0x0040_C040,
        Color::Cyan => 0x0040_C0C0,
        Color::Blue => 0x0040_60E0,
        Color::White => 0x00D0_D0D0,
    }
}

/// Brighter shade for bold sprites (e.g. the shark).
pub fn rgb_bold(color: Color) -> u32 {
    match color {
        Color::Red => 0x00FF_6060,
        Color::Yellow => 0x00FF_F060,
        Color::Green => 0x0060_FF60,
        Color::Cyan => 0x0070_FFFF,
        Color::Blue => 0x0070_90FF,
        Color::White => 0x00FF_FFFF,
    }
}

/// Resolve a cell's pixel color from its style. Uncolored cells get a soft
/// off-white so plain ASCII fish stay visible against `BG`.
pub fn pixel_color(style: Style) -> u32 {
    match (style.color, style.bold) {
        (Some(c), true) => rgb_bold(c),
        (Some(c), false) => rgb(c),
        (None, true) => 0x00FF_FFFF,
        (None, false) => 0x00C0_C8D0,
    }
}

/// The 8×8 bitmap rows for `c`. The sprites use a few non-ASCII art glyphs;
/// map those to ASCII look-alikes. Anything outside printable ASCII is blank.
pub fn glyph(c: char) -> [u8; 8] {
    let c = ascii_fallback(c);
    let code = c as u32;
    if (0x20..0x80).contains(&code) {
        FONT8X8_BASIC[code as usize]
    } else {
        [0; 8]
    }
}

fn ascii_fallback(c: char) -> char {
    match c {
        '°' => 'o', // fish eye
        '⊙' => 'O', // googly eye
        '•' => '*', // food pellet
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_uses_distinct_color() {
        // Bold must be visibly brighter so the bold shark reads as heavier.
        assert_ne!(rgb(Color::Red), rgb_bold(Color::Red));
    }

    #[test]
    fn space_and_unknown_glyphs_are_blank() {
        assert_eq!(glyph(' '), [0; 8]);
        assert_eq!(glyph('\u{2603}'), [0; 8]); // snowman: out of range → blank
    }

    #[test]
    fn art_glyphs_map_to_ascii_and_are_not_blank() {
        // The sprites use these three non-ASCII glyphs; each must render as a
        // visible ASCII look-alike rather than a blank hole.
        for c in ['#', '°', '•', '⊙'] {
            assert!(glyph(c).iter().any(|&r| r != 0), "{c:?} should not be blank");
        }
    }
}

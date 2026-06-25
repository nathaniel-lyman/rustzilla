use crate::font8x8::FONT8X8_BASIC;
use crate::render::Frame;
use crate::sprite::{Color, Style};

/// Tank background (deep blue-black), packed 0x00RRGGBB.
pub const BG: u32 = 0x000A_1428;

/// Map a logical color to a packed 0x00RRGGBB pixel. Symmetric with
/// `render::to_ct` (which maps the same enum to crossterm).
pub fn rgb(color: Color) -> u32 {
    match color {
        Color::Red => 0x00D8_4A4A,
        Color::Yellow => 0x00F2_C641,
        Color::Green => 0x004F_CF6F,
        Color::Cyan => 0x0049_D0E0,
        Color::Blue => 0x005B_8CFF,
        Color::White => 0x00F2_F2F2,
        Color::Orange => 0x00E8_902F,
        Color::Black => 0x0014_1414,
        Color::Grey => 0x008A_93A0,
        Color::Belly => 0x00C9_D0D8,
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
        Color::Orange => 0x00E8_902F,
        Color::Black => 0x0014_1414,
        Color::Grey => 0x008A_93A0,
        Color::Belly => 0x00C9_D0D8,
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

/// Window pixels → tank grid `(cols, rows)`. Floors to whole cells; clamps each
/// dimension to at least 1 so a transient 0-size window never yields a 0 grid
/// (mirrors `main.rs`'s `.max(1)` guard on terminal size).
pub fn grid_dims(px_w: usize, px_h: usize, cell_w: usize, cell_h: usize) -> (u16, u16) {
    let cols = (px_w / cell_w.max(1)).max(1) as u16;
    let rows = (px_h / cell_h.max(1)).max(1) as u16;
    (cols, rows)
}

/// Render `frame` into a `px_w × px_h` buffer of 0x00RRGGBB pixels at integer
/// `scale` (each cell is `8 * scale` square). Spaces are transparent (`BG`
/// shows through). Glyph pixels past the buffer edge are clipped — never a
/// panic — paralleling `Frame::draw_sprite`'s clipping.
pub fn blit(frame: &Frame, scale: u32, px_w: usize, px_h: usize) -> Vec<u32> {
    let mut buf = vec![BG; px_w * px_h];
    let s = scale.max(1) as usize;
    let cell = 8 * s;
    for cy in 0..frame.height {
        for cx in 0..frame.width {
            let styled = frame.styled(cx, cy);
            if styled.ch == ' ' {
                continue; // transparent: let BG show through
            }
            let rows = glyph(styled.ch);
            let color = pixel_color(styled.style);
            let ox = cx as usize * cell;
            let oy = cy as usize * cell;
            for (by, row) in rows.iter().enumerate() {
                for bx in 0..8 {
                    if row & (1 << bx) == 0 {
                        continue; // dark bit
                    }
                    // Scale this lit bit into an s×s block, clipped to the buffer.
                    for sy in 0..s {
                        let py = oy + by * s + sy;
                        if py >= px_h {
                            break;
                        }
                        for sx in 0..s {
                            let px = ox + bx * s + sx;
                            if px >= px_w {
                                break;
                            }
                            buf[py * px_w + px] = color;
                        }
                    }
                }
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Sprite;

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
            assert!(
                glyph(c).iter().any(|&r| r != 0),
                "{c:?} should not be blank"
            );
        }
    }

    #[test]
    fn every_entity_glyph_renders_non_blank() {
        // Regression guard: every glyph any entity actually draws must render
        // as something visible in the window. If a future sprite introduces a
        // new non-ASCII glyph, it would silently become a blank hole on screen
        // (the terminal would still show it) — this fails loudly instead,
        // pointing at the missing `ascii_fallback` entry.
        use crate::entity::{Entity, Food, Shark};
        use crate::fish::{Cool, Ducky, Googly, Upsidedown};
        use crate::geom::Vec2;
        let p = Vec2 { x: 0.0, y: 0.0 };
        let cast: Vec<Box<dyn Entity>> = vec![
            Box::new(Googly::new(p, 1.0)),
            Box::new(Cool::new(p, 1.0)),
            Box::new(Upsidedown::new(p, 1.0)),
            Box::new(Ducky::new(p, 1.0)),
            Box::new(Food::new(p)),
            Box::new(Shark::new(p, 1.0)),
        ];
        for e in &cast {
            for row in e.sprite().rendered_rows() {
                for c in row.chars().filter(|&c| c != ' ') {
                    assert!(
                        glyph(c).iter().any(|&r| r != 0),
                        "glyph for {c:?} (U+{:04X}) renders blank in the window; \
                         add an ascii_fallback entry for it",
                        c as u32
                    );
                }
            }
        }
    }

    #[test]
    fn grid_dims_floors_and_clamps() {
        assert_eq!(grid_dims(240, 120, 24, 24), (10, 5)); // exact fit
        assert_eq!(grid_dims(250, 130, 24, 24), (10, 5)); // remainder floored
        assert_eq!(grid_dims(10, 10, 24, 24), (1, 1)); // sub-cell window → 1×1
        assert_eq!(grid_dims(0, 0, 24, 24), (1, 1)); // degenerate 0-size → 1×1
    }

    #[test]
    fn blit_buffer_size_matches() {
        let f = Frame::new(3, 2);
        assert_eq!(blit(&f, 2, 48, 32).len(), 48 * 32);
    }

    #[test]
    fn blit_blank_frame_is_all_background() {
        let f = Frame::new(3, 2); // all spaces
        assert!(blit(&f, 3, 72, 48).iter().all(|&p| p == BG));
    }

    #[test]
    fn blit_draws_glyph_in_its_own_cell_and_color() {
        // One green glyph at cell (1,0); scale 1 → 8×8 cells.
        let mut f = Frame::new(3, 1);
        f.draw_sprite(1, 0, &Sprite::new(vec!["#".into()]).colored(Color::Green));
        let buf = blit(&f, 1, 24, 8);
        // Some pixel inside cell (1,0) (x in 8..16) is the glyph color...
        let lit = (0..8)
            .flat_map(|y| (8..16).map(move |x| (x, y)))
            .filter(|&(x, y)| buf[y * 24 + x] == rgb(Color::Green))
            .count();
        assert!(lit > 0, "glyph should light pixels in its own cell");
        // ...and the neighbouring cell (0,0) stays all background.
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(buf[y * 24 + x], BG);
            }
        }
    }

    #[test]
    fn blit_clips_glyph_past_the_edge() {
        // A glyph whose cell runs past the buffer must clip, not panic.
        let mut f = Frame::new(2, 2);
        f.draw_sprite(1, 1, &Sprite::new(vec!["#".into()]));
        let buf = blit(&f, 2, 24, 24); // 16px cells; cell (1,1) spills past 24
        assert_eq!(buf.len(), 24 * 24); // reached here = no panic
    }
}
